//! Trazador de Curvas V-I (Análisis de Impedancia y Firmas de Pines).
//!
//! En electrónica automotriz y reparación de ECUs / cuadros de instrumentos:
//! El análisis de firmas analógicas V-I aplica una señal senoidal o triangular (generador interno
//! o externo) al pin bajo prueba (DUT) a través de una resistencia en serie limitadora (R_sense),
//! y grafica la Tensión (CH1: Voltios) vs Corriente (CH2: Tensión en R_sense -> mA).
//!
//! Patrones característicos generados:
//! - Resistencia pura: Línea recta diagonal con pendiente proporcional a 1/R.
//! - Condensador / Capacidad: Elipse u óvalo circular.
//! - Inductor / Bobina: Elipse inclinada según la reactancia inductiva.
//! - Diodo de silicio / Zener: Forma de ángulo recto en L ("codo" de conducción a 0.6V / avalancha Zener).
//! - Transistores / MOSFETs / Pines de microcontroladores (ESD): Curva no lineal con rodilla de protección ESD.
//!
//! Permite detectar pines quemados, diodos ESD abiertos o en cortocircuito sin necesidad de alimentar la placa.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentSignature {
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    Zener,
    TransistorBe,
    OpenCircuit,
    ShortCircuit,
}

impl ComponentSignature {
    pub fn label(self) -> &'static str {
        match self {
            ComponentSignature::Resistor => "Resistencia Pura (Línea Diagonal)",
            ComponentSignature::Capacitor => "Capacidad / Condensador (Elipse)",
            ComponentSignature::Inductor => "Inductor / Bobina (Elipse Invertida)",
            ComponentSignature::Diode => "Diodo Unión P-N (Codo L a 0.6V)",
            ComponentSignature::Zener => "Diodo Zener / Supresor TVS",
            ComponentSignature::TransistorBe => "Unión B-E Transistor Bipolar",
            ComponentSignature::OpenCircuit => "Circuito Abierto (Línea Horizontal)",
            ComponentSignature::ShortCircuit => "Cortocircuito (Línea Vertical)",
        }
    }
}

/// Configuración y perfil de calibración de un equipo trazador de curvas V-I
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracerEquipment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub ch1_vdiv: f64,        // Sensibilidad hardware BNC (ej. 0.5 o 5.0)
    pub ch1_probe: f64,       // 1.0, 10.0, 100.0
    pub ch2_vdiv: f64,        // Sensibilidad hardware BNC CH2
    pub ch2_probe: f64,       // 1.0, 10.0, 100.0
    pub timebase: f64,        // Base de tiempo en segundos (ej. 0.010 para 10ms/div)
    pub invert_vertical: bool,// Inversión CH2 por polaridad del sensado
    pub r_sense_ohms: f32,    // Resistencia de referencia (ej. 1000.0 Ω)
    pub is_factory: bool,     // Si es preset de fábrica o personalizado
    #[serde(default)]
    pub draw_dots: bool,      // Trazado por puntos (true) o por vectores continuos (false)
    #[serde(default = "default_true")]
    pub use_roll: bool,       // Modo Roll (streaming continuo) o Trigger
    #[serde(default)]
    pub trigger_channel: usize,// Canal de disparo (0: CH1, 1: CH2)
    #[serde(default = "default_true")]
    pub trigger_auto: bool,   // Barrido Auto (true) o Normal (false)
    #[serde(default)]
    pub trigger_level: f64,   // Nivel de disparo en Voltios
    #[serde(default = "default_divs")]
    pub grid_divs_h: usize,   // Divisiones horizontales (por defecto 8)
    #[serde(default = "default_divs")]
    pub grid_divs_v: usize,   // Divisiones verticales (por defecto 8)
    #[serde(default = "default_smooth")]
    pub smooth_samples: usize,// Suavizado y reducción de ruido (0: desactivado, 3: leve, 5: medio, 7: fuerte)
    #[serde(default = "default_burst")]
    pub burst_count: usize,   // Cantidad de ciclos por ráfaga para promediado (por defecto 16)
    #[serde(default = "default_skip_samples")]
    pub skip_edge_samples: usize, // Descarte inicial de transitorios (0 a 300 muestras, por defecto 180)
    #[serde(default = "default_true")]
    pub show_tolerance_channel: bool, // Mostrar canal coloreado de tolerancia / envolvente
    #[serde(default = "default_tol_pct")]
    pub tolerance_percent: f32, // Porcentaje de tolerancia automática (ej. 5.0%)
}

fn default_skip_samples() -> usize {
    0
}

fn default_burst() -> usize {
    16
}

fn default_tol_pct() -> f32 {
    5.0
}

fn default_smooth() -> usize {
    5
}

fn default_divs() -> usize {
    8
}

fn default_true() -> bool {
    true
}

impl TracerEquipment {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        ch1_vdiv: f64,
        ch1_probe: f64,
        ch2_vdiv: f64,
        ch2_probe: f64,
        timebase: f64,
        invert_vertical: bool,
        r_sense_ohms: f32,
        is_factory: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            ch1_vdiv,
            ch1_probe,
            ch2_vdiv,
            ch2_probe,
            timebase,
            invert_vertical,
            r_sense_ohms,
            is_factory,
            draw_dots: false,
            use_roll: false,
            trigger_channel: 0,
            trigger_auto: true,
            trigger_level: 0.0,
            grid_divs_h: 8,
            grid_divs_v: 8,
            smooth_samples: 5,
            burst_count: 16,
            skip_edge_samples: 0,
            show_tolerance_channel: false,
            tolerance_percent: 5.0,
        }
    }

    pub fn new_custom(
        name: impl Into<String>,
        ch1_vdiv: f64,
        ch1_probe: f64,
        ch2_vdiv: f64,
        ch2_probe: f64,
        timebase: f64,
        invert_vertical: bool,
        r_sense_ohms: f32,
    ) -> Self {
        let name_str = name.into();
        let id = format!("custom_{}", name_str.to_lowercase().replace(' ', "_"));
        Self::new(
            id,
            name_str,
            "Equipo trazador personalizado",
            ch1_vdiv,
            ch1_probe,
            ch2_vdiv,
            ch2_probe,
            timebase,
            invert_vertical,
            r_sense_ohms,
            false,
        )
    }

    /// Voltaje por división efectivo mostrado en pantalla para CH1
    pub fn eff_ch1_vdiv(&self) -> f64 {
        self.ch1_vdiv * self.ch1_probe
    }

    /// Voltaje por división efectivo mostrado en pantalla para CH2
    pub fn eff_ch2_vdiv(&self) -> f64 {
        self.ch2_vdiv * self.ch2_probe
    }

    /// Corriente en mA por división efectiva calculada a través de R_sense
    pub fn eff_ch2_idiv_ma(&self) -> f64 {
        if self.r_sense_ohms > 0.0 {
            (self.eff_ch2_vdiv() / self.r_sense_ohms as f64) * 1000.0
        } else {
            1.0
        }
    }
}

/// Almacén persistente de equipos trazadores (Crear, Editar, Borrar, Guardar en disco)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracerEquipmentStore {
    pub active_id: String,
    pub equipments: Vec<TracerEquipment>,
}

impl Default for TracerEquipmentStore {
    fn default() -> Self {
        let trafo = TracerEquipment::new(
            "trafo_tracer",
            "Tracer VI con Trafo",
            "Tracer analógico con transformador: ±25V, ±25mA, sondas x10, 5V/div, 10ms/div, CH2 invertido, Rsense 1kΩ",
            0.5,
            10.0,
            0.5,
            10.0,
            10.0e-3,
            true,
            1000.0,
            true,
        );
        let usb = TracerEquipment::new(
            "usb_gen",
            "Generador USB (5Vpk)",
            "Generador de señal USB 5V: sondas x1, 1V/div, 2ms/div, Rsense 1kΩ (±5V, ±5mA)",
            1.0,
            1.0,
            1.0,
            1.0,
            2.0e-3,
            false,
            1000.0,
            true,
        );
        let ecu = TracerEquipment::new(
            "ecu_low_v",
            "ECU 3.3V / 5V Sensible",
            "Para microcontroladores y sensores: 0.5V/div, 0.2V/div, sondas x1, 5ms/div, Rsense 100Ω",
            0.5,
            1.0,
            0.2,
            1.0,
            5.0e-3,
            false,
            100.0,
            true,
        );

        Self {
            active_id: "trafo_tracer".to_string(),
            equipments: vec![trafo, usb, ecu],
        }
    }
}

impl TracerEquipmentStore {
    pub const STORE_PATH: &'static str = "tracer_equipments.json";

