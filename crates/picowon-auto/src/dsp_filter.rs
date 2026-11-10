//! Digital Signal Processing (DSP) and Signal Conditioning Module for PicoWon Automotive
//! Implements PicoScope 7-style non-destructive channel filtering:
//! 1. Software Low-Pass Filters with Edge Preservation and Zero Ringing:
//!    - Gaussian / Bessel: Zero overshoot, strictly monotonic, no ringing on square waves
//!    - Savitzky-Golay: Quadratic/cubic local polynomial, preserves peak heights (Vmax/Vmin) and slopes
//!    - Edge-Preserving Bilateral: Eliminates flat background noise without slowing down fast edges
//!    - Butterworth: Zero-phase forward-backward (filtfilt) for harmonic / sinusoidal signals
//! 2. Resolution Enhancement (ENOB boost up to 12 bits via moving-average)
//! 3. Signal Inversion (Invert)
//! 4. Probe Auto-Zero / Tare Offset calibration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LpfAlgorithm {
    /// Filtro Gaussiano / Bessel: Fase lineal, respuesta impulsional monotónica, cero ringing y cero overshoot en bordes.
    Gaussian,
    /// Filtro Savitzky-Golay (polinomial cuadrático/cúbico): Preserva rigurosamente la altura de picos (Vmax/Vmin), valles y pendientes.
    SavitzkyGolay,
    /// Filtro Bilateral Adaptativo (Edge-Preserving): Suprime ruido plano de fondo sin ralentizar ni distorsionar flancos rápidos.
    EdgePreserving,
    /// Filtro Butterworth IIR (filtfilt de fase cero): Máxima atenuación en banda eliminada para análisis senoidal / armónico.
    Butterworth,
}

impl LpfAlgorithm {
    pub fn name(self) -> &'static str {
        match self {
            LpfAlgorithm::Gaussian => "Gaussiano (Cero Ringing / Alta Fidelidad)",
            LpfAlgorithm::SavitzkyGolay => "Savitzky-Golay (Preserva Picos Vmax/Vmin)",
            LpfAlgorithm::EdgePreserving => "Bilateral Adaptativo (Preserva Flancos)",
            LpfAlgorithm::Butterworth => "Butterworth Fase-Cero (Armónicos)",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            LpfAlgorithm::Gaussian => "Gauss",
            LpfAlgorithm::SavitzkyGolay => "Sav-Gol",
            LpfAlgorithm::EdgePreserving => "Bilateral",
            LpfAlgorithm::Butterworth => "Butter",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            LpfAlgorithm::Gaussian => "Elimina ruido de alta frecuencia sin oscilaciones parásitas (ringing) ni sobreimpulsos artificiales.",
            LpfAlgorithm::SavitzkyGolay => "Ajuste polinomial local por mínimos cuadrados: limpia ruido blanco sin aplanar crestas ni distorsionar amplitudes extremas.",
            LpfAlgorithm::EdgePreserving => "Filtra agresivamente en zonas planas o mesetas sin redondear ni demorar los flancos de subida y bajada rápidos.",
            LpfAlgorithm::Butterworth => "Filtro IIR bidireccional de pendiente abrupta: ideal para formas de onda continuas o análisis de vibraciones.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChannelDspConfig {
    pub lpf_enabled: bool,
    pub lpf_cutoff_hz: f64,
    pub filter_algorithm: LpfAlgorithm,
    /// Umbral de preservación de bordes para el filtro bilateral (fracción del rango ADC, ej. 0.04 = 4%)
    pub edge_threshold_frac: f32,
    /// Resolution Enhancement in bits (0.0 = Off, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0)
    pub res_enhance_bits: f32,
    pub invert: bool,
    pub tare_offset_v: f64,
}

impl Default for ChannelDspConfig {
    fn default() -> Self {
        Self {
            lpf_enabled: false,
            lpf_cutoff_hz: 10_000.0, // 10 kHz default for automotive sensors
            filter_algorithm: LpfAlgorithm::Gaussian,
            edge_threshold_frac: 0.04,
            res_enhance_bits: 0.0,
            invert: false,
            tare_offset_v: 0.0,
        }
    }
}

/// Common low-pass filter cutoff presets for automotive diagnostics
pub const AUTOMOTIVE_LPF_PRESETS: [(f64, &str); 7] = [
    (1_000.0, "1 kHz (Lambda / Temp / MAP lento)"),
    (5_000.0, "5 kHz (Sensores Analógicos TPS/MAF)"),
    (10_000.0, "10 kHz (Sensores CKP / CMP inductivos)"),
    (20_000.0, "20 kHz (Audio / Presión en Cilindro)"),
    (50_000.0, "50 kHz (Corriente Inyectores / Primario)"),
    (100_000.0, "100 kHz (Redes CAN / LIN preliminar)"),
    (500_000.0, "500 kHz (Alta Frecuencia)"),
];

/// Common resolution enhancement presets (PicoScope style)
pub const RES_ENHANCE_PRESETS: [(f32, &str); 8] = [
    (0.0, "Off (8 bits nativo)"),
    (0.5, "+0.5 bits (8.5 bits - n=2)"),
    (1.0, "+1.0 bit (9.0 bits - n=4)"),
    (1.5, "+1.5 bits (9.5 bits - n=8)"),
    (2.0, "+2.0 bits (10.0 bits - n=16)"),
    (2.5, "+2.5 bits (10.5 bits - n=32)"),
    (3.0, "+3.0 bits (11.0 bits - n=64)"),
    (4.0, "+4.0 bits (12.0 bits - n=256)"),
];

/// Applies non-destructive DSP signal conditioning to a slice of ADC counts.
pub fn apply_channel_dsp(
    samples: &[f32],
    sample_rate: f64,
    scale_v_per_count: f64,
    config: &ChannelDspConfig,
) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    // Bypass check: if no DSP is enabled, return clone immediately
    if !config.lpf_enabled
        && config.res_enhance_bits <= 0.0
        && !config.invert
        && config.tare_offset_v.abs() < 1e-6
    {
        return samples.to_vec();
    }

