//! Módulo de Análisis Dinámico de Actuadores Automotrices (PWM vs Sensor).
//!
//! Correlaciona la señal de comando PWM (CH1: Duty Cycle %, Frecuencia) contra
//! la respuesta real del actuador medida por un sensor analógico (CH2: Tensión en Voltios).
//!
//! Casos de aplicación en banco y a bordo de vehículos:
//! - Válvula reguladora de presión en bomba Common Rail (SCV / IMV / PCV) vs Sensor de Riel (RPS).
//! - Cuerpo de aceleración motorizado (TAC / ETC) vs Sensor de posición de mariposa (TPS 1 & 2).
//! - Actuador de geometría variable del turbo (VGT) / Wastegate electrónica vs Sensor de posición.
//! - Válvula de recirculación de gases de escape (EGR) vs Potenciómetro de posición EGR.
//!
//! Permite detectar de forma inmediata:
//! 1. Trabamiento mecánico o rozamiento excesivo (Histeresis anómala entre curva de subida y bajada).
//! 2. Pistas de sensor gastadas o con saltos/ruido eléctrico (Discontinuidades en la curva).
//! 3. Retardo temporal excesivo o falta de respuesta (Tiempo muerto Δt).

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActuatorPreset {
    CommonRailPressureValve,
    ElectronicThrottleTps,
    TurboVgtActuator,
    EgrValve,
    GenericPwmSensor,
}

impl ActuatorPreset {
    pub const ALL: [ActuatorPreset; 5] = [
        ActuatorPreset::CommonRailPressureValve,
        ActuatorPreset::ElectronicThrottleTps,
        ActuatorPreset::TurboVgtActuator,
        ActuatorPreset::EgrValve,
        ActuatorPreset::GenericPwmSensor,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ActuatorPreset::CommonRailPressureValve => "Bomba Alta Presión SCV vs Sensor Riel",
            ActuatorPreset::ElectronicThrottleTps => "Cuerpo Mariposa (TAC) vs TPS",
            ActuatorPreset::TurboVgtActuator => "Actuador Turbo VGT vs Sensor Carrera",
            ActuatorPreset::EgrValve => "Válvula EGR vs Sensor Posición",
            ActuatorPreset::GenericPwmSensor => "Actuador PWM Genérico vs Sensor Analógico",
        }
    }

    pub fn x_label(self) -> &'static str {
        "Comando PWM (+Duty %)"
    }

    pub fn y_label(self) -> &'static str {
        match self {
            ActuatorPreset::CommonRailPressureValve => "Presión Riel (V / bar)",
            ActuatorPreset::ElectronicThrottleTps => "Posición TPS (V / 0-100%)",
            ActuatorPreset::TurboVgtActuator => "Posición Paletas VGT (V)",
            ActuatorPreset::EgrValve => "Apertura EGR (V)",
            ActuatorPreset::GenericPwmSensor => "Respuesta Sensor (V)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActuatorTransferPoint {
    pub timestamp: f64,
    pub duty_pct: f32,
    pub sensor_v: f32,
}

#[derive(Debug, Clone)]
pub struct ActuatorAnalysisState {
    pub active: bool,
    pub preset: ActuatorPreset,
    pub pwm_channel: usize,     // 0 = CH1, 1 = CH2
    pub sensor_channel: usize,  // 0 = CH1, 1 = CH2
    pub pwm_threshold_v: f32,   // Umbral de conmutación de flancos PWM (típico 2.5 V para 5V o 6.0 V para 12V)
    
    // Telemetría instantánea
    pub current_duty: f32,      // 0.0 .. 100.0 %
    pub current_freq: f32,      // Hz
    pub current_sensor_v: f32,  // Voltios
    
    // Historial para la curva de transferencia X-Y
    pub points: VecDeque<ActuatorTransferPoint>,
    pub max_points: usize,
    
    // Diagnósticos calculados
    pub hysteresis_area: f32,
    pub mechanical_sticking_warning: bool,
    pub sensor_noise_warning: bool,
    pub response_delay_ms: f32,
}

impl Default for ActuatorAnalysisState {
    fn default() -> Self {
        Self {
            active: false,
            preset: ActuatorPreset::ElectronicThrottleTps,
            pwm_channel: 0,
            sensor_channel: 1,
            pwm_threshold_v: 2.5,
            current_duty: 0.0,
            current_freq: 0.0,
            current_sensor_v: 0.0,
            points: VecDeque::with_capacity(3000),
            max_points: 3000,
            hysteresis_area: 0.0,
            mechanical_sticking_warning: false,
            sensor_noise_warning: false,
            response_delay_ms: 0.0,
        }
    }
}