    /// Carga los equipos desde el archivo JSON de configuración persistente, o crea los valores por defecto
    pub fn load_or_default() -> Self {
        let path = Path::new(Self::STORE_PATH);
        if path.exists()
            && let Ok(data) = std::fs::read_to_string(path)
                && let Ok(mut store) = serde_json::from_str::<Self>(&data)
                    && !store.equipments.is_empty() {
                        let mut modified = false;
                        for eq in &mut store.equipments {
                            if eq.grid_divs_h == 4 && eq.grid_divs_v == 4 {
                                eq.grid_divs_h = 8;
                                eq.grid_divs_v = 8;
                                modified = true;
                            }
                            if eq.burst_count == 2 || eq.burst_count == 4 {
                                eq.burst_count = 16;
                                modified = true;
                            }
                            if eq.skip_edge_samples == 0 || eq.skip_edge_samples == 80 {
                                eq.skip_edge_samples = 180;
                                modified = true;
                            }
                            if eq.use_roll {
                                eq.use_roll = false;
                                modified = true;
                            }
                        }
                        if modified {
                            let _ = store.save();
                        }
                        return store;
                    }
        let def = Self::default();
        let _ = def.save();
        def
    }

    /// Guarda la lista de equipos y el equipo activo en disco
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::STORE_PATH, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn active_equipment(&self) -> &TracerEquipment {
        self.equipments
            .iter()
            .find(|e| e.id == self.active_id)
            .unwrap_or(&self.equipments[0])
    }

    pub fn set_active(&mut self, id: &str) -> bool {
        if self.equipments.iter().any(|e| e.id == id) {
            self.active_id = id.to_string();
            let _ = self.save();
            true
        } else {
            false
        }
    }

    pub fn add_equipment(&mut self, eq: TracerEquipment) {
        let id = eq.id.clone();
        self.equipments.push(eq);
        self.active_id = id;
        let _ = self.save();
    }

    pub fn update_equipment(&mut self, eq: TracerEquipment) -> bool {
        if let Some(pos) = self.equipments.iter().position(|e| e.id == eq.id) {
            self.equipments[pos] = eq;
            let _ = self.save();
            true
        } else {
            false
        }
    }

    pub fn delete_equipment(&mut self, id: &str) -> bool {
        if self.equipments.len() <= 1 {
            return false;
        }
        if let Some(pos) = self.equipments.iter().position(|e| e.id == id) {
            self.equipments.remove(pos);
            if self.active_id == id {
                self.active_id = self.equipments[0].id.clone();
            }
            let _ = self.save();
            true
        } else {
            false
        }
    }
}

/// Perfil / Preset de Hardware para un equipo trazador en particular
#[derive(Debug, Clone, PartialEq)]
pub struct ViTracerPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub ch1_vdiv: f64,
    pub ch2_vdiv: f64,
    pub ch1_probe: f64,
    pub ch2_probe: f64,
    pub timebase: f64,
    pub r_sense_ohms: f32,
    pub invert_vertical: bool,
    pub auto_scale: bool,
    pub max_voltage_scale: f32,
    pub max_current_scale_ma: f32,
    pub test_freq_hz: f32,
}

impl ViTracerPreset {
    /// Tracer analógico con transformador: ±25V, ±25mA, sondas x10, 5V/div, 10ms/div, CH2 invertido, escala fija.
    pub fn transformer_tracer() -> Self {
        Self {
            name: "Tracer VI con Trafo",
            description: "Tracer analógico con transformador: ±25V, ±25mA, sondas x10, 5V/div, 10ms/div, CH2 invertido, escala fija",
            ch1_vdiv: 0.5,
            ch2_vdiv: 0.5,
            ch1_probe: 10.0,
            ch2_probe: 10.0,
            timebase: 10.0e-3, // 10 ms/div
            r_sense_ohms: 1000.0, // 1 kΩ
            invert_vertical: true,
            auto_scale: false, // FIJO no autoajustable
            max_voltage_scale: 25.0,
            max_current_scale_ma: 25.0,
            test_freq_hz: 50.0,
        }
    }

    /// Generador interno o USB a 5V pico: ±5V, ±20mA, sondas x1, 1V/div, 2ms/div, auto-escala.
    pub fn usb_generator() -> Self {
        Self {
            name: "Generador USB 5V",
            description: "Generador de señal estándar USB: ±5V, ±20mA, sondas x1, 1V/div, 2ms/div, auto-escala activa",
            ch1_vdiv: 1.0,
            ch2_vdiv: 1.0,
            ch1_probe: 1.0,
            ch2_probe: 1.0,
            timebase: 2.0e-3,
            r_sense_ohms: 1000.0,
            invert_vertical: false,
            auto_scale: true,
            max_voltage_scale: 5.0,
            max_current_scale_ma: 20.0,
            test_freq_hz: 50.0,
        }
    }

    /// Comprobación de líneas de ECU de baja tensión (3.3V): sondas x1, 0.5V/div, 5ms/div.
    pub fn ecu_low_voltage() -> Self {
        Self {
            name: "ECU 3.3V Sensible",
            description: "Para circuitos integrados y microcontroladores: ±3.3V, ±5mA, sondas x1, escala fija",
            ch1_vdiv: 0.5,
            ch2_vdiv: 0.5,
            ch1_probe: 1.0,
            ch2_probe: 1.0,
            timebase: 5.0e-3,
            r_sense_ohms: 1000.0,
            invert_vertical: false,
            auto_scale: false,
            max_voltage_scale: 3.3,
            max_current_scale_ma: 5.0,
            test_freq_hz: 50.0,
        }
    }

    pub fn presets() -> Vec<Self> {
        vec![
            Self::transformer_tracer(),
            Self::usb_generator(),
            Self::ecu_low_voltage(),
        ]
    }
}

/// Estimación de circuito equivalente RLC / Semiconductor a partir de la firma V-I
#[derive(Debug, Clone, PartialEq)]
pub struct EquivalentCircuit {
    pub summary: String,
    pub model_type: &'static str,
    pub details: Vec<String>,
    pub estimated_r: Option<f32>,
    pub estimated_c: Option<f32>,
    pub estimated_l: Option<f32>,
    pub estimated_vf: Option<f32>,
    pub estimated_vz: Option<f32>,
}

/// Coincidencia de patrón en la base de datos de firmas automotrices
#[derive(Debug, Clone, PartialEq)]
pub struct PatternMatch {
    pub name: &'static str,
    pub category: &'static str,
    pub description: String,
    pub confidence_percent: f32,
    pub typical_application: &'static str,
}

