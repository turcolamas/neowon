//! Fase 1 (Cap. 2.1 y Cap. 12.6 de PicoScope 7):
//! - Sistema de Sondas, Pinzas Amperimétricas y Transductores Automotrices (`ProbeSpec`).
//! - Unidades Físicas reales en ejes, cursores y mediciones (`PhysicalUnit`: V, A, bar, psi, kPa, °C, °F, kV, RPM, %).
//! - Escalado Lineal ($y = m \cdot x + b$) y Tabular por Interpolación Por Tramos (`LookupTable`).
//! - Rango Vertical Automático Seguro (`Auto-Range`) y Contador de Frecuencia de Hardware por Canal.

use serde::{Deserialize, Serialize};

/// Unidades físicas soportadas nativamente por canal en PicoWon Automotive Suite (Cap. 12.6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PhysicalUnit {
    #[default]
    Volts,
    Amps,
    Bar,
    Psi,
    KPa,
    Celsius,
    Fahrenheit,
    KiloVolts,
    Rpm,
    Percent,
    Ohms,
}

impl PhysicalUnit {
    pub const ALL: [PhysicalUnit; 11] = [
        PhysicalUnit::Volts,
        PhysicalUnit::Amps,
        PhysicalUnit::Bar,
        PhysicalUnit::Psi,
        PhysicalUnit::KPa,
        PhysicalUnit::Celsius,
        PhysicalUnit::Fahrenheit,
        PhysicalUnit::KiloVolts,
        PhysicalUnit::Rpm,
        PhysicalUnit::Percent,
        PhysicalUnit::Ohms,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            PhysicalUnit::Volts => "V",
            PhysicalUnit::Amps => "A",
            PhysicalUnit::Bar => "bar",
            PhysicalUnit::Psi => "psi",
            PhysicalUnit::KPa => "kPa",
            PhysicalUnit::Celsius => "°C",
            PhysicalUnit::Fahrenheit => "°F",
            PhysicalUnit::KiloVolts => "kV",
            PhysicalUnit::Rpm => "RPM",
            PhysicalUnit::Percent => "%",
            PhysicalUnit::Ohms => "Ω",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PhysicalUnit::Volts => "Voltios (V)",
            PhysicalUnit::Amps => "Amperios (A)",
            PhysicalUnit::Bar => "Presión (bar)",
            PhysicalUnit::Psi => "Presión (psi)",
            PhysicalUnit::KPa => "Presión / Vacío (kPa)",
            PhysicalUnit::Celsius => "Temperatura (°C)",
            PhysicalUnit::Fahrenheit => "Temperatura (°F)",
            PhysicalUnit::KiloVolts => "Alta Tensión (kV)",
            PhysicalUnit::Rpm => "Velocidad Giro (RPM)",
            PhysicalUnit::Percent => "Porcentaje (%)",
            PhysicalUnit::Ohms => "Resistencia (Ω)",
        }
    }

    /// Formatea un valor en esta unidad física con prefijos SI cuando corresponde (V, A)
    /// o decimales limpios para presión/temperatura/kV.
    pub fn format_value(self, val: f64) -> String {
        let abs = val.abs();
        match self {
            PhysicalUnit::Volts | PhysicalUnit::Amps => {
                let sym = self.symbol();
                if abs == 0.0 {
                    format!("0.00 {sym}")
                } else if abs < 1.0 {
                    format!("{:.0} m{sym}", val * 1e3)
                } else if abs < 10.0 {
                    format!("{:.2} {sym}", val)
                } else if abs < 100.0 {
                    format!("{:.1} {sym}", val)
                } else {
                    format!("{:.0} {sym}", val)
                }
            }
            PhysicalUnit::Bar | PhysicalUnit::KiloVolts => {
                if abs < 10.0 {
                    format!("{:.2} {}", val, self.symbol())
                } else {
                    format!("{:.1} {}", val, self.symbol())
                }
            }
            PhysicalUnit::Psi | PhysicalUnit::KPa | PhysicalUnit::Celsius | PhysicalUnit::Fahrenheit => {
                format!("{:.1} {}", val, self.symbol())
            }
            PhysicalUnit::Rpm | PhysicalUnit::Ohms => {
                format!("{:.0} {}", val, self.symbol())
            }
            PhysicalUnit::Percent => {
                format!("{:.1} %", val)
            }
        }
    }
}

