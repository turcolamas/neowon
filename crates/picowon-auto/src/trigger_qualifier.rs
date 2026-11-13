//! Calificador de Disparo Automotriz Avanzado (PicoScope 7 Cap. 4: Advanced Triggers).
//! Complementa el motor de disparo en hardware de la FPGA del VDS1022 con 5 modos de
//! calificación y realineación inteligente en el Host:
//! 1. **Interval / Missing-Tooth Trigger (Cap. 4.5)**: Detecta automáticamente el hueco de rueda
//!    fónica CKP (60-2, 36-1, 36-2) comparando cada período $T_k > \text{ratio} \cdot T_{k-1}$ y
//!    fija el Diente #1 en el punto de disparo horizontal.
//! 2. **Window Trigger (Cap. 4.3)**: Dispara cuando la señal entra o sale de una ventana dual
//!    `[lower_v, upper_v]` (ej. vigilancia de línea de 5.0V ± 0.25V).
//! 3. **Dropout Trigger (Cap. 4.7)**: Dispara cuando la señal permanece sin transiciones durante
//!    un tiempo mayor a `timeout_s` (fin de trama CAN/LIN o corte intermitente de sensor).
//! 4. **Runt Pulse Trigger (Cap. 4.6)**: Detecta pulsos "enanos" que cruzan el umbral inicial
//!    `threshold_low_v` pero regresan sin alcanzar jamás `threshold_high_v`.
//! 5. **Logic Pattern Trigger (Cap. 4.8)**: Combinación lógica booleana (`AND`, `OR`, `XOR`)
//!    entre el estado de nivel de CH1 y CH2.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AdvancedTriggerMode {
    /// Motor nativo de hardware FPGA (Flanco, Pulso, Pendiente, Video) con histéresis programable
    #[default]
    StandardHardware,
    /// Cap. 4.5: Hueco de rueda fónica CKP (60-2 / 36-1) o intervalo anormal entre pulsos
    IntervalMissingTooth,
    /// Cap. 4.3: Entrar o salir de ventana de tensión dual [Inferior .. Superior]
    Window,
    /// Cap. 4.7: Caída de señal / silencio en el bus mayor a un tiempo límite (Dropout)
    Dropout,
    /// Cap. 4.6: Pulso enano (Runt) que cruza el primer umbral pero no alcanza el segundo
    RuntPulse,
    /// Cap. 4.8: Disparo lógico combinacional entre CH1 y CH2 (AND / OR / XOR)
    LogicPattern,
}

impl AdvancedTriggerMode {
    pub const ALL: [AdvancedTriggerMode; 6] = [
        AdvancedTriggerMode::StandardHardware,
        AdvancedTriggerMode::IntervalMissingTooth,
        AdvancedTriggerMode::Window,
        AdvancedTriggerMode::Dropout,
        AdvancedTriggerMode::RuntPulse,
        AdvancedTriggerMode::LogicPattern,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AdvancedTriggerMode::StandardHardware => "⚡ Hardware FPGA (Edge/Pulse/Slope)",
            AdvancedTriggerMode::IntervalMissingTooth => "🦷 CKP Hueco / Intervalo (60-2 / 36-1)",
            AdvancedTriggerMode::Window => "🪟 Ventana Dual (Window Enter/Exit)",
            AdvancedTriggerMode::Dropout => "🔇 Silencio / Caída de Señal (Dropout)",
            AdvancedTriggerMode::RuntPulse => "📉 Pulso Enano / Débil (Runt Pulse)",
            AdvancedTriggerMode::LogicPattern => "🔣 Lógico Combinacional (CH1 + CH2)",
        }
    }

    pub fn short_badge(self) -> &'static str {
        match self {
            AdvancedTriggerMode::StandardHardware => "HW",
            AdvancedTriggerMode::IntervalMissingTooth => "CKP-GAP",
            AdvancedTriggerMode::Window => "WINDOW",
            AdvancedTriggerMode::Dropout => "DROPOUT",
            AdvancedTriggerMode::RuntPulse => "RUNT",
            AdvancedTriggerMode::LogicPattern => "LOGIC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogicCombinator {
    #[default]
    And,
    Or,
    Xor,
}