/// Datos de la zona de tolerancia cuantitativa y envolvente del trazador VI
#[derive(Debug, Clone, Default)]
pub struct ToleranceChannelData {
    /// Puntos de la envolvente superior (V en Voltios, I en mA)
    pub upper_points: Vec<(f32, f32)>,
    /// Puntos de la envolvente inferior (V en Voltios, I en mA)
    pub lower_points: Vec<(f32, f32)>,
    /// Dispersión media de tensión en mV
    pub avg_dispersion_mv: f32,
    /// Dispersión media de corriente en mA
    pub avg_dispersion_ma: f32,
    /// Índice de estabilidad (0 a 100%, 100% = repetibilidad perfecta sin fluctuación)
    pub stability_percent: f32,
    /// Porcentaje de puntos dentro de tolerancia contra el Golden Sample (si existe)
    pub in_tolerance_percent: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ViTracerState {
    pub active: bool,
    pub r_sense_ohms: f32, // Típico: 100 Ω, 1 kΩ o 10 kΩ
    pub v_channel: usize,  // Tensión CH1
    pub i_channel: usize,  // Tensión sobre R_sense (CH2)
    pub current_points: Vec<(f32, f32)>, // Puntos (V en Voltios, I en mA)
    pub reference_points: Option<Vec<(f32, f32)>>, // Curva de referencia guardada (Golden Sample)
    pub reference_name: String,
    pub reference_timestamp: Option<String>,
    pub similarity_percent: Option<f32>, // 0.0 .. 100.0% de coincidencia con la referencia
    pub detected_signature: Option<ComponentSignature>,
    pub test_voltage_vpp: f32,
    pub test_freq_hz: f32,

    // Auto-escalado o escala fija del plano V-I
    pub auto_scale: bool,
    pub max_voltage_scale: f32,    // Rango ±V (ej. 25.0 V)
    pub max_current_scale_ma: f32, // Rango ±I (ej. 25.0 mA)

    // Mini-osciloscopio flotante
    pub mini_scope_open: bool,

    // Congelación / Pausa de captura (Freeze)
    pub frozen: bool,

    // Invertir eje vertical (corriente / CH2) por diseño electrónico del trazador
    pub invert_vertical: bool,

    // Modo de dibujo: por vectores continuos (false) o por puntos discretos (true)
    pub draw_dots: bool,

    // Filtro anti-picos (rechazo de glitches y espurias impulsivas de ADC/Roll)
    pub filter_spikes: bool,

    // Muestras de borde a descartar al inicio de la trama (suprime transitorio de arranque de FIFO/ADC)
    pub skip_edge_samples: usize,

    // Divisiones de cuadrícula horizontal y vertical (por defecto 4 x 4)
    pub grid_divs_h: usize,
    pub grid_divs_v: usize,

    // Suavizado y reducción de ruido DSP (0: apagado, 3: leve, 5: medio, 7: fuerte...)
    pub smooth_samples: usize,

    // Estrategia de Ráfagas y Canal de Tolerancia Cuantitativa
    pub burst_count: usize,           // Cantidad de ciclos en ráfaga para promedio (ej. 4)
    pub show_tolerance_channel: bool, // Mostrar canal coloreado de tolerancia / envolvente
    pub tolerance_percent: f32,       // Tolerancia cuantitativa porcentual (ej. 5.0%)
    pub tolerance_channel: Option<ToleranceChannelData>,
    pub recent_cycles: Vec<Vec<(f32, f32)>>,
    pub quasi_stationary_curve: Option<Vec<(f32, f32)>>, // Curva media cuasiestacionaria estabilizada

    // Preset / Perfil de hardware activo
    pub active_preset_name: String,

    // Gestor persistente de Equipos Tracer (Crear, Editar, Borrar, Guardar)
    pub equipment_store: TracerEquipmentStore,
    pub editor_equipment: Option<TracerEquipment>,
    pub is_creating_new: bool,

    // Diálogo de configuración / Setup abierto
    pub setup_dialog_open: bool,

    // Circuito equivalente y patrones
    pub equivalent_circuit: Option<EquivalentCircuit>,
    pub pattern_matches: Vec<PatternMatch>,
    pub show_equivalent_panel: bool,
    pub show_patterns_panel: bool,

    // Mensaje de estado (ej: "Captura guardada...")
    pub status_message: Option<(String, f64)>,

    // Configuración guardada dedicada para el modo Tracer
    pub saved_ch1_vdiv: f64,
    pub saved_ch2_vdiv: f64,
    pub saved_timebase: f64,
    pub saved_ch1_probe: f64,
    pub saved_ch2_probe: f64,
    pub has_saved_config: bool,

    // Buffers crudos para la vista de señales de los 2 canales
    pub raw_ch1: Vec<f32>,
    pub raw_ch2: Vec<f32>,

    // Últimas muestras recibidas para re-procesamiento inmediato al cambiar R_sense o invertir
    pub last_v: Vec<f32>,
    pub last_i: Vec<f32>,
}

impl Default for ViTracerState {
    fn default() -> Self {
        let store = TracerEquipmentStore::load_or_default();
        let eq = store.active_equipment().clone();
        let divs_h = eq.grid_divs_h.max(2);
        let divs_v = eq.grid_divs_v.max(2);
        let eff_v = eq.eff_ch1_vdiv() * (divs_h as f64 / 2.0);
        let eff_i = eq.eff_ch2_idiv_ma() * (divs_v as f64 / 2.0);
        Self {
            equipment_store: store,
            editor_equipment: None,
            is_creating_new: false,
            active: false,
            r_sense_ohms: eq.r_sense_ohms,
            v_channel: 0,
            i_channel: 1,
            current_points: Vec::with_capacity(2048),
            reference_points: None,
            reference_name: "Inyector 1 (Golden)".into(),
            reference_timestamp: None,
            similarity_percent: None,
            detected_signature: None,
            test_voltage_vpp: eff_v as f32,
            test_freq_hz: 50.0,
            auto_scale: false, // Fijo con divisiones configurables de escala
            max_voltage_scale: eff_v as f32,
            max_current_scale_ma: eff_i as f32,
            mini_scope_open: true,
            frozen: false,
            invert_vertical: eq.invert_vertical,
            draw_dots: eq.draw_dots,
            filter_spikes: true,
            skip_edge_samples: 0,
            grid_divs_h: divs_h,
            grid_divs_v: divs_v,
            smooth_samples: eq.smooth_samples,
            burst_count: eq.burst_count,
            show_tolerance_channel: eq.show_tolerance_channel,
            tolerance_percent: eq.tolerance_percent,
            tolerance_channel: None,
            recent_cycles: Vec::with_capacity(32),
            quasi_stationary_curve: None,
            active_preset_name: eq.name.clone(),
            setup_dialog_open: false,
            equivalent_circuit: None,
            pattern_matches: Vec::new(),
            show_equivalent_panel: true,
            show_patterns_panel: false,
            status_message: None,
            saved_ch1_vdiv: eq.ch1_vdiv,
            saved_ch2_vdiv: eq.ch2_vdiv,
            saved_timebase: eq.timebase,
            saved_ch1_probe: eq.ch1_probe,
            saved_ch2_probe: eq.ch2_probe,
            has_saved_config: false,
            raw_ch1: Vec::with_capacity(1024),
            raw_ch2: Vec::with_capacity(1024),
            last_v: Vec::new(),
            last_i: Vec::new(),
        }
    }
}

#[inline]
fn median3(a: f32, b: f32, c: f32) -> f32 {
    if (a <= b && b <= c) || (c <= b && b <= a) {
        b
    } else if (b <= a && a <= c) || (c <= a && a <= b) {
        a
    } else {
        c
    }
}

impl ViTracerState {
    pub fn clear(&mut self) {
        self.current_points.clear();
        self.tolerance_channel = None;
        self.recent_cycles.clear();
        self.quasi_stationary_curve = None;
        self.detected_signature = None;
        self.similarity_percent = None;
        self.equivalent_circuit = None;
        self.pattern_matches.clear();
    }

    /// Alterna el estado de congelación (Freeze / Pausa)
    pub fn toggle_freeze(&mut self) {
        self.frozen = !self.frozen;
    }

    /// Guarda la curva actual como referencia de "Pin Bueno" (Golden Sample)
    pub fn capture_reference(&mut self) {
        self.reference_points = Some(self.current_points.clone());
        self.reference_timestamp = Some(format!("{:.1}s", self.current_points.len() as f32));
        self.compute_similarity_with_reference();
    }

    /// Guarda la curva actual como referencia asignándole un nombre específico (ej. "Inyector 1")
    pub fn capture_reference_with_name(&mut self, name: &str) {
        self.reference_name = name.to_string();
        self.reference_points = Some(self.current_points.clone());
        self.compute_similarity_with_reference();
    }

    /// Borra la referencia almacenada
    pub fn clear_reference(&mut self) {
        self.reference_points = None;
        self.similarity_percent = None;
    }

    /// Aplica la configuración de un equipo trazador al estado del trazador
    pub fn apply_equipment(&mut self, eq: &TracerEquipment) {
        self.active_preset_name = eq.name.clone();
        self.r_sense_ohms = eq.r_sense_ohms;
        self.invert_vertical = eq.invert_vertical;
        self.draw_dots = eq.draw_dots;
        self.smooth_samples = eq.smooth_samples;
        self.burst_count = eq.burst_count.clamp(1, 32);
        self.skip_edge_samples = eq.skip_edge_samples.clamp(0, 300);
        self.show_tolerance_channel = eq.show_tolerance_channel;
        self.tolerance_percent = eq.tolerance_percent;
        self.recent_cycles.clear();
        self.grid_divs_h = eq.grid_divs_h.max(2);
        self.grid_divs_v = eq.grid_divs_v.max(2);
        self.saved_ch1_vdiv = eq.ch1_vdiv;
        self.saved_ch2_vdiv = eq.ch2_vdiv;
        self.saved_ch1_probe = eq.ch1_probe;
        self.saved_ch2_probe = eq.ch2_probe;
        self.saved_timebase = eq.timebase;
        self.max_voltage_scale = (eq.eff_ch1_vdiv() * (self.grid_divs_h as f64 / 2.0)) as f32;
        self.max_current_scale_ma = (eq.eff_ch2_idiv_ma() * (self.grid_divs_v as f64 / 2.0)) as f32;
        self.has_saved_config = true;
        self.reprocess_last();
    }

    /// Aplica un preset completo de hardware al estado del trazador
    pub fn apply_preset(&mut self, preset: &ViTracerPreset) {
        self.active_preset_name = preset.name.to_string();
        self.r_sense_ohms = preset.r_sense_ohms;
        self.invert_vertical = preset.invert_vertical;
        self.auto_scale = preset.auto_scale;
        self.max_voltage_scale = preset.max_voltage_scale;
        self.max_current_scale_ma = preset.max_current_scale_ma;
        self.test_freq_hz = preset.test_freq_hz;
        self.saved_ch1_vdiv = preset.ch1_vdiv;
        self.saved_ch2_vdiv = preset.ch2_vdiv;
        self.saved_ch1_probe = preset.ch1_probe;
        self.saved_ch2_probe = preset.ch2_probe;
        self.saved_timebase = preset.timebase;
        self.has_saved_config = true;
        self.reprocess_last();
    }