/// Categoría de sonda o transductor automotriz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProbeCategory {
    #[default]
    PassiveVoltage,
    CurrentClamp,
    PressureTransducer,
    TemperatureSensor,
    IgnitionPickup,
    Custom,
}

impl ProbeCategory {
    pub const ALL: [ProbeCategory; 6] = [
        ProbeCategory::PassiveVoltage,
        ProbeCategory::CurrentClamp,
        ProbeCategory::PressureTransducer,
        ProbeCategory::TemperatureSensor,
        ProbeCategory::IgnitionPickup,
        ProbeCategory::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ProbeCategory::PassiveVoltage => "📏 Puntas y Atenuadores de Tensión (V)",
            ProbeCategory::CurrentClamp => "🧲 Pinzas Amperimétricas (A)",
            ProbeCategory::PressureTransducer => "🔧 Transductores de Presión / Vacío (bar/psi/kPa)",
            ProbeCategory::TemperatureSensor => "🌡 Sensores de Temperatura (°C/°F)",
            ProbeCategory::IgnitionPickup => "⚡ Captadores de Alta Tensión Secundario (kV)",
            ProbeCategory::Custom => "🛠 Sondas Personalizadas del Usuario",
        }
    }
}

/// Método matemático de conversión entre la tensión en el conector BNC ($V_{\text{bnc}}$)
/// y la magnitud física medida por la sonda o transductor ($Y_{\text{phys}}$).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalingMethod {
    /// Relación de atenuación pasiva directa (ej. 1.0 para x1, 10.0 para x10, 20.0 para 20:1, 100.0 para x100)
    Attenuation(f64),
    /// Ecuación lineal: $Y_{\text{phys}} = \text{gradient} \cdot V_{\text{bnc}} + \text{offset}$
    /// Ejemplo Pinza 60A (100 mV/A): gradient = 10.0 A/V, offset = 0.0
    Linear { gradient: f64, offset: f64 },
    /// Tabla de calibración no lineal por tramos: pares ordenados $(V_{\text{bnc}}, Y_{\text{phys}})$
    LookupTable(Vec<(f64, f64)>),
}

impl Default for ScalingMethod {
    fn default() -> Self {
        ScalingMethod::Attenuation(1.0)
    }
}

impl ScalingMethod {
    /// Convierte tensión en el conector BNC (Voltios) a la magnitud física de la sonda
    pub fn v_bnc_to_physical(&self, v_bnc: f64) -> f64 {
        match self {
            ScalingMethod::Attenuation(ratio) => v_bnc * (*ratio),
            ScalingMethod::Linear { gradient, offset } => v_bnc * (*gradient) + (*offset),
            ScalingMethod::LookupTable(pts) => interpolate_table(pts, v_bnc, false),
        }
    }

    /// Convierte una magnitud física deseada a la tensión equivalente en el conector BNC (Voltios)
    pub fn physical_to_v_bnc(&self, phys: f64) -> f64 {
        match self {
            ScalingMethod::Attenuation(ratio) => {
                if ratio.abs() < 1e-12 {
                    phys
                } else {
                    phys / (*ratio)
                }
            }
            ScalingMethod::Linear { gradient, offset } => {
                if gradient.abs() < 1e-12 {
                    0.0
                } else {
                    (phys - (*offset)) / (*gradient)
                }
            }
            ScalingMethod::LookupTable(pts) => interpolate_table(pts, phys, true),
        }
    }

