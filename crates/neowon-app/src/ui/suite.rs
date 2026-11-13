//! Arquitectura Macro de "PicoWon Automotive Suite" (Fase 0):
//! - Hub Central de Inicio (`Suite Launcher`) con las 5 Estaciones de Trabajo Independientes.
//! - Aislamiento de perfiles de hardware y DSP por estación (`ModuleWorkspaceProfile`).
//! - Ficha Técnica Global del Vehículo en Taller (`VehicleDetails`) compartida por todos los módulos
//!   con historial persistente de vehículos recientes.

use bevy_egui::egui;
use serde::{Deserialize, Serialize};
use picowon_auto::{ChannelDspConfig, PresetCategory, get_all_presets};

use crate::Link;
use crate::derived::MathState;
use crate::ui::automotive_dock::{AppMode, AutomotiveUiState};
use crate::ui::layout::Layout;
use crate::ui::menu::{Menu, MenuState};

/// Las 5 Estaciones de Trabajo Independientes de PicoWon Automotive Suite
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SuiteWorkstation {
    #[default]
    ScopeDatalogger,
    ViTracer,
    CommsMonitor,
    ActuatorsLoop,
    AutomotiveTests,
}

impl SuiteWorkstation {
    pub const ALL: [SuiteWorkstation; 5] = [
        SuiteWorkstation::ScopeDatalogger,
        SuiteWorkstation::ViTracer,
        SuiteWorkstation::CommsMonitor,
        SuiteWorkstation::ActuatorsLoop,
        SuiteWorkstation::AutomotiveTests,
    ];

    pub fn index(self) -> usize {
        match self {
            SuiteWorkstation::ScopeDatalogger => 0,
            SuiteWorkstation::ViTracer => 1,
            SuiteWorkstation::CommsMonitor => 2,
            SuiteWorkstation::ActuatorsLoop => 3,
            SuiteWorkstation::AutomotiveTests => 4,
        }
    }

    pub fn short_title(self) -> &'static str {
        match self {
            SuiteWorkstation::ScopeDatalogger => "📊 Osciloscopio + Datalogger",
            SuiteWorkstation::ViTracer => "⚡ Trazador V-I",
            SuiteWorkstation::CommsMonitor => "🌐 Comunicaciones CAN/LIN",
            SuiteWorkstation::ActuatorsLoop => "🎯 Actuadores y Sensores",
            SuiteWorkstation::AutomotiveTests => "🔧 Testeos Automotrices",
        }
    }

    pub fn window_app_name(self) -> &'static str {
        match self {
            SuiteWorkstation::ScopeDatalogger => "PicoWon — Osciloscopio + Datalogger de Laboratorio",
            SuiteWorkstation::ViTracer => "PicoWon — Trazador de Curvas V-I y Diagnóstico en Banco",
            SuiteWorkstation::CommsMonitor => "PicoWon — Monitor de Comunicaciones y Tráfico CAN / LIN",
            SuiteWorkstation::ActuatorsLoop => "PicoWon — Analizador de Actuadores y Sensores (Lazo Cerrado)",
            SuiteWorkstation::AutomotiveTests => "PicoWon — Estación de Testeos Automotrices Guiados",
        }
    }

    pub fn card_title(self) -> &'static str {
        match self {
            SuiteWorkstation::ScopeDatalogger => "1. OSCILOSCOPIO + DATALOGGER",
            SuiteWorkstation::ViTracer => "2. TRAZADOR DE CURVAS V-I",
            SuiteWorkstation::CommsMonitor => "3. COMUNICACIONES CAN / LIN",
            SuiteWorkstation::ActuatorsLoop => "4. ACTUADORES Y SENSORES",
            SuiteWorkstation::AutomotiveTests => "5. TESTEOS AUTOMOTRICES",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            SuiteWorkstation::ScopeDatalogger => {
                "Osciloscopio de laboratorio de alta velocidad, Registrador continuo Strip-Chart y Analizador de Espectro FFT"
            }
            SuiteWorkstation::ViTracer => {
                "Análisis de firma analógica X-Y (1:1) en banco, comparador Pin-a-Pin de ECU con Golden Sample y detección R/L/C/Diodo"
            }
            SuiteWorkstation::CommsMonitor => {
                "Salud eléctrica de Capa Física ISO 11898 (CAN_H, CAN_L, A-B) + Decodificador de Protocolos y Monitor de Tráfico en Vivo"
            }
            SuiteWorkstation::ActuatorsLoop => {
                "Control en Lazo Cerrado (Mando PWM/Corriente CH1 vs Respuesta Sensor CH2), curva de transferencia e histéresis mecánica"
            }
            SuiteWorkstation::AutomotiveTests => {
                "Ensayos guiados paso a paso: Carga y Arranque, Compresión de cilindros, Sincronismo CKP-CMP, Encendido e Inyección"
            }
        }
    }

    pub fn accent_color(self) -> egui::Color32 {
        match self {
            SuiteWorkstation::ScopeDatalogger => egui::Color32::from_rgb(255, 185, 40),
            SuiteWorkstation::ViTracer => egui::Color32::from_rgb(80, 220, 255),
            SuiteWorkstation::CommsMonitor => egui::Color32::from_rgb(110, 235, 150),
            SuiteWorkstation::ActuatorsLoop => egui::Color32::from_rgb(255, 135, 80),
            SuiteWorkstation::AutomotiveTests => egui::Color32::from_rgb(195, 145, 255),
        }
    }

    pub fn bullet_points(self) -> &'static [&'static str] {
        match self {
            SuiteWorkstation::ScopeDatalogger => &[
                "• Captura dual de alta velocidad (100 MS/s) con detección de Glitches",
                "• Registrador continuo Strip-Chart (18 métricas en tiempo real)",
                "• Acondicionamiento DSP por canal (8.5–12 bits, LPF y Tara/Cero)",
                "• Espectro FFT, Cursores, Canal Matemático y Grabación en RAM",
            ],
            SuiteWorkstation::ViTracer => &[
                "• Cuadrícula X-Y cuadrada (1:1) dedicada (Tensión vs Corriente mA)",
                "• Comparador de pines de ECU con porcentaje de similitud y tolerancia",
                "• Base de datos de firmas Golden Sample e identificación de componentes",
                "• Totalmente aislado del osciloscopio con restauración segura de relés",
            ],
            SuiteWorkstation::CommsMonitor => &[
                "• Vista combinada: Onda física + Tabla de tráfico agrupada por ID",
                "• Diagnóstico automático ISO 11898 (Niveles recesivo/dominante y V_diff)",
                "• Decodificación en tiempo real CAN, LIN, UART, I²C, SPI y 1-Wire",
                "• Resaltado dinámico de bytes cambiantes en el payload",
            ],
            SuiteWorkstation::ActuatorsLoop => &[
                "• Correlación directa Duty Cycle PWM (%) vs Tensión de respuesta (V)",
                "• Detección automática de trabamiento mecánico / histéresis excesiva",
                "• Presets para Cuerpo de Mariposa (ETC), EGR, Turbo VGT, VVT e IAC",
                "• Vista combinada de curva de transferencia X-Y y señales temporales",
            ],
            SuiteWorkstation::AutomotiveTests => &[
                "• Carga y Arranque (12V/24V), Compresión Relativa y Rizado de Alternador",
                "• Sincronismo CKP + CMP y Regla Angular de Ciclo 720° (4 Carreras)",
                "• Guía de conexión paso a paso y señal patrón esperada por componente",
                "• Puente directo de 1 clic para enviar la captura al Osciloscopio completo",
            ],
        }
    }
}

/// Sub-vista dentro de la Estación 3 (Comunicaciones CAN / LIN)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommsViewLayout {
    #[default]
    CombinedSplit,
    PhysicalWaveformOnly,
    TrafficTableOnly,
}

/// Sub-vista dentro de la Estación 4 (Actuadores y Sensores en Lazo Cerrado)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActuatorViewLayout {
    #[default]
    TransferCurveXy,
    SplitCurveAndWaveforms,
}

/// Perfil de trabajo aislado por cada Estación de la Suite
#[derive(Debug, Clone)]
pub struct ModuleWorkspaceProfile {
    pub scope_config: neowon_backend::ScopeConfig,
    pub dsp: [ChannelDspConfig; 2],
    pub ch_ext: [picowon_auto::ChannelExtendedState; 2],
    pub math_enabled: bool,
    pub math_op: neowon_dsp::MathOp,
    pub strip_chart_active: bool,
    pub initialized: bool,
}

impl ModuleWorkspaceProfile {
    pub fn default_for(ws: SuiteWorkstation, base_cfg: &neowon_backend::ScopeConfig) -> Self {
        let mut cfg = base_cfg.clone();
        let mut math_enabled = false;
        let mut math_op = neowon_dsp::MathOp::Sub;
        let mut ch_ext = [
            picowon_auto::ChannelExtendedState::default(),
            picowon_auto::ChannelExtendedState::default(),
        ];

        match ws {
            SuiteWorkstation::ScopeDatalogger => {
                // Mantiene la configuración de osciloscopio general del usuario
            }
            SuiteWorkstation::ViTracer => {
                cfg.channels[0].enabled = true;
                cfg.channels[0].volts_div = 2.0;
                cfg.channels[0].probe = 1.0;
                cfg.channels[1].enabled = true;
                cfg.channels[1].volts_div = 2.0;
                cfg.channels[1].probe = 1.0;
                cfg.roll = false;
                ch_ext[0].alias = "Tensión DUT".into();
                ch_ext[1].alias = "Corriente Rs".into();
            }
            SuiteWorkstation::CommsMonitor => {
                // Preconfigurado para Bus CAN Diferencial (CH1 = CAN_H, CH2 = CAN_L, 1 V/div, 20 µs/div)
                cfg.channels[0].enabled = true;
                cfg.channels[0].volts_div = 1.0;
                cfg.channels[0].probe = 1.0;
                cfg.channels[0].coupling = neowon_core::Coupling::Dc;
                cfg.channels[0].offset = 0.0;

                cfg.channels[1].enabled = true;
                cfg.channels[1].volts_div = 1.0;
                cfg.channels[1].probe = 1.0;
                cfg.channels[1].coupling = neowon_core::Coupling::Dc;
                cfg.channels[1].offset = 0.0;

                cfg.sample_rate = 12_500_000.0; // ~20 µs/div en 20 divs
                cfg.trigger.source = 0;
                cfg.trigger.level = 2.8;
                math_enabled = true;
                math_op = neowon_dsp::MathOp::Sub;
                ch_ext[0].alias = "CAN_H".into();
                ch_ext[1].alias = "CAN_L".into();
            }
            SuiteWorkstation::ActuatorsLoop => {
                // CH1: Mando PWM 12V (5 V/div), CH2: Sensor Posición 0-5V (1 V/div)
                cfg.channels[0].enabled = true;
                cfg.channels[0].volts_div = 5.0;
                cfg.channels[0].probe = 1.0;
                cfg.channels[0].coupling = neowon_core::Coupling::Dc;

                cfg.channels[1].enabled = true;
                cfg.channels[1].volts_div = 1.0;
                cfg.channels[1].probe = 1.0;
                cfg.channels[1].coupling = neowon_core::Coupling::Dc;

                cfg.sample_rate = 500_000.0; // 500 µs/div para ver trenes PWM de 100 Hz - 2 kHz
                cfg.trigger.source = 0;
                cfg.trigger.level = 6.0;
                ch_ext[0].alias = "Mando PWM".into();
                ch_ext[1].alias = "Sensor Pos".into();
            }
            SuiteWorkstation::AutomotiveTests => {
                // CH1 y CH2 listos en escala segura para pruebas guiadas
                cfg.channels[0].enabled = true;
                cfg.channels[0].volts_div = 5.0;
                cfg.channels[1].enabled = true;
                cfg.channels[1].volts_div = 5.0;
            }
        }

        Self {
            scope_config: cfg,
            dsp: [ChannelDspConfig::default(), ChannelDspConfig::default()],
            ch_ext,
            math_enabled,
            math_op,
            strip_chart_active: false,
            initialized: ws == SuiteWorkstation::ScopeDatalogger,
        }
    }
}