    /// Procesa una adquisición de muestras simultáneas CH1 y CH2 para computar la curva V-I
    pub fn process_samples(&mut self, v_samples: &[f32], i_sense_samples: &[f32]) {
        if self.frozen {
            return;
        }

        if v_samples.is_empty() || i_sense_samples.is_empty() {
            return;
        }

        self.last_v = v_samples.to_vec();
        self.last_i = i_sense_samples.to_vec();

        let n = v_samples.len().min(i_sense_samples.len());
        self.current_points.clear();
        let r_sense = self.r_sense_ohms.max(1.0);
        let inv = if self.invert_vertical { -1.0 } else { 1.0 };

        // Descartar muestras de borde al inicio y final del buffer (suprime transitorio de arranque de FIFO/ADC)
        // Rango ampliado hasta 300 puntos, ajustables de 10 en 10 (por defecto 180)
        let skip_start = if n > 350 { self.skip_edge_samples.min(n.saturating_sub(64)) } else { 0 };
        let end_idx = if n > 350 { (n - 20).max(skip_start) } else { n };
        let usable_len = end_idx - skip_start;

        // Guardar buffers para el mini-osciloscopio (a partir de skip_start para ver la forma de onda limpia)
        let stride = (usable_len / 512).max(1);
        self.raw_ch1.clear();
        self.raw_ch2.clear();
        for i in (skip_start..end_idx).step_by(stride) {
            let v = if self.filter_spikes && i > 0 && i < n - 1 {
                median3(v_samples[i - 1], v_samples[i], v_samples[i + 1])
            } else {
                v_samples[i]
            };
            let i_s = if self.filter_spikes && i > 0 && i < n - 1 {
                median3(i_sense_samples[i - 1], i_sense_samples[i], i_sense_samples[i + 1])
            } else {
                i_sense_samples[i]
            };
            self.raw_ch1.push(v);
            self.raw_ch2.push(inv * i_s);
        }

        // Convertir muestras a pares (V_dut en V, I_dut en mA)
        // I_dut = inv * (V_sense / R_sense) (en Amperios) -> * 1000 para mA
        let mut clean_pts: Vec<(f32, f32)> = Vec::with_capacity(usable_len);
        for i in skip_start..end_idx {
            let v_dut = if self.filter_spikes && i > 0 && i < n - 1 {
                median3(v_samples[i - 1], v_samples[i], v_samples[i + 1])
            } else {
                v_samples[i]
            };
            let v_sense = if self.filter_spikes && i > 0 && i < n - 1 {
                median3(i_sense_samples[i - 1], i_sense_samples[i], i_sense_samples[i + 1])
            } else {
                i_sense_samples[i]
            };
            let i_ma = inv * (v_sense / r_sense) * 1000.0;
            clean_pts.push((v_dut, i_ma));
        }

        // Filtro de suavizado y reducción de ruido previo (media móvil centrada de N muestras)
        let filtered_pts = if self.smooth_samples > 1 && clean_pts.len() > 3 {
            let w = self.smooth_samples;
            let half = w / 2;
            let len = clean_pts.len();
            let mut smoothed = Vec::with_capacity(len);
            for i in 0..len {
                let start = i.saturating_sub(half);
                let end = (i + half + 1).min(len);
                let count = (end - start) as f32;
                let mut sum_v = 0.0;
                let mut sum_i = 0.0;
                for p in &clean_pts[start..end] {
                    sum_v += p.0;
                    sum_i += p.1;
                }
                smoothed.push((sum_v / count, sum_i / count));
            }
            smoothed
        } else {
            clean_pts
        };

        // =========================================================================
        // ESTRATEGIA DE RÁFAGA, PROMEDIADO EN FASE Y CANAL DE TOLERANCIA
        // =========================================================================
        let burst_n = self.burst_count.clamp(1, 32);
        let mut extracted_cycles: Vec<Vec<(f32, f32)>> = Vec::new();

        if burst_n > 1 && filtered_pts.len() >= 64 {
            // 1. Detectar rangos de amplitud en CH1 (V_dut en V) y CH2 (I_dut en mA)
            let mut v_min = f32::MAX;
            let mut v_max = f32::MIN;
            let mut i_min = f32::MAX;
            let mut i_max = f32::MIN;
            for p in &filtered_pts {
                v_min = v_min.min(p.0);
                v_max = v_max.max(p.0);
                i_min = i_min.min(p.1);
                i_max = i_max.max(p.1);
            }
            let v_vpp = v_max - v_min;
            let i_vpp = i_max - i_min;

            // Si ambos canales están en reposo o desconectados (ruido residual < 0.35V y < 0.35mA),
            // no hay excitación física: purgar ráfagas y desactivar canal de tolerancia para evitar rayos en el origen
            let has_physical_signal = v_vpp >= 0.35 || i_vpp >= 0.35;
            if !has_physical_signal {
                self.tolerance_channel = None;
                self.quasi_stationary_curve = None;
                self.recent_cycles.clear();
                self.current_points = filtered_pts;
                return;
            }

            let scale_v = self.max_voltage_scale.max(1.0);
            let scale_i = self.max_current_scale_ma.max(1.0);

            let norm_v = v_vpp / scale_v;
            let norm_i = i_vpp / scale_i;

            // Decidir canal de sincronización para extracción de ciclos:
            // - Si hay buena tensión (circuito abierto, diodo, R alta, C, L): sinc en V (CH1)
            // - Si la tensión es casi nula y la corriente es alta (cortocircuito, R muy baja): sinc en I (CH2)
            let sync_on_current = (norm_i > norm_v * 1.5 && norm_i >= 0.08 && v_vpp < 1.0)
                || (v_vpp < 0.25 && i_vpp >= 0.5);

            let can_sync = if sync_on_current {
                i_vpp >= 0.5
            } else {
                v_vpp >= 0.25
            };

            if can_sync {
                let (mid, hyst) = if sync_on_current {
                    let m = if i_min < -0.1 && i_max > 0.1 { 0.0 } else { (i_min + i_max) * 0.5 };
                    let h = (i_vpp * 0.08).clamp(0.05, 5.0);
                    (m, h)
                } else {
                    let m = if v_min < -0.1 && v_max > 0.1 { 0.0 } else { (v_min + v_max) * 0.5 };
                    let h = (v_vpp * 0.08).clamp(0.04, 1.0);
                    (m, h)
                };

                // Encontrar cruces por cero ascendentes (flancos positivos) en la señal dominante
                let mut crossings: Vec<f32> = Vec::new();
                let mut armed = false;
                for idx in 1..filtered_pts.len() {
                    let prev_val = if sync_on_current { filtered_pts[idx - 1].1 } else { filtered_pts[idx - 1].0 };
                    let curr_val = if sync_on_current { filtered_pts[idx].1 } else { filtered_pts[idx].0 };
                    if curr_val < mid - hyst {
                        armed = true;
                    } else if armed && prev_val <= mid && curr_val > mid {
                        let d = curr_val - prev_val;
                        let frac = if d.abs() > 1e-4 { (mid - prev_val) / d } else { 0.5 };
                        crossings.push((idx - 1) as f32 + frac);
                        armed = false;
                    }
                }

                // Si encontramos al menos 2 cruces, podemos extraer ciclos completos de 360°
                if crossings.len() >= 2 {
                    const CYCLE_POINTS: usize = 256;
                    for w in crossings.windows(2) {
                        let t_start = w[0];
                        let t_end = w[1];
                        let period_len = t_end - t_start;
                        if period_len >= 16.0 {
                            let mut cycle = Vec::with_capacity(CYCLE_POINTS);
                            let mut has_discontinuity = false;
                            let mut prev_pt = (0.0f32, 0.0f32);
                            for j in 0..CYCLE_POINTS {
                                let phase = j as f32 / CYCLE_POINTS as f32;
                                let sample_pos = t_start + phase * period_len;
                                let i0 = (sample_pos.floor() as usize).min(filtered_pts.len() - 1);
                                let i1 = (i0 + 1).min(filtered_pts.len() - 1);
                                let frac = sample_pos - i0 as f32;
                                let v = filtered_pts[i0].0 * (1.0 - frac) + filtered_pts[i1].0 * frac;
                                let i_val = filtered_pts[i0].1 * (1.0 - frac) + filtered_pts[i1].1 * frac;
                                if j > 0 {
                                    let dv = (v - prev_pt.0).abs() / scale_v;
                                    let di = (i_val - prev_pt.1).abs() / scale_i;
                                    if dv > 0.30 || di > 0.30 {
                                        has_discontinuity = true;
                                        break;
                                    }
                                }
                                prev_pt = (v, i_val);
                                cycle.push((v, i_val));
                            }
                            if !has_discontinuity && cycle.len() == CYCLE_POINTS {
                                extracted_cycles.push(cycle);
                            }
                        }
                    }
                }
            }
        }

        if !extracted_cycles.is_empty() {
            let scale_v = self.max_voltage_scale.max(1.0);
            let scale_i = self.max_current_scale_ma.max(1.0);

            // Acumular en la cola FIFO de ráfaga con detección automática de transitorio/cambio de componente
            for cyc in extracted_cycles {
                // Verificar si hay una transición brusca con respecto al ciclo anterior
                let is_transition = if let Some(last_cyc) = self.recent_cycles.last() {
                    let mut sum_d2 = 0.0f32;
                    const CHECK_PTS: usize = 32;
                    for k in 0..CHECK_PTS {
                        let idx = (k * 256) / CHECK_PTS;
                        let dv = (cyc[idx].0 - last_cyc[idx].0) / scale_v;
                        let di = (cyc[idx].1 - last_cyc[idx].1) / scale_i;
                        sum_d2 += dv * dv + di * di;
                    }
                    let rms = (sum_d2 / CHECK_PTS as f32).sqrt();
                    rms > 0.08 // Umbral sensible para purgar transitorios inmediatamente
                } else {
                    false
                };

                if is_transition {
                    // Transición detectada (ej. cambio entre circuito abierto, diodo o resistencia):
                    // Purgar de inmediato el FIFO para no mezclar componentes distintos en la ráfaga
                    self.recent_cycles.clear();
                    self.quasi_stationary_curve = None;
                }

                if self.recent_cycles.len() >= burst_n {
                    self.recent_cycles.remove(0);
                }
                self.recent_cycles.push(cyc);
            }

            let n_burst = self.recent_cycles.len();
            const CYCLE_POINTS: usize = 256;
            let mut mean_pts = Vec::with_capacity(CYCLE_POINTS);

            let mut sum_sigma_v = 0.0f32;
            let mut sum_sigma_i = 0.0f32;
            let tol_factor = (self.tolerance_percent / 100.0).clamp(0.01, 0.30);

            // 1. Calcular promedio y desviación por cada fase j
            let mut means_v = [0.0f32; 256];
            let mut means_i = [0.0f32; 256];
            let mut sigmas_v = [0.0f32; 256];
            let mut sigmas_i = [0.0f32; 256];

            for j in 0..CYCLE_POINTS {
                let mut sv = 0.0;
                let mut si = 0.0;
                for c in 0..n_burst {
                    sv += self.recent_cycles[c][j].0;
                    si += self.recent_cycles[c][j].1;
                }
                let mv = sv / n_burst as f32;
                let mi = si / n_burst as f32;
                means_v[j] = mv;
                means_i[j] = mi;

                if n_burst > 1 {
                    let mut var_v = 0.0;
                    let mut var_i = 0.0;
                    for c in 0..n_burst {
                        let dv = self.recent_cycles[c][j].0 - mv;
                        let di = self.recent_cycles[c][j].1 - mi;
                        var_v += dv * dv;
                        var_i += di * di;
                    }
                    sigmas_v[j] = (var_v / (n_burst - 1) as f32).sqrt();
                    sigmas_i[j] = (var_i / (n_burst - 1) as f32).sqrt();
                } else {
                    sigmas_v[j] = 0.0;
                    sigmas_i[j] = 0.0;
                }
                sum_sigma_v += sigmas_v[j];
                sum_sigma_i += sigmas_i[j];
                mean_pts.push((mv, mi));
            }

            // Actualizar la curva cuasiestacionaria con filtro suavizado temporal (IIR)
            let stable_pts: Vec<(f32, f32)> = match self.quasi_stationary_curve.as_ref() {
                Some(prev_stable) if prev_stable.len() == CYCLE_POINTS => {
                    let alpha = 0.35f32;
                    (0..CYCLE_POINTS)
                        .map(|j| {
                            (
                                prev_stable[j].0 * (1.0 - alpha) + means_v[j] * alpha,
                                prev_stable[j].1 * (1.0 - alpha) + means_i[j] * alpha,
                            )
                        })
                        .collect()
                }
                _ => mean_pts.clone(),
            };
            self.quasi_stationary_curve = Some(stable_pts.clone());

            let avg_disp_mv = (sum_sigma_v / CYCLE_POINTS as f32) * 1000.0;
            let avg_disp_ma = sum_sigma_i / CYCLE_POINTS as f32;
            let stability = (100.0 - (avg_disp_mv / (scale_v * 10.0))).clamp(0.0, 100.0);

            // 2. Algoritmo de Tolerancia Cuantitativa contra Curva de Referencia (Golden Sample)
            let in_tol_pct = if let Some(ref ref_pts) = self.reference_points {
                if !ref_pts.is_empty() {
                    let mut inside_count = 0;
                    for &(mv, mi) in &mean_pts {
                        // Distancia mínima normalizada al Golden Sample
                        let mut min_d = f32::MAX;
                        for &(rv, ri) in ref_pts {
                            let dv = (mv - rv) / scale_v;
                            let di = (mi - ri) / scale_i;
                            let d = (dv * dv + di * di).sqrt();
                            if d < min_d {
                                min_d = d;
                            }
                        }
                        if min_d <= tol_factor {
                            inside_count += 1;
                        }
                    }
                    Some((inside_count as f32 / mean_pts.len() as f32) * 100.0)
                } else {
                    None
                }
            } else {
                None
            };

            self.tolerance_channel = Some(ToleranceChannelData {
                upper_points: Vec::new(),
                lower_points: Vec::new(),
                avg_dispersion_mv: avg_disp_mv,
                avg_dispersion_ma: avg_disp_ma,
                stability_percent: stability,
                in_tolerance_percent: in_tol_pct,
            });

            self.current_points = stable_pts;
        } else {
            // Señal plana / DC o ráfaga deshabilitada
            self.tolerance_channel = None;
            self.current_points = filtered_pts;
        }

        if self.auto_scale {
            self.auto_adjust_scale();
        }

        self.classify_signature();
        self.compute_similarity_with_reference();
    }