    /// Pendiente efectiva en unidades físicas por cada 1.0 V en el conector BNC (para escalar V/div -> Unidad/div)
    pub fn effective_units_per_bnc_volt(&self) -> f64 {
        match self {
            ScalingMethod::Attenuation(ratio) => ratio.abs().max(1e-6),
            ScalingMethod::Linear { gradient, .. } => gradient.abs().max(1e-6),
            ScalingMethod::LookupTable(pts) => {
                if pts.len() >= 2 {
                    let (v0, y0) = pts[0];
                    let (v1, y1) = pts[pts.len() - 1];
                    let dv = (v1 - v0).abs();
                    if dv > 1e-6 {
                        ((y1 - y0).abs() / dv).max(1e-6)
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            }
        }
    }
}

fn interpolate_table(pts: &[(f64, f64)], x: f64, inverse: bool) -> f64 {
    if pts.is_empty() {
        return x;
    }
    if pts.len() == 1 {
        return if inverse { pts[0].0 } else { pts[0].1 };
    }
    // Construir pares (in, out) según dirección
    let mut pairs: Vec<(f64, f64)> = pts
        .iter()
        .map(|&(v, y)| if inverse { (y, v) } else { (v, y) })
        .collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if x <= pairs[0].0 {
        return pairs[0].1;
    }
    let last = pairs.len() - 1;
    if x >= pairs[last].0 {
        return pairs[last].1;
    }
    for w in pairs.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if x >= x0 && x <= x1 {
            let dx = x1 - x0;
            if dx.abs() < 1e-12 {
                return y0;
            }
            let t = (x - x0) / dx;
            return y0 + t * (y1 - y0);
        }
    }
    pairs[last].1
}

/// Especificación completa de una Sonda o Transductor Automotriz (Cap. 12.6 Custom Probes)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeSpec {
    pub id: String,
    pub name: String,
    pub short_badge: String,
    pub category: ProbeCategory,
    pub unit: PhysicalUnit,
    pub scaling: ScalingMethod,
    pub recommended_coupling_ac: bool,
    pub recommended_lpf_hz: Option<f32>,
    pub range_min: f64,
    pub range_max: f64,
    pub is_builtin: bool,
}

impl Default for ProbeSpec {
    fn default() -> Self {
        Self::probe_x1()
    }
}

impl ProbeSpec {
    pub fn probe_x1() -> Self {
        Self {
            id: "x1_direct".into(),
            name: "Punta Directa 1:1 (V)".into(),
            short_badge: "x1".into(),
            category: ProbeCategory::PassiveVoltage,
            unit: PhysicalUnit::Volts,
            scaling: ScalingMethod::Attenuation(1.0),
            recommended_coupling_ac: false,
            recommended_lpf_hz: None,
            range_min: -50.0,
            range_max: 50.0,
            is_builtin: true,
        }
    }

    pub fn probe_x10() -> Self {
        Self {
            id: "x10_passive".into(),
            name: "Punta Atenuadora 10:1 (V)".into(),
            short_badge: "x10".into(),
            category: ProbeCategory::PassiveVoltage,
            unit: PhysicalUnit::Volts,
            scaling: ScalingMethod::Attenuation(10.0),
            recommended_coupling_ac: false,
            recommended_lpf_hz: None,
            range_min: -200.0,
            range_max: 200.0,
            is_builtin: true,
        }
    }

    pub fn probe_x100() -> Self {
        Self {
            id: "x100_hv".into(),
            name: "Punta Alta Tensión 100:1 (V)".into(),
            short_badge: "x100".into(),
            category: ProbeCategory::PassiveVoltage,
            unit: PhysicalUnit::Volts,
            scaling: ScalingMethod::Attenuation(100.0),
            recommended_coupling_ac: false,
            recommended_lpf_hz: None,
            range_min: -500.0,
            range_max: 500.0,
            is_builtin: true,
        }
    }

    /// Factor de escala equivalente que se pasa a `ChannelConfig.probe` en `neowon-core`
    pub fn effective_probe_factor(&self) -> f64 {
        self.scaling.effective_units_per_bnc_volt()
    }

    /// Convierte Voltios en la entrada BNC a unidades físicas de la sonda
    pub fn v_bnc_to_physical(&self, v_bnc: f64) -> f64 {
        self.scaling.v_bnc_to_physical(v_bnc)
    }

    /// Formatea la sensibilidad por división (ej. `500 mV/div`, `2 A/div`, `5 bar/div`, `10 kV/div`)
    pub fn format_per_div(&self, bnc_volts_div: f64) -> String {
        let phys_per_div = bnc_volts_div * self.effective_probe_factor();
        format!("{}/div", self.unit.format_value(phys_per_div))
    }

