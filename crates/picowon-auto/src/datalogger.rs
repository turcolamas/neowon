//! Motor de DataLogger Automotriz y Registrador de Tendencias (Trend Engine).
//! Diseñado para registrar eventos lentos de minutos u horas (calentamiento, fallas térmicas,
//! caídas de tensión, deriva de frecuencia, variaciones de Duty Cycle y retardos de sincronismo)
//! capturados continuamente desde el osciloscopio en tiempo real.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StripChartMode {
    #[default]
    Voltage,
    Frequency,
    DutyCycle,
    Vrms,
    RiseTime,
    FallTime,
    Vpp,
    RiseDelay,
}

/// Magnitud física para agrupar y unificar escalas entre variables del mismo tipo
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricKind {
    Voltage,
    Frequency,
    Time,
    Percentage,
    Phase,
    SlewRate,
}

impl MetricKind {
    pub fn name(self) -> &'static str {
        match self {
            MetricKind::Voltage => "Tensión (V)",
            MetricKind::Frequency => "Frecuencia (Hz)",
            MetricKind::Time => "Tiempo (µs)",
            MetricKind::Percentage => "Porcentaje (%)",
            MetricKind::Phase => "Fase (°)",
            MetricKind::SlewRate => "Pendiente (V/µs)",
        }
    }
}

pub const N_DATALOG_METRICS: usize = 24;

/// Lista completa de las 24 métricas de medición del osciloscopio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatalogMetric {
    Vactual,       // 0: Tensión Instantánea CH1 (V)
    Period,        // 1: Período (µs)
    Frequency,     // 2: Frecuencia (Hz)
    RiseTime,      // 3: Tiempo de Subida (µs)
    FallTime,      // 4: Tiempo de Bajada (µs)
    PosPulseWidth, // 5: Ancho Pulso + (µs)
    NegPulseWidth, // 6: Ancho Pulso - (µs)
    PosDutyCycle,  // 7: Ciclo de Trabajo + (%)
    NegDutyCycle,  // 8: Ciclo de Trabajo - (%)
    Vpp,           // 9: Tensión Pico a Pico (V)
    Vmax,          // 10: Tensión Máxima (V)
    Vmin,          // 11: Tensión Mínima (V)
    Vamp,          // 12: Amplitud Vamp (V)
    Vtop,          // 13: Nivel Alto Vtop (V)
    Vbase,         // 14: Nivel Bajo Vbase (V)
    Overshoot,     // 15: Sobredisparo Overshoot (%)
    Preshoot,      // 16: Predisparo Preshoot (%)
    Vavg,          // 17: Tensión Media (V)
    Vrms,          // 18: Tensión Eficaz Real (V)
    CycleVrms,     // 19: Cycle Vrms (V)
    SlewRate,      // 20: Slew Rate (V/µs)
    RiseDelay,     // 21: Retardo Subida 1->2 (µs)
    FallDelay,     // 22: Retardo Bajada 1->2 (µs)
    Phase,         // 23: Desfase 1->2 (°)
}

