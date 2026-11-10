//! Diagnóstico de Capa Física y Canal Diferencial CAN Bus (ISO 11898)
//!
//! En sistemas automotrices CAN Bus (CAN 2.0A/B e ISO 11898-2):
//! - CH1 se conecta a CAN_H (Nivel recesivo ~2.5 V, dominante ~3.5 V)
//! - CH2 se conecta a CAN_L (Nivel recesivo ~2.5 V, dominante ~1.5 V)
//! - Canal Matemático A - B genera V_diff = CAN_H - CAN_L:
//!     * Nivel recesivo nominal:  0.0 V (-0.5 V .. +0.5 V)
//!     * Nivel dominante nominal: 2.0 V (+1.5 V .. +3.0 V)
//!
//! Este módulo analiza en tiempo real las tensiones físicas para diagnosticar
//! la salud del bus, estado de resistencias de terminación (120 Ω) y ruidos comunes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanBusHealth {
    /// Niveles de tensión dentro de tolerancia ISO 11898
    Healthy,
    /// Amplitud dominante baja (<1.5 V): sobrecarga, corto parcial o terminación < 45 Ω
    LowAmplitude,
    /// Amplitud dominante excesiva (>3.0 V): falta resistencia de terminación (>100 Ω)
    HighAmplitude,
    /// Tensión de modo común desplazada (>0.7 V de 2.5 V): masa flotante o caída de tierra
    CommonModeOffset,
    /// Fuga o nivel recesivo anormal (>0.4 V diferencial): línea tirada a potencial indebido
    RecessiveLeak,
    /// Sondas invertidas (CH1 en CAN_L y CH2 en CAN_H)
    ProbesInverted,
    /// Sin tráfico o bus en reposo continuo (amplitud < 0.4 V)
    NoTraffic,
    /// Datos insuficientes para análisis
    InsufficientData,
}

impl CanBusHealth {
    pub fn badge_text(self) -> &'static str {
        match self {
            CanBusHealth::Healthy => "✅ Físico Saludable (ISO 11898 OK)",
            CanBusHealth::LowAmplitude => "⚠️ Tensión Baja (<1.5 V): Posible sobrecarga o corto",
            CanBusHealth::HighAmplitude => "⚠️ Tensión Alta (>3.0 V): Falta terminación 120 Ω",
            CanBusHealth::CommonModeOffset => "⚠️ Desbalance Modo Común: Falla de masa / tierra",
            CanBusHealth::RecessiveLeak => "⚠️ Fuga Nivel Recesivo: Tensión de polarización corrida",
            CanBusHealth::ProbesInverted => "🔄 Sondas Invertidas: Invierta CH1 (CAN_H) y CH2 (CAN_L)",
            CanBusHealth::NoTraffic => "💤 Bus en Reposo / Sin Actividad",
            CanBusHealth::InsufficientData => "⏳ Analizando señal CAN...",
        }
    }

    pub fn is_ok(self) -> bool {
        matches!(self, CanBusHealth::Healthy)
    }
}

/// Diagnóstico cuantitativo de la capa física CAN
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanPhysicalMetrics {
    pub v_h_rec: f32,
    pub v_h_dom: f32,
    pub v_l_rec: f32,
    pub v_l_dom: f32,
    pub v_diff_rec: f32,
    pub v_diff_dom: f32,
    pub v_common_mode: f32,
    pub delta_v: f32,
    pub health: CanBusHealth,
}

impl Default for CanPhysicalMetrics {
    fn default() -> Self {
        Self {
            v_h_rec: 2.5,
            v_h_dom: 3.5,
            v_l_rec: 2.5,
            v_l_dom: 1.5,
            v_diff_rec: 0.0,
            v_diff_dom: 2.0,
            v_common_mode: 2.5,
            delta_v: 2.0,
            health: CanBusHealth::InsufficientData,
        }
    }
}