    let mut out = samples.to_vec();

    // 1. Tare Offset & Invert
    let tare_counts = if scale_v_per_count.abs() > 1e-12 {
        (config.tare_offset_v / scale_v_per_count) as f32
    } else {
        0.0
    };

    if config.invert || tare_counts.abs() > 1e-5 {
        for s in out.iter_mut() {
            let val = *s - tare_counts;
            *s = if config.invert { -val } else { val };
        }
    }

    // 2. Filtro Pasa-Bajos con Algoritmo Seleccionado
    if config.lpf_enabled && config.lpf_cutoff_hz > 0.0 && sample_rate > 10.0 {
        let nyquist = sample_rate * 0.5;
        let fc = config.lpf_cutoff_hz.min(nyquist * 0.999);
        if fc < nyquist && out.len() >= 4 {
            out = match config.filter_algorithm {
                LpfAlgorithm::Gaussian => gaussian_low_pass(&out, sample_rate, fc),
                LpfAlgorithm::SavitzkyGolay => savitzky_golay_filter(&out, sample_rate, fc),
                LpfAlgorithm::EdgePreserving => bilateral_edge_preserving_filter(
                    &out,
                    sample_rate,
                    fc,
                    scale_v_per_count,
                    config.edge_threshold_frac,
                ),
                LpfAlgorithm::Butterworth => low_pass_filtfilt(&out, sample_rate, fc),
            };
        }
    }

    // 3. Resolution Enhancement (Moving Average ENOB boost)
    if config.res_enhance_bits > 0.0 && out.len() >= 2 {
        out = resolution_enhance(&out, config.res_enhance_bits);
    }

    out
}