/// Ficha Técnica del Vehículo Activo en Taller (Cap. 10.2 Vehicle Details)
/// Se comparte transversalmente entre las 5 Estaciones de la Suite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VehicleDetails {
    pub plate_or_vin: String,
    pub make: String,
    pub model: String,
    pub year: String,
    pub engine_code: String,
    pub displacement_l: String,
    pub cylinders: u8,
    pub primary_fuel: String,
    pub transmission: String,
    pub ecu_make_model: String,
    pub test_condition: String,
    pub work_order: String,
    pub customer_or_notes: String,
}

impl Default for VehicleDetails {
    fn default() -> Self {
        Self {
            plate_or_vin: String::new(),
            make: String::new(),
            model: String::new(),
            year: String::new(),
            engine_code: String::new(),
            displacement_l: String::new(),
            cylinders: 4,
            primary_fuel: "Nafta / Gasolina".to_string(),
            transmission: "Manual".to_string(),
            ecu_make_model: String::new(),
            test_condition: "Ralentí (KOER)".to_string(),
            work_order: String::new(),
            customer_or_notes: String::new(),
        }
    }
}

impl VehicleDetails {
    pub fn is_empty(&self) -> bool {
        self.plate_or_vin.trim().is_empty()
            && self.make.trim().is_empty()
            && self.model.trim().is_empty()
            && self.engine_code.trim().is_empty()
    }

    pub fn summary_badge(&self) -> String {
        if self.is_empty() {
            "Sin vehículo asignado (Clic para completar ficha)".to_string()
        } else {
            let mut parts = Vec::new();
            if !self.make.trim().is_empty() || !self.model.trim().is_empty() {
                parts.push(format!("{} {}", self.make.trim(), self.model.trim()).trim().to_string());
            }
            if !self.year.trim().is_empty() {
                parts.push(self.year.trim().to_string());
            }
            if !self.engine_code.trim().is_empty() {
                parts.push(format!("Motor {}", self.engine_code.trim()));
            }
            if !self.plate_or_vin.trim().is_empty() {
                parts.push(format!("[{}]", self.plate_or_vin.trim()));
            }
            parts.join(" · ")
        }
    }