/// Analiza las formas de onda crudas de CH1 (CAN_H) y CH2 (CAN_L)
pub fn analyze_can_physical_layer(can_h: &[f32], can_l: &[f32]) -> CanPhysicalMetrics {
    let n = can_h.len().min(can_l.len());
    if n < 64 {
        return CanPhysicalMetrics::default();
    }

    // 1. Calcular V_diff muestra a muestra
    let mut diff = Vec::with_capacity(n);
    let mut min_diff = f32::INFINITY;
    let mut max_diff = f32::NEG_INFINITY;

    for i in 0..n {
        let d = can_h[i] - can_l[i];
        diff.push(d);
        if d < min_diff { min_diff = d; }
        if d > max_diff { max_diff = d; }
    }

    // Verificar si las sondas están invertidas (predominio diferencial fuertemente negativo)
    if min_diff < -1.2 && max_diff < 0.6 {
        return CanPhysicalMetrics {
            v_diff_dom: min_diff,
            v_diff_rec: max_diff,
            health: CanBusHealth::ProbesInverted,
            ..Default::default()
        };
    }

    let span = max_diff - min_diff;
    if span < 0.45 {
        // Amplitud muy baja: o no hay pulsos dominantes o la línea no está activa
        let v_h_avg: f32 = can_h.iter().take(n).sum::<f32>() / n as f32;
        let v_l_avg: f32 = can_l.iter().take(n).sum::<f32>() / n as f32;
        return CanPhysicalMetrics {
            v_h_rec: v_h_avg,
            v_h_dom: v_h_avg,
            v_l_rec: v_l_avg,
            v_l_dom: v_l_avg,
            v_diff_rec: v_h_avg - v_l_avg,
            v_diff_dom: v_h_avg - v_l_avg,
            v_common_mode: (v_h_avg + v_l_avg) * 0.5,
            delta_v: span,
            health: CanBusHealth::NoTraffic,
        };
    }

    // Umbral de separación entre bit recesivo (nominal 0 V) y bit dominante (nominal 2 V)
    let threshold = min_diff + span * 0.45;

    let mut dom_h = Vec::new();
    let mut dom_l = Vec::new();
    let mut rec_h = Vec::new();
    let mut rec_l = Vec::new();

    for i in 0..n {
        if diff[i] > threshold {
            dom_h.push(can_h[i]);
            dom_l.push(can_l[i]);
        } else {
            rec_h.push(can_h[i]);
            rec_l.push(can_l[i]);
        }
    }

    if dom_h.len() < 5 || rec_h.len() < 5 {
        return CanPhysicalMetrics {
            delta_v: span,
            health: CanBusHealth::InsufficientData,
            ..Default::default()
        };
    }

    // Mediana o promedio recortado para descartar sobrepicos (ringing) de conmutación
    let v_h_dom = median_or_avg(&mut dom_h);
    let v_l_dom = median_or_avg(&mut dom_l);
    let v_h_rec = median_or_avg(&mut rec_h);
    let v_l_rec = median_or_avg(&mut rec_l);

    let v_diff_dom = v_h_dom - v_l_dom;
    let v_diff_rec = v_h_rec - v_l_rec;
    let v_common_mode = (v_h_rec + v_l_rec) * 0.5;
    let delta_v = v_diff_dom - v_diff_rec;

    // Evaluación según ISO 11898
    let health = if v_diff_dom < 1.45 {
        CanBusHealth::LowAmplitude
    } else if v_diff_dom > 3.10 {
        CanBusHealth::HighAmplitude
    } else if (v_common_mode - 2.5).abs() > 0.75 {
        CanBusHealth::CommonModeOffset
    } else if v_diff_rec.abs() > 0.45 {
        CanBusHealth::RecessiveLeak
    } else {
        CanBusHealth::Healthy
    };

    CanPhysicalMetrics {
        v_h_rec,
        v_h_dom,
        v_l_rec,
        v_l_dom,
        v_diff_rec,
        v_diff_dom,
        v_common_mode,
        delta_v,
        health,
    }
}

fn median_or_avg(data: &mut [f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = data.len() / 2;
    data[mid]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_can_bus() {
        // Generar señal CAN ideal: 1000 muestras
        // Recesivo: CAN_H = 2.5 V, CAN_L = 2.5 V -> V_diff = 0.0 V
        // Dominante: CAN_H = 3.5 V, CAN_L = 1.5 V -> V_diff = 2.0 V
        let mut ch1 = Vec::new();
        let mut ch2 = Vec::new();
        for i in 0..1000 {
            let is_dominant = (i / 50) % 2 == 1;
            if is_dominant {
                ch1.push(3.52);
                ch2.push(1.48);
            } else {
                ch1.push(2.51);
                ch2.push(2.49);
            }
        }

        let m = analyze_can_physical_layer(&ch1, &ch2);
        assert_eq!(m.health, CanBusHealth::Healthy);
        assert!((m.v_diff_dom - 2.04).abs() < 0.1);
        assert!((m.v_diff_rec - 0.02).abs() < 0.05);
        assert!((m.v_common_mode - 2.5).abs() < 0.1);
    }

    #[test]
    fn test_missing_termination_high_amplitude() {
        // Falta de terminación produce V_diff dominante alto > 3.2 V
        let mut ch1 = Vec::new();
        let mut ch2 = Vec::new();
        for i in 0..1000 {
            let is_dominant = (i / 50) % 2 == 1;
            if is_dominant {
                ch1.push(4.2);
                ch2.push(0.8);
            } else {
                ch1.push(2.5);
                ch2.push(2.5);
            }
        }

        let m = analyze_can_physical_layer(&ch1, &ch2);
        assert_eq!(m.health, CanBusHealth::HighAmplitude);
        assert!(m.v_diff_dom > 3.0);
    }

    #[test]
    fn test_short_circuit_low_amplitude() {
        // Línea con carga excesiva o corto parcial: V_diff dominante < 1.2 V
        let mut ch1 = Vec::new();
        let mut ch2 = Vec::new();
        for i in 0..1000 {
            let is_dominant = (i / 50) % 2 == 1;
            if is_dominant {
                ch1.push(3.0);
                ch2.push(2.1); // Solo 0.9 V diferencial
            } else {
                ch1.push(2.5);
                ch2.push(2.5);
            }
        }

        let m = analyze_can_physical_layer(&ch1, &ch2);
        assert_eq!(m.health, CanBusHealth::LowAmplitude);
    }
}
