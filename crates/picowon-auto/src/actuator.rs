//! Módulo de Análisis Dinámico de Actuadores Automotrices en Lazo Cerrado (PWM vs Sensor).
//!
//! Correlaciona la señal de comando PWM (`CH1`: Duty Cycle %, Frecuencia) contra
//! la respuesta real del actuador medida por un sensor analógico (`CH2`: Tensión en Voltios).
//!
//! Características de diagnóstico de histéresis X-Y:
//! 1. Separación de ramas de **Ascenso (+dDuty)** vs **Descenso (-dDuty)** para visualizar el lazo de histéresis real.
//! 2. Detección automática de trabamiento mecánico (histéresis ΔV entre apertura y cierre).
//! 3. Retardo temporal de respuesta (Δt ms) y detección de ruido/pistas gastadas en el sensor.
//! 4. Captura de **Curva Patrón Dorado** con banda de tolerancia ±V en vivo.

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
    /// +1 ascenso (+dDuty), -1 descenso (-dDuty), 0 estacionario.
    pub direction: i8,
}

#[derive(Debug, Clone)]
pub struct ActuatorAnalysisState {
    pub active: bool,
    pub preset: ActuatorPreset,
    pub pwm_channel: usize,     // 0 = CH1, 1 = CH2
    pub sensor_channel: usize,  // 0 = CH1, 1 = CH2
    pub pwm_threshold_v: f32,   // Umbral de conmutación de flancos PWM

    // Telemetría instantánea
    pub current_duty: f32,      // 0.0 .. 100.0 %
    pub current_freq: f32,      // Hz
    pub current_sensor_v: f32,  // Voltios

    // Historial para la curva de transferencia X-Y e histéresis
    pub points: VecDeque<ActuatorTransferPoint>,
    pub max_points: usize,