    pub fn filename_tag(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let raw = format!("{}_{}_{}", self.make.trim(), self.model.trim(), self.plate_or_vin.trim());
        let clean: String = raw
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        clean.trim_matches('_').to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentVehiclesStore {
    pub show_launcher_on_startup: bool,
    pub active: VehicleDetails,
    pub recents: Vec<VehicleDetails>,
}

impl RecentVehiclesStore {
    const FILE_PATH: &'static str = "Capturas/suite_vehicle_context.json";

    pub fn load_or_default() -> Self {
        if let Ok(txt) = std::fs::read_to_string(Self::FILE_PATH) {
            if let Ok(parsed) = serde_json::from_str::<Self>(&txt) {
                return parsed;
            }
        }
        Self {
            show_launcher_on_startup: true,
            active: VehicleDetails::default(),
            recents: Vec::new(),
        }
    }

    pub fn save(&self) {
        if let Some(parent) = std::path::Path::new(Self::FILE_PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::FILE_PATH, json);
        }
    }

    pub fn commit_active_to_recents(&mut self) {
        if self.active.is_empty() {
            self.save();
            return;
        }
        self.recents.retain(|v| {
            !(v.plate_or_vin == self.active.plate_or_vin
                && v.make == self.active.make
                && v.model == self.active.model)
        });
        self.recents.insert(0, self.active.clone());
        if self.recents.len() > 15 {
            self.recents.truncate(15);
        }
        self.save();
    }
}

/// Estado Macro de la Suite dentro de `AutomotiveUiState`
pub struct SuiteState {
    pub active_workstation: SuiteWorkstation,
    pub launcher_open: bool,
    pub vehicle_dialog_open: bool,
    pub device_dialog_open: bool,
    pub vehicle_store: RecentVehiclesStore,
    pub profiles: Option<[ModuleWorkspaceProfile; 5]>,
    pub comms_layout: CommsViewLayout,
    pub actuator_layout: ActuatorViewLayout,
    pub last_window_title: String,
    /// Herramientas favoritas por estación (ids de `ToolDef`), mostradas en la columna izquierda.
    pub favorites: [Vec<String>; 5],
    /// Solicitud diferida de alternar la Galería del Búfer (el historial vive fuera del panel izquierdo).
    pub pending_gallery_toggle: bool,
}

impl Default for SuiteState {
    fn default() -> Self {
        let store = RecentVehiclesStore::load_or_default();
        let launcher_open = store.show_launcher_on_startup;
        Self {
            active_workstation: SuiteWorkstation::ScopeDatalogger,
            launcher_open,
            vehicle_dialog_open: false,
            device_dialog_open: false,
            vehicle_store: store,
            profiles: None,
            comms_layout: CommsViewLayout::CombinedSplit,
            actuator_layout: ActuatorViewLayout::SplitCurveAndWaveforms,
            last_window_title: String::new(),
            favorites: load_favorites(),
            pending_gallery_toggle: false,
        }
    }
}

// ───────────────────────── Catálogo "More..." y Favoritos ─────────────────────────

/// Definición de una función/ajuste disponible en el panel "More..." (estilo PicoScope 7).
#[derive(Debug, Clone, Copy)]
pub struct ToolDef {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

const fn tool(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    category: &'static str,
    description: &'static str,
) -> ToolDef {
    ToolDef {
        id,
        icon,
        label,
        category,
        description,
    }
}

const CAT_SETUP: &str = "Configuración y ajustes";
const CAT_CAPTURE: &str = "Captura, importación y exportación";
const CAT_EVAL: &str = "Evaluación de formas de onda";
const CAT_HELP: &str = "Ayuda y soporte";

/// Todas las funciones disponibles para una estación (no incluye las de otras estaciones).
pub fn tools_for(ws: SuiteWorkstation) -> Vec<ToolDef> {
    let mut v = vec![
        tool("connect", "🔌", "Conectar", CAT_SETUP, "Conectar y detectar hardware VDS1022"),
        tool("vehicle", "🚗", "Vehículo", CAT_SETUP, "Ficha del vehículo, motor, VIN y notas del cliente"),
        tool("ch1", "🅰", "Canal 1 (A)", CAT_SETUP, "Ajustes de acoplamiento, escala V/div y atenuación de CH1"),
        tool("ch2", "🅱", "Canal 2 (B)", CAT_SETUP, "Ajustes de acoplamiento, escala V/div y atenuación de CH2"),
        tool("probes", "🔧", "Sondas", CAT_SETUP, "Sondas atenuadoras, pinzas amperimétricas y transductores de presión"),
        tool("horizontal", "⏱", "Base Tiempo", CAT_SETUP, "Base de tiempo por división, posición de disparo y frecuencia de muestreo"),
        tool("trigger", "⚡", "Trigger", CAT_SETUP, "Disparo por flanco, nivel, modo (Auto/Norm/Single) y fuente"),
    ];
    match ws {
        SuiteWorkstation::ScopeDatalogger => {
            v.push(tool("acquire", "📥", "Adquisición", CAT_SETUP, "Modo de muestreo, detección de picos (Peak) y promedio"));
            v.push(tool("display", "🖥", "Pantalla", CAT_SETUP, "Persistencia de fósforo digital, rejilla y colores de onda"));
            v.push(tool("capture", "📸", "Capturar", CAT_CAPTURE, "Guardar imagen PNG o exportar datos CSV de la pantalla"));
            v.push(tool("load", "📂", "Cargar", CAT_CAPTURE, "Cargar capturas de osciloscopio o sesiones históricas"));
            v.push(tool("record", "🎙", "Búfer/Export", CAT_CAPTURE, "Búfer continuo circular y exportación de archivos .nwc/.csv"));
            v.push(tool("gallery", "🖼", "Galería", CAT_CAPTURE, "Explorar miniaturas de formas de onda capturadas en el búfer"));
            v.push(tool("measure", "📊", "Mediciones", CAT_EVAL, "Tabla flotante de mediciones automáticas en tiempo real (Vpp, Freq, RMS)"));
            v.push(tool("rulers", "📏", "Reglas", CAT_EVAL, "Reglas manuales de tiempo y cursor de tensión en pantalla"));
            v.push(tool("math", "∑", "Matemática", CAT_EVAL, "Canales matemáticos de suma, resta, producto y FFT espectral"));
            v.push(tool("masks", "🎭", "Máscaras", CAT_EVAL, "Máscaras de tolerancia Pasa/Falla y acciones automáticas por evento"));
            v.push(tool("zoom", "🔍", "Zoom", CAT_EVAL, "Ventana de lupa y zoom horizontal detallado"));
            v.push(tool("datalog", "📈", "Datalogger", CAT_EVAL, "Modo registrador gráfico continuo de tendencias temporales lentas"));
        }
        SuiteWorkstation::CommsMonitor => {
            v.push(tool("capture", "📸", "Capturar", CAT_CAPTURE, "Guardar imagen o trama decodificada"));
            v.push(tool("load", "📂", "Cargar", CAT_CAPTURE, "Cargar archivos de tráfico o link files"));
            v.push(tool("decode", "📡", "Decodificador", CAT_EVAL, "Decodificación serie de buses CAN, CAN FD, LIN, SENT, UART, SPI, I2C"));
            v.push(tool("math", "∑", "Matemática", CAT_EVAL, "Operaciones matemáticas entre señales de comunicación"));
            v.push(tool("diff_ab", "∑", "Diferencial", CAT_EVAL, "Cálculo diferencial CH1 - CH2 (ej. CAN_H - CAN_L)"));
            v.push(tool("zoom", "🔍", "Zoom", CAT_EVAL, "Exploración ampliada de tramas y transiciones de bit"));
        }
        SuiteWorkstation::ActuatorsLoop => {
            v.push(tool("capture", "📸", "Capturar", CAT_CAPTURE, "Guardar curva de histéresis y lazo del actuador"));
            v.push(tool("load", "📂", "Cargar", CAT_CAPTURE, "Cargar ensayos previos y curva dorada"));
            v.push(tool("measure", "📊", "Mediciones", CAT_EVAL, "Mediciones automotrices de ciclo PWM y retardo de respuesta"));
            v.push(tool("rulers", "📏", "Reglas", CAT_EVAL, "Reglas para medir umbrales de histéresis"));
            v.push(tool("zoom", "🔍", "Zoom", CAT_EVAL, "Zoom sobre la curva de transferencia X-Y"));
        }
        SuiteWorkstation::AutomotiveTests => {
            v.push(tool("capture", "📸", "Capturar", CAT_CAPTURE, "Guardar reporte de prueba guiada del vehículo"));
            v.push(tool("load", "📂", "Cargar", CAT_CAPTURE, "Cargar plantilla o captura de referencia"));
            v.push(tool("guide", "📋", "Guía Test", CAT_EVAL, "Biblioteca de pruebas guiadas por componentes (sensores, inyectores, encendido)"));
            v.push(tool("ruler720", "📐", "Regla 720°", CAT_EVAL, "Regla de grados de cigüeñal 720° con particiones de 4 tiempos"));
            v.push(tool("measure", "📊", "Mediciones", CAT_EVAL, "Valores característicos del componente en prueba"));
            v.push(tool("rulers", "📏", "Reglas", CAT_EVAL, "Reglas de tiempo y tensión"));
            v.push(tool("zoom", "🔍", "Zoom", CAT_EVAL, "Zoom sobre pulsos y eventos de encendido/inyección"));
        }
        SuiteWorkstation::ViTracer => {}
    }
    v.push(tool("copy_text", "📋", "Copiar Texto", CAT_CAPTURE, "Copia al portapapeles el resumen de mediciones y telemetría en formato texto"));
    v.push(tool("annotations", "🖊", "Anotaciones", CAT_EVAL, "Colocar notas de texto y marcadores sobre las formas de onda"));
    v.push(tool("launcher", "🏠", "Estaciones", CAT_HELP, "Cambiar entre los 5 módulos de trabajo"));
    v.push(tool("help", "❓", "Ayuda", CAT_HELP, "Manual de usuario y atajos de teclado"));
    v
}

fn default_favorites(ws: SuiteWorkstation) -> Vec<String> {
    let ids: &[&str] = match ws {
        SuiteWorkstation::ScopeDatalogger => {
            &["gallery", "measure", "rulers", "math", "datalog", "record", "zoom", "masks"]
        }
        SuiteWorkstation::CommsMonitor => &["decode", "diff_ab"],
        SuiteWorkstation::ActuatorsLoop => &[],
        SuiteWorkstation::AutomotiveTests => &["guide", "ruler720", "measure", "rulers"],
        SuiteWorkstation::ViTracer => &[],
    };
    ids.iter().map(|s| s.to_string()).collect()
}

const FAVORITES_FILE: &str = "Capturas/favorite_tools.json";

fn load_favorites() -> [Vec<String>; 5] {
    let defaults = || SuiteWorkstation::ALL.map(default_favorites);
    match std::fs::read_to_string(FAVORITES_FILE)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Vec<String>>>(&t).ok())
    {
        Some(v) if v.len() == 5 => {
            let mut it = v.into_iter();
            std::array::from_fn(|_| it.next().unwrap_or_default())
        }
        _ => defaults(),
    }
}

fn save_favorites(fav: &[Vec<String>; 5]) {
    let _ = std::fs::create_dir_all("Capturas");
    if let Ok(json) = serde_json::to_string_pretty(&fav.to_vec()) {
        let _ = std::fs::write(FAVORITES_FILE, json);
    }
}

/// ¿El ajuste está activo (resaltado) ahora mismo?
pub fn tool_active(id: &str, math: &MathState, menus: &MenuState, auto: &AutomotiveUiState) -> bool {
    match id {
        "ch1" => menus.is_open(Menu::Channel(0)),
        "ch2" => menus.is_open(Menu::Channel(1)),
        "probes" => auto.probe_dialog_ch.is_some(),
        "horizontal" => menus.is_open(Menu::Horizontal),
        "trigger" => menus.is_open(Menu::Trigger),
        "acquire" => menus.is_open(Menu::Acquire),
        "display" => menus.is_open(Menu::Display),
        "record" => menus.is_open(Menu::Record),
        "measure" => menus.is_open(Menu::Measure),
        "rulers" => menus.is_open(Menu::Cursor),
        "math" => menus.is_open(Menu::Math),
        "masks" => menus.is_open(Menu::Utility),
        "decode" => menus.is_open(Menu::Decode),
        "annotations" => menus.is_open(Menu::Annotations),
        "guide" => menus.is_open(Menu::Automotive),
        "zoom" => auto.zoom_window_open,
        "datalog" => auto.strip_chart_view_active,
        "ruler720" => auto.ruler.enabled,
        "diff_ab" => math.enabled && math.op == neowon_dsp::MathOp::Sub,
        "vehicle" => auto.suite.vehicle_dialog_open,
        "connect" => auto.suite.device_dialog_open,
        "help" => auto.help_window_open,
        _ => false,
    }
}

/// Ejecuta la función asociada al id (la misma acción desde "More..." o desde un favorito).
pub fn run_tool(
    ui: &mut egui::Ui,
    id: &str,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) {
    let _ = link;
    match id {
        "connect" => auto.suite.device_dialog_open = true,
        "vehicle" => auto.suite.vehicle_dialog_open = true,
        "ch1" => menus.toggle(Menu::Channel(0)),
        "ch2" => menus.toggle(Menu::Channel(1)),
        "probes" => {
            auto.probe_dialog_ch = if auto.probe_dialog_ch.is_some() { None } else { Some(0) }
        }
        "horizontal" => menus.toggle(Menu::Horizontal),
        "trigger" => menus.toggle(Menu::Trigger),
        "acquire" => menus.toggle(Menu::Acquire),
        "display" => menus.toggle(Menu::Display),
        "capture" => auto.scope_capture.dialog_open = true,
        "load" => {
            auto.capture_browser_open = true;
            auto.capture_browser_tab = 0;
        }
        "record" => menus.toggle(Menu::Record),
        "gallery" => auto.suite.pending_gallery_toggle = true,
        "measure" => menus.toggle(Menu::Measure),
        "rulers" => menus.toggle(Menu::Cursor),
        "math" => menus.toggle(Menu::Math),
        "masks" => menus.toggle(Menu::Utility),
        "zoom" => auto.zoom_window_open = !auto.zoom_window_open,
        "datalog" => {
            auto.strip_chart_view_active = !auto.strip_chart_view_active;
            auto.app_mode = if auto.strip_chart_view_active {
                AppMode::Datalogger
            } else {
                AppMode::Oscilloscope
            };
        }
        "decode" => menus.toggle(Menu::Decode),
        "annotations" => menus.toggle(Menu::Annotations),
        "diff_ab" => {
            math.enabled = !math.enabled;
            math.op = neowon_dsp::MathOp::Sub;
            math.rescale = true;
        }
        "guide" => menus.toggle(Menu::Automotive),
        "ruler720" => auto.ruler.enabled = !auto.ruler.enabled,
        "launcher" => auto.suite.launcher_open = true,
        "help" => auto.help_window_open = !auto.help_window_open,
        "copy_text" => {
            // Generar informe de texto del estado actual de canales y vehículo
            let veh = &auto.suite.vehicle_store.active;
            let mut report = format!(
                "=== PICOWON AUTOMOTIVE REPORT ===\nVehículo: {} {} (Año: {}, VIN/Patente: {})\nEstación: {}\n\n",
                veh.make, veh.model, veh.year, veh.plate_or_vin,
                auto.suite.active_workstation.card_title(),
            );
            report.push_str("--- CANALES ---\n");
            for ch in 0..2 {
                let c = link.config.channels[ch];
                let alias = auto.ch_ext[ch].alias.trim();
                let spec = &auto.ch_ext[ch].probe_spec;
                report.push_str(&format!(
                    "CH{} [{}]: Habilitado={}, Escala={:.3} V/div, Atenuación={:.1}x, Acoplamiento={:?}\n",
                    ch + 1,
                    if alias.is_empty() { spec.short_badge.as_str() } else { alias },
                    c.enabled,
                    c.volts_div,
                    c.probe,
                    c.coupling
                ));
            }
            ui.ctx().copy_text(report);
        }
        _ => {}
    }
}

/// Cuadrícula de favoritos de la columna izquierda (2 columnas uniformes).
pub fn favorites_grid(
    ui: &mut egui::Ui,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
    col_w: f32,
    gap: f32,
    btn_h: f32,
    accent: egui::Color32,
) {
    let ws = auto.suite.active_workstation;
    let catalog = tools_for(ws);
    let favs: Vec<ToolDef> = auto.suite.favorites[ws.index()]
        .iter()
        .filter_map(|id| catalog.iter().find(|t| t.id == id).copied())
        .collect();
    for pair in favs.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for t in pair {
                let active = tool_active(t.id, math, menus, auto);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(col_w, btn_h), egui::Sense::click());
                let is_hovered = resp.hovered();

                let bg_col = if active {
                    accent
                } else if is_hovered {
                    egui::Color32::from_rgb(38, 46, 62)
                } else {
                    egui::Color32::from_rgb(26, 30, 42)
                };
                let border_col = if active {
                    accent
                } else if is_hovered {
                    egui::Color32::from_rgb(90, 130, 185)
                } else {
                    egui::Color32::from_rgb(52, 60, 78)
                };

                let p = ui.painter();
                p.rect_filled(rect, 4.0, bg_col);
                p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border_col), egui::StrokeKind::Inside);

                // Icono visible grande arriba y texto en miniatura abajo
                let icon_col = if active {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::from_rgb(255, 230, 140)
                };
                let text_col = if active {
                    egui::Color32::from_rgb(20, 20, 20)
                } else {
                    egui::Color32::from_rgb(220, 230, 245)
                };

                p.text(
                    egui::pos2(rect.center().x, rect.top() + 11.0),
                    egui::Align2::CENTER_CENTER,
                    t.icon,
                    egui::FontId::proportional(14.0),
                    icon_col,
                );
                p.text(
                    egui::pos2(rect.center().x, rect.bottom() - 8.0),
                    egui::Align2::CENTER_CENTER,
                    t.label,
                    egui::FontId::proportional(8.5),
                    text_col,
                );

                let resp = resp.on_hover_ui(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(t.icon).size(16.0));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(t.label).strong().size(11.5).color(egui::Color32::from_rgb(108, 185, 245)));
                            ui.label(egui::RichText::new(t.description).size(10.0).color(egui::Color32::from_gray(215)));
                        });
                    });
                });

                if resp.clicked() {
                    run_tool(ui, t.id, link, math, menus, auto);
                }
            }
        });
        ui.add_space(3.0);
    }
    // Botón "More..." que despliega la columna adyacente con todas las funciones
    let more_active = menus.is_open(Menu::More);
    let more = egui::Button::new(
        egui::RichText::new("⊞ More… (Todas las funciones)")
            .size(10.5)
            .strong()
            .color(if more_active {
                egui::Color32::BLACK
            } else {
                egui::Color32::from_rgb(190, 225, 255)
            }),
    )
    .fill(if more_active {
        egui::Color32::from_rgb(108, 185, 245)
    } else {
        egui::Color32::from_rgb(24, 40, 62)
    })
    .stroke(egui::Stroke::new(if more_active { 1.5 } else { 1.0 }, egui::Color32::from_rgb(88, 168, 238)))
    .corner_radius(4.0);
    if ui
        .add_sized(egui::vec2(col_w * 2.0 + gap, btn_h + 2.0), more)
        .on_hover_ui(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⊞").size(18.0).color(egui::Color32::from_rgb(108, 185, 245)));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("More... / Catálogo Completo").strong().size(11.5).color(egui::Color32::from_rgb(108, 185, 245)));
                    ui.label(egui::RichText::new("Abre la columna adyacente con todas las herramientas organizadas por categoría y estrellas ★ para favoritas").size(10.0).color(egui::Color32::from_gray(215)));
                });
            });
        })
        .clicked()
    {
        menus.toggle(Menu::More);
    }
}