    /// Calcula en tiempo real el porcentaje de similitud (0 a 100%) con la curva de referencia (Golden Sample)
    /// usando distancia de Chamfer bidireccional normalizada a las escalas del trazador.
    pub fn compute_similarity_with_reference(&mut self) {
        let (ref_pts, live_pts) = match (&self.reference_points, &self.current_points) {
            (Some(r), l) if !r.is_empty() && !l.is_empty() => (r, l),
            _ => {
                self.similarity_percent = None;
                return;
            }
        };

        let scale_v = self.max_voltage_scale.max(0.1);
        let scale_i = self.max_current_scale_ma.max(0.1);

        let step_l = (live_pts.len() / 64).max(1);
        let step_r = (ref_pts.len() / 64).max(1);

        // 1. Distancia de Live a Ref
        let mut d_l2r = 0.0;
        let mut count_l = 0;
        for &(lv, li) in live_pts.iter().step_by(step_l) {
            let mut min_d_sq = f32::MAX;
            for &(rv, ri) in ref_pts.iter().step_by(step_r) {
                let dv = (lv - rv) / scale_v;
                let di = (li - ri) / scale_i;
                let d2 = dv * dv + di * di;
                if d2 < min_d_sq {
                    min_d_sq = d2;
                }
            }
            d_l2r += min_d_sq.sqrt();
            count_l += 1;
        }

        // 2. Distancia de Ref a Live (Bidireccional)
        let mut d_r2l = 0.0;
        let mut count_r = 0;
        for &(rv, ri) in ref_pts.iter().step_by(step_r) {
            let mut min_d_sq = f32::MAX;
            for &(lv, li) in live_pts.iter().step_by(step_l) {
                let dv = (rv - lv) / scale_v;
                let di = (ri - li) / scale_i;
                let d2 = dv * dv + di * di;
                if d2 < min_d_sq {
                    min_d_sq = d2;
                }
            }
            d_r2l += min_d_sq.sqrt();
            count_r += 1;
        }

        if count_l > 0 && count_r > 0 {
            let avg_l2r = d_l2r / count_l as f32;
            let avg_r2l = d_r2l / count_r as f32;
            let avg_bidir = (avg_l2r + avg_r2l) * 0.5;
            let score = ((1.0 - (avg_bidir / 0.25)) * 100.0).clamp(0.0, 100.0);
            self.similarity_percent = Some(score);
        } else {
            self.similarity_percent = None;
        }
    }