impl LogicCombinator {
    pub const ALL: [LogicCombinator; 3] = [
        LogicCombinator::And,
        LogicCombinator::Or,
        LogicCombinator::Xor,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LogicCombinator::And => "AND (Ambos cumplen)",
            LogicCombinator::Or => "OR (Al menos uno)",
            LogicCombinator::Xor => "XOR (Solo uno cumple)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerQualifierConfig {
    pub mode: AdvancedTriggerMode,
    /// Si está activo, descarta las tramas que no cumplan la condición avanzada (retiene la última válida)
    pub hold_until_qualified: bool,
    /// Si está activo, desplaza horizontalmente la trama capturada para alinear el evento al punto de disparo
    pub realign_to_trigger_pos: bool,

    // 1. Parámetros Interval / Missing Tooth (CKP 60-2 / 36-1)
    /// Factor multiplicador para detectar el diente faltante (ej. 1.75x del período normal del diente)
    pub ckp_gap_ratio: f32,
    /// Número total de dientes teóricos de la rueda fónica (60 para 60-2, 36 para 36-1) para cálculo de RPM
    pub ckp_wheel_teeth: u16,
    /// Flanco de cruce para medir dientes (true = subida, false = bajada)
    pub ckp_rising_edge: bool,

    // 2. Parámetros Window Trigger (Cap. 4.3)
    pub window_upper_v: f64,
    pub window_lower_v: f64,
    /// true = dispara al ENTRAR a la ventana; false = dispara al SALIR de la ventana
    pub window_enter: bool,

    // 3. Parámetros Dropout Trigger (Cap. 4.7)
    /// Tiempo mínimo de silencio/inactividad sin cruzar el umbral para disparar (segundos)
    pub dropout_timeout_s: f64,

    // 4. Parámetros Runt Pulse Trigger (Cap. 4.6)
    pub runt_low_v: f64,
    pub runt_high_v: f64,
    /// true = pulso positivo que no llega a runt_high_v; false = pulso negativo que no baja a runt_low_v
    pub runt_positive: bool,

    // 5. Parámetros Logic Pattern Trigger (Cap. 4.8)
    pub logic_ch1_level_v: f64,
    pub logic_ch1_above: bool,
    pub logic_ch2_level_v: f64,
    pub logic_ch2_above: bool,
    pub logic_op: LogicCombinator,
}

impl Default for TriggerQualifierConfig {
    fn default() -> Self {
        Self {
            mode: AdvancedTriggerMode::StandardHardware,
            hold_until_qualified: true,
            realign_to_trigger_pos: true,
            ckp_gap_ratio: 1.75,
            ckp_wheel_teeth: 60,
            ckp_rising_edge: true,
            window_upper_v: 5.25,
            window_lower_v: 4.75,
            window_enter: false, // Por defecto: disparar cuando la tensión SALE de la banda saludable
            dropout_timeout_s: 0.001, // 1.0 ms por defecto
            runt_low_v: 1.5,
            runt_high_v: 4.0,
            runt_positive: true,
            logic_ch1_level_v: 2.5,
            logic_ch1_above: true,
            logic_ch2_level_v: 2.5,
            logic_ch2_above: true,
            logic_op: LogicCombinator::And,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TriggerQualificationResult {
    pub matched: bool,
    pub event_sample_idx: Option<usize>,
    pub diagnostic_summary: String,
    pub estimated_rpm: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct TriggerQualifierRuntime {
    pub matched_frames: u64,
    pub rejected_frames: u64,
    pub last_result: TriggerQualificationResult,
}

/// Evalúa si una captura cumple la condición del Calificador de Disparo Avanzado.
/// `primary_volts` es la señal en Voltios del canal fuente de disparo (`source`),
/// y `ch1_volts` / `ch2_volts` se usan para el modo `LogicPattern`.
pub fn evaluate_trigger_qualifier(
    cfg: &TriggerQualifierConfig,
    primary_volts: &[f64],
    ch1_volts: Option<&[f64]>,
    ch2_volts: Option<&[f64]>,
    sample_rate: f64,
    base_level_v: f64,
    hysteresis_v: f64,
) -> TriggerQualificationResult {
    let n = primary_volts.len();
    if n < 4 || sample_rate <= 0.0 {
        return TriggerQualificationResult {
            matched: true,
            event_sample_idx: None,
            diagnostic_summary: String::new(),
            estimated_rpm: None,
        };
    }

    match cfg.mode {
        AdvancedTriggerMode::StandardHardware => TriggerQualificationResult {
            matched: true,
            event_sample_idx: None,
            diagnostic_summary: "Disparo por hardware FPGA activo".into(),
            estimated_rpm: None,
        },
        AdvancedTriggerMode::IntervalMissingTooth => qualify_missing_tooth(
            primary_volts,
            sample_rate,
            base_level_v,
            hysteresis_v,
            cfg.ckp_rising_edge,
            cfg.ckp_gap_ratio as f64,
            cfg.ckp_wheel_teeth,
        ),
        AdvancedTriggerMode::Window => qualify_window(
            primary_volts,
            cfg.window_lower_v.min(cfg.window_upper_v),
            cfg.window_upper_v.max(cfg.window_lower_v),
            cfg.window_enter,
        ),
        AdvancedTriggerMode::Dropout => qualify_dropout(
            primary_volts,
            sample_rate,
            base_level_v,
            hysteresis_v.max(0.05),
            cfg.dropout_timeout_s,
        ),
        AdvancedTriggerMode::RuntPulse => qualify_runt_pulse(
            primary_volts,
            cfg.runt_low_v.min(cfg.runt_high_v),
            cfg.runt_high_v.max(cfg.runt_low_v),
            cfg.runt_positive,
        ),
        AdvancedTriggerMode::LogicPattern => qualify_logic_pattern(
            ch1_volts.unwrap_or(primary_volts),
            ch2_volts.unwrap_or(primary_volts),
            cfg.logic_ch1_level_v,
            cfg.logic_ch1_above,
            cfg.logic_ch2_level_v,
            cfg.logic_ch2_above,
            cfg.logic_op,
        ),
    }
}

/// Detecta el hueco de referencia de una rueda fónica (60-2, 36-1, etc.) buscando un período
/// entre flancos consecutivos $T_k \ge \text{gap\_ratio} \cdot T_{k-1}$.
fn qualify_missing_tooth(
    volts: &[f64],
    sample_rate: f64,
    level_v: f64,
    hysteresis_v: f64,
    rising: bool,
    gap_ratio: f64,
    wheel_teeth: u16,
) -> TriggerQualificationResult {
    let hyst = hysteresis_v.abs().max(0.02);
    let mut crossings = Vec::new();

    if rising {
        let mut armed = volts[0] <= level_v - hyst;
        for (i, &v) in volts.iter().enumerate().skip(1) {
            if !armed && v <= level_v - hyst {
                armed = true;
            } else if armed && v >= level_v {
                crossings.push(i);
                armed = false;
            }
        }
    } else {
        let mut armed = volts[0] >= level_v + hyst;
        for (i, &v) in volts.iter().enumerate().skip(1) {
            if !armed && v >= level_v + hyst {
                armed = true;
            } else if armed && v <= level_v {
                crossings.push(i);
                armed = false;
            }
        }
    }

    if crossings.len() < 4 {
        return TriggerQualificationResult {
            matched: false,
            event_sample_idx: None,
            diagnostic_summary: "⏳ Esperando tren de pulsos CKP (mín. 4 dientes)...".into(),
            estimated_rpm: None,
        };
    }

    let mut periods: Vec<usize> = Vec::with_capacity(crossings.len() - 1);
    for w in crossings.windows(2) {
        periods.push(w[1].saturating_sub(w[0]));
    }

    // Buscar el hueco comparando cada período con el período previo (y mediano local)
    let ratio_thresh = gap_ratio.clamp(1.25, 4.0);
    for idx in 1..periods.len() {
        let prev_p = periods[idx - 1].max(1) as f64;
        let curr_p = periods[idx] as f64;
        let ratio = curr_p / prev_p;
        if ratio >= ratio_thresh {
            // El evento de interés es el Diente #1 (el flanco que cierra el hueco en crossings[idx + 1])
            let tooth1_sample = crossings[idx + 1];
            let tooth_period_s = prev_p / sample_rate;
            let teeth = (wheel_teeth.max(4)) as f64;
            let rpm = if tooth_period_s > 1e-9 {
                Some(60.0 / (tooth_period_s * teeth))
            } else {
                None
            };
            let rpm_str = rpm.map(|r| format!(" · {:.0} RPM", r)).unwrap_or_default();
            return TriggerQualificationResult {
                matched: true,
                event_sample_idx: Some(tooth1_sample),
                diagnostic_summary: format!(
                    "🦷 Hueco CKP detectado: {:.2}× diente ({:.2} ms{})",
                    ratio,
                    (curr_p / sample_rate) * 1e3,
                    rpm_str
                ),
                estimated_rpm: rpm,
            };
        }
    }

    TriggerQualificationResult {
        matched: false,
        event_sample_idx: None,
        diagnostic_summary: "⏳ Sin hueco CKP en la ventana actual".into(),
        estimated_rpm: None,
    }
}

/// Cap. 4.3: Disparo por Ventana Dual (Entrar o Salir de `[lower_v, upper_v]`).
fn qualify_window(
    volts: &[f64],
    lower_v: f64,
    upper_v: f64,
    enter: bool,
) -> TriggerQualificationResult {
    let inside = |v: f64| v >= lower_v && v <= upper_v;
    let mut was_inside = inside(volts[0]);

    for (i, &v) in volts.iter().enumerate().skip(1) {
        let now_inside = inside(v);
        if enter && !was_inside && now_inside {
            return TriggerQualificationResult {
                matched: true,
                event_sample_idx: Some(i),
                diagnostic_summary: format!(
                    "🪟 Señal ENTRÓ a ventana [{:.2}V .. {:.2}V]",
                    lower_v, upper_v
                ),
                estimated_rpm: None,
            };
        } else if !enter && was_inside && !now_inside {
            let dir = if v > upper_v { "SUPERIOR" } else { "INFERIOR" };
            return TriggerQualificationResult {
                matched: true,
                event_sample_idx: Some(i),
                diagnostic_summary: format!(
                    "🪟 Señal SALIÓ de ventana por límite {dir} ({:.2}V)",
                    v
                ),
                estimated_rpm: None,
            };
        }
        was_inside = now_inside;
    }

    TriggerQualificationResult {
        matched: false,
        event_sample_idx: None,
        diagnostic_summary: "⏳ Señal sin cruzar límites de ventana".into(),
        estimated_rpm: None,
    }
}

/// Cap. 4.7: Disparo por Caída de Señal / Silencio (Dropout).
/// Detecta cuando transcurre al menos `timeout_s` sin que la señal cruce el umbral `level_v`.
fn qualify_dropout(
    volts: &[f64],
    sample_rate: f64,
    level_v: f64,
    hysteresis_v: f64,
    timeout_s: f64,
) -> TriggerQualificationResult {
    let min_samples = ((timeout_s.max(1e-6)) * sample_rate).round().max(2.0) as usize;
    let mut last_activity_idx: Option<usize> = None;
    let mut prev_above = volts[0] >= level_v;

    for (i, &v) in volts.iter().enumerate().skip(1) {
        let now_above = if prev_above {
            v > level_v - hysteresis_v
        } else {
            v >= level_v + hysteresis_v
        };
        if now_above != prev_above {
            last_activity_idx = Some(i);
            prev_above = now_above;
        } else if let Some(last_idx) = last_activity_idx {
            let quiet_samples = i.saturating_sub(last_idx);
            if quiet_samples >= min_samples {
                return TriggerQualificationResult {
                    matched: true,
                    event_sample_idx: Some(i),
                    diagnostic_summary: format!(
                        "🔇 Dropout detectado tras {:.2} ms de silencio",
                        (quiet_samples as f64 / sample_rate) * 1e3
                    ),
                    estimated_rpm: None,
                };
            }
        }
    }

    TriggerQualificationResult {
        matched: false,
        event_sample_idx: None,
        diagnostic_summary: "⏳ Sin silencio/dropout mayor al tiempo configurado".into(),
        estimated_rpm: None,
    }
}

/// Cap. 4.6: Disparo por Pulso Enano (Runt Pulse).
/// Para pulso positivo: sube por encima de `low_v`, alcanza un máximo menor a `high_v`, y vuelve a caer bajo `low_v`.
fn qualify_runt_pulse(
    volts: &[f64],
    low_v: f64,
    high_v: f64,
    positive: bool,
) -> TriggerQualificationResult {
    if positive {
        let mut in_candidate = false;
        let mut disqualified_high = false;
        let mut peak_v = f64::NEG_INFINITY;

        for (i, &v) in volts.iter().enumerate() {
            if !in_candidate {
                if v > low_v && v < high_v {
                    in_candidate = true;
                    disqualified_high = false;
                    peak_v = v;
                }
            } else {
                if v >= high_v {
                    disqualified_high = true;
                }
                if v > peak_v {
                    peak_v = v;
                }
                if v <= low_v {
                    if !disqualified_high {
                        return TriggerQualificationResult {
                            matched: true,
                            event_sample_idx: Some(i),
                            diagnostic_summary: format!(
                                "📉 Pulso Runt (+) detectado: pico {:.2}V (entre {:.2}V y {:.2}V)",
                                peak_v, low_v, high_v
                            ),
                            estimated_rpm: None,
                        };
                    }
                    in_candidate = false;
                }
            }
        }
    } else {
        let mut in_candidate = false;
        let mut disqualified_low = false;
        let mut min_v = f64::INFINITY;

        for (i, &v) in volts.iter().enumerate() {
            if !in_candidate {
                if v < high_v && v > low_v {
                    in_candidate = true;
                    disqualified_low = false;
                    min_v = v;
                }
            } else {
                if v <= low_v {
                    disqualified_low = true;
                }
                if v < min_v {
                    min_v = v;
                }
                if v >= high_v {
                    if !disqualified_low {
                        return TriggerQualificationResult {
                            matched: true,
                            event_sample_idx: Some(i),
                            diagnostic_summary: format!(
                                "📉 Pulso Runt (-) detectado: valle {:.2}V (entre {:.2}V y {:.2}V)",
                                min_v, low_v, high_v
                            ),
                            estimated_rpm: None,
                        };
                    }
                    in_candidate = false;
                }
            }
        }
    }

    TriggerQualificationResult {
        matched: false,
        event_sample_idx: None,
        diagnostic_summary: "⏳ Sin pulsos enanos (Runt) detectados".into(),
        estimated_rpm: None,
    }
}

/// Cap. 4.8: Disparo Lógico Combinacional entre CH1 y CH2.
fn qualify_logic_pattern(
    ch1_volts: &[f64],
    ch2_volts: &[f64],
    ch1_level: f64,
    ch1_above: bool,
    ch2_level: f64,
    ch2_above: bool,
    op: LogicCombinator,
) -> TriggerQualificationResult {
    let len = ch1_volts.len().min(ch2_volts.len());
    if len < 2 {
        return TriggerQualificationResult::default();
    }

    let eval_at = |i: usize| -> bool {
        let c1 = if ch1_above {
            ch1_volts[i] >= ch1_level
        } else {
            ch1_volts[i] <= ch1_level
        };
        let c2 = if ch2_above {
            ch2_volts[i] >= ch2_level
        } else {
            ch2_volts[i] <= ch2_level
        };
        match op {
            LogicCombinator::And => c1 && c2,
            LogicCombinator::Or => c1 || c2,
            LogicCombinator::Xor => c1 ^ c2,
        }
    };

    let mut prev_match = eval_at(0);
    for i in 1..len {
        let now_match = eval_at(i);
        if !prev_match && now_match {
            return TriggerQualificationResult {
                matched: true,
                event_sample_idx: Some(i),
                diagnostic_summary: format!("🔣 Condición lógica {:?} cumplida en muestra #{i}", op),
                estimated_rpm: None,
            };
        }
        prev_match = now_match;
    }

    TriggerQualificationResult {
        matched: false,
        event_sample_idx: None,
        diagnostic_summary: "⏳ Esperando transición lógica CH1/CH2".into(),
        estimated_rpm: None,
    }
}

/// Desplaza una serie de muestras `data` para que el índice `event_idx` quede exactamente
/// en `target_idx`, rellenando los extremos con el valor de borde para evitar saltos artificiales.
pub fn shift_samples_to_trigger(data: &[f32], event_idx: usize, target_idx: usize) -> Vec<f32> {
    let len = data.len();
    if len == 0 || event_idx == target_idx {
        return data.to_vec();
    }
    let shift = event_idx as isize - target_idx as isize;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let src_i = (i as isize + shift).clamp(0, (len - 1) as isize) as usize;
        out.push(data[src_i]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_tooth_60_2_detection() {
        // Simular una rueda fónica 60-2 a 1000 RPM (1000 Hz de diente normal -> período = 20 muestras a 20 kHz)
        let fs = 20_000.0;
        let mut signal = Vec::new();
        for tooth in 0..70 {
            // En los dientes 30 y 31 simulamos el hueco 60-2 (silencio durante 2 dientes -> período triple = 60 muestras)
            if tooth == 30 || tooth == 31 {
                signal.extend(std::iter::repeat(-2.0).take(20));
            } else {
                signal.extend(std::iter::repeat(-2.0).take(10));
                signal.extend(std::iter::repeat(2.0).take(10));
            }
        }

        let mut cfg = TriggerQualifierConfig::default();
        cfg.mode = AdvancedTriggerMode::IntervalMissingTooth;
        cfg.ckp_gap_ratio = 1.75;
        cfg.ckp_wheel_teeth = 60;

        let res = evaluate_trigger_qualifier(&cfg, &signal, None, None, fs, 0.0, 0.2);
        assert!(res.matched, "Debe detectar el hueco 60-2: {:?}", res);
        assert!(res.event_sample_idx.is_some());
        let rpm = res.estimated_rpm.expect("Debe estimar RPM");
        assert!((rpm - 1000.0).abs() < 50.0, "RPM esperado ~1000, obtenido {rpm}");
    }

    #[test]
    fn test_window_exit_trigger() {
        let mut signal = vec![5.0; 100];
        signal[55] = 4.2; // Cae fuera de la ventana [4.75, 5.25]

        let mut cfg = TriggerQualifierConfig::default();
        cfg.mode = AdvancedTriggerMode::Window;
        cfg.window_lower_v = 4.75;
        cfg.window_upper_v = 5.25;
        cfg.window_enter = false;

        let res = evaluate_trigger_qualifier(&cfg, &signal, None, None, 10_000.0, 5.0, 0.1);
        assert!(res.matched);
        assert_eq!(res.event_sample_idx, Some(55));
    }

    #[test]
    fn test_runt_pulse_trigger() {
        let mut signal = vec![0.0; 100];
        // Pulso normal (alcanza 5V -> supera high_v = 4.0V)
        for i in 10..20 {
            signal[i] = 5.0;
        }
        // Pulso Runt enano (cruza low_v = 1.5V llegando a 2.8V, pero cae sin llegar a 4.0V)
        for i in 50..58 {
            signal[i] = 2.8;
        }

        let mut cfg = TriggerQualifierConfig::default();
        cfg.mode = AdvancedTriggerMode::RuntPulse;
        cfg.runt_low_v = 1.5;
        cfg.runt_high_v = 4.0;
        cfg.runt_positive = true;

        let res = evaluate_trigger_qualifier(&cfg, &signal, None, None, 100_000.0, 1.5, 0.1);
        assert!(res.matched);
        assert_eq!(res.event_sample_idx, Some(58));
    }

    #[test]
    fn test_dropout_trigger() {
        let fs = 10_000.0;
        let mut signal = Vec::new();
        // Ráfaga activa durante las primeras 40 muestras
        for i in 0..40 {
            signal.push(if i % 2 == 0 { 3.5 } else { 1.5 });
        }
        // Silencio de 50 muestras (5 ms > timeout de 2 ms)
        signal.extend(std::iter::repeat(2.5).take(50));

        let mut cfg = TriggerQualifierConfig::default();
        cfg.mode = AdvancedTriggerMode::Dropout;
        cfg.dropout_timeout_s = 0.002; // 2 ms = 20 muestras

        let res = evaluate_trigger_qualifier(&cfg, &signal, None, None, fs, 2.5, 0.2);
        assert!(res.matched);
        assert!(res.event_sample_idx.unwrap() >= 58);
    }

    #[test]
    fn test_logic_pattern_trigger() {
        let ch1 = vec![0.0, 0.0, 5.0, 5.0, 5.0];
        let ch2 = vec![0.0, 5.0, 0.0, 5.0, 5.0];

        let mut cfg = TriggerQualifierConfig::default();
        cfg.mode = AdvancedTriggerMode::LogicPattern;
        cfg.logic_ch1_level_v = 2.5;
        cfg.logic_ch1_above = true;
        cfg.logic_ch2_level_v = 2.5;
        cfg.logic_ch2_above = true;
        cfg.logic_op = LogicCombinator::And;

        let res = evaluate_trigger_qualifier(&cfg, &ch1, Some(&ch1), Some(&ch2), 10_000.0, 2.5, 0.1);
        assert!(res.matched);
        assert_eq!(res.event_sample_idx, Some(3));
    }
}