    // Comparación con Curva Patrón Dorado
    pub golden_points: Vec<ActuatorTransferPoint>,
    pub tolerance_v: f32,
    pub golden_deviation_max_v: f32,
    pub golden_pass: bool,

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
            golden_points: Vec::new(),
            tolerance_v: 0.35,
            golden_deviation_max_v: 0.0,
            golden_pass: true,
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
        self.response_delay_ms = 0.0;
        self.golden_deviation_max_v = 0.0;
        self.golden_pass = true;
    }

    pub fn capture_golden(&mut self) {
        self.golden_points = self.points.iter().cloned().collect();
        self.evaluate_diagnostics();
    }

    pub fn clear_golden(&mut self) {
        self.golden_points.clear();
        self.golden_deviation_max_v = 0.0;
        self.golden_pass = true;
    }

    /// Procesa una trama de muestras simultáneas en voltios de ambos canales.
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
            if let Some(&last_v) = sensor_samples.last() {
                self.current_sensor_v = last_v;
            }
            return;
        }

        let total_samples = rising_edges.last().unwrap() - rising_edges.first().unwrap();
        let avg_period_samples = total_samples as f64 / (rising_edges.len() - 1) as f64;
        let period_sec = avg_period_samples * dt;
        if period_sec > 0.0 {
            self.current_freq = (1.0 / period_sec) as f32;
        }

        let mut prev_duty = self.points.back().map_or(0.0, |p| p.duty_pct);
        let mut last_duty = prev_duty;
        for k in 0..rising_edges.len() - 1 {
            let r0 = rising_edges[k];
            let r1 = rising_edges[k + 1];
            let cycle_len = (r1 - r0) as f32;
            if cycle_len < 2.0 {
                continue;
            }

            if let Some(&f0) = falling_edges.iter().find(|&&idx| idx >= r0 && idx < r1) {
                let high_time = (f0 - r0) as f32;
                let duty = ((high_time / cycle_len) * 100.0).clamp(0.0, 100.0);
                let delta = duty - prev_duty;
                let direction = if delta > 0.4 {
                    1
                } else if delta < -0.4 {
                    -1
                } else {
                    self.points.back().map_or(0, |p| p.direction)
                };
                prev_duty = duty;
                last_duty = duty;

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
                    direction,
                });
            }
        }

        self.current_duty = last_duty;
        if let Some(&last_v) = sensor_samples.last() {
            self.current_sensor_v = last_v;
        }

        self.evaluate_diagnostics();
    }

    /// Evalúa la histéresis direccional (ascenso vs descenso), retardo y desviación respecto al Patrón Dorado.
    fn evaluate_diagnostics(&mut self) {
        if self.points.len() < 4 {
            return;
        }

        let mut max_dir_hyst = 0.0f32;
        let mut max_any_hyst = 0.0f32;
        let mut noise_spikes = 0;

        let points_vec: Vec<_> = self.points.iter().collect();
        for i in 1..points_vec.len() {
            let d_duty = points_vec[i].duty_pct - points_vec[i - 1].duty_pct;
            let d_v = (points_vec[i].sensor_v - points_vec[i - 1].sensor_v).abs();

            if d_duty.abs() < 1.0 && d_v > 0.4 {
                noise_spikes += 1;
            }

            for j in 0..i {
                if (points_vec[i].duty_pct - points_vec[j].duty_pct).abs() <= 2.5 {
                    let v_diff = (points_vec[i].sensor_v - points_vec[j].sensor_v).abs();
                    if points_vec[i].direction * points_vec[j].direction < 0 {
                        max_dir_hyst = max_dir_hyst.max(v_diff);
                    }
                    max_any_hyst = max_any_hyst.max(v_diff);
                }
            }
        }

        self.hysteresis_area = if max_dir_hyst > 0.0 { max_dir_hyst } else { max_any_hyst };
        self.mechanical_sticking_warning = self.hysteresis_area > 0.6;
        self.sensor_noise_warning = noise_spikes > 3;

        // Estimación de retardo entre pico de cambio de Duty y respuesta del sensor
        if points_vec.len() >= 6 {
            let max_duty_idx = (1..points_vec.len())
                .max_by(|&a, &b| {
                    let da = (points_vec[a].duty_pct - points_vec[a - 1].duty_pct).abs();
                    let db = (points_vec[b].duty_pct - points_vec[b - 1].duty_pct).abs();
                    da.total_cmp(&db)
                })
                .unwrap_or(1);
            let max_sens_idx = (max_duty_idx..points_vec.len())
                .max_by(|&a, &b| {
                    let va = (points_vec[a].sensor_v - points_vec[a - 1].sensor_v).abs();
                    let vb = (points_vec[b].sensor_v - points_vec[b - 1].sensor_v).abs();
                    va.total_cmp(&vb)
                })
                .unwrap_or(max_duty_idx);
            let dt_s = (points_vec[max_sens_idx].timestamp - points_vec[max_duty_idx].timestamp).max(0.0);
            self.response_delay_ms = (dt_s * 1000.0) as f32;
        }

        // Comparación con Curva Patrón Dorado (si está capturada)
        if !self.golden_points.is_empty() {
            let mut max_dev = 0.0f32;
            for p in &self.points {
                if let Some(nearest) = self
                    .golden_points
                    .iter()
                    .filter(|g| (g.duty_pct - p.duty_pct).abs() <= 5.0)
                    .min_by(|a, b| {
                        (a.duty_pct - p.duty_pct)
                            .abs()
                            .total_cmp(&(b.duty_pct - p.duty_pct).abs())
                    })
                {
                    max_dev = max_dev.max((p.sensor_v - nearest.sensor_v).abs());
                }
            }
            self.golden_deviation_max_v = max_dev;
            self.golden_pass = max_dev <= self.tolerance_v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actuator_hysteresis_direction_and_golden() {
        let mut st = ActuatorAnalysisState::default();
        st.points.push_back(ActuatorTransferPoint { timestamp: 0.00, duty_pct: 20.0, sensor_v: 1.0, direction: 1 });
        st.points.push_back(ActuatorTransferPoint { timestamp: 0.01, duty_pct: 50.0, sensor_v: 2.2, direction: 1 });
        st.points.push_back(ActuatorTransferPoint { timestamp: 0.02, duty_pct: 80.0, sensor_v: 4.0, direction: 1 });
        st.capture_golden();
        assert_eq!(st.golden_points.len(), 3);

        st.points.push_back(ActuatorTransferPoint { timestamp: 0.03, duty_pct: 50.0, sensor_v: 3.1, direction: -1 });
        st.evaluate_diagnostics();
        assert!(st.hysteresis_area >= 0.89, "hyst={}", st.hysteresis_area);
        assert!(st.mechanical_sticking_warning);
        assert!(!st.golden_pass);
    }
}