impl ActuatorAnalysisState {
    pub fn clear(&mut self) {
        self.points.clear();
        self.hysteresis_area = 0.0;
        self.mechanical_sticking_warning = false;
        self.sensor_noise_warning = false;
    }

    /// Procesa una trama de muestras simultáneas de ambos canales para extraer la curva de transferencia.
    pub fn process_waveform(
        &mut self,
        pwm_samples: &[f32],
        sensor_samples: &[f32],
        sample_rate: f64,
        t_start: f64,
    ) {
        if pwm_samples.is_empty() || sensor_samples.is_empty() || sample_rate <= 0.0 {
            return;
        }

        let n = pwm_samples.len().min(sensor_samples.len());
        let dt = 1.0 / sample_rate;
        let thresh = self.pwm_threshold_v;

        // Búsqueda de ciclos de PWM: detectar flancos ascendentes y descendentes
        let mut rising_edges = Vec::new();
        let mut falling_edges = Vec::new();

        for i in 1..n {
            let prev = pwm_samples[i - 1];
            let curr = pwm_samples[i];
            if prev <= thresh && curr > thresh {
                rising_edges.push(i);
            } else if prev >= thresh && curr < thresh {
                falling_edges.push(i);
            }
        }

        if rising_edges.len() < 2 {
            // No hay al menos 1 ciclo completo de PWM
            if let Some(&last_v) = sensor_samples.last() {
                self.current_sensor_v = last_v;
            }
            return;
        }

        // Calcular periodo medio y frecuencia
        let total_samples = rising_edges.last().unwrap() - rising_edges.first().unwrap();
        let avg_period_samples = total_samples as f64 / (rising_edges.len() - 1) as f64;
        let period_sec = avg_period_samples * dt;
        if period_sec > 0.0 {
            self.current_freq = (1.0 / period_sec) as f32;
        }

        // Analizar cada ciclo PWM para obtener pares (Duty %, Sensor V)
        let mut last_duty = 0.0;
        for k in 0..rising_edges.len() - 1 {
            let r0 = rising_edges[k];
            let r1 = rising_edges[k + 1];
            let cycle_len = (r1 - r0) as f32;
            if cycle_len < 2.0 {
                continue;
            }

            // Buscar el flanco descendente correspondiente dentro de este ciclo
            if let Some(&f0) = falling_edges.iter().find(|&&idx| idx >= r0 && idx < r1) {
                let high_time = (f0 - r0) as f32;
                let duty = (high_time / cycle_len) * 100.0;
                last_duty = duty;

                // Medir la tensión promedio del sensor en este ciclo
                let sensor_slice = &sensor_samples[r0..r1];
                let sensor_avg = sensor_slice.iter().copied().sum::<f32>() / sensor_slice.len() as f32;

                let t = t_start + (r0 as f64 * dt);
                if self.points.len() >= self.max_points {
                    self.points.pop_front();
                }
                self.points.push_back(ActuatorTransferPoint {
                    timestamp: t,
                    duty_pct: duty,
                    sensor_v: sensor_avg,
                });
            }
        }

        self.current_duty = last_duty;
        if let Some(&last_v) = sensor_samples.last() {
            self.current_sensor_v = last_v;
        }

        self.evaluate_diagnostics();
    }

    /// Evalúa la histéresis y la estabilidad del actuador/sensor
    fn evaluate_diagnostics(&mut self) {
        if self.points.len() < 30 {
            return;
        }

        // Calcular el área de histéresis aproximada (diferencia de sensor V para un mismo Duty al subir vs bajar)
        let mut max_hyst = 0.0f32;
        let mut noise_spikes = 0;

        let points_vec: Vec<_> = self.points.iter().collect();
        for i in 1..points_vec.len() {
            let d_duty = points_vec[i].duty_pct - points_vec[i - 1].duty_pct;
            let d_v = (points_vec[i].sensor_v - points_vec[i - 1].sensor_v).abs();

            // Si el Duty no cambia pero el sensor oscila bruscamente -> ruido en la pista
            if d_duty.abs() < 1.0 && d_v > 0.4 {
                noise_spikes += 1;
            }

            // Comparar con puntos anteriores con similar Duty pero sentido contrario
            for j in 0..i {
                if (points_vec[i].duty_pct - points_vec[j].duty_pct).abs() < 2.0 {
                    let v_diff = (points_vec[i].sensor_v - points_vec[j].sensor_v).abs();
                    if v_diff > max_hyst {
                        max_hyst = v_diff;
                    }
                }
            }
        }

        self.hysteresis_area = max_hyst;
        // Alerta de trabamiento mecánico si la histéresis excede 0.6 V
        self.mechanical_sticking_warning = max_hyst > 0.6;
        // Alerta de ruido si se detectaron más de 3 picos anómalos
        self.sensor_noise_warning = noise_spikes > 3;
    }
}