/// Panel "More..." de la columna adyacente: todas las funciones agrupadas por categoría,
/// con estrella para marcarlas como favoritas (se muestran en la columna izquierda).
pub fn show_more_tools_panel(
    ui: &mut egui::Ui,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) {
    let ws = auto.suite.active_workstation;
    let accent = ws.accent_color();
    let catalog = tools_for(ws);
    let mut categories: Vec<&str> = Vec::new();
    for t in &catalog {
        if !categories.contains(&t.category) {
            categories.push(t.category);
        }
    }
    let mut fav_changed = false;
    let mut to_run: Option<&'static str> = None;

    for cat in categories {
        egui::CollapsingHeader::new(egui::RichText::new(cat).strong().size(11.5))
            .default_open(true)
            .show(ui, |ui| {
                let items: Vec<ToolDef> = catalog.iter().filter(|t| t.category == cat).copied().collect();
                for pair in items.chunks(2) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for t in pair {
                            let is_fav = auto.suite.favorites[ws.index()].iter().any(|f| f == t.id);
                            let star = egui::Button::new(
                                egui::RichText::new(if is_fav { "★" } else { "☆" })
                                    .size(14.0)
                                    .color(if is_fav {
                                        egui::Color32::from_rgb(255, 205, 60)
                                    } else {
                                        egui::Color32::from_gray(150)
                                    }),
                            )
                            .frame(false);
                            if ui
                                .add_sized(egui::vec2(20.0, 44.0), star)
                                .on_hover_text(if is_fav {
                                    "Quitar de favoritas"
                                } else {
                                    "Agregar a favoritas"
                                })
                                .clicked()
                            {
                                let fav = &mut auto.suite.favorites[ws.index()];
                                if let Some(i) = fav.iter().position(|f| f == t.id) {
                                    fav.remove(i);
                                } else {
                                    fav.push(t.id.to_string());
                                }
                                fav_changed = true;
                            }
                            let active = tool_active(t.id, math, menus, auto);
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(78.0, 46.0), egui::Sense::click());
                            let is_hovered = resp.hovered();

                            let bg_col = if active {
                                accent
                            } else if is_hovered {
                                egui::Color32::from_rgb(32, 40, 56)
                            } else {
                                egui::Color32::from_rgb(22, 28, 40)
                            };
                            let border_col = if active {
                                accent
                            } else if is_hovered {
                                egui::Color32::from_rgb(88, 128, 175)
                            } else {
                                egui::Color32::from_rgb(52, 64, 84)
                            };

                            let p = ui.painter();
                            p.rect_filled(rect, 4.0, bg_col);
                            p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border_col), egui::StrokeKind::Inside);

                            let icon_col = if active {
                                egui::Color32::BLACK
                            } else {
                                egui::Color32::from_rgb(255, 230, 140)
                            };
                            let text_col = if active {
                                egui::Color32::from_rgb(20, 20, 20)
                            } else {
                                egui::Color32::from_rgb(215, 228, 245)
                            };

                            p.text(
                                egui::pos2(rect.center().x, rect.top() + 13.0),
                                egui::Align2::CENTER_CENTER,
                                t.icon,
                                egui::FontId::proportional(15.0),
                                icon_col,
                            );
                            p.text(
                                egui::pos2(rect.center().x, rect.bottom() - 9.0),
                                egui::Align2::CENTER_CENTER,
                                t.label,
                                egui::FontId::proportional(8.5),
                                text_col,
                            );

                            let resp = resp.on_hover_ui(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(t.icon).size(16.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(t.label).strong().size(11.5).color(egui::Color32::from_rgb(108, 185, 245)));
                                        ui.label(egui::RichText::new(t.description).size(10.0).color(egui::Color32::from_gray(215)));
                                    });
                                });
                            });

                            if resp.clicked() {
                                to_run = Some(t.id);
                            }
                            ui.add_space(4.0);
                        }
                    });
                    ui.add_space(2.0);
                }
            });
    }
    if fav_changed {
        save_favorites(&auto.suite.favorites);
    }
    if let Some(id) = to_run {
        run_tool(ui, id, link, math, menus, auto);
        // La selección de un ajuste reemplaza la columna adyacente; "More" se reabre con su botón.
    }
}

impl SuiteState {
    pub fn ensure_profiles(&mut self, base_cfg: &neowon_backend::ScopeConfig) {
        if self.profiles.is_none() {
            self.profiles = Some([
                ModuleWorkspaceProfile::default_for(SuiteWorkstation::ScopeDatalogger, base_cfg),
                ModuleWorkspaceProfile::default_for(SuiteWorkstation::ViTracer, base_cfg),
                ModuleWorkspaceProfile::default_for(SuiteWorkstation::CommsMonitor, base_cfg),
                ModuleWorkspaceProfile::default_for(SuiteWorkstation::ActuatorsLoop, base_cfg),
                ModuleWorkspaceProfile::default_for(SuiteWorkstation::AutomotiveTests, base_cfg),
            ]);
        }
    }
}

/// Conmuta de forma segura entre cualquiera de las 5 Estaciones de la Suite,
/// guardando el perfil de la estación saliente y restaurando el perfil propio de la entrante.
pub fn switch_workstation(
    auto: &mut AutomotiveUiState,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    target: SuiteWorkstation,
) {
    auto.suite.ensure_profiles(&link.config);
    let current = auto.suite.active_workstation;

    // 1. Guardar estado actual en el perfil de la estación saliente
    if let Some(ref mut profiles) = auto.suite.profiles {
        let cur_idx = current.index();
        profiles[cur_idx].scope_config = link.config.clone();
        profiles[cur_idx].dsp = auto.dsp;
        profiles[cur_idx].ch_ext = auto.ch_ext.clone();
        profiles[cur_idx].math_enabled = math.enabled;
        profiles[cur_idx].math_op = math.op;
        profiles[cur_idx].strip_chart_active = auto.strip_chart_view_active;
        profiles[cur_idx].initialized = true;
    }

    // 2. Restaurar perfil de la estación entrante si es distinta
    if current != target {
        if let Some(ref mut profiles) = auto.suite.profiles {
            let tgt_idx = target.index();
            let prof = &profiles[tgt_idx];
            link.config = prof.scope_config.clone();
            auto.dsp = prof.dsp;
            auto.ch_ext = prof.ch_ext.clone();
            math.enabled = prof.math_enabled;
            math.op = prof.math_op;
            math.rescale = true;
            link.dirty = true;
        }
    }

    auto.suite.active_workstation = target;
    auto.suite.launcher_open = false;

    // 3. Limpiar cualquier panel lateral (dock) perteneciente a otra estación para que cada estación sea una ventana limpia
    menus.set_exclusive(None);
    auto.dsp_floating_ch = None;
    auto.probe_dialog_ch = None;

    // 4. Sincronizar AppMode y abrir únicamente el panel propio si corresponde
    match target {
        SuiteWorkstation::ScopeDatalogger => {
            let sc = auto
                .suite
                .profiles
                .as_ref()
                .map(|p| p[0].strip_chart_active)
                .unwrap_or(false);
            auto.strip_chart_view_active = sc;
            auto.app_mode = if sc {
                AppMode::Datalogger
            } else {
                AppMode::Oscilloscope
            };
        }
        SuiteWorkstation::ViTracer => {
            auto.strip_chart_view_active = false;
            auto.app_mode = AppMode::ViCurveTracer;
            let eq = auto.vi_tracer.equipment_store.active_equipment().clone();
            crate::ui::automotive_dock::apply_tracer_equipment_to_link(link, auto, &eq);
        }
        SuiteWorkstation::CommsMonitor => {
            auto.strip_chart_view_active = false;
            auto.app_mode = match auto.suite.comms_layout {
                CommsViewLayout::TrafficTableOnly => AppMode::CanLinTraffic,
                _ => AppMode::CanLinWaveform,
            };
            menus.open(Menu::Decode);
        }
        SuiteWorkstation::ActuatorsLoop => {
            auto.strip_chart_view_active = false;
            auto.app_mode = AppMode::ActuatorPwmVsSensor;
        }
        SuiteWorkstation::AutomotiveTests => {
            auto.strip_chart_view_active = false;
            auto.app_mode = AppMode::Oscilloscope;
            menus.open(Menu::Automotive);
        }
    }
}