/// Filtro Gaussiano de Fase Lineal (Linear-Phase Gaussian Filter)
/// Propiedad clave: CERO ringing, CERO overshoot. Es estrictamente monotónico en la respuesta escalón,
/// por lo que elimina el ruido de cuantización y espurias de alta frecuencia sin deformar flancos ni picos.
pub fn gaussian_low_pass(samples: &[f32], fs: f64, fc: f64) -> Vec<f32> {
    let len = samples.len();
    if len < 3 || fs <= 0.0 || fc <= 0.0 {
        return samples.to_vec();
    }

    // sigma en muestras = fs / (2 * pi * fc)
    let sigma = (fs / (2.0 * std::f64::consts::PI * fc)).max(0.4);
    let radius = (3.0 * sigma).round() as usize;
    let radius = radius.clamp(1, 48.min(len / 2));

    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut kernel = Vec::with_capacity(2 * radius + 1);
    let mut sum_weights = 0.0f64;

    for k in -(radius as isize)..=(radius as isize) {
        let w = (-((k * k) as f64) / two_sigma_sq).exp();
        kernel.push(w);
        sum_weights += w;
    }

    let inv_sum = (1.0 / sum_weights) as f32;
    let kernel_f32: Vec<f32> = kernel.iter().map(|&w| (w as f32) * inv_sum).collect();

    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let mut acc = 0.0f32;
        for (k_idx, k) in (-(radius as isize)..=(radius as isize)).enumerate() {
            let sample_idx = (i as isize + k).clamp(0, (len - 1) as isize) as usize;
            acc += samples[sample_idx] * kernel_f32[k_idx];
        }
        out.push(acc);
    }

    out
}

/// Filtro Savitzky-Golay (Ajuste polinomial local cuadrático/cúbico por mínimos cuadrados)
/// Propiedad clave: Limpia el ruido preservando el área bajo la curva y la amplitud exacta de picos y valles (Vmax, Vmin),
/// impidiendo que el promediado aplaste la señal como ocurre con filtros pasa-bajos convencionales.
pub fn savitzky_golay_filter(samples: &[f32], fs: f64, fc: f64) -> Vec<f32> {
    let len = samples.len();
    if len < 5 || fs <= 0.0 || fc <= 0.0 {
        return samples.to_vec();
    }

    let m_f = (fs / (4.0 * fc)).round();
    let m = (m_f as usize).clamp(2, 24.min((len - 1) / 2));

    let m_f64 = m as f64;
    let denom = (2.0 * m_f64 - 1.0) * (2.0 * m_f64 + 1.0) * (2.0 * m_f64 + 3.0);
    let const_term = 3.0 * m_f64 * m_f64 + 3.0 * m_f64 - 1.0;

    let mut kernel = Vec::with_capacity(2 * m + 1);
    for j in -(m as isize)..=(m as isize) {
        let j_sq = (j * j) as f64;
        let c = 3.0 * (const_term - 5.0 * j_sq) / denom;
        kernel.push(c as f32);
    }

    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let mut acc = 0.0f32;
        for (k_idx, k) in (-(m as isize)..=(m as isize)).enumerate() {
            let sample_idx = (i as isize + k).clamp(0, (len - 1) as isize) as usize;
            acc += samples[sample_idx] * kernel[k_idx];
        }
        out.push(acc);
    }

    out
}

/// Filtro Bilateral Adaptativo con Preservación de Flancos (Edge-Preserving Filter)
/// Propiedad clave: En zonas planas o mesetas (donde la señal varía lentamente), filtra con máxima intensidad
/// eliminando el ruido blanco de fondo. Cuando detecta un flanco rápido (salto abrupto de tensión), anula
/// la mezcla a través del borde, preservando el tiempo de subida y bajada intacto al 100%.
pub fn bilateral_edge_preserving_filter(
    samples: &[f32],
    fs: f64,
    fc: f64,
    _scale_v_per_count: f64,
    threshold_frac: f32,
) -> Vec<f32> {
    let len = samples.len();
    if len < 3 || fs <= 0.0 || fc <= 0.0 {
        return samples.to_vec();
    }

    let sigma_t = (fs / (2.0 * std::f64::consts::PI * fc)).max(0.5);
    let radius = (2.5 * sigma_t).round() as usize;
    let radius = radius.clamp(1, 32.min((len - 1) / 2));
    let two_sigma_t_sq = 2.0 * sigma_t * sigma_t;

    let span_counts = 256.0f32;
    let sigma_v = (span_counts * threshold_frac.clamp(0.01, 0.25)).max(1.0);
    let two_sigma_v_sq = 2.0 * (sigma_v as f64) * (sigma_v as f64);

    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let center_val = samples[i];
        let mut sum_weights = 0.0f64;
        let mut sum_val = 0.0f64;

        for k in -(radius as isize)..=(radius as isize) {
            let sample_idx = (i as isize + k).clamp(0, (len - 1) as isize) as usize;
            let val = samples[sample_idx];
            let dv = (val - center_val) as f64;

            let wt = (-((k * k) as f64) / two_sigma_t_sq).exp();
            let wv = (-(dv * dv) / two_sigma_v_sq).exp();
            let w = wt * wv;

            sum_weights += w;
            sum_val += w * (val as f64);
        }

        let res = if sum_weights > 1e-12 {
            (sum_val / sum_weights) as f32
        } else {
            center_val
        };
        out.push(res);
    }

    out
}