    /// Estima el circuito equivalente (R, C, L, Diodo, Zener) a partir de la firma V-I actual
    pub fn estimate_equivalent_circuit(&mut self) {
        if self.current_points.len() < 3 {
            self.equivalent_circuit = None;
            return;
        }

        let mut max_v = f32::MIN;
        let mut min_v = f32::MAX;
        let mut max_i = f32::MIN;
        let mut min_i = f32::MAX;

        for &(v, i) in &self.current_points {
            if v > max_v { max_v = v; }
            if v < min_v { min_v = v; }
            if i > max_i { max_i = i; }
            if i < min_i { min_i = i; }
        }

        let delta_v = max_v - min_v;
        let delta_i = max_i - min_i;
        let freq = self.test_freq_hz.max(1.0);

        // 1. Circuito Abierto
        if delta_i < 0.15 && delta_v > 1.0 {
            self.equivalent_circuit = Some(EquivalentCircuit {
                summary: "Circuito Abierto (Puntas al aire / Sin carga)".into(),
                model_type: "Circuito Abierto",
                details: vec![
                    format!("Tensión pico: {:.1} V", delta_v / 2.0),
                    "Corriente de fuga: < 0.1 mA (Despreciable)".into(),
                    "Impedancia equivalente: > 1 MΩ (Aislado)".into(),
                ],
                estimated_r: Some(1_000_000.0),
                estimated_c: None,
                estimated_l: None,
                estimated_vf: None,
                estimated_vz: None,
            });
            return;
        }

        // 2. Cortocircuito
        if delta_v < 0.35 && delta_i > 1.0 {
            let r_short = if delta_i > 0.001 { (delta_v / (delta_i / 1000.0)).max(0.1) } else { 0.1 };
            self.equivalent_circuit = Some(EquivalentCircuit {
                summary: "Cortocircuito / Continuidad Directa a Masa".into(),
                model_type: "Cortocircuito",
                details: vec![
                    format!("Caída de tensión residual: {:.2} V", delta_v),
                    format!("Corriente límite de sensado: {:.1} mA", delta_i),
                    format!("Resistencia de contacto: {:.2} Ω", r_short),
                ],
                estimated_r: Some(r_short),
                estimated_c: None,
                estimated_l: None,
                estimated_vf: None,
                estimated_vz: None,
            });
            return;
        }

        // 3. Área del bucle (Shoelace) para detectar reactancia capacitiva / inductiva
        let mut loop_area = 0.0;
        let n = self.current_points.len();
        for k in 0..n {
            let (v0, i0) = self.current_points[k];
            let (v1, i1) = self.current_points[(k + 1) % n];
            loop_area += v0 * i1 - v1 * i0;
        }
        let norm_area = (loop_area.abs() * 0.5) / (delta_v * delta_i + 1e-4);

        // 4. Linealidad óhmica y simetría (Resistencia Pura)
        let slope_pos = if max_v > 0.1 { max_i / max_v } else { 0.0 };
        let slope_neg = if min_v < -0.1 { min_i / min_v } else { 0.0 };
        let is_linear_resistor = norm_area < 0.08
            && slope_pos > 0.01
            && slope_neg > 0.01
            && ((slope_pos - slope_neg).abs() / (slope_pos + slope_neg)) < 0.25;

        if is_linear_resistor {
            let r_val = if delta_i > 0.01 { (delta_v / (delta_i / 1000.0)).max(1.0) } else { 1000.0 };
            let r_str = if r_val >= 1000.0 { format!("{:.2} kΩ", r_val / 1000.0) } else { format!("{:.0} Ω", r_val) };

            self.equivalent_circuit = Some(EquivalentCircuit {
                summary: format!("Resistencia Pura ({r_str})"),
                model_type: "Resistencia",
                details: vec![
                    format!("Resistencia equivalente: {r_str}"),
                    "Respuesta puramente óhmica (Ley de Ohm V = I * R)".into(),
                    "Trazado simétrico lineal en cuadrantes I y III".into(),
                ],
                estimated_r: Some(r_val),
                estimated_c: None,
                estimated_l: None,
                estimated_vf: None,
                estimated_vz: None,
            });
            return;
        }

        // 4. Conducción de semiconductor (asimetría en cuadrante 1 vs 3)
        let mut pos_i_sum = 0.0;
        let mut neg_i_sum = 0.0;
        let mut vf_candidate = None;
        let mut vz_candidate = None;
        let mut zero_band_i = 0.0;
        let mut zero_band_count = 0;

        for &(v, i) in &self.current_points {
            if v > 0.3 && i > 0.5 && vf_candidate.is_none() {
                vf_candidate = Some(v);
            }
            if v < -2.0 && i < -0.8 && vz_candidate.is_none() {
                vz_candidate = Some(v.abs());
            }
            if v > -1.0 && v < 0.3 {
                zero_band_i += i.abs();
                zero_band_count += 1;
            }
            if v > 0.5 { pos_i_sum += i.max(0.0); }
            if v < -0.5 { neg_i_sum += (-i).max(0.0); }
        }

        let avg_zero_band = if zero_band_count > 0 { zero_band_i / zero_band_count as f32 } else { 0.0 };

        if pos_i_sum > 4.0 * (neg_i_sum + 0.1) && norm_area < 0.20 {
            let vf = vf_candidate.unwrap_or(0.65);
            self.equivalent_circuit = Some(EquivalentCircuit {
                summary: "Diodo Unión P-N (Silicio / Rectificador)".into(),
                model_type: "Diodo P-N",
                details: vec![
                    format!("Tensión de umbral directo (Vf): {:.2} V", vf),
                    "Conducción directa en cuadrante I (+V, +I)".into(),
                    "Bloqueo inverso de corriente en cuadrante III".into(),
                    "Típico en: diodos rectificadores, clamping ESD, unión B-E".into(),
                ],
                estimated_r: None,
                estimated_c: None,
                estimated_l: None,
                estimated_vf: Some(vf),
                estimated_vz: None,
            });
            return;
        }

        if let (Some(vf), Some(vz)) = (vf_candidate, vz_candidate)
            && vz > 2.5 && norm_area < 0.20 && avg_zero_band < 0.4 {
                self.equivalent_circuit = Some(EquivalentCircuit {
                    summary: format!("Diodo Zener / Supresor TVS ({:.1} V)", vz),
                    model_type: "Zener / TVS",
                    details: vec![
                        format!("Tensión Zener inversa (Vz): {:.1} V", vz),
                        format!("Caída directa (Vf): {:.2} V", vf),
                        "Protección de sobretensión en ECU / Driver inyector".into(),
                    ],
                    estimated_r: None,
                    estimated_c: None,
                    estimated_l: None,
                    estimated_vf: Some(vf),
                    estimated_vz: Some(vz),
                });
                return;
            }

        // 5. Condensador o Inductor (elipse con bucle abierto)
        if norm_area > 0.15 {
            let xc = if delta_i > 0.1 { (delta_v / (delta_i / 1000.0)).max(1.0) } else { 1000.0 };
            let cap_uf = (1.0 / (2.0 * std::f32::consts::PI * freq * xc)) * 1_000_000.0;
            let ind_mh = (xc / (2.0 * std::f32::consts::PI * freq)) * 1000.0;

            if loop_area > 0.0 {
                self.equivalent_circuit = Some(EquivalentCircuit {
                    summary: "Capacitivo / Condensador (Elipse)".into(),
                    model_type: "Condensador",
                    details: vec![
                        format!("Capacidad estimada: {:.2} µF (a {:.0} Hz)", cap_uf, freq),
                        format!("Reactancia capacitiva Xc: {:.0} Ω", xc),
                        "Fase: Corriente adelanta a la tensión (+90°)".into(),
                    ],
                    estimated_r: None,
                    estimated_c: Some(cap_uf),
                    estimated_l: None,
                    estimated_vf: None,
                    estimated_vz: None,
                });
            } else {
                self.equivalent_circuit = Some(EquivalentCircuit {
                    summary: "Inductivo / Bobina (Elipse)".into(),
                    model_type: "Inductor",
                    details: vec![
                        format!("Inductancia estimada: {:.1} mH (a {:.0} Hz)", ind_mh, freq),
                        format!("Reactancia inductiva Xl: {:.0} Ω", xc),
                        "Fase: Tensión adelanta a la corriente (+90°)".into(),
                    ],
                    estimated_r: None,
                    estimated_c: None,
                    estimated_l: Some(ind_mh),
                    estimated_vf: None,
                    estimated_vz: None,
                });
            }
            return;
        }

        // 6. Resistencia Pura
        let r_val = if delta_i > 0.01 { (delta_v / (delta_i / 1000.0)).max(1.0) } else { 1000.0 };
        let r_str = if r_val >= 1000.0 { format!("{:.2} kΩ", r_val / 1000.0) } else { format!("{:.0} Ω", r_val) };

        self.equivalent_circuit = Some(EquivalentCircuit {
            summary: format!("Resistencia Pura ({r_str})"),
            model_type: "Resistencia",
            details: vec![
                format!("Resistencia equivalente: {r_str}"),
                "Respuesta puramente óhmica (Ley de Ohm V = I * R)".into(),
                "Trazado simétrico lineal en cuadrantes I y III".into(),
            ],
            estimated_r: Some(r_val),
            estimated_c: None,
            estimated_l: None,
            estimated_vf: None,
            estimated_vz: None,
        });
    }