/// Transfiere la captura actual desde cualquier módulo especializado (ej. Testeos Automotrices,
/// Comunicaciones o Actuadores) hacia la Estación 1 (Osciloscopio + Datalogger) conservando
/// las escalas y datos actuales para análisis profundo de laboratorio.
pub fn transfer_capture_to_scope_workstation(
    auto: &mut AutomotiveUiState,
    link: &mut Link,
    math: &mut MathState,
) {
    auto.suite.ensure_profiles(&link.config);
    if let Some(ref mut profiles) = auto.suite.profiles {
        // Copiar la configuración de adquisición actual al perfil del Osciloscopio para ver exactamente la misma onda
        profiles[0].scope_config = link.config.clone();
        profiles[0].dsp = auto.dsp;
        profiles[0].ch_ext = auto.ch_ext.clone();
        profiles[0].math_enabled = math.enabled;
        profiles[0].math_op = math.op;
        profiles[0].strip_chart_active = false;
        profiles[0].initialized = true;
    }
    auto.suite.active_workstation = SuiteWorkstation::ScopeDatalogger;
    auto.suite.launcher_open = false;
    auto.strip_chart_view_active = false;
    auto.app_mode = AppMode::Oscilloscope;
    auto.banner_message = Some("🔬 Captura transferida al Osciloscopio de Laboratorio para análisis detallado.".into());
}

/// Dibuja el Hub Principal de Selección de la Suite ("PicoWon Automotive Suite Launcher")
pub fn show_suite_launcher_modal(
    ctx: &egui::Context,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    settings: &mut crate::ui::settings::Settings,
    auto: &mut AutomotiveUiState,
) {
    if !auto.suite.launcher_open {
        return;
    }

    let screen = ctx.input(|i| i.viewport_rect());
    // Fondo translúcido oscuro de pantalla completa
    egui::Area::new("suite_launcher_backdrop".into())
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(6, 8, 12, 232),
            );
        });

    let panel_w = (screen.width() - 48.0).clamp(680.0, 1240.0);
    let panel_h = (screen.height() - 44.0).clamp(480.0, 820.0);
    let panel_pos = egui::pos2(
        screen.center().x - panel_w * 0.5,
        screen.center().y - panel_h * 0.5,
    );

    egui::Area::new("suite_launcher_hub".into())
        .order(egui::Order::Tooltip)
        .fixed_pos(panel_pos)
        .show(ctx, |ui| {
            ui.set_width(panel_w);
            ui.set_height(panel_h);

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(15, 18, 25))
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(65, 85, 120)))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    // =========================================================
                    // 1. CABECERA GLOBAL DE LA SUITE
                    // =========================================================
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("🚗 PICOWON AUTOMOTIVE SUITE")
                                        .size(20.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(255, 195, 50)),
                                );
                                ui.label(
                                    egui::RichText::new("v0.3 · Arquitectura Modular de Diagnóstico")
                                        .size(11.5)
                                        .color(egui::Color32::from_rgb(130, 175, 220)),
                                );
                            });
                            ui.label(
                                egui::RichText::new(
                                    "Seleccione una Estación de Trabajo independiente. Todas las estaciones comparten el motor de captura, seguridad de relés y ficha del vehículo.",
                                )
                                .size(11.5)
                                .color(egui::Color32::from_gray(180)),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cur_ws = auto.suite.active_workstation;
                            let close_btn = egui::Button::new(
                                egui::RichText::new(format!("✕ Volver a {}", cur_ws.short_title()))
                                    .size(11.5)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(42, 52, 70))
                            .min_size(egui::vec2(180.0, 30.0));
                            if ui.add(close_btn).clicked() {
                                auto.suite.launcher_open = false;
                            }

                            if ui
                                .add_sized(
                                    [120.0, 30.0],
                                    egui::Button::new("⚙ Preferencias").fill(egui::Color32::from_rgb(28, 34, 46)),
                                )
                                .clicked()
                            {
                                settings.open = true;
                                auto.suite.launcher_open = false;
                            }

                            if ui
                                .add_sized(
                                    [155.0, 30.0],
                                    egui::Button::new("📂 Biblioteca Capturas").fill(egui::Color32::from_rgb(28, 46, 68)),
                                )
                                .clicked()
                            {
                                auto.capture_browser_open = true;
                                auto.suite.launcher_open = false;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // =========================================================
                    // 2. BARRA TRANSVERSAL: FICHA DEL VEHÍCULO + ESTADO HARDWARE
                    // =========================================================
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(22, 27, 38))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 65, 90)))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("🚘 VEHÍCULO EN TALLER:")
                                        .size(11.5)
                                        .strong()
                                        .color(egui::Color32::from_rgb(100, 210, 255)),
                                );
                                let badge_txt = auto.suite.vehicle_store.active.summary_badge();
                                let is_empty = auto.suite.vehicle_store.active.is_empty();
                                let veh_btn = egui::Button::new(
                                    egui::RichText::new(&badge_txt)
                                        .size(11.5)
                                        .strong()
                                        .color(if is_empty {
                                            egui::Color32::from_gray(170)
                                        } else {
                                            egui::Color32::from_rgb(120, 245, 160)
                                        }),
                                )
                                .fill(egui::Color32::from_rgb(30, 38, 52));
                                if ui.add(veh_btn).on_hover_text("Abrir Ficha Técnica del Vehículo (Cap. 10.2)").clicked() {
                                    auto.suite.vehicle_dialog_open = true;
                                }

                                if ui.button("✏ Editar Ficha").clicked() {
                                    auto.suite.vehicle_dialog_open = true;
                                }

                                // Selector rápido de vehículos recientes si existen
                                if !auto.suite.vehicle_store.recents.is_empty() {
                                    let mut pick_idx = None;
                                    egui::ComboBox::from_id_salt("suite_recent_vehicles")
                                        .selected_text("🕘 Recientes...")
                                        .show_ui(ui, |ui| {
                                            for (idx, v) in auto.suite.vehicle_store.recents.iter().enumerate() {
                                                if ui.selectable_label(false, v.summary_badge()).clicked() {
                                                    pick_idx = Some(idx);
                                                }
                                            }
                                        });
                                    if let Some(idx) = pick_idx {
                                        auto.suite.vehicle_store.active = auto.suite.vehicle_store.recents[idx].clone();
                                        auto.suite.vehicle_store.save();
                                    }
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let hw_txt = if link.status.to_lowercase().contains("sim") {
                                        "🧪 Modo Simulador / Demo Activo"
                                    } else {
                                        "🔌 Hardware OWON VDS1022 USB"
                                    };
                                    ui.label(
                                        egui::RichText::new(hw_txt)
                                            .size(11.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(255, 210, 80)),
                                    );
                                });
                            });
                        });

                    ui.add_space(10.0);

                    // =========================================================
                    // 3. GRILLA DE LAS 5 ESTACIONES DE TRABAJO INDEPENDIENTES
                    // =========================================================
                    egui::ScrollArea::vertical()
                        .max_height(panel_h - 175.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let avail_w = ui.available_width();
                            let gap = 12.0;

                            // Fila 1: Las 3 primeras estaciones (Osciloscopio+Datalogger, Trazador V-I, Comunicaciones)
                            let card_w_row1 = ((avail_w - gap * 2.0) / 3.0).max(200.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for ws in [
                                    SuiteWorkstation::ScopeDatalogger,
                                    SuiteWorkstation::ViTracer,
                                    SuiteWorkstation::CommsMonitor,
                                ] {
                                    draw_workstation_card(ui, ws, card_w_row1, 235.0, link, math, menus, auto);
                                }
                            });

                            ui.add_space(gap);

                            // Fila 2: Las 2 estaciones de sistemas y ensayos guiados (Actuadores Lazo Cerrado y Testeos Automotrices)
                            let card_w_row2 = ((avail_w - gap) / 2.0).max(280.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for ws in [
                                    SuiteWorkstation::ActuatorsLoop,
                                    SuiteWorkstation::AutomotiveTests,
                                ] {
                                    draw_workstation_card(ui, ws, card_w_row2, 225.0, link, math, menus, auto);
                                }
                            });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Pie del Hub
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(
                                &mut auto.suite.vehicle_store.show_launcher_on_startup,
                                "Mostrar este selector de Suite al iniciar PicoWon",
                            )
                            .changed()
                        {
                            auto.suite.vehicle_store.save();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("Atajo rápido en cualquier momento: Botón [🏠 Suite] arriba a la izquierda o tecla F2")
                                    .size(10.5)
                                    .color(egui::Color32::from_gray(155)),
                            );
                        });
                    });
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn draw_workstation_card(
    ui: &mut egui::Ui,
    ws: SuiteWorkstation,
    width: f32,
    height: f32,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) {
    let is_current = auto.suite.active_workstation == ws;
    let accent = ws.accent_color();
    let border_col = if is_current {
        accent
    } else {
        egui::Color32::from_rgb(48, 58, 78)
    };

    egui::Frame::new()
        .fill(if is_current {
            egui::Color32::from_rgb(24, 30, 42)
        } else {
            egui::Color32::from_rgb(19, 23, 32)
        })
        .stroke(egui::Stroke::new(if is_current { 2.0 } else { 1.2 }, border_col))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width((width - 26.0).max(160.0));
            ui.set_min_height((height - 26.0).max(160.0));

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(ws.card_title())
                            .size(13.5)
                            .strong()
                            .color(accent),
                    );
                    if is_current {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("● ACTIVA")
                                    .size(10.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(100, 240, 140)),
                            );
                        });
                    }
                });

                ui.add_space(3.0);
                ui.label(
                    egui::RichText::new(ws.subtitle())
                        .size(11.0)
                        .color(egui::Color32::from_rgb(210, 218, 230)),
                );
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                for line in ws.bullet_points() {
                    ui.label(
                        egui::RichText::new(*line)
                            .size(10.5)
                            .color(egui::Color32::from_gray(175)),
                    );
                }

                ui.add_space(8.0);

                // Botones de acción primaria y sub-accesos rápidos según la estación
                ui.horizontal(|ui| {
                    let main_btn = egui::Button::new(
                        egui::RichText::new(format!("▶ ABRIR {}", ws.short_title()))
                            .size(11.5)
                            .strong()
                            .color(egui::Color32::BLACK),
                    )
                    .fill(accent)
                    .min_size(egui::vec2(170.0, 28.0));

                    if ui.add(main_btn).clicked() {
                        switch_workstation(auto, link, math, menus, ws);
                    }

                    // Botones secundarios directos por estación
                    match ws {
                        SuiteWorkstation::ScopeDatalogger => {
                            if ui
                                .add_sized(
                                    [110.0, 28.0],
                                    egui::Button::new(
                                        egui::RichText::new("📈 Ir a Datalogger")
                                            .size(10.5)
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(egui::Color32::from_rgb(35, 55, 80)),
                                )
                                .clicked()
                            {
                                switch_workstation(auto, link, math, menus, SuiteWorkstation::ScopeDatalogger);
                                auto.strip_chart_view_active = true;
                                auto.app_mode = AppMode::Datalogger;
                            }
                        }
                        SuiteWorkstation::CommsMonitor => {
                            if ui
                                .add_sized(
                                    [105.0, 28.0],
                                    egui::Button::new(
                                        egui::RichText::new("🌐 Solo Tráfico")
                                            .size(10.5)
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(egui::Color32::from_rgb(32, 52, 68)),
                                )
                                .clicked()
                            {
                                auto.suite.comms_layout = CommsViewLayout::TrafficTableOnly;
                                switch_workstation(auto, link, math, menus, SuiteWorkstation::CommsMonitor);
                            }
                        }
                        SuiteWorkstation::AutomotiveTests => {
                            for (label, cat) in [
                                ("🔋 Carga/Arranque", PresetCategory::StartingCharging),
                                ("⚡ Encendido", PresetCategory::Ignition),
                                ("📡 Sensores", PresetCategory::Sensors),
                            ] {
                                if ui
                                    .add_sized(
                                        [105.0, 28.0],
                                        egui::Button::new(egui::RichText::new(label).size(10.0))
                                            .fill(egui::Color32::from_rgb(38, 34, 54)),
                                    )
                                    .clicked()
                                {
                                    auto.selected_category = Some(cat);
                                    let presets = get_all_presets();
                                    if let Some(first) = presets.iter().find(|p| p.category == cat) {
                                        auto.selected_preset_id = Some(first.id);
                                    }
                                    switch_workstation(auto, link, math, menus, SuiteWorkstation::AutomotiveTests);
                                }
                            }
                        }
                        _ => {}
                    }
                });
            });
        });
}