/// Zero-phase forward-backward 2nd-order Butterworth low-pass filter (filtfilt).
/// Guarantees zero phase shift (no horizontal delay on waveforms).
pub fn low_pass_filtfilt(samples: &[f32], fs: f64, fc: f64) -> Vec<f32> {
    let len = samples.len();
    if len < 4 || fs <= 0.0 || fc <= 0.0 {
        return samples.to_vec();
    }

    // Pre-warped digital cutoff
    let k = (std::f64::consts::PI * fc / fs).tan();
    let k2 = k * k;
    let sqrt2 = std::f64::consts::SQRT_2;
    let norm = 1.0 / (1.0 + sqrt2 * k + k2);

    let b0 = (k2 * norm) as f32;
    let b1 = (2.0 * k2 * norm) as f32;
    let b2 = b0;
    let a1 = (2.0 * (k2 - 1.0) * norm) as f32;
    let a2 = ((1.0 - sqrt2 * k + k2) * norm) as f32;

    // Forward pass
    let mut fwd = vec![0.0_f32; len];
    fwd[0] = samples[0];
    fwd[1] = samples[1];
    for i in 2..len {
        fwd[i] = b0 * samples[i] + b1 * samples[i - 1] + b2 * samples[i - 2]
            - a1 * fwd[i - 1]
            - a2 * fwd[i - 2];
    }

    // Backward pass for zero-phase correction
    let mut bwd = vec![0.0_f32; len];
    bwd[len - 1] = fwd[len - 1];
    bwd[len - 2] = fwd[len - 2];
    for i in (0..len - 2).rev() {
        bwd[i] = b0 * fwd[i] + b1 * fwd[i + 1] + b2 * fwd[i + 2]
            - a1 * bwd[i + 1]
            - a2 * bwd[i + 2];
    }

    bwd
}