impl DatalogMetric {
    pub const ALL: [DatalogMetric; N_DATALOG_METRICS] = [
        DatalogMetric::Vactual,
        DatalogMetric::Period,
        DatalogMetric::Frequency,
        DatalogMetric::RiseTime,
        DatalogMetric::FallTime,
        DatalogMetric::PosPulseWidth,
        DatalogMetric::NegPulseWidth,
        DatalogMetric::PosDutyCycle,
        DatalogMetric::NegDutyCycle,
        DatalogMetric::Vpp,
        DatalogMetric::Vmax,
        DatalogMetric::Vmin,
        DatalogMetric::Vamp,
        DatalogMetric::Vtop,
        DatalogMetric::Vbase,
        DatalogMetric::Overshoot,
        DatalogMetric::Preshoot,
        DatalogMetric::Vavg,
        DatalogMetric::Vrms,
        DatalogMetric::CycleVrms,
        DatalogMetric::SlewRate,
        DatalogMetric::RiseDelay,
        DatalogMetric::FallDelay,
        DatalogMetric::Phase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DatalogMetric::Vactual => "Vactual",
            DatalogMetric::Period => "Period",
            DatalogMetric::Frequency => "Frequency",
            DatalogMetric::RiseTime => "RiseTime",
            DatalogMetric::FallTime => "FallTime",
            DatalogMetric::PosPulseWidth => "+PulseWidth",
            DatalogMetric::NegPulseWidth => "-PulseWidth",
            DatalogMetric::PosDutyCycle => "+DutyCycle",
            DatalogMetric::NegDutyCycle => "-DutyCycle",
            DatalogMetric::Vpp => "Vpp",
            DatalogMetric::Vmax => "Vmax",
            DatalogMetric::Vmin => "Vmin",
            DatalogMetric::Vamp => "Vamp",
            DatalogMetric::Vtop => "Vtop",
            DatalogMetric::Vbase => "Vbase",
            DatalogMetric::Overshoot => "Overshoot",
            DatalogMetric::Preshoot => "Preshoot",
            DatalogMetric::Vavg => "Vavg",
            DatalogMetric::Vrms => "Vrms Real",
            DatalogMetric::CycleVrms => "Cycle Vrms",
            DatalogMetric::SlewRate => "SlewRate",
            DatalogMetric::RiseDelay => "RiseDelay 1->2",
            DatalogMetric::FallDelay => "FallDelay 1->2",
            DatalogMetric::Phase => "Phase 1->2",
        }
    }

    pub fn kind(self) -> MetricKind {
        match self {
            DatalogMetric::Vactual
            | DatalogMetric::Vpp
            | DatalogMetric::Vmax
            | DatalogMetric::Vmin
            | DatalogMetric::Vamp
            | DatalogMetric::Vtop
            | DatalogMetric::Vbase
            | DatalogMetric::Vavg
            | DatalogMetric::Vrms
            | DatalogMetric::CycleVrms => MetricKind::Voltage,
            DatalogMetric::Frequency => MetricKind::Frequency,
            DatalogMetric::Period
            | DatalogMetric::RiseTime
            | DatalogMetric::FallTime
            | DatalogMetric::PosPulseWidth
            | DatalogMetric::NegPulseWidth
            | DatalogMetric::RiseDelay
            | DatalogMetric::FallDelay => MetricKind::Time,
            DatalogMetric::PosDutyCycle
            | DatalogMetric::NegDutyCycle
            | DatalogMetric::Overshoot
            | DatalogMetric::Preshoot => MetricKind::Percentage,
            DatalogMetric::Phase => MetricKind::Phase,
            DatalogMetric::SlewRate => MetricKind::SlewRate,
        }
    }

    pub fn unit(self) -> &'static str {
        match self.kind() {
            MetricKind::Voltage => "V",
            MetricKind::Frequency => "Hz",
            MetricKind::Time => "s",
            MetricKind::Percentage => "%",
            MetricKind::Phase => "°",
            MetricKind::SlewRate => "V/µs",
        }
    }

    pub fn display_mult(self) -> f32 {
        match self.kind() {
            MetricKind::Time => 1e6, // mostrar tiempos en microsegundos (µs)
            _ => 1.0,
        }
    }

    pub fn display_unit(self) -> &'static str {
        match self.kind() {
            MetricKind::Time => "µs",
            _ => self.unit(),
        }
    }

    pub fn default_color(self) -> [u8; 4] {
        match self {
            DatalogMetric::Vactual => [80, 255, 120, 255],    // Verde lima
            DatalogMetric::Period => [180, 220, 255, 255],     // Celeste claro
            DatalogMetric::Frequency => [50, 220, 255, 255],   // Cian brillante
            DatalogMetric::RiseTime => [120, 255, 180, 255],   // Menta
            DatalogMetric::FallTime => [255, 120, 180, 255],   // Rosa
            DatalogMetric::PosPulseWidth => [140, 200, 255, 255],
            DatalogMetric::NegPulseWidth => [160, 160, 255, 255],
            DatalogMetric::PosDutyCycle => [255, 140, 40, 255],  // Naranja
            DatalogMetric::NegDutyCycle => [255, 180, 70, 255],
            DatalogMetric::Vpp => [200, 140, 255, 255],        // Violeta
            DatalogMetric::Vmax => [255, 80, 80, 255],         // Rojo
            DatalogMetric::Vmin => [80, 160, 255, 255],        // Azul
            DatalogMetric::Vamp => [255, 130, 130, 255],
            DatalogMetric::Vtop => [255, 100, 100, 255],
            DatalogMetric::Vbase => [100, 140, 255, 255],
            DatalogMetric::Overshoot => [255, 200, 80, 255],
            DatalogMetric::Preshoot => [255, 220, 120, 255],
            DatalogMetric::Vavg => [255, 220, 60, 255],        // Amarillo
            DatalogMetric::Vrms => [255, 180, 50, 255],        // Dorado
            DatalogMetric::CycleVrms => [240, 160, 40, 255],
            DatalogMetric::SlewRate => [220, 255, 100, 255],
            DatalogMetric::RiseDelay => [100, 220, 255, 255],
            DatalogMetric::FallDelay => [120, 180, 255, 255],
            DatalogMetric::Phase => [255, 160, 220, 255],
        }
    }

    pub fn default_range(self) -> (i32, i32) {
        match self.kind() {
            MetricKind::Voltage => (-1, 6),
            MetricKind::Frequency => (0, 2000),
            MetricKind::Percentage => (0, 100),
            MetricKind::Time => (0, 1000), // 1000 µs (1 ms)
            MetricKind::Phase => (-180, 180),
            MetricKind::SlewRate => (0, 50),
        }
    }
}