/// Diálogo modal para completar y gestionar la Ficha Técnica del Vehículo Activo (Cap. 10.2)
pub fn show_vehicle_details_dialog(ctx: &egui::Context, auto: &mut AutomotiveUiState) {
    if !auto.suite.vehicle_dialog_open {
        return;
    }

    let mut open = true;
    let mut should_close = false;

    egui::Window::new("🚘 Ficha Técnica del Vehículo y Orden de Trabajo")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(
                    "Los datos ingresados aquí se asocian automáticamente a todas las capturas e informes de las 5 estaciones de la Suite.",
                )
                .size(11.0)
                .color(egui::Color32::from_gray(180)),
            );
            ui.add_space(6.0);

            // Selector rápido de vehículos recientes
            if !auto.suite.vehicle_store.recents.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🕘 Cargar Vehículo Reciente:").strong());
                    let mut load_idx = None;
                    egui::ComboBox::from_id_salt("veh_dialog_recents")
                        .selected_text("Seleccionar del historial...")
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            for (i, r) in auto.suite.vehicle_store.recents.iter().enumerate() {
                                if ui.selectable_label(false, r.summary_badge()).clicked() {
                                    load_idx = Some(i);
                                }
                            }
                        });
                    if let Some(i) = load_idx {
                        auto.suite.vehicle_store.active = auto.suite.vehicle_store.recents[i].clone();
                    }
                });
                ui.separator();
            }

            let v = &mut auto.suite.vehicle_store.active;

            egui::Grid::new("vehicle_details_form_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Patente / Dominio o VIN:");
                    ui.text_edit_singleline(&mut v.plate_or_vin);
                    ui.end_row();

                    ui.label("Marca (ej. Volkswagen, Ford, Toyota):");
                    ui.text_edit_singleline(&mut v.make);
                    ui.end_row();

                    ui.label("Modelo (ej. Amarok, Ranger, Hilux):");
                    ui.text_edit_singleline(&mut v.model);
                    ui.end_row();

                    ui.label("Año / Generación:");
                    ui.text_edit_singleline(&mut v.year);
                    ui.end_row();

                    ui.label("Código de Motor (ej. CNEA, PUMA 3.2):");
                    ui.text_edit_singleline(&mut v.engine_code);
                    ui.end_row();

                    ui.label("Cilindrada (Litros) y Cilindros:");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut v.displacement_l).desired_width(70.0));
                        ui.label("L  ·  Cilindros:");
                        for c in [3u8, 4, 5, 6, 8] {
                            if ui.selectable_label(v.cylinders == c, format!("{c}")).clicked() {
                                v.cylinders = c;
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Combustible / Propulsión:");
                    egui::ComboBox::from_id_salt("veh_fuel_combo")
                        .selected_text(&v.primary_fuel)
                        .show_ui(ui, |ui| {
                            for fuel in [
                                "Nafta / Gasolina",
                                "Diesel Common-Rail",
                                "Nafta + GNC / GLP",
                                "Híbrido (HEV / PHEV)",
                                "Eléctrico (BEV)",
                            ] {
                                ui.selectable_value(&mut v.primary_fuel, fuel.to_string(), fuel);
                            }
                        });
                    ui.end_row();

                    ui.label("Transmisión:");
                    ui.horizontal(|ui| {
                        for tr in ["Manual", "Automática", "CVT / DSG"] {
                            if ui.selectable_label(v.transmission == tr, tr).clicked() {
                                v.transmission = tr.to_string();
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Marca / Modelo de ECU:");
                    ui.text_edit_singleline(&mut v.ecu_make_model);
                    ui.end_row();

                    ui.label("Condición de Prueba:");
                    egui::ComboBox::from_id_salt("veh_cond_combo")
                        .selected_text(&v.test_condition)
                        .show_ui(ui, |ui| {
                            for cond in [
                                "Contacto Dado Motor Apagado (KOEO)",
                                "Arranque / Cranking",
                                "Ralentí (KOER)",
                                "Aceleración / Carga Parcial",
                                "Prueba en Ruta / Dinamómetro",
                                "En Banco (ECU / Componente)",
                            ] {
                                ui.selectable_value(&mut v.test_condition, cond.to_string(), cond);
                            }
                        });
                    ui.end_row();

                    ui.label("Orden de Trabajo / Cliente:");
                    ui.text_edit_singleline(&mut v.work_order);
                    ui.end_row();

                    ui.label("Notas / Síntoma / Códigos DTC:");
                    ui.text_edit_singleline(&mut v.customer_or_notes);
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("💾 Guardar y Aplicar a la Suite")
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(28, 100, 58));

                if ui.add(save_btn).clicked() {
                    auto.suite.vehicle_store.commit_active_to_recents();
                    // Sincronizar también el prefijo de archivo si el vehículo tiene identificación
                    let tag = auto.suite.vehicle_store.active.filename_tag();
                    if !tag.is_empty() {
                        auto.scope_capture.filename_prefix = tag.clone();
                        auto.datalogger_session.filename_prefix = format!("Datalog_{tag}");
                    }
                    should_close = true;
                }

                if ui.button("🗑 Limpiar Ficha").clicked() {
                    auto.suite.vehicle_store.active = VehicleDetails::default();
                    auto.suite.vehicle_store.save();
                }

                if ui.button("Cerrar").clicked() {
                    should_close = true;
                }
            });
        });

    if !open || should_close {
        auto.suite.vehicle_dialog_open = false;
    }
}

/// Dibuja la Vista Combinada de la Estación 3 (Comunicaciones CAN / LIN):
/// Mitad superior = Señal física del osciloscopio + Diagnóstico ISO 11898.
/// Mitad inferior = Monitor de tráfico agrupado por ID con bytes cambiantes resaltados.
pub fn show_comms_workstation_overlay(
    ctx: &egui::Context,
    layout: &Layout,
    link: &mut Link,
    math: &mut MathState,
    auto: &mut AutomotiveUiState,
    dec: &crate::decode::DecodeState,
) {
    if auto.suite.active_workstation != SuiteWorkstation::CommsMonitor {
        return;
    }

    match auto.suite.comms_layout {
        CommsViewLayout::PhysicalWaveformOnly => {
            auto.app_mode = AppMode::CanLinWaveform;
            crate::ui::automotive_dock::show_can_diff_hud(ctx, layout, link, math, auto);
        }
        CommsViewLayout::TrafficTableOnly => {
            auto.app_mode = AppMode::CanLinTraffic;
            crate::ui::automotive_dock::show_bus_traffic_view(ctx, layout, auto, dec);
        }
        CommsViewLayout::CombinedSplit => {
            auto.app_mode = AppMode::CanLinWaveform;
            // 1. Arriba: HUD compacto de Capa Física ISO 11898 sobre la onda en vivo
            crate::ui::automotive_dock::show_can_diff_hud(ctx, layout, link, math, auto);

            // 2. Abajo: Panel acoplado con el Monitor de Tráfico en vivo (ocupa el 44% inferior del área de gráfica)
            let plot_rect = layout.points(layout.plot);
            if plot_rect.width() < 120.0 || plot_rect.height() < 180.0 {
                return;
            }

            // Ingestar paquetes del decodificador serie (CAN, CAN FD, J1939, LIN, SENT)
            crate::ui::automotive_dock::ingest_decoded_traffic(&mut auto.traffic, dec);

            let split_h = (plot_rect.height() * 0.44).clamp(150.0, 340.0);
            let bottom_rect = egui::Rect::from_min_max(
                egui::pos2(plot_rect.min.x, plot_rect.max.y - split_h),
                plot_rect.max,
            );

            egui::Area::new("comms_split_traffic_dock".into())
                .fixed_pos(bottom_rect.min)
                .show(ctx, |ui| {
                    ui.set_width(bottom_rect.width());
                    ui.set_height(bottom_rect.height());

                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_unmultiplied(12, 15, 22, 244))
                        .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(65, 130, 95)))
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("🌐 MONITOR DE TRÁFICO EN VIVO (CAN / CAN FD / J1939 / LIN / SENT)")
                                        .size(11.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(110, 235, 150)),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .small_button("🔬 Pasar Captura a Osciloscopio")
                                        .on_hover_text("Analizar esta señal de bus con las herramientas completas del Módulo 1")
                                        .clicked()
                                    {
                                        transfer_capture_to_scope_workstation(auto, link, math);
                                    }
                                });
                            });
                            ui.separator();

                            crate::ui::automotive_dock::render_traffic_monitor_table(
                                ui,
                                &mut auto.traffic,
                                "comms_split_traffic_grid",
                                split_h - 68.0,
                            );
                        });
                });
        }
    }
}