    /// Busca coincidencias en la biblioteca de patrones automotrices típicos
    pub fn search_pattern_library(&mut self) {
        if self.current_points.len() < 10 {
            self.pattern_matches.clear();
            return;
        }

        if self.equivalent_circuit.is_none() {
            self.estimate_equivalent_circuit();
        }

        let eq = match &self.equivalent_circuit {
            Some(e) => e,
            None => return,
        };

        let mut list = Vec::new();

        match eq.model_type {
            "Diodo P-N" => {
                list.push(PatternMatch {
                    name: "Diodo Rectificador de Silicio (1N4007 / SMD M7)",
                    category: "Semiconductores",
                    description: "Curva L característica con codo a 0.65V. Bloqueo inverso hasta >50V.".into(),
                    confidence_percent: 96.0,
                    typical_application: "Protección de polaridad invertida en alimentación de ECUs",
                });
                list.push(PatternMatch {
                    name: "Diodo de Protección ESD de Entrada Microcontrolador",
                    category: "ECU / Lógica",
                    description: "Diodo clamping interno del pin del microcontrolador conectado a GND.".into(),
                    confidence_percent: 91.0,
                    typical_application: "Entradas de sensores digitales, líneas SPI, I2C y CAN_RX",
                });
                list.push(PatternMatch {
                    name: "Diodo Schottky de Rápida Conmutación (BAT54 / SS14)",
                    category: "Semiconductores",
                    description: "Caída directa más baja (0.2V a 0.4V) con conducción rápida.".into(),
                    confidence_percent: 78.0,
                    typical_application: "Fuentes conmutadas step-down y reguladores internos de ECU",
                });
            }
            "Zener / TVS" => {
                let vz = eq.estimated_vz.unwrap_or(12.0);
                list.push(PatternMatch {
                    name: "Salida a Inyector (Driver Low-Side MOSFET con Zener Clamp)",
                    category: "Actuadores Automotrices",
                    description: format!("Driver con rodilla de avalancha interna para absorber el pulso inductivo del inyector (clamp ~{:.0}V).", vz),
                    confidence_percent: 95.0,
                    typical_application: "Salidas de inyección de gasolina/diésel en Bosch, Marelli, Delphi",
                });
                list.push(PatternMatch {
                    name: "Diodo Supresor de Transitorios TVS Bidireccional/Unidireccional",
                    category: "Protección",
                    description: format!("Supresor de sobretensiones transitorias con breakdown a {:.1}V.", vz),
                    confidence_percent: 90.0,
                    typical_application: "Líneas de alimentación automotriz (Borne 15, Borne 30) y transceptores CAN",
                });
                list.push(PatternMatch {
                    name: "Driver de Válvula PWM / VVT con Clamp Activo",
                    category: "Actuadores",
                    description: "MOSFET inteligente con protección de sobretensión en drenador.".into(),
                    confidence_percent: 82.0,
                    typical_application: "Válvulas de solenoide de distribución variable (VVT), cánister EVAP",
                });
            }
            "Condensador" => {
                let c = eq.estimated_c.unwrap_or(1.0);
                list.push(PatternMatch {
                    name: "Condensador de Filtrado de Alimentación ECU",
                    category: "Filtrado",
                    description: format!("Elipse reactiva correspondiente a capacidad de aprox. {:.2} µF.", c),
                    confidence_percent: 94.0,
                    typical_application: "Líneas de 5V de referencia a sensores y desacoplo de microcontrolador",
                });
                list.push(PatternMatch {
                    name: "Filtro RC de Supresión EMI en Entrada de Sensor",
                    category: "Entradas Analógicas",
                    description: "Red de filtro pasa-bajos contra interferencias de encendido.".into(),
                    confidence_percent: 86.0,
                    typical_application: "Entradas de sonda Lambda, temperatura de motor (ECT, IAT)",
                });
            }
            "Inductor" => {
                let l = eq.estimated_l.unwrap_or(10.0);
                list.push(PatternMatch {
                    name: "Bobina de Inyector / Solenoide (Carga RL)",
                    category: "Bobinados",
                    description: format!("Elipse inclinada con inductancia aprox. {:.1} mH y resistencia de devanado.", l),
                    confidence_percent: 95.0,
                    typical_application: "Inyector de gasolina multipunto (12 a 16 Ω), electroválvulas",
                });
                list.push(PatternMatch {
                    name: "Bobina de Relé de Unidad de Mando",
                    category: "Actuadores",
                    description: "Devanado inductivo de relé principal o de bomba de combustible.".into(),
                    confidence_percent: 88.0,
                    typical_application: "Relé de bomba de nafta, relé de corte de inyección",
                });
            }
            "Resistencia" => {
                let r = eq.estimated_r.unwrap_or(1000.0);
                list.push(PatternMatch {
                    name: "Resistencia Pull-Up / Divisor de Tensión",
                    category: "Pasivos",
                    description: format!("Línea recta con valor medido de {:.0} Ω.", r),
                    confidence_percent: 95.0,
                    typical_application: "Resistencias de polarización para sensores NTC o sensores Hall",
                });
                list.push(PatternMatch {
                    name: "Resistencia de Terminación de Bus CAN (60 Ω o 120 Ω)",
                    category: "Comunicaciones",
                    description: "Terminación de alta velocidad en líneas CAN_H / CAN_L.".into(),
                    confidence_percent: if (r - 60.0).abs() < 15.0 || (r - 120.0).abs() < 20.0 { 93.0 } else { 55.0 },
                    typical_application: "Resistencia split o de nodo terminal en módulos ECU / Gateway",
                });
            }
            "Circuito Abierto" => {
                list.push(PatternMatch {
                    name: "Pin Sin Conexión / Pista Cortada / Soldadura Fría",
                    category: "Defectos",
                    description: "Línea puramente horizontal sin paso de corriente.".into(),
                    confidence_percent: 99.0,
                    typical_application: "Pin desconectado internamente o pista de circuito impreso abierta",
                });
            }
            "Cortocircuito" => {
                list.push(PatternMatch {
                    name: "Pin en Cortocircuito Directo a Masa (GND) o VCC",
                    category: "Defectos",
                    description: "Línea vertical con máxima corriente y cero voltios.".into(),
                    confidence_percent: 99.0,
                    typical_application: "MOSFET en cortocircuito D-S, condensador tantalio perforado, diodo en corto",
                });
            }
            _ => {}
        }

        self.pattern_matches = list;
    }

    /// Re-procesa las últimas muestras almacenadas (al alternar inversión o cambiar R_sense)
    pub fn reprocess_last(&mut self) {
        if !self.last_v.is_empty() && !self.last_i.is_empty() {
            let v = self.last_v.clone();
            let i = self.last_i.clone();
            self.process_samples(&v, &i);
        }
    }

    /// Auto-ajusta las escalas de tensión y corriente según la señal real medida
    pub fn auto_adjust_scale(&mut self) {
        if self.current_points.is_empty() {
            return;
        }

        let mut max_v: f32 = 0.2;
        let mut max_i: f32 = 0.5;
        for &(v, i) in &self.current_points {
            if v.abs() > max_v {
                max_v = v.abs();
            }
            if i.abs() > max_i {
                max_i = i.abs();
            }
        }

        let v_targets = [0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 50.0];
        let i_targets = [0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 50.0, 100.0, 200.0];

        let want_v = max_v * 1.30;
        self.max_voltage_scale = v_targets
            .iter()
            .copied()
            .find(|&s| s >= want_v)
            .unwrap_or(50.0);

        let want_i = max_i * 1.30;
        self.max_current_scale_ma = i_targets
            .iter()
            .copied()
            .find(|&s| s >= want_i)
            .unwrap_or(200.0);
    }