    /// Formatea un valor ya multiplicado por `effective_probe_factor()` (como los que calcula `MeasureState` o los cursores)
    pub fn format_scaled_value(&self, scaled_val: f64) -> String {
        let offset = match &self.scaling {
            ScalingMethod::Linear { offset, .. } => *offset,
            _ => 0.0,
        };
        self.unit.format_value(scaled_val + offset)
    }
}

/// Catálogo integrado de Sondas y Transductores Automotrices (Cap. 2.1 y Cap. 12.6)
pub fn builtin_automotive_probes() -> Vec<ProbeSpec> {
    vec![
        ProbeSpec::probe_x1(),
        ProbeSpec::probe_x10(),
        ProbeSpec {
            id: "x20_auto".into(),
            name: "Atenuador Automotriz 20:1 (Inyectores / Primario)".into(),
            short_badge: "20:1".into(),
            category: ProbeCategory::PassiveVoltage,
            unit: PhysicalUnit::Volts,
            scaling: ScalingMethod::Attenuation(20.0),
            recommended_coupling_ac: false,
            recommended_lpf_hz: None,
            range_min: -400.0,
            range_max: 400.0,
            is_builtin: true,
        },
        ProbeSpec::probe_x100(),
        // Pinzas Amperimétricas
        ProbeSpec {
            id: "clamp_60a_100mv".into(),
            name: "Pinza Amperimétrica 20A/60A — Modo 20A (100 mV/A)".into(),
            short_badge: "20A".into(),
            category: ProbeCategory::CurrentClamp,
            unit: PhysicalUnit::Amps,
            scaling: ScalingMethod::Linear {
                gradient: 10.0, // 100 mV/A -> 10 A por cada 1V en BNC
                offset: 0.0,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(10_000.0),
            range_min: -20.0,
            range_max: 20.0,
            is_builtin: true,
        },
        ProbeSpec {
            id: "clamp_60a_10mv".into(),
            name: "Pinza Amperimétrica 60A — Modo 60A (10 mV/A)".into(),
            short_badge: "60A".into(),
            category: ProbeCategory::CurrentClamp,
            unit: PhysicalUnit::Amps,
            scaling: ScalingMethod::Linear {
                gradient: 100.0, // 10 mV/A -> 100 A por cada 1V en BNC
                offset: 0.0,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(10_000.0),
            range_min: -60.0,
            range_max: 60.0,
            is_builtin: true,
        },
        ProbeSpec {
            id: "clamp_600a_1mv".into(),
            name: "Pinza Amperimétrica 600A Arranque/Carga (1 mV/A)".into(),
            short_badge: "600A".into(),
            category: ProbeCategory::CurrentClamp,
            unit: PhysicalUnit::Amps,
            scaling: ScalingMethod::Linear {
                gradient: 1000.0, // 1 mV/A -> 1000 A por cada 1V en BNC
                offset: 0.0,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(2_000.0),
            range_min: -600.0,
            range_max: 600.0,
            is_builtin: true,
        },
        ProbeSpec {
            id: "clamp_2000a".into(),
            name: "Pinza Amperimétrica 2000A Pesados 24V (0.1 mV/A)".into(),
            short_badge: "2000A".into(),
            category: ProbeCategory::CurrentClamp,
            unit: PhysicalUnit::Amps,
            scaling: ScalingMethod::Linear {
                gradient: 10_000.0,
                offset: 0.0,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(1_000.0),
            range_min: -2000.0,
            range_max: 2000.0,
            is_builtin: true,
        },
        // Transductores de Presión y Vacío
        ProbeSpec {
            id: "wps_compression_35bar".into(),
            name: "Transductor Presión Cilindro Compresión (0–35 bar)".into(),
            short_badge: "35bar".into(),
            category: ProbeCategory::PressureTransducer,
            unit: PhysicalUnit::Bar,
            scaling: ScalingMethod::Linear {
                gradient: 8.75,  // 0.5V = 0 bar, 4.5V = 35 bar -> m = 35 / 4 = 8.75 bar/V
                offset: -4.375,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(1_000.0),
            range_min: -1.0,
            range_max: 35.0,
            is_builtin: true,
        },
        ProbeSpec {
            id: "wps_fuel_10bar".into(),
            name: "Transductor Presión Riel Combustible (0–10 bar)".into(),
            short_badge: "10bar".into(),
            category: ProbeCategory::PressureTransducer,
            unit: PhysicalUnit::Bar,
            scaling: ScalingMethod::Linear {
                gradient: 2.5,   // 0.5V = 0 bar, 4.5V = 10 bar -> m = 2.5 bar/V
                offset: -1.25,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(1_000.0),
            range_min: 0.0,
            range_max: 10.0,
            is_builtin: true,
        },
        ProbeSpec {
            id: "wps_intake_kpa".into(),
            name: "Transductor Vacío Admisión / Pulso Escape (-100..+100 kPa)".into(),
            short_badge: "kPa".into(),
            category: ProbeCategory::PressureTransducer,
            unit: PhysicalUnit::KPa,
            scaling: ScalingMethod::Linear {
                gradient: 50.0,
                offset: -125.0,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(500.0),
            range_min: -100.0,
            range_max: 100.0,
            is_builtin: true,
        },
        ProbeSpec {
            id: "wps_psi_150".into(),
            name: "Transductor Presión Hidráulica / Neumática (0–150 psi)".into(),
            short_badge: "150psi".into(),
            category: ProbeCategory::PressureTransducer,
            unit: PhysicalUnit::Psi,
            scaling: ScalingMethod::Linear {
                gradient: 37.5,
                offset: -18.75,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(1_000.0),
            range_min: 0.0,
            range_max: 150.0,
            is_builtin: true,
        },
        // Captadores de Ignición Secundaria (kV)
        ProbeSpec {
            id: "ign_kv_secondary".into(),
            name: "Captador Capacitivo Secundario Encendido (10 kV/V)".into(),
            short_badge: "kV".into(),
            category: ProbeCategory::IgnitionPickup,
            unit: PhysicalUnit::KiloVolts,
            scaling: ScalingMethod::Linear {
                gradient: 10.0,
                offset: 0.0,
            },
            recommended_coupling_ac: false,
            recommended_lpf_hz: None,
            range_min: -50.0,
            range_max: 50.0,
            is_builtin: true,
        },
        // Sensor de Temperatura NTC con Tabla de Calibración No Lineal
        ProbeSpec {
            id: "temp_ntc_coolant".into(),
            name: "Sensor Temperatura NTC Refrigerante/Aceite (°C Tabla)".into(),
            short_badge: "°C".into(),
            category: ProbeCategory::TemperatureSensor,
            unit: PhysicalUnit::Celsius,
            scaling: ScalingMethod::LookupTable(vec![
                (0.25, 120.0),
                (0.50, 100.0),
                (1.00, 80.0),
                (1.65, 60.0),
                (2.50, 40.0),
                (3.35, 20.0),
                (4.10, 0.0),
                (4.55, -20.0),
            ]),
            recommended_coupling_ac: false,
            recommended_lpf_hz: Some(100.0),
            range_min: -20.0,
            range_max: 120.0,
            is_builtin: true,
        },
    ]
}

/// Estado extendido por canal (Cap. 2.1: Sonda/Transductor, Unidad Física, Alias, Auto-Range y Contador HW)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelExtendedState {
    pub probe_spec: ProbeSpec,
    pub alias: String,
    pub auto_range: bool,
    pub freq_counter: bool,
    pub custom_color_rgb: Option<[u8; 3]>,
}

impl Default for ChannelExtendedState {
    fn default() -> Self {
        Self {
            probe_spec: ProbeSpec::probe_x1(),
            alias: String::new(),
            auto_range: false,
            freq_counter: false,
            custom_color_rgb: None,
        }
    }
}

/// Almacén persistente de Sondas Personalizadas del Usuario (`Capturas/custom_probes.json`)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomProbeStore {
    pub custom_probes: Vec<ProbeSpec>,
}

impl CustomProbeStore {
    const FILE_PATH: &'static str = "Capturas/custom_probes.json";

    pub fn load_or_default() -> Self {
        if let Ok(txt) = std::fs::read_to_string(Self::FILE_PATH) {
            if let Ok(store) = serde_json::from_str::<Self>(&txt) {
                return store;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(parent) = std::path::Path::new(Self::FILE_PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::FILE_PATH, json);
        }
    }

    pub fn all_probes(&self) -> Vec<ProbeSpec> {
        let mut all = builtin_automotive_probes();
        all.extend(self.custom_probes.iter().cloned());
        all
    }
}

/// Escalones de sensibilidad BNC del hardware OWON VDS1022 con piso seguro automotriz (>= 50 mV/div en modo Auto)
pub const SAFE_AUTO_VDIV_STEPS: [f64; 7] = [0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0];

/// Calcula la escala vertical `volts_div` óptima y segura en el conector BNC a partir
/// de las tensiones mínima y máxima medidas en el canal (Cap. 2.1 `Auto` Range).
pub fn compute_safe_auto_vdiv(vmin_scaled: f64, vmax_scaled: f64, probe_factor: f64, current_vdiv: f64) -> f64 {
    let factor = probe_factor.abs().max(1e-6);
    let max_abs_bnc = (vmin_scaled.abs().max(vmax_scaled.abs())) / factor;
    // La pantalla tiene 10 divisiones verticales (±5 divisiones desde el centro).
    // Buscamos que la señal ocupe entre el 35% y el 80% de la semialtura (4 divisiones).
    let needed_vdiv = (max_abs_bnc / 3.6).clamp(0.05, 5.0);

    let target = SAFE_AUTO_VDIV_STEPS
        .iter()
        .copied()
        .find(|&step| step >= needed_vdiv)
        .unwrap_or(5.0);

    // Protección escalonada: si necesita bajar de escala, bajar como máximo un escalón por ciclo
    if target < current_vdiv {
        let cur_idx = SAFE_AUTO_VDIV_STEPS
            .iter()
            .position(|&s| (s - current_vdiv).abs() < 1e-4)
            .unwrap_or(SAFE_AUTO_VDIV_STEPS.len() - 1);
        if cur_idx > 0 {
            return SAFE_AUTO_VDIV_STEPS[cur_idx - 1].max(target);
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_current_clamp_and_pressure_transducer_scaling() {
        let probes = builtin_automotive_probes();
        let clamp_60a = probes.iter().find(|p| p.id == "clamp_60a_10mv").unwrap();
        // 10 mV/A -> 0.25 V en BNC = 25.0 A
        assert!((clamp_60a.v_bnc_to_physical(0.25) - 25.0).abs() < 1e-6);
        assert_eq!(clamp_60a.format_per_div(0.1), "10.0 A/div");

        let wps_35 = probes.iter().find(|p| p.id == "wps_compression_35bar").unwrap();
        // 0.5V en BNC = 0.0 bar; 4.5V en BNC = 35.0 bar
        assert!((wps_35.v_bnc_to_physical(0.5) - 0.0).abs() < 1e-6);
        assert!((wps_35.v_bnc_to_physical(4.5) - 35.0).abs() < 1e-6);
    }

    #[test]
    fn lookup_table_ntc_temperature_interpolation() {
        let probes = builtin_automotive_probes();
        let ntc = probes.iter().find(|p| p.id == "temp_ntc_coolant").unwrap();
        // En 0.50V -> 100°C, en 1.00V -> 80°C; punto medio 0.75V -> 90°C
        let t_mid = ntc.v_bnc_to_physical(0.75);
        assert!((t_mid - 90.0).abs() < 1e-6);
    }

    #[test]
    fn auto_range_respects_step_down_safety() {
        // Si la señal es pequeña (0.2V pico) pero estamos en 5.0 V/div, baja escalonadamente a 2.0 V/div primero
        let next = compute_safe_auto_vdiv(-0.2, 0.2, 1.0, 5.0);
        assert!((next - 2.0).abs() < 1e-6);

        // Si la señal sube de golpe a 16V pico mientras estamos en 0.5 V/div, sube inmediatamente a 5.0 V/div para proteger
        let up = compute_safe_auto_vdiv(-1.0, 16.0, 1.0, 0.5);
        assert!((up - 5.0).abs() < 1e-6);
    }
}