/// Escala vertical para una métrica con soporte semiautomático, valores enteros y color configurable
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetricScale {
    pub y_min: i32,
    pub y_max: i32,
    pub is_locked: bool,
    pub color: [u8; 4],
}

impl MetricScale {
    pub fn new(y_min: i32, y_max: i32, color: [u8; 4]) -> Self {
        Self {
            y_min,
            y_max,
            is_locked: false,
            color,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DvmReading {
    pub ch1_actual: f64,
    pub ch1_max: f64,
    pub ch1_min: f64,
    pub ch1_mean: f64,
    pub ch2_actual: f64,
    pub freq: f64,
    pub duty: f64,
    pub vrms: f64,
    pub rise: f64,
    pub delay: f64,
}

pub struct AutomotiveStripChart {
    pub is_recording: bool,
    pub start_time: f64,
    pub last_logged_time: f64,
    pub target_interval_s: f64,
    pub output_path: Option<PathBuf>,
    csv_writer: Option<BufWriter<File>>,

    // Modos / Métricas multiselección y escalas
    pub scales: [MetricScale; N_DATALOG_METRICS],
    pub active_metrics: [bool; N_DATALOG_METRICS],

    // Ventana flotante de configuración abierta
    pub config_window_open: bool,

    // Historial en memoria para graficación en tiempo real (un vector por cada métrica)
    pub max_display_points: usize,
    pub time_history: Vec<f64>,
    pub histories: Vec<Vec<f32>>,

    // Últimas muestras de mediciones recibidas del osciloscopio (Zero-Order Hold / Latch)
    pub latest_values: [f32; N_DATALOG_METRICS],
    pub latest_v2act: f32,

    pub has_received_data: bool,
}

impl Default for AutomotiveStripChart {
    fn default() -> Self {
        let mut scales = [MetricScale::new(0, 1, [255, 255, 255, 255]); N_DATALOG_METRICS];
        for i in 0..N_DATALOG_METRICS {
            let m = DatalogMetric::ALL[i];
            let (dmin, dmax) = m.default_range();
            scales[i] = MetricScale::new(dmin, dmax, m.default_color());
        }
        let mut active_metrics = [false; N_DATALOG_METRICS];
        // Por defecto, Vactual, Vmax, Vmin, Vavg activos
        active_metrics[0] = true;  // Vactual
        active_metrics[9] = false; // Vpp
        active_metrics[10] = true; // Vmax
        active_metrics[11] = true; // Vmin
        active_metrics[17] = true; // Vavg

        let mut histories = Vec::with_capacity(N_DATALOG_METRICS);
        for _ in 0..N_DATALOG_METRICS {
            histories.push(Vec::with_capacity(10000));
        }

        Self {
            is_recording: false,
            start_time: 0.0,
            last_logged_time: 0.0,
            target_interval_s: 0.1, // 10 Hz por defecto
            output_path: None,
            csv_writer: None,
            scales,
            active_metrics,
            config_window_open: false,
            max_display_points: 100_000,
            time_history: Vec::with_capacity(10000),
            histories,
            latest_values: [0.0; N_DATALOG_METRICS],
            latest_v2act: 0.0,
            has_received_data: false,
        }
    }
}

impl AutomotiveStripChart {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cantidad de puntos / registros acumulados en la sesión actual
    pub fn records_count(&self) -> usize {
        self.time_history.len()
    }

    /// Inicia una nueva sesión de grabación a disco y limpia los historiales en memoria.
    pub fn start_recording<P: AsRef<Path>>(&mut self, path: P, now: f64) -> std::io::Result<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);

        // Cabecera CSV multivariable completa con todos los parámetros
        writeln!(
            writer,
            "Tiempo_s,Vactual,Period_s,Frequency_Hz,RiseTime_s,FallTime_s,PosWidth_s,NegWidth_s,PosDuty_pct,NegDuty_pct,Vpp,Vmax,Vmin,Vamp,Vtop,Vbase,Overshoot_pct,Preshoot_pct,Vavg,Vrms,CycleVrms,SlewRate_V_us,RiseDelay_s,FallDelay_s,Phase_deg,CH2_Vactual"
        )?;

        self.output_path = Some(path.as_ref().to_path_buf());
        self.csv_writer = Some(writer);
        self.is_recording = true;
        self.start_time = now;
        self.last_logged_time = now;

        self.clear();

        Ok(())
    }

    /// Detiene la grabación y cierra el archivo CSV. Los datos en memoria se conservan para análisis.
    pub fn stop_recording(&mut self) {
        self.is_recording = false;
        if let Some(mut writer) = self.csv_writer.take() {
            let _ = writer.flush();
        }
    }

    /// Limpia todos los historiales en memoria.
    pub fn clear(&mut self) {
        self.time_history.clear();
        for h in &mut self.histories {
            h.clear();
        }
        self.start_time = 0.0;
    }

    /// Actualiza el último valor medido para una métrica específica (Zero-Order Hold / Latch)
    pub fn update_metric_val(&mut self, idx: usize, val: f32) {
        if idx < N_DATALOG_METRICS && val.is_finite() {
            self.latest_values[idx] = val;
            self.has_received_data = true;
        }
    }

    /// Tickeado periódico para consolidar y almacenar una muestra con retención de orden cero (ZOH).
    /// Si en el intervalo actual no hubo nueva captura de hardware, se repite el último valor válido
    /// en lugar de caer a 0.0, eliminando completamente los saltos y serruchos.
    pub fn tick(&mut self, now: f64) -> Option<DvmReading> {
        let dt = now - self.last_logged_time;
        if dt < self.target_interval_s {
            return None;
        }
        self.last_logged_time = now;
        if self.start_time == 0.0 {
            self.start_time = now;
        }
        let t_rel = now - self.start_time;

        if !self.has_received_data {
            return None;
        }

        if self.is_recording
            && let Some(ref mut writer) = self.csv_writer {
                let mut line = format!("{:.3}", t_rel);
                for i in 0..N_DATALOG_METRICS {
                    line.push_str(&format!(",{:.4}", self.latest_values[i]));
                }
                line.push_str(&format!(",{:.4}", self.latest_v2act));
                let _ = writeln!(writer, "{}", line);
            }

        // Historial en memoria para visualización gráfica continua
        self.time_history.push(t_rel);
        for i in 0..N_DATALOG_METRICS {
            self.histories[i].push(self.latest_values[i]);
        }

        if self.time_history.len() > self.max_display_points {
            self.time_history.remove(0);
            for h in &mut self.histories {
                if !h.is_empty() {
                    h.remove(0);
                }
            }
        }

        Some(DvmReading {
            ch1_actual: self.latest_values[0] as f64,
            ch1_max: self.latest_values[10] as f64,
            ch1_min: self.latest_values[11] as f64,
            ch1_mean: self.latest_values[17] as f64,
            ch2_actual: self.latest_v2act as f64,
            freq: self.latest_values[2] as f64,
            duty: self.latest_values[7] as f64,
            vrms: self.latest_values[18] as f64,
            rise: self.latest_values[3] as f64,
            delay: self.latest_values[21] as f64,
        })
    }

    /// Sugiere y aplica la misma escala unificada para todas las variables que midan la misma magnitud física
    /// garantizando valores enteros en los límites mínimo y máximo.
    pub fn suggest_unified_scales(&mut self) {
        let kinds = [
            MetricKind::Voltage,
            MetricKind::Frequency,
            MetricKind::Percentage,
            MetricKind::Time,
            MetricKind::Phase,
            MetricKind::SlewRate,
        ];

        for kind in kinds {
            let mut global_min = f32::MAX;
            let mut global_max = f32::MIN;
            let mut any_active = false;

            for (idx, &m) in DatalogMetric::ALL.iter().enumerate() {
                if self.active_metrics[idx] && m.kind() == kind {
                    any_active = true;
                    let mult = m.display_mult();
                    let cur = self.latest_values[idx] * mult;
                    if cur.is_finite() && self.has_received_data {
                        global_min = global_min.min(cur);
                        global_max = global_max.max(cur);
                    }
                    for &val in &self.histories[idx] {
                        let vm = val * mult;
                        global_min = global_min.min(vm);
                        global_max = global_max.max(vm);
                    }
                }
            }

            if !any_active {
                continue;
            }

            let (def_min, def_max) = match kind {
                MetricKind::Voltage => (-1, 6),
                MetricKind::Frequency => (0, 2000),
                MetricKind::Percentage => (0, 100),
                MetricKind::Time => (0, 1000),
                MetricKind::Phase => (-180, 180),
                MetricKind::SlewRate => (0, 50),
            };

            let (y_min_int, y_max_int) = if global_min < global_max && global_min != f32::MAX && global_max != f32::MIN {
                let span = (global_max - global_min).abs();
                let margin = (span * 0.15).max(1.0);
                let mn = (global_min - margin).floor() as i32;
                let mx = (global_max + margin).ceil() as i32;
                (mn, mx.max(mn + 1))
            } else if global_min.is_finite() && global_min != f32::MAX {
                let m = (global_min.abs() * 0.25).max(1.0).ceil() as i32;
                let c = global_min.round() as i32;
                (c - m, c + m)
            } else {
                (def_min, def_max)
            };

            for (idx, &m) in DatalogMetric::ALL.iter().enumerate() {
                if m.kind() == kind
                    && !self.scales[idx].is_locked {
                        self.scales[idx].y_min = y_min_int;
                        self.scales[idx].y_max = y_max_int;
                    }
            }
        }
    }

    /// Sugiere una escala individual entera para una métrica puntual
    pub fn suggest_single_metric_scale(&mut self, idx: usize) {
        if idx >= N_DATALOG_METRICS {
            return;
        }
        let m = DatalogMetric::ALL[idx];
        let mult = m.display_mult();
        let mut mn = f32::MAX;
        let mut mx = f32::MIN;
        for &v in &self.histories[idx] {
            let vm = v * mult;
            mn = mn.min(vm);
            mx = mx.max(vm);
        }
        let cur = self.latest_values[idx] * mult;
        if cur.is_finite() && self.has_received_data {
            mn = mn.min(cur);
            mx = mx.max(cur);
        }
        let (def_min, def_max) = m.default_range();

        let (y_min_int, y_max_int) = if mn < mx && mn != f32::MAX && mx != f32::MIN {
            let span = (mx - mn).abs();
            let margin = (span * 0.15).max(1.0);
            let n_min = (mn - margin).floor() as i32;
            let n_max = (mx + margin).ceil() as i32;
            (n_min, n_max.max(n_min + 1))
        } else if cur.is_finite() && cur != 0.0 {
            let m_span = (cur.abs() * 0.25).max(1.0).ceil() as i32;
            let c = cur.round() as i32;
            (c - m_span, c + m_span)
        } else {
            (def_min, def_max)
        };

        self.scales[idx].y_min = y_min_int;
        self.scales[idx].y_max = y_max_int;
    }
}