/// Moving average filter corresponding to PicoScope Resolution Enhancement formula:
/// Number of averaged points N = 4^bits (clamped to available length).
pub fn resolution_enhance(samples: &[f32], bits: f32) -> Vec<f32> {
    let len = samples.len();
    if len < 2 || bits <= 0.0 {
        return samples.to_vec();
    }

    let n = (4.0_f32.powf(bits).round() as usize).clamp(2, (len / 2).max(2));
    let half = n / 2;

    let mut out = Vec::with_capacity(len);

    // Initial sum for first window
    let mut sum: f64 = 0.0;
    let initial_count = (half + 1).min(len);
    for i in 0..initial_count {
        sum += samples[i] as f64;
    }

    for i in 0..len {
        let left_edge = i.saturating_sub(half);
        let right_edge = (i + half).min(len - 1);
        let count = (right_edge - left_edge + 1) as f64;

        // Sliding window update
        if i > 0 {
            let prev_left = (i - 1).saturating_sub(half);
            let curr_left = i.saturating_sub(half);
            let prev_right = (i - 1 + half).min(len - 1);
            let curr_right = (i + half).min(len - 1);

            if curr_left > prev_left {
                sum -= samples[prev_left] as f64;
            }
            if curr_right > prev_right {
                sum += samples[curr_right] as f64;
            }
        }

        out.push((sum / count) as f32);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_when_disabled() {
        let raw = vec![1.0, 2.0, 3.0, 4.0];
        let cfg = ChannelDspConfig::default();
        let res = apply_channel_dsp(&raw, 100_000.0, 0.01, &cfg);
        assert_eq!(res, raw);
    }

    #[test]
    fn test_invert_and_tare() {
        let raw = vec![10.0, 20.0, 30.0];
        let mut cfg = ChannelDspConfig::default();
        cfg.invert = true;
        cfg.tare_offset_v = 0.10;
        let res = apply_channel_dsp(&raw, 100_000.0, 0.01, &cfg);
        assert!((res[0] - 0.0).abs() < 1e-4);
        assert!((res[1] - (-10.0)).abs() < 1e-4);
        assert!((res[2] - (-20.0)).abs() < 1e-4);
    }

    #[test]
    fn test_gaussian_low_pass_no_ringing() {
        let fs = 100_000.0;
        let fc = 5_000.0;
        let mut raw = Vec::new();
        for i in 0..1000 {
            let t = i as f64 / fs;
            let ripple = (2.0 * std::f64::consts::PI * 45_000.0 * t).sin() * 5.0;
            raw.push((10.0 + ripple) as f32);
        }

        let filtered = gaussian_low_pass(&raw, fs, fc);
        assert_eq!(filtered.len(), raw.len());

        let mid = &filtered[200..800];
        let max_val = mid.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let min_val = mid.iter().copied().fold(f32::INFINITY, f32::min);
        let ripple_p2p = max_val - min_val;
        assert!(ripple_p2p < 2.0, "Gaussiano debe atenuar fuertemente el rizado: {ripple_p2p}");
    }

    #[test]
    fn test_savitzky_golay_preserves_peak() {
        let fs = 100_000.0;
        let fc = 10_000.0;
        // Señal plana con un pulso central de 20.0
        let mut raw = vec![0.0f32; 200];
        for i in 90..110 {
            let p = (i - 90) as f32 / 20.0;
            raw[i] = 20.0 * (p * std::f32::consts::PI).sin();
        }

        let filtered = savitzky_golay_filter(&raw, fs, fc);
        let peak_raw = raw.iter().copied().fold(0.0f32, f32::max);
        let peak_filt = filtered.iter().copied().fold(0.0f32, f32::max);
        // Savitzky-Golay debe mantener >90% de la altura del pico
        assert!(peak_filt > peak_raw * 0.90, "Savitzky-Golay debe preservar el pico: {peak_filt} vs {peak_raw}");
    }

    #[test]
    fn test_edge_preserving_bilateral_filter() {
        let fs = 100_000.0;
        let fc = 10_000.0;
        // Onda cuadrada: 0 a 100 es 0.0 + ruido, 100 a 200 es 100.0 + ruido
        let mut raw = Vec::with_capacity(200);
        for i in 0..200 {
            let base = if i < 100 { 0.0f32 } else { 100.0f32 };
            let noise = if i % 2 == 0 { 2.0 } else { -2.0 };
            raw.push(base + noise);
        }

        let filtered = bilateral_edge_preserving_filter(&raw, fs, fc, 0.01, 0.05);
        // Ruido en la meseta 20..80 debe ser menor al ruido original (4.0 p-p)
        let mut max_plateau = f32::MIN;
        let mut min_plateau = f32::MAX;
        for &v in &filtered[20..80] {
            if v > max_plateau { max_plateau = v; }
            if v < min_plateau { min_plateau = v; }
        }
        let plateau_noise = max_plateau - min_plateau;
        assert!(plateau_noise < 2.0, "Ruido en meseta debe atenuarse a < 2.0, dio: {plateau_noise}");

        // El salto abrupto en 99 -> 101 debe seguir siendo rápido
        let step = filtered[105] - filtered[95];
        assert!(step > 90.0, "El flanco no debe destruirse: salto de {step}");
    }
}