/// Dibuja la barra superior de contexto cuando el usuario se encuentra en la Estación 5 (Testeos Automotrices Guiados)
pub fn show_guided_tests_top_banner(
    ctx: &egui::Context,
    layout: &Layout,
    link: &mut Link,
    math: &mut MathState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) {
    if auto.suite.active_workstation != SuiteWorkstation::AutomotiveTests {
        return;
    }

    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 160.0 {
        return;
    }

    let presets = get_all_presets();
    let active_cat = auto.selected_category.unwrap_or(PresetCategory::StartingCharging);
    let active_preset = auto
        .selected_preset_id
        .and_then(|id| presets.iter().find(|p| p.id == id))
        .or_else(|| presets.iter().find(|p| p.category == active_cat));

    let banner_pos = egui::pos2(plot_rect.min.x + 10.0, plot_rect.min.y + 8.0);
    egui::Area::new("guided_test_active_hud".into())
        .fixed_pos(banner_pos)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(20, 18, 32, 238))
                .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(185, 135, 255)))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🔧 ESTACIÓN DE TESTEOS AUTOMOTRICES:")
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::from_rgb(205, 160, 255)),
                        );

                        // Selector de categoría directo
                        for cat in [
                            PresetCategory::StartingCharging,
                            PresetCategory::Ignition,
                            PresetCategory::Injection,
                            PresetCategory::Sensors,
                            PresetCategory::Actuators,
                            PresetCategory::Networks,
                        ] {
                            let sel = active_cat == cat;
                            if ui
                                .selectable_label(
                                    sel,
                                    egui::RichText::new(cat.label()).size(10.5).strong(),
                                )
                                .clicked()
                            {
                                auto.selected_category = Some(cat);
                                if let Some(first) = presets.iter().find(|p| p.category == cat) {
                                    auto.selected_preset_id = Some(first.id);
                                }
                            }
                        }

                        ui.separator();

                        if ui.small_button("📐 Regla 720°").on_hover_text("Activar/Desactivar regla de ciclo de 4 tiempos (720°)").clicked() {
                            auto.ruler.enabled = !auto.ruler.enabled;
                        }

                        if ui.small_button("📋 Panel Guía").on_hover_text("Mostrar guía de conexión y señal esperada").clicked() {
                            menus.open(Menu::Automotive);
                        }

                        if ui
                            .button(
                                egui::RichText::new("🔬 Abrir en Osciloscopio")
                                    .size(10.5)
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 215, 70)),
                            )
                            .on_hover_text("Transferir esta captura al Módulo 1 (Osciloscopio + Datalogger) para análisis profundo")
                            .clicked()
                        {
                            transfer_capture_to_scope_workstation(auto, link, math);
                        }
                    });

                    if let Some(p) = active_preset {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Ensayo activo: {}", p.name))
                                    .size(11.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                            ui.label(
                                egui::RichText::new(format!("· Conexión: {}", p.connection_guide))
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(175, 220, 255)),
                            );
                        });
                    }
                });
        });
}

/// Sincroniza el título de la ventana nativa del sistema operativo para que cada Estación
/// de Trabajo se comporte y se perciba como un software dedicado independiente.
pub fn sync_window_title(ctx: &egui::Context, auto: &mut AutomotiveUiState) {
    let base_title = if auto.suite.launcher_open {
        "PicoWon Automotive Suite v0.3 — Centro de Estaciones de Diagnóstico".to_string()
    } else {
        let ws_name = auto.suite.active_workstation.window_app_name();
        if auto.suite.vehicle_store.active.is_empty() {
            ws_name.to_string()
        } else {
            format!(
                "{} · 🚘 {}",
                ws_name,
                auto.suite.vehicle_store.active.summary_badge()
            )
        }
    };

    if auto.suite.last_window_title != base_title {
        auto.suite.last_window_title = base_title.clone();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(base_title));
    }
}

/// Ventana flotante de Fase 1 (Cap. 2.1 y Cap. 12.6):
/// Selector de Sondas, Pinzas Amperimétricas, Transductores de Presión/Temperatura,
/// Unidades Físicas (`V, A, bar, psi, kPa, °C, kV`), Alias de Canal, Auto-Range y Contador de Frecuencia.
pub fn show_probe_transducer_window(
    ctx: &egui::Context,
    link: &mut Link,
    auto: &mut AutomotiveUiState,
) {
    let Some(ch) = auto.probe_dialog_ch else {
        return;
    };
    if ch >= 2 {
        auto.probe_dialog_ch = None;
        return;
    }

    let mut open = true;
    let mut close_requested = false;
    let title = format!("🔌 Sonda, Transductor y Unidad Física — CH{}", ch + 1);

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(520.0)
        .show(ctx, |ui| {
            let all_probes = auto.probe_store.all_probes();

            // 1. Alias del Canal y Controles Rápidos (Cap. 2.1)
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(20, 25, 35))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 70, 95)))
                .corner_radius(6.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🏷 Alias / Nombre del Canal:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut auto.ch_ext[ch].alias)
                                .hint_text("Ej. Inyector Cil 1, Pinza 60A, CAN_H, Presión Riel...")
                                .desired_width(240.0),
                        );
                        if ui.small_button("Limpiar").clicked() {
                            auto.ch_ext[ch].alias.clear();
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut auto.ch_ext[ch].auto_range,
                            "📏 Escala Vertical Automática Segura (Auto-Range)",
                        );
                        ui.separator();
                        ui.checkbox(
                            &mut auto.ch_ext[ch].freq_counter,
                            "⏱ Contador de Frecuencia HW dedic. en tarjeta",
                        );
                    });
                });

            ui.add_space(8.0);

            // 2. Sonda / Transductor actualmente seleccionado
            let cur_probe = &auto.ch_ext[ch].probe_spec;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Sonda Activa:").strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{}  [Unidad: {} · Escala actual: {}]",
                        cur_probe.name,
                        cur_probe.unit.symbol(),
                        cur_probe.format_per_div(link.config.channels[ch].volts_div)
                    ))
                    .strong()
                    .color(egui::Color32::from_rgb(255, 210, 75)),
                );
            });

            ui.add_space(6.0);
            ui.separator();

            // 3. Biblioteca de Sondas y Transductores Automotrices por Categoría
            ui.label(
                egui::RichText::new("📚 BIBLIOTECA DE SONDAS Y TRANSDUCTORES AUTOMOTRICES (Cap. 12.6):")
                    .strong()
                    .color(egui::Color32::from_rgb(110, 210, 255)),
            );
            ui.add_space(4.0);

            let mut selected_new_probe: Option<picowon_auto::ProbeSpec> = None;

            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for cat in picowon_auto::ProbeCategory::ALL {
                        let cat_probes: Vec<&picowon_auto::ProbeSpec> =
                            all_probes.iter().filter(|p| p.category == cat).collect();
                        if cat_probes.is_empty() {
                            continue;
                        }
                        ui.label(
                            egui::RichText::new(cat.label())
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 185, 65)),
                        );
                        for p in cat_probes {
                            let is_sel = auto.ch_ext[ch].probe_spec.id == p.id;
                            let row_txt = format!(
                                "{}   ({}) — Rango: {:.0} a {:.0} {}",
                                p.name,
                                p.short_badge,
                                p.range_min,
                                p.range_max,
                                p.unit.symbol()
                            );
                            if ui.selectable_label(is_sel, row_txt).clicked() {
                                selected_new_probe = Some(p.clone());
                            }
                        }
                        ui.add_space(4.0);
                    }
                });

            if let Some(chosen) = selected_new_probe {
                link.config.channels[ch].probe = chosen.effective_probe_factor();
                if let Some(rec_lpf) = chosen.recommended_lpf_hz {
                    auto.dsp[ch].lpf_cutoff_hz = rec_lpf as f64;
                }
                auto.ch_ext[ch].probe_spec = chosen;
                link.dirty = true;
            }

            ui.separator();

            // 4. Creador de Sonda Personalizada Lineal (y = m*x + b)
            ui.collapsing("🛠 Crear / Guardar Sonda o Transductor Personalizado (y = m·x + b)", |ui| {
                let draft = &mut auto.custom_probe_draft;
                egui::Grid::new("custom_probe_creator_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Nombre de la sonda:");
                        ui.text_edit_singleline(&mut draft.name);
                        ui.end_row();

                        ui.label("Abreviatura en botón (ej. 100A, 50bar):");
                        ui.add(egui::TextEdit::singleline(&mut draft.short_badge).desired_width(80.0));
                        ui.end_row();

                        ui.label("Unidad Física:");
                        egui::ComboBox::from_id_salt("custom_probe_unit")
                            .selected_text(draft.unit.label())
                            .show_ui(ui, |ui| {
                                for u in picowon_auto::PhysicalUnit::ALL {
                                    ui.selectable_value(&mut draft.unit, u, u.label());
                                }
                            });
                        ui.end_row();

                        if let picowon_auto::ScalingMethod::Linear {
                            ref mut gradient,
                            ref mut offset,
                        } = draft.scaling
                        {
                            ui.label("Pendiente (Unidades por cada 1 V en BNC):");
                            ui.add(egui::DragValue::new(gradient).speed(0.5));
                            ui.end_row();

                            ui.label("Offset / Ordenada al origen (en 0 V BNC):");
                            ui.add(egui::DragValue::new(offset).speed(0.1));
                            ui.end_row();
                        }
                    });

                ui.add_space(4.0);
                if ui.button("💾 Guardar en Biblioteca y Aplicar al Canal").clicked() {
                    let mut new_spec = auto.custom_probe_draft.clone();
                    new_spec.id = format!(
                        "cust_{}_{}",
                        new_spec.short_badge.to_lowercase(),
                        auto.probe_store.custom_probes.len() + 1
                    );
                    new_spec.category = picowon_auto::ProbeCategory::Custom;
                    new_spec.is_builtin = false;
                    auto.probe_store.custom_probes.push(new_spec.clone());
                    auto.probe_store.save();

                    link.config.channels[ch].probe = new_spec.effective_probe_factor();
                    auto.ch_ext[ch].probe_spec = new_spec;
                    link.dirty = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Cerrar").clicked() {
                    close_requested = true;
                }
            });
        });

    if !open || close_requested {
        auto.probe_dialog_ch = None;
    }
}