    /// Clasificación heurística de la curva obtenida
    fn classify_signature(&mut self) {
        if self.current_points.len() < 20 {
            return;
        }

        let mut max_v = f32::MIN;
        let mut min_v = f32::MAX;
        let mut max_i = f32::MIN;
        let mut min_i = f32::MAX;

        for &(v, i) in &self.current_points {
            if v > max_v { max_v = v; }
            if v < min_v { min_v = v; }
            if i > max_i { max_i = i; }
            if i < min_i { min_i = i; }
        }

        let delta_v = max_v - min_v;
        let delta_i = max_i - min_i;

        if delta_v < 0.2 && delta_i > 1.0 {
            self.detected_signature = Some(ComponentSignature::ShortCircuit);
        } else if delta_i < 0.05 && delta_v > 0.5 {
            self.detected_signature = Some(ComponentSignature::OpenCircuit);
        } else {
            // Verificar asimetría (unión P-N de diodo: conduce en v > 0.5V y no en v < 0)
            let mut pos_i_sum = 0.0;
            let mut neg_i_sum = 0.0;
            for &(v, i) in &self.current_points {
                if v > 0.6 {
                    pos_i_sum += i.abs();
                } else if v < -0.6 {
                    neg_i_sum += i.abs();
                }
            }

            if pos_i_sum > 10.0 * (neg_i_sum + 0.01) {
                self.detected_signature = Some(ComponentSignature::Diode);
            } else {
                self.detected_signature = Some(ComponentSignature::Resistor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_tracer_preset() {
        let p = ViTracerPreset::transformer_tracer();
        assert_eq!(p.name, "Tracer VI con Trafo");
        assert_eq!(p.ch1_vdiv, 0.5);
        assert_eq!(p.ch2_vdiv, 0.5);
        assert_eq!(p.ch1_probe, 10.0);
        assert_eq!(p.ch2_probe, 10.0);
        assert_eq!(p.timebase, 10.0e-3);
        assert!(p.invert_vertical);
        assert!(!p.auto_scale);
        assert_eq!(p.max_voltage_scale, 25.0);
        assert_eq!(p.max_current_scale_ma, 25.0);
    }

    #[test]
    fn test_pin_comparison_similarity() {
        let mut state = ViTracerState::default();
        // Curva 1: senoidal
        let mut pts = Vec::new();
        for k in 0..100 {
            let t = k as f32 * 0.0628;
            pts.push((5.0 * t.sin(), 10.0 * t.cos()));
        }
        state.current_points = pts.clone();
        state.capture_reference_with_name("Inyector 1");

        // Misma curva -> Similitud cercana al 100%
        state.compute_similarity_with_reference();
        let sim = state.similarity_percent.unwrap();
        assert!(sim > 95.0, "Misma curva debe tener >95% de similitud, dio {sim}%");

        // Curva diferente (circuito abierto con I=0) -> Similitud baja
        state.current_points = pts.iter().map(|&(v, _)| (v, 0.0)).collect();
        state.compute_similarity_with_reference();
        let sim_diff = state.similarity_percent.unwrap();
        assert!(sim_diff < 50.0, "Curva diferente debe tener <50% de similitud, dio {sim_diff}%");
    }

    #[test]
    fn test_equivalent_circuit_open_and_resistor() {
        let mut state = ViTracerState::default();
        // Circuito abierto: I=0, V oscilando
        state.current_points = vec![(-10.0, 0.0), (0.0, 0.0), (10.0, 0.0)];
        state.estimate_equivalent_circuit();
        let eq = state.equivalent_circuit.as_ref().unwrap();
        assert_eq!(eq.model_type, "Circuito Abierto");

        // Resistencia pura de 1k: 10V -> 10mA, -10V -> -10mA
        state.current_points = vec![(-10.0, -10.0), (0.0, 0.0), (10.0, 10.0)];
        state.estimate_equivalent_circuit();
        let eq_r = state.equivalent_circuit.as_ref().unwrap();
        assert_eq!(eq_r.model_type, "Resistencia");
        assert!((eq_r.estimated_r.unwrap() - 1000.0).abs() < 50.0);
    }

    #[test]
    fn test_tracer_equipment_store_crud() {
        let mut store = TracerEquipmentStore::default();
        assert!(store.equipments.len() >= 3);
        assert_eq!(store.active_id, "trafo_tracer");

        let active = store.active_equipment();
        assert_eq!(active.name, "Tracer VI con Trafo");
        assert_eq!(active.eff_ch1_vdiv(), 5.0); // 0.5 * 10.0 = 5.0 V/div
        assert_eq!(active.eff_ch2_idiv_ma(), 5.0); // (5.0 / 1000) * 1000 = 5.0 mA/div

        // Agregar nuevo equipo
        let custom = TracerEquipment::new(
            "custom_1",
            "Mi Tracer Casero",
            "Tracer de prueba 12V",
            2.0,
            1.0,
            1.0,
            1.0,
            5.0e-3,
            false,
            500.0,
            false,
        );
        store.add_equipment(custom);
        assert_eq!(store.active_id, "custom_1");
        assert_eq!(store.active_equipment().name, "Mi Tracer Casero");
        assert_eq!(store.active_equipment().eff_ch2_idiv_ma(), 2.0); // (1.0 / 500) * 1000 = 2 mA/div

        // Editar equipo
        let mut edited = store.active_equipment().clone();
        edited.name = "Mi Tracer Casero Modificado".into();
        assert!(store.update_equipment(edited));
        assert_eq!(store.active_equipment().name, "Mi Tracer Casero Modificado");

        // Borrar equipo
        assert!(store.delete_equipment("custom_1"));
        assert_ne!(store.active_id, "custom_1");
    }

    #[test]
    fn test_burst_averaging_and_tolerance_channel() {
        let mut state = ViTracerState::default();
        state.burst_count = 4;
        state.show_tolerance_channel = true;
        state.skip_edge_samples = 0;

        // Generar 3 ciclos completos de onda senoidal (1000 muestras/ciclo)
        let total_samples = 3000;
        let mut v_samples = Vec::with_capacity(total_samples);
        let mut i_samples = Vec::with_capacity(total_samples);

        for idx in 0..total_samples {
            let t = idx as f32 / 1000.0;
            let phase = t * std::f32::consts::TAU;
            let v = 10.0 * phase.sin();
            let i = 5.0 * phase.cos();
            v_samples.push(v);
            i_samples.push(i);
        }

        state.process_samples(&v_samples, &i_samples);

        assert_eq!(state.current_points.len(), 256);
        assert!(state.tolerance_channel.is_some());

        let tol = state.tolerance_channel.as_ref().unwrap();
        assert!(tol.stability_percent >= 90.0, "La estabilidad debe ser alta para senos puros: {}", tol.stability_percent);
        assert!(tol.avg_dispersion_mv < 50.0, "La dispersión en tensión debe ser muy baja: {}", tol.avg_dispersion_mv);
    }

    #[test]
    fn test_short_circuit_burst_synchronization() {
        let mut state = ViTracerState::default();
        state.burst_count = 16;
        state.show_tolerance_channel = true;
        state.skip_edge_samples = 0;
        state.max_voltage_scale = 20.0;
        state.max_current_scale_ma = 20.0;

        // En cortocircuito: Tensión DUT en CH1 es prácticamente cero (ruido ~0.02V)
        // Corriente en CH2 es una onda senoidal completa de 20 mA a través de R_sense
        let total_samples = 3000;
        let mut v_samples = Vec::with_capacity(total_samples);
        let mut i_samples = Vec::with_capacity(total_samples);

        for idx in 0..total_samples {
            let t = idx as f32 / 1000.0;
            let phase = t * std::f32::consts::TAU;
            // Ruido residual en V_dut (menor a 0.05V)
            let v_noise = 0.02 * (phase * 7.0).sin();
            // Corriente fuerte senoidal en CH2: 20Vpk sobre 1000Ω = 20V en V_sense
            let i_v = 20.0 * phase.sin();
            v_samples.push(v_noise);
            i_samples.push(i_v);
        }

        state.process_samples(&v_samples, &i_samples);

        // Debe extraer ciclo sincronizado en corriente y promediar en ráfaga
        assert_eq!(state.current_points.len(), 256);
        assert!(state.tolerance_channel.is_some());

        // La tensión promedio en CH1 debe tender a 0.0V (línea puramente vertical)
        let max_v = state.current_points.iter().map(|p| p.0.abs()).fold(0.0f32, f32::max);
        assert!(max_v < 0.05, "En cortocircuito V debe estar cerca de 0: {}", max_v);

        // La corriente debe oscilar con amplitud completa (aprox 20 mA)
        let max_i = state.current_points.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let min_i = state.current_points.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        assert!(max_i > 15.0 && min_i < -15.0, "La corriente debe abarcar ciclo completo vertical: max={}, min={}", max_i, min_i);
    }

    #[test]
    fn test_transition_tolerance_channel_stability() {
        let mut state = ViTracerState::default();
        state.burst_count = 16;
        state.show_tolerance_channel = true;
        state.skip_edge_samples = 0;
        state.max_voltage_scale = 20.0;
        state.max_current_scale_ma = 20.0;

        // Fase 1: Circuito abierto estable (V senoidal 15V, I = 0)
        let total_samples = 3000;
        let mut v_samples_open = Vec::with_capacity(total_samples);
        let mut i_samples_open = Vec::with_capacity(total_samples);
        for idx in 0..total_samples {
            let phase = (idx as f32 / 1000.0) * std::f32::consts::TAU;
            v_samples_open.push(15.0 * phase.sin());
            i_samples_open.push(0.0);
        }
        for _ in 0..5 {
            state.process_samples(&v_samples_open, &i_samples_open);
        }

        assert!(state.tolerance_channel.is_some());
        let tol1 = state.tolerance_channel.as_ref().unwrap();
        assert!(tol1.avg_dispersion_mv < 50.0, "Dispersión en circuito abierto debe ser mínima");

        // Fase 2: Transición abrupta a Diodo (V limitado a 0.7V en positivo, I fluye en positivo)
        let mut v_samples_diode = Vec::with_capacity(total_samples);
        let mut i_samples_diode = Vec::with_capacity(total_samples);
        for idx in 0..total_samples {
            let phase = (idx as f32 / 1000.0) * std::f32::consts::TAU;
            let raw_v = 15.0 * phase.sin();
            if raw_v > 0.7 {
                v_samples_diode.push(0.7);
                i_samples_diode.push((raw_v - 0.7) * 2.0);
            } else {
                v_samples_diode.push(raw_v);
                i_samples_diode.push(0.0);
            }
        }

        // Procesar la primera ráfaga del diodo tras el circuito abierto
        state.process_samples(&v_samples_diode, &i_samples_diode);

        // Gracias a la detección de transición y purga de ráfaga, la dispersión no debe dispararse a 5800 mV
        let tol2 = state.tolerance_channel.as_ref().unwrap();
        assert!(tol2.avg_dispersion_mv < 500.0, "La dispersión tras la transición debe purgarse y ser baja: {} mV", tol2.avg_dispersion_mv);
        assert!(tol2.stability_percent >= 50.0, "La estabilidad debe recuperarse rápidamente tras purgar: {}", tol2.stability_percent);
    }

    #[test]
    fn test_disconnected_channels_tolerance_channel_none() {
        let mut state = ViTracerState::default();
        state.burst_count = 16;
        state.show_tolerance_channel = true;

        // Canales desconectados: solo ruido térmico residual (<0.05V, <0.05mA)
        let total_samples = 3000;
        let mut v_samples = Vec::with_capacity(total_samples);
        let mut i_samples = Vec::with_capacity(total_samples);
        for idx in 0..total_samples {
            let noise_v = 0.03 * ((idx as f32 * 0.1).sin());
            let noise_i = 0.02 * ((idx as f32 * 0.13).cos());
            v_samples.push(noise_v);
            i_samples.push(noise_i);
        }

        state.process_samples(&v_samples, &i_samples);

        // Cuando ambos canales están desconectados, NO debe generarse canal de tolerancia (rayos parásitos)
        assert!(state.tolerance_channel.is_none(), "Con canales desconectados el canal de tolerancia debe ser None");
    }
}