/// Ventana modal de Conexión de Dispositivo y Conmutador en Vivo USB VDS1022 ⇄ Simulador Demo (Cap. 1.7)
pub fn show_connect_device_dialog(
    ctx: &egui::Context,
    link: &mut Link,
    auto: &mut AutomotiveUiState,
) {
    if !auto.suite.device_dialog_open {
        return;
    }

    let mut open = true;
    let mut close_now = false;

    egui::Window::new("🔌 Conectar Dispositivo / Fuente de Adquisición (Cap. 1.7)")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(540.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 24, 34))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(65, 95, 135)))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Estado Actual del Motor de Captura:")
                                .strong()
                                .color(egui::Color32::from_rgb(190, 215, 245)),
                        );
                        let col = if link.caps.is_some() {
                            egui::Color32::from_rgb(115, 245, 160)
                        } else {
                            egui::Color32::from_rgb(255, 195, 90)
                        };
                        ui.colored_label(col, egui::RichText::new(&link.status).strong().monospace());
                    });
                });

            ui.label(
                egui::RichText::new("Seleccione la fuente de adquisición activa (conmutación en caliente sin reiniciar):")
                    .size(11.5)
                    .color(egui::Color32::from_gray(215)),
            );

            ui.columns(2, |cols| {
                // Columna 1: Hardware USB VDS1022
                let hw_active = !link.is_sim_mode();
                egui::Frame::new()
                    .fill(if hw_active {
                        egui::Color32::from_rgb(20, 44, 34)
                    } else {
                        egui::Color32::from_rgb(22, 26, 34)
                    })
                    .stroke(egui::Stroke::new(
                        if hw_active { 2.0 } else { 1.0 },
                        if hw_active {
                            egui::Color32::from_rgb(100, 235, 150)
                        } else {
                            egui::Color32::from_rgb(60, 70, 88)
                        },
                    ))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(10))
                    .show(&mut cols[0], |ui| {
                        ui.label(
                            egui::RichText::new("🔌 Hardware USB OWON VDS1022")
                                .strong()
                                .size(13.0)
                                .color(egui::Color32::from_rgb(115, 245, 160)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "• Adquisición real de 2 canales BNC (100 MS/s, 5000 muestras)\n• Carga automática de bitstream FPGA y calibración Flash\n• Reconexión automática en caliente ante desconexión USB",
                            )
                            .size(10.0)
                            .color(egui::Color32::from_gray(195)),
                        );
                        ui.add_space(6.0);
                        let btn = egui::Button::new(
                            egui::RichText::new(if hw_active {
                                "✔ MODO HARDWARE USB ACTIVO"
                            } else {
                                "🔌 CONECTAR A HARDWARE USB"
                            })
                            .strong()
                            .color(if hw_active {
                                egui::Color32::BLACK
                            } else {
                                egui::Color32::WHITE
                            }),
                        )
                        .fill(if hw_active {
                            egui::Color32::from_rgb(100, 235, 150)
                        } else {
                            egui::Color32::from_rgb(35, 85, 55)
                        })
                        .min_size(egui::vec2(ui.available_width(), 28.0));
                        if ui.add(btn).clicked() {
                            link.switch_device_source(false);
                        }
                    });

                // Columna 2: Simulador Demo Automotriz
                let sim_active = link.is_sim_mode();
                egui::Frame::new()
                    .fill(if sim_active {
                        egui::Color32::from_rgb(20, 38, 54)
                    } else {
                        egui::Color32::from_rgb(22, 26, 34)
                    })
                    .stroke(egui::Stroke::new(
                        if sim_active { 2.0 } else { 1.0 },
                        if sim_active {
                            egui::Color32::from_rgb(90, 215, 255)
                        } else {
                            egui::Color32::from_rgb(60, 70, 88)
                        },
                    ))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(10))
                    .show(&mut cols[1], |ui| {
                        ui.label(
                            egui::RichText::new("🧪 Simulador Demo Automotriz")
                                .strong()
                                .size(13.0)
                                .color(egui::Color32::from_rgb(95, 220, 255)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "• Generador virtual de doble canal sin requerir hardware USB\n• Incluye señales automotrices CKP 60-2 + CMP, Bus CAN Diferencial y PWM\n• Ideal para entrenamiento, pruebas de disparo y demostraciones",
                            )
                            .size(10.0)
                            .color(egui::Color32::from_gray(195)),
                        );
                        ui.add_space(6.0);
                        let btn = egui::Button::new(
                            egui::RichText::new(if sim_active {
                                "✔ SIMULADOR DEMO ACTIVO"
                            } else {
                                "🧪 ACTIVAR SIMULADOR DEMO"
                            })
                            .strong()
                            .color(if sim_active {
                                egui::Color32::BLACK
                            } else {
                                egui::Color32::WHITE
                            }),
                        )
                        .fill(if sim_active {
                            egui::Color32::from_rgb(90, 215, 255)
                        } else {
                            egui::Color32::from_rgb(30, 70, 105)
                        })
                        .min_size(egui::vec2(ui.available_width(), 28.0));
                        if ui.add(btn).clicked() {
                            link.switch_device_source(true);
                        }
                    });
            });

            // Si el Simulador Demo está activo, mostrar selector rápido de escenarios automotrices
            if link.is_sim_mode() {
                ui.separator();
                ui.label(
                    egui::RichText::new("🚗 Señales Patrón del Simulador Demo (1 Clic configura Generador + Escalas):")
                        .strong()
                        .color(egui::Color32::from_rgb(255, 210, 90)),
                );

                egui::Grid::new("sim_scenarios_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        let scenarios = [
                            (
                                "auto-ckp-60-2",
                                "🦷 CKP 60-2 Inductivo (CH1) + CMP Hall (CH2) @ 1200 RPM",
                            ),
                            (
                                "auto-can-diff",
                                "🌐 Bus CAN Diferencial ISO 11898 (CH1=CAN_H, CH2=CAN_L)",
                            ),
                            (
                                "auto-pwm-actuator",
                                "🎯 Actuador PWM 12V (CH1) + Sensor Posición (CH2)",
                            ),
                            (
                                "probe-comp",
                                "📏 Compensación de Sonda 1 kHz (CH1 Cuadrada 5V, CH2 Seno)",
                            ),
                            ("sine-1k", "〰 Onda Senoidal Pura 1 kHz (CH1)"),
                            ("uart-hello", "📡 Trama Serial UART 9600 bps (CH1)"),
                        ];

                        for (idx, (stim_id, label)) in scenarios.iter().enumerate() {
                            let is_sel = link.stimulus == *stim_id;
                            let b = egui::Button::new(
                                egui::RichText::new(*label).size(10.5).color(if is_sel {
                                    egui::Color32::BLACK
                                } else {
                                    egui::Color32::WHITE
                                }),
                            )
                            .fill(if is_sel {
                                egui::Color32::from_rgb(100, 225, 255)
                            } else {
                                egui::Color32::from_rgb(28, 36, 48)
                            })
                            .min_size(egui::vec2(248.0, 26.0));

                            if ui.add(b).clicked() {
                                link.set_sim_stimulus(stim_id);
                                match *stim_id {
                                    "auto-ckp-60-2" => {
                                        link.config.channels[0].enabled = true;
                                        link.config.channels[0].volts_div = 2.0;
                                        link.config.channels[0].probe = 1.0;
                                        link.config.channels[1].enabled = true;
                                        link.config.channels[1].volts_div = 2.0;
                                        link.config.channels[1].probe = 1.0;
                                        link.config.sample_rate = 50_000.0;
                                        link.config.trigger.source = 0;
                                        link.config.trigger.level = 0.5;
                                        auto.ch_ext[0].alias = "CKP 60-2".into();
                                        auto.ch_ext[1].alias = "CMP Hall".into();
                                        auto.trigger_qualifier.mode =
                                            picowon_auto::AdvancedTriggerMode::IntervalMissingTooth;
                                        auto.trigger_qualifier.ckp_wheel_teeth = 60;
                                        auto.trigger_qualifier.ckp_gap_ratio = 1.75;
                                        link.dirty = true;
                                    }
                                    "auto-can-diff" => {
                                        link.config.channels[0].enabled = true;
                                        link.config.channels[0].volts_div = 1.0;
                                        link.config.channels[0].probe = 1.0;
                                        link.config.channels[1].enabled = true;
                                        link.config.channels[1].volts_div = 1.0;
                                        link.config.channels[1].probe = 1.0;
                                        link.config.sample_rate = 1_000_000.0;
                                        link.config.trigger.source = 0;
                                        link.config.trigger.level = 3.0;
                                        auto.ch_ext[0].alias = "CAN_H".into();
                                        auto.ch_ext[1].alias = "CAN_L".into();
                                        auto.trigger_qualifier.mode =
                                            picowon_auto::AdvancedTriggerMode::StandardHardware;
                                        link.dirty = true;
                                    }
                                    "auto-pwm-actuator" => {
                                        link.config.channels[0].enabled = true;
                                        link.config.channels[0].volts_div = 5.0;
                                        link.config.channels[1].enabled = true;
                                        link.config.channels[1].volts_div = 1.0;
                                        link.config.sample_rate = 250_000.0;
                                        link.config.trigger.source = 0;
                                        link.config.trigger.level = 6.0;
                                        auto.ch_ext[0].alias = "PWM 12V".into();
                                        auto.ch_ext[1].alias = "Sensor Pos".into();
                                        auto.trigger_qualifier.mode =
                                            picowon_auto::AdvancedTriggerMode::StandardHardware;
                                        link.dirty = true;
                                    }
                                    _ => {
                                        auto.trigger_qualifier.mode =
                                            picowon_auto::AdvancedTriggerMode::StandardHardware;
                                    }
                                }
                            }
                            if idx % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cerrar").clicked() {
                    close_now = true;
                }
            });
        });

    if !open || close_now {
        auto.suite.device_dialog_open = false;
    }
}


#[cfg(test)]
mod tests {
    use neowon_backend::Backend;

    use super::*;

    fn test_link() -> Link {
        let sup = neowon_backend::spawn(|| -> Result<Box<dyn Backend>, String> {
            Err("no hardware in tests".into())
        });
        Link::new(sup, crate::view::startup_config())
    }

    #[test]
    fn workstation_switching_isolates_and_restores_profiles() {
        let mut link = test_link();
        let mut auto = AutomotiveUiState::default();
        let mut math = MathState::default();
        let mut menus = MenuState::default();

        // Configurar valores personalizados en Estación 1 (ScopeDatalogger)
        link.config.channels[0].volts_div = 0.5;
        auto.dsp[0].lpf_enabled = true;
        auto.dsp[0].lpf_cutoff_hz = 2500.0;

        // Conmutar a Estación 3 (CommsMonitor): debe cargar perfil CAN (1.0 V/div, Math A-B activo)
        switch_workstation(&mut auto, &mut link, &mut math, &mut menus, SuiteWorkstation::CommsMonitor);
        assert_eq!(auto.suite.active_workstation, SuiteWorkstation::CommsMonitor);
        assert!((link.config.channels[0].volts_div - 1.0).abs() < 1e-6);
        assert!(math.enabled);
        assert!(!auto.dsp[0].lpf_enabled);

        // Modificar algo dentro de CommsMonitor y volver a ScopeDatalogger
        link.config.channels[0].volts_div = 2.0;
        switch_workstation(&mut auto, &mut link, &mut math, &mut menus, SuiteWorkstation::ScopeDatalogger);
        assert_eq!(auto.suite.active_workstation, SuiteWorkstation::ScopeDatalogger);
        assert!((link.config.channels[0].volts_div - 0.5).abs() < 1e-6);
        assert!(auto.dsp[0].lpf_enabled);
        assert!((auto.dsp[0].lpf_cutoff_hz - 2500.0).abs() < 1e-3);

        // Volver a CommsMonitor: conserva el 2.0 V/div dejado en CommsMonitor
        switch_workstation(&mut auto, &mut link, &mut math, &mut menus, SuiteWorkstation::CommsMonitor);
        assert!((link.config.channels[0].volts_div - 2.0).abs() < 1e-6);

        // Transferir captura desde CommsMonitor al Osciloscopio
        transfer_capture_to_scope_workstation(&mut auto, &mut link, &mut math);
        assert_eq!(auto.suite.active_workstation, SuiteWorkstation::ScopeDatalogger);
        assert!((link.config.channels[0].volts_div - 2.0).abs() < 1e-6);
    }
}

