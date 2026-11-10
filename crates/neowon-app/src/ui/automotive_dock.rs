//! Sección del Dock y Vistas de Pantalla dedicadas a Diagnóstico Automotriz:
//! Presets Guiados, Regla Angular de 720° sobre el osciloscopio y Registrador Strip-Chart (Datalogger).

use bevy_egui::egui;
use picowon_auto::{
    ActuatorAnalysisState, ActuatorPreset, AutomotivePreset, AutomotiveStripChart, BusProtocol,
    ChannelDspConfig, CrankAngleRuler, DatalogMetric, MetricKind, N_DATALOG_METRICS, PresetCategory,
    TrafficMonitor, TracerEquipment, ViTracerPreset, ViTracerState, get_all_presets,
};
use crate::ui::layout::Layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Oscilloscope,
    Datalogger,
    CanLinWaveform,
    CanLinTraffic,
    ActuatorPwmVsSensor,
    ViCurveTracer,
}

impl AppMode {
    pub const ALL: [AppMode; 6] = [
        AppMode::Oscilloscope,
        AppMode::Datalogger,
        AppMode::CanLinWaveform,
        AppMode::CanLinTraffic,
        AppMode::ActuatorPwmVsSensor,
        AppMode::ViCurveTracer,
    ];

    pub fn title(self) -> &'static str {
        match self {
            AppMode::Oscilloscope => "📊 Osciloscopio",
            AppMode::Datalogger => "📈 Datalogger",
            AppMode::CanLinWaveform => "🚗 Red CAN/LIN",
            AppMode::CanLinTraffic => "🌐 Tráfico Bus",
            AppMode::ActuatorPwmVsSensor => "🎯 Actuadores PWM",
            AppMode::ViCurveTracer => "⚡ Trazador VI",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavoriteTool {
    Automotive,
    Measures,
    Cursors,
    SerialDecode,
    Datalogger,
    ActuatorTest,
    ViTracer,
    Math,
}

impl FavoriteTool {
    pub const ALL: [FavoriteTool; 8] = [
        FavoriteTool::Automotive,
        FavoriteTool::Measures,
        FavoriteTool::Cursors,
        FavoriteTool::SerialDecode,
        FavoriteTool::Datalogger,
        FavoriteTool::ActuatorTest,
        FavoriteTool::ViTracer,
        FavoriteTool::Math,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FavoriteTool::Automotive => "🚗 Auto",
            FavoriteTool::Measures => "📊 Medir",
            FavoriteTool::Cursors => "📐 Curs.",
            FavoriteTool::SerialDecode => "📡 Bus",
            FavoriteTool::Datalogger => "📈 Data",
            FavoriteTool::ActuatorTest => "🎯 Actuad",
            FavoriteTool::ViTracer => "⚡ VI",
            FavoriteTool::Math => "∑ Math",
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub struct AutomotiveUiState {
    pub app_mode: AppMode,
    pub selected_category: Option<PresetCategory>,
    pub selected_preset_id: Option<&'static str>,
    pub ruler: CrankAngleRuler,
    pub ruler_x0: f32, // 0.0 .. 1.0 (default 0.15)
    pub ruler_x1: f32, // 0.0 .. 1.0 (default 0.85)
    pub banner_message: Option<String>,

    // Cinta Superior (Minimap / Overview)
    pub overview_ribbon_visible: bool,

    // Ventana de Ayuda
    pub help_window_open: bool,

    // Ventana flotante de Zoom estilo PicoScope 7
    pub zoom_window_open: bool,

    // Motores de análisis dedicados
    pub actuator: ActuatorAnalysisState,
    pub traffic: TrafficMonitor,
    pub vi_tracer: ViTracerState,

    // Modo Strip-Chart / Datalogger
    pub strip_chart_view_active: bool,
    pub strip_chart: AutomotiveStripChart,
    pub show_vmax: bool,
    pub show_vmin: bool,
    pub show_vmean: bool,
    pub show_vactual: bool,
    pub show_ch2: bool,
    pub time_window_secs: f32, // 10, 30, 60, 300, 0 (todo)
    pub memory_depth_points: usize, // 5000, 20000, 50000, 100000

    // Arrastre con puntero
    pub dragging_ruler: Option<usize>, // 0 para 0°, 1 para 720°

    // Configuración de mediciones automotrices visibles en pantalla principal
    pub meas_show_freq: bool,
    pub meas_show_duty: bool,
    pub meas_show_pwm: bool,
    pub meas_show_vpp: bool,
    pub meas_show_vmax: bool,
    pub meas_show_vmin: bool,
    pub meas_show_vrms: bool,
    pub meas_popup_open: bool,
    pub meas_floating: bool,

    // Configuración de botones favoritos (matriz 2 columnas en riel izquierdo)
    pub favorite_slots: [FavoriteTool; 8],
    pub favorite_setup_open: bool,

    // Acondicionamiento DSP no destructivo por canal (Estilo PicoScope 7)
    pub dsp: [ChannelDspConfig; 2],

    // Ventana flotante de procesamiento DSP por canal (Some(0) para CH1, Some(1) para CH2, None si está cerrada)
    pub dsp_floating_ch: Option<usize>,

    // Secuencia de arranque seguro y persistencia de canales
    pub safety: picowon_auto::StartupSafetyManager,

    // Diagnóstico de Capa Física CAN Bus (ISO 11898)
    pub can_diag: picowon_auto::CanPhysicalMetrics,

    // Respaldo de configuración del osciloscopio al ingresar a modos dedicados (ej. Trazador VI)
    pub saved_scope_config: Option<neowon_backend::ScopeConfig>,

    // Gestores de Almacenamiento y Diálogos de Captura por Módulo
    pub scope_capture: picowon_auto::ScopeCaptureDialogState,
    pub datalogger_session: picowon_auto::DataloggerSessionDialogState,
    pub vi_capture: picowon_auto::ViCaptureDialogState,

    // Explorador y Gestor de Capturas Almacenadas en Disco
    pub capture_browser_open: bool,
    pub capture_browser_tab: usize, // 0: Osciloscopio, 1: Datalogger, 2: Tracer VI
    pub capture_browser_search: String,
    pub loaded_capture_info: Option<LoadedCaptureBannerInfo>,
    pub pending_dialog_result: std::sync::Arc<std::sync::Mutex<Option<NativeDialogResult>>>,
    pub is_dialog_waiting: bool,
}

#[derive(Debug, Clone)]
pub enum NativeDialogResult {
    ScopeFolder(String),
    DataloggerFolder(String),
    TracerFolder(String),
    OpenCaptureFile(std::path::PathBuf),
}

#[derive(Debug, Clone)]
pub struct LoadedCaptureBannerInfo {
    pub title: String,
    pub timestamp: String,
    pub operator: String,
    pub notes: String,
    pub details: String,
    pub file_path: String,
}

impl AutomotiveUiState {
    /// Ejecuta la secuencia escalonada de arranque seguro reduciendo la escala desde 5.0 V/div hasta el valor grabado
    pub fn tick_safety_startup(&mut self, link: &mut crate::Link) {
        if !self.safety.active {
            // Sincronizar y persistir automáticamente cualquier ajuste de canal realizado por el usuario
            let ch0 = &link.config.channels[0];
            let ch1 = &link.config.channels[1];
            let new_store = picowon_auto::ScopePersistenceStore {
                ch1: picowon_auto::SavedChannelConfig {
                    enabled: ch0.enabled,
                    volts_div: ch0.volts_div,
                    probe: ch0.probe,
                    coupling_dc: matches!(ch0.coupling, neowon_core::Coupling::Dc),
                    offset: ch0.offset,
                },
                ch2: picowon_auto::SavedChannelConfig {
                    enabled: ch1.enabled,
                    volts_div: ch1.volts_div,
                    probe: ch1.probe,
                    coupling_dc: matches!(ch1.coupling, neowon_core::Coupling::Dc),
                    offset: ch1.offset,
                },
            };
            if new_store != self.safety.store {
                self.safety.store = new_store;
                let _ = self.safety.store.save();
            }
            return;
        }

        let elapsed = self.safety.boot_time.elapsed().as_secs_f32();

        // Etapa 0: [0.0s .. 0.30s] Mantener máxima protección de hardware en 5.0 V/div
        if elapsed < 0.30 {
            link.config.channels[0].volts_div = 5.0;
            link.config.channels[1].volts_div = 5.0;
            self.banner_message = Some("🛡 PROTECCIÓN FRONT-END: Hardware iniciado en 5.0 V/div (Máxima Seguridad)...".into());
            return;
        }

        // Etapa 1: [0.30s .. 0.50s] Reducir a escala intermedia 2.0 V/div si el objetivo es menor
        if elapsed < 0.50 {
            for (ch, target) in [(0, &self.safety.store.ch1), (1, &self.safety.store.ch2)] {
                if target.volts_div < 5.0 {
                    link.config.channels[ch].volts_div = target.volts_div.max(2.0);
                    link.dirty = true;
                }
            }
            self.banner_message = Some("🛡 PROTECCIÓN FRONT-END: Reduciendo escalonadamente hacia ajuste previo...".into());
            return;
        }

        // Etapa 2: [0.50s .. 0.70s] Reducir a 1.0 V/div si el objetivo es menor
        if elapsed < 0.70 {
            for (ch, target) in [(0, &self.safety.store.ch1), (1, &self.safety.store.ch2)] {
                if target.volts_div < 2.0 {
                    link.config.channels[ch].volts_div = target.volts_div.max(1.0);
                    link.dirty = true;
                }
            }
            return;
        }

        // Etapa Final: [>= 0.70s] Establecer exactamente los ajustes grabados en el uso anterior
        let store = &self.safety.store;
        link.config.channels[0].volts_div = store.ch1.volts_div;
        link.config.channels[0].probe = store.ch1.probe;
        link.config.channels[0].enabled = store.ch1.enabled;
        link.config.channels[0].coupling = if store.ch1.coupling_dc { neowon_core::Coupling::Dc } else { neowon_core::Coupling::Ac };
        link.config.channels[0].offset = store.ch1.offset;

        link.config.channels[1].volts_div = store.ch2.volts_div;
        link.config.channels[1].probe = store.ch2.probe;
        link.config.channels[1].enabled = store.ch2.enabled;
        link.config.channels[1].coupling = if store.ch2.coupling_dc { neowon_core::Coupling::Dc } else { neowon_core::Coupling::Ac };
        link.config.channels[1].offset = store.ch2.offset;

        link.dirty = true;
        self.safety.active = false;
        self.banner_message = None;
    }

    /// Cambia de modo de aplicación grabando y restaurando parámetros del osciloscopio cuando corresponda
    pub fn switch_app_mode(&mut self, new_mode: AppMode, link: &mut crate::Link) {
        if self.app_mode == new_mode {
            return;
        }

        if new_mode == AppMode::ViCurveTracer {
            // Guardar configuración actual del osciloscopio
            self.saved_scope_config = Some(link.config.clone());
            // Aplicar configuración del equipo trazador activo
            let eq = self.vi_tracer.equipment_store.active_equipment().clone();
            apply_tracer_equipment_to_link(link, self, &eq);
        } else if self.app_mode == AppMode::ViCurveTracer {
            // Al salir del modo Trazador VI, restaurar los parámetros previos del osciloscopio
            if let Some(saved) = self.saved_scope_config.take() {
                link.config = saved;
                link.dirty = true;
            }
        }

        self.app_mode = new_mode;
        self.strip_chart_view_active = new_mode == AppMode::Datalogger;
    }
}

impl Default for AutomotiveUiState {
    fn default() -> Self {
        let mut ruler = CrankAngleRuler::default();
        ruler.enabled = false;
        ruler.cylinder_count = 4;
        ruler.show_phase_bands = true;

        Self {
            app_mode: AppMode::Oscilloscope,
            selected_category: Some(PresetCategory::Ignition),
            selected_preset_id: Some("ign_cop_primary"),
            ruler,
            ruler_x0: 0.15,
            ruler_x1: 0.85,
            banner_message: None,
            overview_ribbon_visible: true,
            help_window_open: false,
            zoom_window_open: true,
            actuator: ActuatorAnalysisState::default(),
            traffic: TrafficMonitor::default(),
            vi_tracer: ViTracerState::default(),
            strip_chart_view_active: false,
            strip_chart: AutomotiveStripChart::new(),
            show_vmax: true,
            show_vmin: true,
            show_vmean: true,
            meas_show_freq: true,
            meas_show_duty: true,
            meas_show_pwm: true,
            meas_show_vpp: true,
            meas_show_vmax: false,
            meas_show_vmin: false,
            meas_show_vrms: false,
            meas_popup_open: false,
            meas_floating: false,
            favorite_slots: [
                FavoriteTool::Automotive,
                FavoriteTool::Measures,
                FavoriteTool::Cursors,
                FavoriteTool::SerialDecode,
                FavoriteTool::Datalogger,
                FavoriteTool::ActuatorTest,
                FavoriteTool::ViTracer,
                FavoriteTool::Math,
            ],
            favorite_setup_open: false,
            dsp: [ChannelDspConfig::default(), ChannelDspConfig::default()],
            dsp_floating_ch: None,
            safety: picowon_auto::StartupSafetyManager::new(),
            can_diag: picowon_auto::CanPhysicalMetrics::default(),
            saved_scope_config: None,
            scope_capture: picowon_auto::ScopeCaptureDialogState::default(),
            datalogger_session: picowon_auto::DataloggerSessionDialogState::default(),
            vi_capture: picowon_auto::ViCaptureDialogState::default(),
            capture_browser_open: false,
            capture_browser_tab: 0,
            capture_browser_search: String::new(),
            loaded_capture_info: None,
            pending_dialog_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
            is_dialog_waiting: false,
            show_vactual: true,
            show_ch2: true,
            time_window_secs: 30.0,
            memory_depth_points: 5000,
            dragging_ruler: None,
        }
    }
}

/// Dibuja el panel lateral dock de Diagnóstico Automotriz
pub fn show(
    ui: &mut egui::Ui,
    state: &mut AutomotiveUiState,
    link: &mut crate::Link,
    _rec: &mut crate::record::Recorder,
    _hist: &mut crate::record::History,
) {
    ui.vertical(|ui| {
        ui.heading("🚗 Diagnóstico Automotriz");
        ui.add_space(4.0);

        // ====================================================================
        // SECCIÓN 1: PRESETS AUTOMOTRICES GUIADOS
        // ====================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new("📋 PRESETS DE PRUEBA GUIADA").strong().color(egui::Color32::from_rgb(255, 170, 0)));
            ui.add_space(2.0);

            let presets = get_all_presets();

            let categories = [
                PresetCategory::Ignition,
                PresetCategory::Injection,
                PresetCategory::Sensors,
                PresetCategory::Actuators,
                PresetCategory::Networks,
                PresetCategory::StartingCharging,
            ];

            let cur_cat = state.selected_category.unwrap_or(PresetCategory::Ignition);

            egui::ComboBox::from_label("Categoría")
                .selected_text(cur_cat.label())
                .show_ui(ui, |ui| {
                    for cat in categories {
                        if ui.selectable_label(state.selected_category == Some(cat), cat.label()).clicked() {
                            state.selected_category = Some(cat);
                        }
                    }
                });

            ui.add_space(4.0);

            let active_cat = state.selected_category.unwrap_or(PresetCategory::Ignition);
            let filtered: Vec<&AutomotivePreset> = presets.iter().filter(|p| p.category == active_cat).collect();
            let active_id = state.selected_preset_id.unwrap_or_else(|| filtered.first().map(|p| p.id).unwrap_or(""));

            egui::ComboBox::from_label("Prueba")
                .selected_text(filtered.iter().find(|p| p.id == active_id).map(|p| p.name).unwrap_or("Seleccionar..."))
                .show_ui(ui, |ui| {
                    for p in &filtered {
                        if ui.selectable_label(state.selected_preset_id == Some(p.id), p.name).clicked() {
                            state.selected_preset_id = Some(p.id);
                        }
                    }
                });

            if let Some(selected_preset) = presets.iter().find(|p| p.id == active_id) {
                ui.add_space(6.0);

                if ui.button(egui::RichText::new("⚡ APLICAR CONFIGURACIÓN DE HARDWARE").strong().color(egui::Color32::BLACK))
                    .on_hover_text("Configura canales, acoplamiento, atenuación y disparo según la prueba.")
                    .clicked()
                {
                    link.config.channels[0].enabled = selected_preset.ch1.enabled;
                    link.config.channels[0].volts_div = selected_preset.ch1.volts_per_div;
                    link.config.channels[0].coupling = selected_preset.ch1.coupling;
                    link.config.channels[0].probe = selected_preset.ch1.attenuation;

                    link.config.channels[1].enabled = selected_preset.ch2.enabled;
                    link.config.channels[1].volts_div = selected_preset.ch2.volts_per_div;
                    link.config.channels[1].coupling = selected_preset.ch2.coupling;
                    link.config.channels[1].probe = selected_preset.ch2.attenuation;

                    link.config.trigger.source = selected_preset.trigger.source as usize;
                    link.config.trigger.kind = neowon_core::TriggerKind::Edge { slope: selected_preset.trigger.slope };
                    link.config.trigger.level = selected_preset.trigger.level_volts;
                    link.config.trigger.sweep = selected_preset.trigger.sweep;

                    link.dirty = true;
                    state.banner_message = Some(format!("Preset aplicado: {}", selected_preset.name));
                }

                ui.add_space(6.0);
                ui.separator();

                ui.label(egui::RichText::new("🔌 Guía de Conexión:").strong());
                ui.label(egui::RichText::new(selected_preset.connection_guide).size(11.0).color(egui::Color32::from_rgb(180, 220, 255)));

                ui.add_space(4.0);
                ui.label(egui::RichText::new("📈 Señal Normal Esperada:").strong());
                ui.label(egui::RichText::new(selected_preset.expected_waveform).size(11.0).color(egui::Color32::from_rgb(200, 255, 200)));
            }
        });

        ui.add_space(8.0);

        // ====================================================================
        // SECCIÓN 2: REGLA ANGULAR DE CIGÜEÑAL (720°)
        // ====================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new("📐 REGLA DE CICLO 720° (4 TIEMPOS)").strong().color(egui::Color32::from_rgb(100, 200, 255)));
            ui.checkbox(&mut state.ruler.enabled, "Visualizar Regla en Pantalla");

            if state.ruler.enabled {
                ui.add_space(2.0);
                ui.checkbox(&mut state.ruler.show_phase_bands, "Colorear Fases (Adm, Comp, Exp, Esc)");

                ui.horizontal(|ui| {
                    ui.label("Cilindros:");
                    for c in [3u32, 4, 5, 6, 8] {
                        if ui.selectable_label(state.ruler.cylinder_count == c, format!("{c}")).clicked() {
                            state.ruler.cylinder_count = c;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Posición 0°:");
                    ui.add(egui::Slider::new(&mut state.ruler_x0, 0.01..=0.90).show_value(false));
                });
                ui.horizontal(|ui| {
                    ui.label("Posición 720°:");
                    ui.add(egui::Slider::new(&mut state.ruler_x1, 0.10..=0.99).show_value(false));
                });
                ui.label(egui::RichText::new("💡 Tip: Puedes arrastrar directamente las líneas de 0° y 720° con el ratón sobre el osciloscopio.").small().color(egui::Color32::from_gray(170)));
            }
        });

        ui.add_space(8.0);

        // ====================================================================
        // SECCIÓN 3: PROFUNDIDAD DE MEMORIA / HISTORIAL EN RAM
        // ====================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new("💾 PROFUNDIDAD DE MEMORIA").strong().color(egui::Color32::from_rgb(255, 140, 0)));
            ui.label(egui::RichText::new("Búfer de Historial (Scope) / Ventana de Registro (Strip-Chart):").small().color(egui::Color32::from_gray(180)));
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                for (lbl, pts) in [
                    ("5K pts (1 tr)", 5000),
                    ("20K pts (4 tr)", 20000),
                    ("50K pts (10 tr)", 50000),
                    ("100K pts (20 tr)", 100000),
                ] {
                    if ui.selectable_label(state.memory_depth_points == pts, lbl).clicked() {
                        state.memory_depth_points = pts;
                    }
                }
            });
            ui.add_space(3.0);
            ui.label(egui::RichText::new("ℹ️ Hardware VDS1022: Ráfaga física de 5.000 pts/disparo. La profundidad 20K/50K/100K almacena 4, 10 o 20 tramas consecutivas en RAM para navegar por capturas anteriores con STOP.").small().color(egui::Color32::from_gray(160)));

            ui.label(egui::RichText::new("ℹ️ Navegador de tramas ubicado abajo, al pie de la cuadrícula.").small().color(egui::Color32::from_rgb(100, 200, 255)));
        });

        ui.add_space(8.0);

        // ====================================================================
        // SECCIÓN 4: REGISTRADOR STRIP-CHART
        // ====================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new("📈 REGISTRADOR STRIP-CHART").strong().color(egui::Color32::from_rgb(120, 220, 140)));
            if ui.button(if state.strip_chart_view_active { "Volver al Osciloscopio" } else { "Abrir Pantalla Strip-Chart" }).clicked() {
                state.strip_chart_view_active = !state.strip_chart_view_active;
            }
        });
    });
}

/// Dibuja la Regla de 720° y las 4 fases directamente sobre el lienzo del osciloscopio
pub fn draw_crank_overlay(
    ctx: &egui::Context,
    layout: &Layout,
    link: &crate::Link,
    state: &mut AutomotiveUiState,
) {
    if !state.ruler.enabled || state.strip_chart_view_active {
        return;
    }
    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 100.0 || plot_rect.height() < 100.0 {
        return;
    }

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("crank_ruler_overlay"),
    ));

    // Detección y arrastre con el ratón
    let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
    let pointer_down = ctx.input(|i| i.pointer.primary_down());

    let mut x0 = plot_rect.min.x + state.ruler_x0.clamp(0.01, 0.95) * plot_rect.width();
    let mut x1 = plot_rect.min.x + state.ruler_x1.clamp(0.05, 0.99) * plot_rect.width();
    if x1 <= x0 + 10.0 {
        x1 = x0 + 10.0;
    }

    if let Some(pos) = pointer_pos {
        if pointer_down {
            if state.dragging_ruler.is_none() {
                if (pos.x - x0).abs() < 16.0 && pos.y >= plot_rect.min.y && pos.y <= plot_rect.max.y {
                    state.dragging_ruler = Some(0);
                } else if (pos.x - x1).abs() < 16.0 && pos.y >= plot_rect.min.y && pos.y <= plot_rect.max.y {
                    state.dragging_ruler = Some(1);
                }
            }
            if let Some(drag_idx) = state.dragging_ruler {
                let frac = ((pos.x - plot_rect.min.x) / plot_rect.width()).clamp(0.01, 0.99);
                if drag_idx == 0 {
                    state.ruler_x0 = frac.min(state.ruler_x1 - 0.05);
                } else {
                    state.ruler_x1 = frac.max(state.ruler_x0 + 0.05);
                }
                x0 = plot_rect.min.x + state.ruler_x0 * plot_rect.width();
                x1 = plot_rect.min.x + state.ruler_x1 * plot_rect.width();
            }
        } else {
            state.dragging_ruler = None;
        }
    } else {
        state.dragging_ruler = None;
    }

    // Cálculo temporal y RPM
    let sample_rate = link.config.sample_rate.max(1.0);
    let total_time_s = 5000.0 / sample_rate;
    let dt = (state.ruler_x1 - state.ruler_x0).abs() as f64 * total_time_s;
    let rpm = if dt > 1e-5 { (120.0 / dt).min(25000.0) } else { 0.0 };
    let span_x = x1 - x0;

    // 1. Bandas de Fases de los 4 Tiempos
    if state.ruler.show_phase_bands {
        let phases = [
            ("ADMISIÓN (0°-180°)", egui::Color32::from_rgba_unmultiplied(0, 160, 255, 35)),
            ("COMPRESIÓN (180°-360°)", egui::Color32::from_rgba_unmultiplied(255, 210, 0, 35)),
            ("COMBUSTIÓN (360°-540°)", egui::Color32::from_rgba_unmultiplied(255, 60, 40, 45)),
            ("ESCAPE (540°-720°)", egui::Color32::from_rgba_unmultiplied(40, 220, 80, 35)),
        ];

        for (idx, (name, col)) in phases.iter().enumerate() {
            let px0 = x0 + span_x * (idx as f32 / 4.0);
            let px1 = x0 + span_x * ((idx + 1) as f32 / 4.0);
            let band_rect = egui::Rect::from_min_max(
                egui::pos2(px0, plot_rect.min.y + 34.0),
                egui::pos2(px1, plot_rect.max.y),
            );
            painter.rect_filled(band_rect, 0.0, *col);

            let font = egui::FontId::proportional(11.0);
            let text_pos = egui::pos2((px0 + px1) / 2.0, plot_rect.min.y + 40.0);
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                *name,
                font,
                egui::Color32::from_gray(240),
            );
        }
    }

    // 2. Líneas Divisorias Verticales
    let angles = ["0° (PMS)", "180° (PMI)", "360° (PMS)", "540° (PMI)", "720° (PMS)"];
    for i in 0..=4 {
        let lx = x0 + span_x * (i as f32 / 4.0);
        let stroke_col = if i == 0 || i == 4 {
            egui::Color32::WHITE
        } else if i == 2 {
            egui::Color32::from_rgb(255, 120, 40)
        } else {
            egui::Color32::from_rgb(180, 220, 255)
        };
        let stroke_w = if i == 0 || i == 4 { 2.5 } else { 1.5 };
        painter.line_segment(
            [egui::pos2(lx, plot_rect.min.y + 34.0), egui::pos2(lx, plot_rect.max.y)],
            egui::Stroke::new(stroke_w, stroke_col),
        );

        let font = egui::FontId::monospace(10.0);
        painter.text(
            egui::pos2(lx, plot_rect.min.y + 34.0),
            egui::Align2::CENTER_BOTTOM,
            angles[i],
            font,
            stroke_col,
        );

        if i == 0 || i == 4 {
            painter.circle_filled(egui::pos2(lx, plot_rect.min.y + 24.0), 5.0, stroke_col);
            painter.circle_filled(egui::pos2(lx, plot_rect.max.y - 6.0), 5.0, stroke_col);
        }
    }

    // 3. Banner Superior Informativo
    let banner_rect = egui::Rect::from_min_max(
        plot_rect.min,
        egui::pos2(plot_rect.max.x, plot_rect.min.y + 34.0),
    );
    painter.rect_filled(banner_rect, 4.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
    painter.rect_stroke(banner_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 160, 240)), egui::StrokeKind::Middle);

    let summary_text = format!(
        "🚗 REGLA DE CICLO 720° | Δt: {:.2} ms | RPM Estimado: {:.0} RPM | Motor: {} Cilindros | Arrastra las líneas de 0° y 720° para calibrar",
        dt * 1000.0,
        rpm,
        state.ruler.cylinder_count
    );
    painter.text(
        banner_rect.center(),
        egui::Align2::CENTER_CENTER,
        summary_text,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(255, 220, 100),
    );
}

/// Diálogo flotante modal de configuración de parámetros, colores y escalas del Datalogger
pub fn show_datalog_config_dialog(ctx: &egui::Context, chart: &mut AutomotiveStripChart) {
    if !chart.config_window_open {
        return;
    }

    let mut open = chart.config_window_open;
    let mut close_clicked = false;
    egui::Window::new("⚙ Parámetros y Escalas - Datalogger")
        .open(&mut open)
        .resizable(true)
        .default_size([590.0, 520.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Selecciona los parámetros a graficar, sus colores y límites de escala:")
                        .size(11.0)
                        .color(egui::Color32::from_gray(190)),
                );
            });
            ui.add_space(4.0);

            // Botones de filtro rápido
            ui.horizontal(|ui| {
                if ui.button("⚡ Solo Voltajes").clicked() {
                    for (i, &m) in DatalogMetric::ALL.iter().enumerate() {
                        chart.active_metrics[i] = m.kind() == MetricKind::Voltage;
                    }
                }
                if ui.button("⚡ Marcar Básicos").clicked() {
                    for i in 0..N_DATALOG_METRICS {
                        chart.active_metrics[i] = i == 0 || i == 10 || i == 11 || i == 17; // Vactual, Vmax, Vmin, Vavg
                    }
                }
                if ui.button("✓ Marcar Todo").clicked() {
                    for i in 0..N_DATALOG_METRICS {
                        chart.active_metrics[i] = true;
                    }
                }
                if ui.button("✗ Desmarcar Todo").clicked() {
                    for i in 0..N_DATALOG_METRICS {
                        chart.active_metrics[i] = false;
                    }
                }
            });

            ui.separator();

            // Encabezado de la tabla
            egui::Grid::new("datalog_config_header_grid")
                .striped(false)
                .min_col_width(55.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Activo").strong().size(10.5));
                    ui.label(egui::RichText::new("Parámetro (Unidad)").strong().size(10.5));
                    ui.label(egui::RichText::new("Color Trazo").strong().size(10.5));
                    ui.label(egui::RichText::new("Rango Mín").strong().size(10.5));
                    ui.label(egui::RichText::new("Rango Máx").strong().size(10.5));
                    ui.label(egui::RichText::new("Modo Escala").strong().size(10.5));
                    ui.label(egui::RichText::new("Auto").strong().size(10.5));
                    ui.end_row();
                });

            ui.separator();

            let mut suggest_single = None;

            egui::ScrollArea::vertical()
                .max_height(340.0)
                .show(ui, |ui| {
                    egui::Grid::new("datalog_metrics_table_grid")
                        .striped(true)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            for (idx, &m) in DatalogMetric::ALL.iter().enumerate() {
                                let scale = &mut chart.scales[idx];
                                let is_active = &mut chart.active_metrics[idx];
                                let unit_str = m.display_unit();

                                // 1. Checkbox activo
                                ui.checkbox(is_active, "");

                                // 2. Nombre del parámetro con su color
                                ui.horizontal(|ui| {
                                    let col_egui = egui::Color32::from_rgba_premultiplied(
                                        scale.color[0], scale.color[1], scale.color[2], scale.color[3],
                                    );
                                    ui.label(
                                        egui::RichText::new(m.label())
                                            .strong()
                                            .color(if *is_active { col_egui } else { egui::Color32::from_gray(130) }),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("({})", unit_str))
                                            .size(9.0)
                                            .color(egui::Color32::from_gray(140)),
                                    );
                                });

                                // 3. Selector de color interactivo
                                ui.color_edit_button_srgba_unmultiplied(&mut scale.color);

                                // 4. Rango Mínimo (Entero)
                                ui.add(egui::DragValue::new(&mut scale.y_min).speed(1.0).suffix(unit_str));

                                // 5. Rango Máximo (Entero)
                                ui.add(egui::DragValue::new(&mut scale.y_max).speed(1.0).suffix(unit_str));

                                // 6. Candado Fijo / Auto
                                let lock_txt = if scale.is_locked { "🔒 Fijo" } else { "🔓 Auto" };
                                if ui.button(egui::RichText::new(lock_txt).size(9.5)).clicked() {
                                    scale.is_locked = !scale.is_locked;
                                }

                                // 7. Botón puntual de auto-ajuste
                                if ui.button(egui::RichText::new("⚡").size(9.5))
                                    .on_hover_text("Sugerir rango entero óptimo y bloquear")
                                    .clicked()
                                {
                                    suggest_single = Some(idx);
                                }

                                ui.end_row();
                            }
                        });
                });

            if let Some(idx) = suggest_single {
                chart.suggest_single_metric_scale(idx);
                chart.scales[idx].is_locked = true;
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("⚡ Sugerir Escalas Unificadas por Magnitud").strong().color(egui::Color32::from_rgb(255, 220, 60)))
                    .on_hover_text("Calcula y unifica escalas idénticas y enteras para todos los voltajes, tiempos, porcentajes, etc.")
                    .clicked()
                {
                    chart.suggest_unified_scales();
                }

                if ui.button("Valores por Defecto").clicked() {
                    for i in 0..N_DATALOG_METRICS {
                        let m = DatalogMetric::ALL[i];
                        let (dmin, dmax) = m.default_range();
                        chart.scales[i].y_min = dmin;
                        chart.scales[i].y_max = dmax;
                        chart.scales[i].color = m.default_color();
                        chart.scales[i].is_locked = false;
                    }
                    chart.active_metrics[0] = true;
                    chart.active_metrics[10] = true;
                    chart.active_metrics[11] = true;
                    chart.active_metrics[17] = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Cerrar").strong()).clicked() {
                        close_clicked = true;
                    }
                });
            });
        });

    chart.config_window_open = open && !close_clicked;
}

/// Dibuja la vista Strip-Chart (Datalogger multivariable con captura lenta de parámetros calculados)
pub fn show_strip_chart_view(
    ctx: &egui::Context,
    layout: &Layout,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
    meas: &crate::derived::MeasureState,
    now: f64,
) {
    if !state.strip_chart_view_active {
        return;
    }
    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 100.0 || plot_rect.height() < 100.0 {
        return;
    }

    // Ingestar mediciones físicas calibradas en voltios y segundos (sin cuentas ADC crudas)
    let mut v_act = None;
    let mut v2_act = None;
    for f in &link.arrived {
        if let Some(c1) = f.channels.iter().find(|c| c.ch == 0) {
            if let Some(&last) = c1.data.last() {
                v_act = Some(last as f64 * c1.cal.scale_i + c1.cal.offset_i);
            }
        }
        if let Some(c2) = f.channels.iter().find(|c| c.ch == 1) {
            if let Some(&last) = c2.data.last() {
                v2_act = Some(last as f64 * c2.cal.scale_i + c2.cal.offset_i);
            }
        }
    }

    if let Some(v) = v_act { state.strip_chart.update_metric_val(0, v as f32); }
    if let Some(v2) = v2_act { state.strip_chart.latest_v2act = v2 as f32; }

    let ch1_m = meas.latest[0].as_ref();
    if let Some(m) = ch1_m {
        if let Some(v) = m.period { state.strip_chart.update_metric_val(1, v as f32); }
        if let Some(v) = m.freq { state.strip_chart.update_metric_val(2, v as f32); }
        if let Some(v) = m.rise { state.strip_chart.update_metric_val(3, v as f32); }
        if let Some(v) = m.fall { state.strip_chart.update_metric_val(4, v as f32); }
        if let Some(v) = m.pwidth { state.strip_chart.update_metric_val(5, v as f32); }
        if let Some(v) = m.nwidth { state.strip_chart.update_metric_val(6, v as f32); }
        if let Some(v) = m.pduty { state.strip_chart.update_metric_val(7, (v * 100.0) as f32); }
        if let Some(v) = m.nduty { state.strip_chart.update_metric_val(8, (v * 100.0) as f32); }
        state.strip_chart.update_metric_val(9, m.vpp as f32);
        state.strip_chart.update_metric_val(10, m.vmax as f32);
        state.strip_chart.update_metric_val(11, m.vmin as f32);
        state.strip_chart.update_metric_val(12, m.vamp as f32);
        state.strip_chart.update_metric_val(13, m.vtop as f32);
        state.strip_chart.update_metric_val(14, m.vbase as f32);
        if let Some(v) = m.overshoot { state.strip_chart.update_metric_val(15, v as f32); }
        if let Some(v) = m.preshoot { state.strip_chart.update_metric_val(16, v as f32); }
        state.strip_chart.update_metric_val(17, m.vavg as f32);
        if let Some(v) = m.vrms { state.strip_chart.update_metric_val(18, v as f32); }
        if let Some(v) = m.cycle_vrms { state.strip_chart.update_metric_val(19, v as f32); }
        if let Some(v) = m.slew_rate { state.strip_chart.update_metric_val(20, v as f32); }
    }

    if let Some(bi) = meas.bichannel {
        if let Some(v) = bi.rise_delay { state.strip_chart.update_metric_val(21, v as f32); }
        if let Some(v) = bi.fall_delay { state.strip_chart.update_metric_val(22, v as f32); }
        if let Some(v) = bi.phase_deg { state.strip_chart.update_metric_val(23, v as f32); }
    }

    let _dvm = state.strip_chart.tick(now);

    // Mostrar ventana flotante de configuración si fue solicitada
    show_datalog_config_dialog(ctx, &mut state.strip_chart);

    egui::Area::new("automotive_strip_chart_canvas".into())
        .fixed_pos(plot_rect.min)
        .show(ctx, |ui| {
            ui.set_width(plot_rect.width());
            ui.set_height(plot_rect.height());

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(14, 17, 22))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 60, 80)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // 1. Fila de Controles y Herramientas principales del Datalogger (altura fija, sin wrapping oscilante)
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;

                        // Botón de ventana flotante de configuración
                        if ui.button(egui::RichText::new("⚙ Parámetros y Escalas...").strong().color(egui::Color32::from_rgb(100, 200, 255))).clicked() {
                            state.strip_chart.config_window_open = true;
                        }

                        ui.separator();

                        if !state.strip_chart.is_recording {
                            let rec_btn = egui::Button::new(
                                egui::RichText::new("🔴 INICIAR GRABACIÓN").strong().color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(150, 35, 30));
                            if ui.add(rec_btn).on_hover_text("Iniciar sesión de registro continuo en disco").clicked() {
                                state.datalogger_session.is_starting_dialog_open = true;
                            }
                        } else {
                            let stop_btn = egui::Button::new(
                                egui::RichText::new("⏹ DETENER GRABACIÓN").strong().color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(180, 40, 40));
                            if ui.add(stop_btn).on_hover_text("Detener sesión y confirmar guardado en disco").clicked() {
                                state.strip_chart.stop_recording();
                                state.datalogger_session.recorded_duration_secs = (now - state.datalogger_session.session_start_time).max(0.0);
                                state.datalogger_session.recorded_samples = state.strip_chart.records_count();
                                state.datalogger_session.is_stopping_dialog_open = true;
                            }

                            let elapsed = (now - state.datalogger_session.session_start_time).max(0.0);
                            let mins = (elapsed / 60.0).floor() as u32;
                            let secs = (elapsed % 60.0).floor() as u32;
                            let n_pts = state.strip_chart.records_count();
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 90, 90),
                                format!("● GRABANDO ({mins:02}:{secs:02} · {n_pts} pts)"),
                            );
                        }

                        if ui.button("🧹 Limpiar").clicked() {
                            state.strip_chart.clear();
                        }

                        ui.separator();

                        ui.label(egui::RichText::new("Ventana:").strong().size(11.0));
                        for (lbl, win) in [("10s", 10.0f32), ("30s", 30.0), ("1m", 60.0), ("5m", 300.0), ("Todo", 0.0)] {
                            if ui.selectable_label(state.time_window_secs == win, lbl).clicked() {
                                state.time_window_secs = win;
                            }
                        }

                        ui.separator();

                        if ui.button(egui::RichText::new("⚡ Auto-Ajustar Escalas").strong().color(egui::Color32::from_rgb(255, 220, 60)))
                            .on_hover_text("Sugerir escalas enteras unificadas por magnitud física")
                            .clicked()
                        {
                            state.strip_chart.suggest_unified_scales();
                        }
                    });

                    // 2. Fila secundaria con Badges interactivos de parámetros activos en ScrollArea horizontal dedicada
                    let has_active_metrics = DatalogMetric::ALL.iter().enumerate().any(|(i, _)| state.strip_chart.active_metrics[i]);
                    if has_active_metrics {
                        ui.add_space(2.0);
                        egui::ScrollArea::horizontal()
                            .auto_shrink([false, true])
                            .max_height(28.0)
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    for (idx, &m) in DatalogMetric::ALL.iter().enumerate() {
                                        if state.strip_chart.active_metrics[idx] {
                                            let scale = state.strip_chart.scales[idx];
                                            let col = egui::Color32::from_rgba_premultiplied(
                                                scale.color[0], scale.color[1], scale.color[2], scale.color[3]
                                            );
                                            let live_val = state.strip_chart.latest_values[idx] * m.display_mult();
                                            let mut frame = egui::Frame::new();
                                            frame.fill = egui::Color32::from_rgb(24, 28, 36);
                                            frame.stroke = egui::Stroke::new(1.0, col);
                                            frame.inner_margin = egui::Margin::symmetric(6, 2);
                                            frame.show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(m.label()).size(9.5).color(egui::Color32::from_gray(180)));
                                                    ui.label(egui::RichText::new(format!("{:.2} {}", live_val, m.display_unit())).monospace().strong().size(11.0).color(col));
                                                });
                                            });
                                        }
                                    }
                                });
                            });
                    }

                    ui.add_space(3.0);

                    // Lienzo del gráfico Strip-Chart Multiescala
                    let avail = ui.available_size();
                    let canvas_w = avail.x.max(100.0);
                    let canvas_h = avail.y.max(100.0);
                    let (canvas_rect, _) = ui.allocate_exact_size(
                        egui::vec2(canvas_w, canvas_h),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(canvas_rect);
                    let mut canvas_rect = canvas_rect;
                    canvas_rect.min.x = canvas_rect.min.x.round();
                    canvas_rect.min.y = canvas_rect.min.y.round();
                    canvas_rect.max.x = canvas_rect.max.x.round();
                    canvas_rect.max.y = canvas_rect.max.y.round();

                    painter.rect_filled(canvas_rect, 2.0, egui::Color32::from_rgb(10, 12, 16));
                    let grid_stroke_color = egui::Color32::from_rgba_unmultiplied(65, 75, 95, 75);

                    // 10 divisiones verticales (11 líneas horizontales) punteadas suaves, redondeadas al pixel
                    let h_step = canvas_rect.height() / 10.0;
                    for div in 0..=10 {
                        let y = (canvas_rect.min.y + div as f32 * h_step).round();
                        let mut x = canvas_rect.min.x;
                        while x < canvas_rect.max.x {
                            let next_x = (x + 3.0).min(canvas_rect.max.x);
                            painter.line_segment([egui::pos2(x, y), egui::pos2(next_x, y)], egui::Stroke::new(1.0, grid_stroke_color));
                            x += 7.0;
                        }
                    }

                    // 20 divisiones horizontales (21 líneas verticales) punteadas suaves, redondeadas al pixel
                    let w_step = canvas_rect.width() / 20.0;
                    for div in 0..=20 {
                        let x = (canvas_rect.min.x + div as f32 * w_step).round();
                        let mut y = canvas_rect.min.y;
                        while y < canvas_rect.max.y {
                            let next_y = (y + 3.0).min(canvas_rect.max.y);
                            painter.line_segment([egui::pos2(x, y), egui::pos2(x, next_y)], egui::Stroke::new(1.0, grid_stroke_color));
                            y += 7.0;
                        }
                    }

                    // Identificar si hay alguna métrica activa de tipo Voltaje para la escala principal izquierda
                    let active_voltage_scale = DatalogMetric::ALL.iter().enumerate()
                        .find(|(i, m)| state.strip_chart.active_metrics[*i] && m.kind() == MetricKind::Voltage)
                        .map(|(i, _)| state.strip_chart.scales[i]);

                    // Resaltado de la línea de CERO (0 V) si está en el rango
                    if let Some(v_scale) = active_voltage_scale {
                        if v_scale.y_min <= 0 && v_scale.y_max >= 0 && v_scale.y_max > v_scale.y_min {
                            let zero_norm = (0 - v_scale.y_min) as f32 / (v_scale.y_max - v_scale.y_min) as f32;
                            let y_zero = (canvas_rect.max.y - zero_norm * canvas_rect.height()).round();
                            painter.line_segment(
                                [egui::pos2(canvas_rect.min.x, y_zero), egui::pos2(canvas_rect.max.x, y_zero)],
                                egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(130, 175, 230, 190)),
                            );
                            painter.text(
                                egui::pos2(canvas_rect.min.x + 3.0, y_zero - 6.0),
                                egui::Align2::LEFT_BOTTOM,
                                "0 V ──",
                                egui::FontId::monospace(9.5),
                                egui::Color32::from_rgb(130, 175, 230),
                            );
                        }

                        // Etiquetas enteras de las 10 divisiones verticales a la izquierda
                        let font = egui::FontId::monospace(9.0);
                        for div in 0..=10 {
                            let y = (canvas_rect.min.y + div as f32 * h_step).round();
                            let val_at_div = v_scale.y_max as f32 - div as f32 * ((v_scale.y_max - v_scale.y_min) as f32 / 10.0);
                            painter.text(
                                egui::pos2(canvas_rect.min.x + 4.0, y),
                                egui::Align2::LEFT_CENTER,
                                format!("{:.0} V", val_at_div.round()),
                                font.clone(),
                                egui::Color32::from_rgba_unmultiplied(160, 180, 200, 160),
                            );
                        }
                    }

                    let n = state.strip_chart.time_history.len();
                    if n >= 2 {
                        let t_last = *state.strip_chart.time_history.last().unwrap_or(&0.0);
                        let t_min = if state.time_window_secs > 0.0 {
                            (t_last - state.time_window_secs as f64).max(0.0)
                        } else {
                            0.0
                        };
                        let t_span = (t_last - t_min).max(0.5);

                        let font = egui::FontId::monospace(9.0);
                        let mut right_axis_offset = 0.0f32;

                        // Graficar cada métrica activa con su escala entera
                        for (idx, &m) in DatalogMetric::ALL.iter().enumerate() {
                            if !state.strip_chart.active_metrics[idx] {
                                continue;
                            }
                            let scale = state.strip_chart.scales[idx];
                            let data = &state.strip_chart.histories[idx];
                            let mult = m.display_mult();
                            let col = egui::Color32::from_rgba_premultiplied(
                                scale.color[0], scale.color[1], scale.color[2], scale.color[3]
                            );
                            let unit_str = m.display_unit();

                            let y_min = scale.y_min as f32;
                            let y_max = scale.y_max as f32;
                            let y_span = (y_max - y_min).max(0.001);

                            let map_pt = |t: f64, v: f32| -> egui::Pos2 {
                                let x_norm = ((t - t_min) / t_span).clamp(0.0, 1.0) as f32;
                                let y_norm = (((v * mult) - y_min) / y_span).clamp(0.0, 1.0);
                                egui::pos2(
                                    canvas_rect.min.x + x_norm * canvas_rect.width(),
                                    canvas_rect.max.y - y_norm * canvas_rect.height(),
                                )
                            };

                            for i in 1..n {
                                if state.strip_chart.time_history[i] >= t_min && i < data.len() {
                                    let p0 = map_pt(state.strip_chart.time_history[i - 1], data[i - 1]);
                                    let p1 = map_pt(state.strip_chart.time_history[i], data[i]);
                                    painter.line_segment([p0, p1], egui::Stroke::new(1.8, col));
                                }
                            }

                            // Ejes secundarios derechos para magnitudes no de voltaje
                            if m.kind() != MetricKind::Voltage {
                                let x_pos = canvas_rect.max.x - 6.0 - right_axis_offset;
                                painter.text(egui::pos2(x_pos, canvas_rect.min.y + 10.0), egui::Align2::RIGHT_CENTER, format!("{:.0} {}", y_max, unit_str), font.clone(), col);
                                painter.text(egui::pos2(x_pos, canvas_rect.center().y), egui::Align2::RIGHT_CENTER, format!("{:.0} {}", (y_max + y_min) / 2.0, unit_str), font.clone(), col);
                                painter.text(egui::pos2(x_pos, canvas_rect.max.y - 10.0), egui::Align2::RIGHT_CENTER, format!("{:.0} {}", y_min, unit_str), font.clone(), col);
                                right_axis_offset += 65.0;
                            }
                        }
                    } else {
                        painter.text(
                            canvas_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Esperando capturas continuas... Conecte una señal o pulse 'Iniciar Grabación'",
                            egui::FontId::proportional(14.0),
                            egui::Color32::from_gray(140),
                        );
                    }
                });
        });
}

/// Dibuja la Cinta Superior (Minimap / Overview Ribbon) de la captura total
/// con la ventana de zoom visualizable y desplazable interactivamente.
pub fn draw_overview_ribbon(
    ctx: &egui::Context,
    layout: &Layout,
    link: &crate::Link,
    phosphor: &mut crate::gpu::Phosphor,
    state: &mut AutomotiveUiState,
) {
    if !state.overview_ribbon_visible || state.strip_chart_view_active {
        return;
    }
    let ribbon_rect = layout.points(layout.ribbon);
    if ribbon_rect.width() < 120.0 || ribbon_rect.height() < 10.0 {
        return;
    }
    let ribbon_h = ribbon_rect.height();

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("overview_ribbon_overlay"),
    ));

    // Fondo oscuro con borde
    painter.rect_filled(ribbon_rect, 2.0, egui::Color32::from_rgb(16, 20, 26));
    painter.rect_stroke(ribbon_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 70)), egui::StrokeKind::Middle);

    // Líneas guía sutiles de la cinta
    let center_y = ribbon_rect.center().y;
    painter.line_segment(
        [egui::pos2(ribbon_rect.min.x, center_y), egui::pos2(ribbon_rect.max.x, center_y)],
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(60, 70, 90, 80)),
    );

    // Dibujar la señal total de los 5.000 puntos en la cinta
    if let Some(frame) = &link.latest {
        for cap in &frame.channels {
            if cap.data.is_empty() {
                continue;
            }
            let col = if cap.ch == 0 {
                egui::Color32::from_rgb(255, 215, 0) // CH1 Amarillo
            } else {
                egui::Color32::from_rgb(0, 200, 255) // CH2 Cian
            };
            let step = (cap.data.len() / (ribbon_rect.width() as usize).max(1)).max(1);
            let mut prev_pt: Option<egui::Pos2> = None;
            for i in (0..cap.data.len()).step_by(step) {
                let norm_x = i as f32 / cap.data.len() as f32;
                let norm_y = (cap.data[i] / 250.0).clamp(-0.46, 0.46);
                let pt = egui::pos2(
                    ribbon_rect.min.x + norm_x * ribbon_rect.width(),
                    center_y - norm_y * ribbon_h,
                );
                if let Some(p0) = prev_pt {
                    painter.line_segment([p0, pt], egui::Stroke::new(1.0, col));
                }
                prev_pt = Some(pt);
            }
        }
    }

    // Ventana interactiva de Zoom (Viewport Box)
    let (center, span) = phosphor.hview;
    let box_x0 = ribbon_rect.min.x + ((center - span / 2.0).clamp(0.0, 1.0) as f32) * ribbon_rect.width();
    let box_x1 = ribbon_rect.min.x + ((center + span / 2.0).clamp(0.0, 1.0) as f32) * ribbon_rect.width();

    let box_rect = egui::Rect::from_min_max(
        egui::pos2(box_x0, ribbon_rect.min.y),
        egui::pos2(box_x1.max(box_x0 + 8.0), ribbon_rect.max.y),
    );

    // Resaltado de la ventana de zoom ampliada
    painter.rect_filled(box_rect, 2.0, egui::Color32::from_rgba_unmultiplied(60, 140, 255, 55));
    painter.rect_stroke(box_rect, 2.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 195, 255)), egui::StrokeKind::Middle);

    // Tiradores laterales de agarre
    painter.line_segment(
        [egui::pos2(box_x0 + 2.0, ribbon_rect.min.y + 4.0), egui::pos2(box_x0 + 2.0, ribbon_rect.max.y - 4.0)],
        egui::Stroke::new(2.0, egui::Color32::WHITE),
    );
    painter.line_segment(
        [egui::pos2(box_rect.max.x - 2.0, ribbon_rect.min.y + 4.0), egui::pos2(box_rect.max.x - 2.0, ribbon_rect.max.y - 4.0)],
        egui::Stroke::new(2.0, egui::Color32::WHITE),
    );

    // Interacción: Clic o Arrastre en la cinta para navegar por la forma de onda
    let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
    let pointer_down = ctx.input(|i| i.pointer.primary_down());
    if let Some(pos) = pointer_pos {
        if pointer_down && ribbon_rect.contains(pos) {
            let frac = ((pos.x - ribbon_rect.min.x) / ribbon_rect.width()).clamp(0.0, 1.0) as f64;
            phosphor.hview.0 = frac;
            phosphor.hview = crate::view::hview_clamp(phosphor.hview.0, phosphor.hview.1);
        }
    }

    // Etiquetas y controles de navegación rápida
    let font = egui::FontId::monospace(10.0);
    let pts_str = if state.memory_depth_points >= 1000 {
        format!("{}.000", state.memory_depth_points / 1000)
    } else {
        format!("{}", state.memory_depth_points)
    };
    painter.text(
        egui::pos2(ribbon_rect.min.x + 6.0, center_y),
        egui::Align2::LEFT_CENTER,
        format!("📡 CAPTURA TOTAL ({} PTS)", pts_str),
        font.clone(),
        egui::Color32::from_gray(180),
    );

    let zoom_pct = (1.0 / span.max(0.001)) as f32;
    painter.text(
        egui::pos2(ribbon_rect.max.x - 85.0, center_y),
        egui::Align2::RIGHT_CENTER,
        format!("ZOOM {:.1}x", zoom_pct),
        font,
        egui::Color32::from_rgb(255, 200, 60),
    );

    // Botón de restablecer Zoom 1:1 en la esquina de la cinta
    let btn_rect = egui::Rect::from_min_size(
        egui::pos2(ribbon_rect.max.x - 70.0, ribbon_rect.min.y + 4.0),
        egui::vec2(64.0, ribbon_h - 8.0),
    );
    painter.rect_filled(btn_rect, 3.0, egui::Color32::from_rgb(34, 40, 52));
    painter.rect_stroke(btn_rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(90)), egui::StrokeKind::Middle);
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "🔍 1:1 (Todo)",
        egui::FontId::proportional(10.0),
        egui::Color32::WHITE,
    );
    if let Some(pos) = pointer_pos {
        if ctx.input(|i| i.pointer.primary_clicked()) && btn_rect.contains(pos) {
            crate::view::hview_home(phosphor);
        }
    }
}

/// Modal flotante de Ayuda de Atajos y Navegación
pub fn show_help_window(ctx: &egui::Context, state: &mut AutomotiveUiState) {
    if !state.help_window_open {
        return;
    }

    let mut is_open = true;
    let mut want_close = false;
    egui::Window::new("❓ Guía Rápida y Atajos - PicoWon Automotive")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("🚗 Navegación Intuitiva de Formas de Onda");
                ui.add_space(4.0);

                egui::Grid::new("nav_grid").striped(true).spacing([16.0, 8.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Acción").strong());
                    ui.label(egui::RichText::new("Control / Gesto").strong());
                    ui.end_row();

                    ui.label("🔍 Zoom Horizontal");
                    ui.label(egui::RichText::new("Rueda del ratón hacia adelante / atrás (centrado en el cursor)").color(egui::Color32::from_rgb(255, 215, 0)));
                    ui.end_row();

                    ui.label("📏 Zoom Vertical (V/div)");
                    ui.label("Ctrl + Rueda del ratón");
                    ui.end_row();

                    ui.label("↔ Pan Horizontal (Desplazamiento)");
                    ui.label(egui::RichText::new("Clic Izquierdo y arrastrar la señal horizontalmente").color(egui::Color32::from_rgb(100, 200, 255)));
                    ui.end_row();

                    ui.label("🎞️ Cinta Superior (Minimap)");
                    ui.label("Clic o arrastre en la cinta superior para saltar a cualquier punto de los 5.000 pts");
                    ui.end_row();

                    ui.label("⏮ Restablecer Zoom (100%)");
                    ui.label("Doble clic en la pantalla o pulsar tecla 'H' (Home)");
                    ui.end_row();

                    ui.label("📐 Regla 720° (4 Tiempos)");
                    ui.label("Hacer clic y arrastrar directamente las líneas de 0° o 720° sobre el gráfico");
                    ui.end_row();

                    ui.label("🎯 Nivel de Disparo (Trigger)");
                    ui.label("Arrastrar la flecha indicadora del margen derecho");
                    ui.end_row();
                });

                ui.add_space(8.0);
                ui.separator();
                ui.heading("⌨️ Atajos de Teclado");
                ui.add_space(4.0);

                egui::Grid::new("key_grid").striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Tecla").strong());
                    ui.label(egui::RichText::new("Función").strong());
                    ui.end_row();

                    ui.label(egui::RichText::new("Espacio").monospace().strong());
                    ui.label("RUN / STOP (Pausar o reanudar adquisición)");
                    ui.end_row();

                    ui.label(egui::RichText::new("H").monospace().strong());
                    ui.label("HOME: Restablecer zoom a captura completa (1:1)");
                    ui.end_row();

                    ui.label(egui::RichText::new("Flechas Izq / Der").monospace().strong());
                    ui.label("Base de tiempo (más rápido / más lento)");
                    ui.end_row();

                    ui.label(egui::RichText::new("Flechas Arriba / Abajo").monospace().strong());
                    ui.label("Escala vertical CH1 (Volts/div)");
                    ui.end_row();

                    ui.label(egui::RichText::new("A").monospace().strong());
                    ui.label("Auto-Set (Ajuste automático de escala y trigger)");
                    ui.end_row();

                    ui.label(egui::RichText::new("N").monospace().strong());
                    ui.label("Alternar modo de disparo (Auto → Normal → Single)");
                    ui.end_row();

                    ui.label(egui::RichText::new("F").monospace().strong());
                    ui.label("Force Trigger (Disparo manual forzado)");
                    ui.end_row();

                    ui.label(egui::RichText::new("Ctrl + 1 / Ctrl + 2").monospace().strong());
                    ui.label("Alternar entre modo Osciloscopio y modo Strip-Chart");
                    ui.end_row();

                    ui.label(egui::RichText::new("F1").monospace().strong());
                    ui.label("Abrir / Cerrar esta ventana de ayuda");
                    ui.end_row();
                });

                ui.add_space(10.0);
                if ui.button("Cerrar").clicked() {
                    want_close = true;
                }
            });
        });
    state.help_window_open = is_open && !want_close;
}

/// Ventana flotante de Zoom estilo PicoScope 7 con minimapa interactivo,
/// deslizador de ampliación, lupa, mano de paneo y restablecimiento 1:1.
pub fn show_zoom_window(
    ctx: &egui::Context,
    layout: &Layout,
    link: &crate::Link,
    phosphor: &mut crate::gpu::Phosphor,
    state: &mut AutomotiveUiState,
) {
    if !state.zoom_window_open || state.strip_chart_view_active {
        return;
    }

    let plot_rect = layout.points(layout.plot);
    let default_pos = egui::pos2(plot_rect.max.x - 235.0, plot_rect.min.y + 12.0);

    egui::Window::new("🔍 Zoom")
        .open(&mut state.zoom_window_open)
        .collapsible(true)
        .resizable(false)
        .default_pos(default_pos)
        .default_width(220.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // 1. Minimapa panorámico de la captura completa
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(210.0, 52.0),
                    egui::Sense::click_and_drag(),
                );
                let map_rect = response.rect;

                // Fondo oscuro del minimapa con marco
                painter.rect_filled(map_rect, 3.0, egui::Color32::from_rgb(14, 18, 24));
                painter.rect_stroke(
                    map_rect,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 70)),
                    egui::StrokeKind::Middle,
                );

                // Eje central de referencia
                let center_y = map_rect.center().y;
                painter.line_segment(
                    [egui::pos2(map_rect.min.x, center_y), egui::pos2(map_rect.max.x, center_y)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(60, 70, 90, 70)),
                );

                // Dibujar las señales completas de CH1 y CH2 en el minimapa
                if let Some(frame) = &link.latest {
                    for cap in &frame.channels {
                        if cap.data.is_empty() {
                            continue;
                        }
                        let col = if cap.ch == 0 {
                            egui::Color32::from_rgb(229, 192, 123) // CH1
                        } else {
                            egui::Color32::from_rgb(97, 175, 239) // CH2
                        };
                        let step = (cap.data.len() / (map_rect.width() as usize).max(1)).max(1);
                        let mut prev_pt: Option<egui::Pos2> = None;
                        for i in (0..cap.data.len()).step_by(step) {
                            let norm_x = i as f32 / cap.data.len() as f32;
                            let norm_y = (cap.data[i] / 250.0).clamp(-0.46, 0.46);
                            let pt = egui::pos2(
                                map_rect.min.x + norm_x * map_rect.width(),
                                center_y - norm_y * map_rect.height(),
                            );
                            if let Some(p0) = prev_pt {
                                painter.line_segment([p0, pt], egui::Stroke::new(1.0, col));
                            }
                            prev_pt = Some(pt);
                        }
                    }
                }

                // Ventana visible de zoom (Viewport translúcido)
                let (center, span) = phosphor.hview;
                let box_x0 = map_rect.min.x + ((center - span / 2.0).clamp(0.0, 1.0) as f32) * map_rect.width();
                let box_x1 = map_rect.min.x + ((center + span / 2.0).clamp(0.0, 1.0) as f32) * map_rect.width();

                let box_rect = egui::Rect::from_min_max(
                    egui::pos2(box_x0, map_rect.min.y),
                    egui::pos2(box_x1.max(box_x0 + 6.0), map_rect.max.y),
                );

                painter.rect_filled(
                    box_rect,
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(60, 140, 255, 60),
                );
                painter.rect_stroke(
                    box_rect,
                    2.0,
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 195, 255)),
                    egui::StrokeKind::Middle,
                );

                // Navegación en el tiempo al hacer clic o arrastrar en el minimapa
                if response.clicked() || response.dragged() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let frac = ((pointer_pos.x - map_rect.min.x) / map_rect.width()).clamp(0.0, 1.0) as f64;
                        phosphor.hview.0 = frac;
                        phosphor.hview = crate::view::hview_clamp(phosphor.hview.0, phosphor.hview.1);
                    }
                }

                ui.add_space(4.0);

                // 2. Controles de Zoom: [-] [ Slider de Factor ] [+]
                let zoom_mag = (1.0 / span.max(0.001)) as f32;
                ui.horizontal(|ui| {
                    if ui.add_sized(egui::vec2(22.0, 20.0), egui::Button::new("◀ -")).on_hover_text("Reducir zoom (alejar)").clicked() {
                        if span >= 0.98 {
                            crate::view::hview_home(phosphor);
                        } else {
                            crate::view::hview_zoom(phosphor, phosphor.hview.0, false);
                        }
                    }

                    // Deslizador interactivo de factor de aumento
                    let mut current_mag = zoom_mag;
                    let slider = egui::Slider::new(&mut current_mag, 1.0..=50.0)
                        .logarithmic(true)
                        .show_value(false);
                    if ui.add(slider).changed() {
                        let new_span = (1.0 / current_mag as f64).clamp(0.005, 1.0);
                        phosphor.hview.1 = new_span;
                        phosphor.hview = crate::view::hview_clamp(phosphor.hview.0, phosphor.hview.1);
                    }

                    if ui.add_sized(egui::vec2(22.0, 20.0), egui::Button::new("+ ▶")).on_hover_text("Aumentar zoom (acercar)").clicked() {
                        crate::view::hview_zoom(phosphor, phosphor.hview.0, true);
                    }
                });

                ui.add_space(2.0);

                // 3. Barra de herramientas estilo PicoScope 7
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{:.1}x", zoom_mag))
                            .monospace()
                            .strong()
                            .color(egui::Color32::from_rgb(255, 215, 60)),
                    );

                    ui.separator();

                    // Botón 1:1 restablecer
                    if ui.button("↶ 1:1").on_hover_text("Restablecer zoom al 100% de la captura").clicked() {
                        crate::view::hview_home(phosphor);
                    }

                    // Botón Mano / Paneo
                    let btn_hand = egui::Button::new("✋ Pan")
                        .fill(egui::Color32::from_rgb(38, 48, 66));
                    if ui.add(btn_hand).on_hover_text("Arrastre la forma de onda con el ratón: clic izq = horizontal, clic der = vertical").clicked() {
                    }

                    // Botón Lupa
                    if ui.button("🔍 Lupa").on_hover_text("Gire la rueda del ratón sobre la señal para hacer zoom centrado en el cursor").clicked() {
                        crate::view::hview_zoom(phosphor, phosphor.hview.0, true);
                    }
                });
            });
        });
}

/// Dibuja la vista de Análisis Dinámico de Actuadores (PWM vs Sensor / Curva de Transferencia)
pub fn show_actuator_analysis_view(
    ctx: &egui::Context,
    layout: &Layout,
    link: &crate::Link,
    state: &mut AutomotiveUiState,
    now: f64,
) {
    if state.app_mode != AppMode::ActuatorPwmVsSensor {
        return;
    }
    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 100.0 || plot_rect.height() < 100.0 {
        return;
    }

    // Ingestar tramas crudas del osciloscopio para correlacionar PWM vs Sensor
    for f in &link.arrived {
        let ch1 = f.channels.iter().find(|c| c.ch == 0).map(|c| &c.data[..]).unwrap_or(&[]);
        let ch2 = f.channels.iter().find(|c| c.ch == 1).map(|c| &c.data[..]).unwrap_or(&[]);
        if !ch1.is_empty() && !ch2.is_empty() {
            let v_div1 = (link.config.channels[0].volts_div * link.config.channels[0].probe * 5.0) as f32;
            let v_div2 = (link.config.channels[1].volts_div * link.config.channels[1].probe * 5.0) as f32;
            let ch1_v: Vec<f32> = ch1.iter().map(|&b| (b as f32 - 128.0) / 128.0 * v_div1).collect();
            let ch2_v: Vec<f32> = ch2.iter().map(|&b| (b as f32 - 128.0) / 128.0 * v_div2).collect();
            state.actuator.process_waveform(&ch1_v, &ch2_v, link.config.sample_rate, now);
        }
    }

    egui::Area::new("actuator_analysis_canvas".into())
        .fixed_pos(plot_rect.min)
        .show(ctx, |ui| {
            ui.set_width(plot_rect.width());
            ui.set_height(plot_rect.height());

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(14, 16, 22))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 60, 80)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // Cabecera con selector de Preset y Badges de Telemetría
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🎯 ANÁLISIS DE ACTUADORES: PWM vs SENSOR")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 175, 40)),
                        );

                        ui.separator();

                        // Selector de Preset
                        egui::ComboBox::from_id_salt("actuator_preset_combo")
                            .selected_text(state.actuator.preset.title())
                            .show_ui(ui, |ui| {
                                for p in ActuatorPreset::ALL {
                                    if ui.selectable_label(state.actuator.preset == p, p.title()).clicked() {
                                        state.actuator.preset = p;
                                    }
                                }
                            });

                        ui.separator();

                        if ui.button("🗑 Limpiar Curva").clicked() {
                            state.actuator.clear();
                        }
                    });

                    ui.add_space(4.0);

                    // Badges de telemetría y diagnósticos
                    ui.horizontal(|ui| {
                        let duty = state.actuator.current_duty;
                        let freq = state.actuator.current_freq;
                        let s_v = state.actuator.current_sensor_v;
                        let hyst = state.actuator.hysteresis_area;

                        ui.label(egui::RichText::new(format!("PWM Freq: {:.1} Hz", freq)).monospace().strong());
                        ui.label(egui::RichText::new(format!("Duty: {:.1} %", duty)).monospace().strong().color(egui::Color32::from_rgb(255, 215, 60)));
                        ui.label(egui::RichText::new(format!("Sensor: {:.2} V", s_v)).monospace().strong().color(egui::Color32::from_rgb(80, 200, 255)));
                        ui.label(egui::RichText::new(format!("Histéresis ΔV: {:.2} V", hyst)).monospace().strong());

                        ui.separator();

                        if state.actuator.mechanical_sticking_warning {
                            ui.label(
                                egui::RichText::new("⚠️ ALERTA: TRABAMIENTO MECÁNICO / ROZAMIENTO EXCESIVO")
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 70, 70)),
                            );
                        } else if state.actuator.sensor_noise_warning {
                            ui.label(
                                egui::RichText::new("⚠️ ADVERTENCIA: RUIDO / DISCONTINUIDAD EN SENSOR")
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 200, 40)),
                            );
                        } else if !state.actuator.points.is_empty() {
                            ui.label(
                                egui::RichText::new("✅ RESPUESTA NORMAL")
                                    .strong()
                                    .color(egui::Color32::from_rgb(80, 230, 120)),
                            );
                        }
                    });

                    ui.add_space(6.0);

                    // Lienzo Gráfico X-Y de la Curva de Transferencia
                    let avail = ui.available_size();
                    let (canvas_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                    let painter = ui.painter_at(canvas_rect);

                    // Fondo de cuadrícula
                    painter.rect_filled(canvas_rect, 4.0, egui::Color32::from_rgb(10, 12, 16));
                    painter.rect_stroke(canvas_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 48, 65)), egui::StrokeKind::Inside);

                    let grid_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(28, 34, 46));
                    let _axis_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 85, 115));

                    // Ejes: X = 0..100% Duty, Y = 0..5V Sensor
                    let pad_x = 40.0;
                    let pad_y = 25.0;
                    let plot_w = canvas_rect.width() - pad_x * 2.0;
                    let plot_h = canvas_rect.height() - pad_y * 2.0;

                    // Dibujar rejilla vertical (cada 10% Duty)
                    for k in 0..=10 {
                        let frac = k as f32 / 10.0;
                        let x = canvas_rect.min.x + pad_x + frac * plot_w;
                        painter.line_segment([egui::pos2(x, canvas_rect.min.y + pad_y), egui::pos2(x, canvas_rect.max.y - pad_y)], grid_stroke);
                        painter.text(
                            egui::pos2(x, canvas_rect.max.y - pad_y + 12.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{}%", k * 10),
                            egui::FontId::monospace(9.5),
                            egui::Color32::from_gray(140),
                        );
                    }

                    // Dibujar rejilla horizontal (cada 0.5V Sensor)
                    for k in 0..=10 {
                        let frac = k as f32 / 10.0;
                        let y = canvas_rect.max.y - pad_y - frac * plot_h;
                        painter.line_segment([egui::pos2(canvas_rect.min.x + pad_x, y), egui::pos2(canvas_rect.max.x - pad_x, y)], grid_stroke);
                        painter.text(
                            egui::pos2(canvas_rect.min.x + pad_x - 18.0, y),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.1}V", k as f32 * 0.5),
                            egui::FontId::monospace(9.5),
                            egui::Color32::from_gray(140),
                        );
                    }

                    // Línea ideal teórica (referencia lineal diagonal)
                    let ideal_p0 = egui::pos2(canvas_rect.min.x + pad_x, canvas_rect.max.y - pad_y - (0.5 / 5.0) * plot_h);
                    let ideal_p1 = egui::pos2(canvas_rect.max.x - pad_x, canvas_rect.max.y - pad_y - (4.5 / 5.0) * plot_h);
                    painter.line_segment([ideal_p0, ideal_p1], egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 200, 50, 70)));

                    // Trazar puntos de la curva de transferencia adquiridos
                    let points_count = state.actuator.points.len();
                    if points_count >= 2 {
                        let mut prev_screen_pos: Option<egui::Pos2> = None;
                        for (idx, p) in state.actuator.points.iter().enumerate() {
                            let norm_x = (p.duty_pct / 100.0).clamp(0.0, 1.0);
                            let norm_y = (p.sensor_v / 5.0).clamp(0.0, 1.0);
                            let sx = canvas_rect.min.x + pad_x + norm_x * plot_w;
                            let sy = canvas_rect.max.y - pad_y - norm_y * plot_h;
                            let screen_pos = egui::pos2(sx, sy);

                            let age_frac = idx as f32 / points_count as f32;
                            let alpha = (age_frac * 200.0 + 55.0) as u8;
                            let curve_col = egui::Color32::from_rgba_unmultiplied(50, 210, 255, alpha);

                            if let Some(prev) = prev_screen_pos {
                                painter.line_segment([prev, screen_pos], egui::Stroke::new(1.5, curve_col));
                            }
                            painter.circle_filled(screen_pos, 1.8, curve_col);
                            prev_screen_pos = Some(screen_pos);
                        }

                        // Marcador de punto de operación actual
                        if let Some(last_pos) = prev_screen_pos {
                            painter.circle_stroke(last_pos, 6.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 50)));
                            painter.circle_filled(last_pos, 3.0, egui::Color32::from_rgb(255, 80, 50));
                        }
                    }

                    // Etiquetas de los ejes
                    painter.text(
                        egui::pos2(canvas_rect.center().x, canvas_rect.max.y - 4.0),
                        egui::Align2::CENTER_CENTER,
                        state.actuator.preset.x_label(),
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(220, 230, 245),
                    );
                    painter.text(
                        egui::pos2(canvas_rect.min.x + 12.0, canvas_rect.center().y),
                        egui::Align2::CENTER_CENTER,
                        state.actuator.preset.y_label(),
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(220, 230, 245),
                    );
                });
        });
}

/// Dibuja la vista de Monitor de Tráfico de Bus de Red (CAN 2.0A/B & LIN)
pub fn show_bus_traffic_view(
    ctx: &egui::Context,
    layout: &Layout,
    state: &mut AutomotiveUiState,
    dec: &crate::decode::DecodeState,
) {
    if state.app_mode != AppMode::CanLinTraffic {
        return;
    }
    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 100.0 || plot_rect.height() < 100.0 {
        return;
    }

    // Ingestar paquetes del decodificador serie
    for pkt in &dec.packets {
        let parsed_id = u32::from_str_radix(pkt.id_or_pid.trim_start_matches("0x"), 16).unwrap_or(0);
        match pkt.protocol {
            crate::decode::Protocol::Can => {
                state.traffic.ingest_can(pkt.start_time, parsed_id, false, pkt.data.len(), &pkt.data);
            }
            crate::decode::Protocol::Lin => {
                state.traffic.ingest_lin(pkt.start_time, parsed_id as u8, pkt.data.len(), &pkt.data);
            }
            _ => {}
        }
    }

    egui::Area::new("bus_traffic_canvas".into())
        .fixed_pos(plot_rect.min)
        .show(ctx, |ui| {
            ui.set_width(plot_rect.width());
            ui.set_height(plot_rect.height());

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(12, 14, 18))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // Cabecera del analizador de tráfico
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🌐 MONITOR DE TRÁFICO DE RED Y TRAMAS DE PROTOCOLO")
                                .strong()
                                .color(egui::Color32::from_rgb(100, 200, 255)),
                        );

                        ui.separator();

                        let bus_name = match state.traffic.bus {
                            BusProtocol::Can => "CAN 2.0A/B",
                            BusProtocol::Lin => "LIN 1.3/2.x",
                        };
                        if ui.button(egui::RichText::new(format!("Bus: {bus_name}")).strong()).clicked() {
                            state.traffic.bus = match state.traffic.bus {
                                BusProtocol::Can => BusProtocol::Lin,
                                BusProtocol::Lin => BusProtocol::Can,
                            };
                        }

                        ui.separator();

                        ui.label(
                            egui::RichText::new(format!("Tramas Capturadas: {}", state.traffic.total_frames_captured))
                                .monospace(),
                        );

                        ui.separator();

                        let pause_text = if state.traffic.pause_updates { "▶ Reanudar" } else { "⏸ Pausar" };
                        if ui.button(pause_text).clicked() {
                            state.traffic.pause_updates = !state.traffic.pause_updates;
                        }

                        if ui.button("🗑 Limpiar").clicked() {
                            state.traffic.clear();
                        }
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // Tabla de tramas agrupadas por ID
                    egui::ScrollArea::both()
                        .max_height(plot_rect.height() - 70.0)
                        .show(ui, |ui| {
                            egui::Grid::new("bus_traffic_grid")
                                .striped(true)
                                .spacing([12.0, 4.0])
                                .show(ui, |ui| {
                                    // Encabezados
                                    ui.label(egui::RichText::new("ID (Hex)").strong().color(egui::Color32::from_rgb(255, 175, 50)));
                                    ui.label(egui::RichText::new("Conteo").strong().color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new("Tasa (Hz)").strong().color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new("Δt (ms)").strong().color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new("DLC").strong().color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new("Datos / Payload (Hex) [Cambios Resaltados]").strong().color(egui::Color32::from_rgb(120, 220, 150)));
                                    ui.end_row();

                                    let rows = state.traffic.get_sorted_rows();
                                    for r in rows {
                                        let id_str = format!("0x{:03X}", r.id);
                                        ui.label(egui::RichText::new(id_str).monospace().strong().color(egui::Color32::from_rgb(255, 200, 60)));
                                        ui.label(egui::RichText::new(format!("{}", r.count)).monospace());
                                        ui.label(egui::RichText::new(format!("{:.1}", r.freq_hz)).monospace());
                                        ui.label(egui::RichText::new(format!("{:.1}", r.delta_ms)).monospace());
                                        ui.label(egui::RichText::new(format!("{}", r.dlc)).monospace());

                                        // Renderizado de bytes de datos con resaltado dinámico en vivo
                                        ui.horizontal(|ui| {
                                            for (byte_idx, &b) in r.data.iter().enumerate() {
                                                let has_changed = (r.changed_mask & (1 << byte_idx)) != 0;
                                                let byte_text = format!("{:02X}", b);
                                                if has_changed {
                                                    ui.label(
                                                        egui::RichText::new(byte_text)
                                                            .monospace()
                                                            .strong()
                                                            .color(egui::Color32::BLACK)
                                                            .background_color(egui::Color32::from_rgb(255, 185, 30)),
                                                    );
                                                } else {
                                                    ui.label(
                                                        egui::RichText::new(byte_text)
                                                            .monospace()
                                                            .color(egui::Color32::from_gray(210)),
                                                    );
                                                }
                                            }
                                        });

                                        ui.end_row();
                                    }
                                });
                        });
                });
        });
}

/// Aplica los parámetros de un equipo trazador al hardware del osciloscopio
pub fn apply_tracer_equipment_to_link(
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
    eq: &TracerEquipment,
) {
    link.config.channels[0].enabled = true;
    link.config.channels[0].coupling = neowon_core::Coupling::Dc;
    link.config.channels[0].volts_div = eq.ch1_vdiv;
    link.config.channels[0].probe = eq.ch1_probe;
    link.config.channels[0].offset = 0.0;

    link.config.channels[1].enabled = true;
    link.config.channels[1].coupling = neowon_core::Coupling::Dc;
    link.config.channels[1].volts_div = eq.ch2_vdiv;
    link.config.channels[1].probe = eq.ch2_probe;
    link.config.channels[1].offset = 0.0;

    crate::view::set_timebase(link, eq.timebase);
    link.config.trigger.sweep = if eq.trigger_auto { neowon_core::Sweep::Auto } else { neowon_core::Sweep::Normal };
    link.config.trigger.level = eq.trigger_level;
    link.config.trigger.source = eq.trigger_channel.min(1);
    link.config.roll = false; // Modo síncrono obligatorio para ráfagas promediadas
    link.config.running = true;
    link.dirty = true;

    state.vi_tracer.apply_equipment(eq);
}

/// Aplica un preset de trazador al hardware del osciloscopio
fn apply_tracer_preset_to_link(
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
    preset: &ViTracerPreset,
) {
    link.config.channels[0].enabled = true;
    link.config.channels[0].coupling = neowon_core::Coupling::Dc;
    link.config.channels[0].volts_div = preset.ch1_vdiv;
    link.config.channels[0].probe = preset.ch1_probe;
    link.config.channels[0].offset = 0.0;

    link.config.channels[1].enabled = true;
    link.config.channels[1].coupling = neowon_core::Coupling::Dc;
    link.config.channels[1].volts_div = preset.ch2_vdiv;
    link.config.channels[1].probe = preset.ch2_probe;
    link.config.channels[1].offset = 0.0;

    crate::view::set_timebase(link, preset.timebase);
    link.config.trigger.sweep = neowon_core::Sweep::Auto;
    link.config.trigger.level = 0.0;
    link.config.roll = false; // Modo síncrono obligatorio para ráfagas promediadas
    link.dirty = true;

    state.vi_tracer.apply_preset(preset);
}

/// Guarda un snapshot PNG de la curva V-I y referencia en la ruta indicada
pub fn save_vi_snapshot_png_to_path(
    points: &[(f32, f32)],
    ref_points: Option<&[(f32, f32)]>,
    v_div: f32,
    i_div_ma: f32,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let width = 600u32;
    let height = 600u32;
    let mut img = vec![8u8; (width * height * 3) as usize];

    let mut put_pixel = |x: i32, y: i32, r: u8, g: u8, b: u8| {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let idx = ((y as usize * width as usize) + x as usize) * 3;
            img[idx] = r;
            img[idx + 1] = g;
            img[idx + 2] = b;
        }
    };

    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let step_x = width as i32 / 10;
    let step_y = height as i32 / 10;

    // Retícula técnica de 10 divisiones exactas
    for i in 1..10 {
        let gx = step_x * i;
        let gy = step_y * i;
        for y in 0..height as i32 {
            put_pixel(gx, y, 22, 28, 38);
        }
        for x in 0..width as i32 {
            put_pixel(x, gy, 22, 28, 38);
        }
    }

    // Ejes ortogonales principales (V = 0 e I = 0)
    for y in 0..height as i32 {
        put_pixel(cx, y, 70, 95, 130);
        put_pixel(cx + 1, y, 70, 95, 130);
    }
    for x in 0..width as i32 {
        put_pixel(x, cy, 70, 95, 130);
        put_pixel(x, cy + 1, 70, 95, 130);
    }

    // Escala física: cada división = step_x pixels
    let scale_v = (step_x as f32) / v_div.max(0.001);
    let scale_i = (step_y as f32) / i_div_ma.max(0.001);

    // Dibujar trazo de referencia (verde esmeralda)
    if let Some(refs) = ref_points {
        for w in refs.windows(2) {
            let x0 = (cx + (w[0].0 * scale_v) as i32).clamp(0, width as i32 - 1);
            let y0 = (cy - (w[0].1 * scale_i) as i32).clamp(0, height as i32 - 1);
            let x1 = (cx + (w[1].0 * scale_v) as i32).clamp(0, width as i32 - 1);
            let y1 = (cy - (w[1].1 * scale_i) as i32).clamp(0, height as i32 - 1);
            draw_thick_line(&mut put_pixel, x0, y0, x1, y1, 80, 220, 120);
        }
    }

    // Dibujar trazo en vivo (cian brillante)
    for w in points.windows(2) {
        let x0 = (cx + (w[0].0 * scale_v) as i32).clamp(0, width as i32 - 1);
        let y0 = (cy - (w[0].1 * scale_i) as i32).clamp(0, height as i32 - 1);
        let x1 = (cx + (w[1].0 * scale_v) as i32).clamp(0, width as i32 - 1);
        let y1 = (cy - (w[1].1 * scale_i) as i32).clamp(0, height as i32 - 1);
        draw_thick_line(&mut put_pixel, x0, y0, x1, y1, 40, 220, 255);
    }

    let file = std::fs::File::create(path)?;
    let w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&img)?;

    Ok(())
}

/// Guarda un snapshot PNG de la curva V-I y referencia en la carpeta Capturas/Tracer_VI/
fn save_vi_snapshot_png(
    points: &[(f32, f32)],
    ref_points: Option<&[(f32, f32)]>,
    v_div: f32,
    i_div_ma: f32,
    ref_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let folder = std::path::Path::new("Capturas/Tracer_VI");
    if !folder.exists() {
        let _ = std::fs::create_dir_all(folder);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sanitized_name: String = ref_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let filename = format!("Capturas/Tracer_VI/TracerVI_{}_{}.png", sanitized_name, now);
    let path = std::path::Path::new(&filename);
    save_vi_snapshot_png_to_path(points, ref_points, v_div, i_div_ma, path)?;
    Ok(filename)
}

fn draw_thick_line<F: FnMut(i32, i32, u8, u8, u8)>(
    put: &mut F,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    r: u8,
    g: u8,
    b: u8,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        put(x0, y0, r, g, b);
        put(x0 + 1, y0, r, g, b);
        put(x0, y0 + 1, r, g, b);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Dibuja la vista de Trazador de Curvas V-I (Análisis de Impedancia y Firmas de Pines)
pub fn show_vi_tracer_view(
    ctx: &egui::Context,
    layout: &Layout,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
) {
    if state.app_mode != AppMode::ViCurveTracer {
        return;
    }
    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 100.0 || plot_rect.height() < 100.0 {
        return;
    }

    // 1. Configuración de hardware dedicada para el Trazador VI:
    // Auto-inicializar al entrar si no se ha configurado (Cargar equipo trazador activo)
    if !state.vi_tracer.has_saved_config {
        let active_eq = state.vi_tracer.equipment_store.active_equipment().clone();
        apply_tracer_equipment_to_link(link, state, &active_eq);
    }

    // 2. Ingestar muestras simultáneas CH1 (Tensión DUT) y CH2 (Corriente vía R_sense) si no está congelado
    if !state.vi_tracer.frozen {
        // Drenar cola de captura para máxima velocidad de refresco (captura directa sin retrasos de cola)
        let mut latest_frame = None;
        while let Some(f) = link.arrived.pop() {
            latest_frame = Some(f);
        }
        let frame_ref = latest_frame.as_deref().or_else(|| link.latest.as_deref());
        if let Some(f) = frame_ref {
            let c1 = f.channels.iter().find(|c| c.ch == 0);
            let c2 = f.channels.iter().find(|c| c.ch == 1);
            if let (Some(c1), Some(c2)) = (c1, c2) {
                if !c1.data.is_empty() && !c2.data.is_empty() {
                    let v_samples: Vec<f32> = c1.iter_volts().map(|v| v as f32).collect();
                    let i_samples: Vec<f32> = c2.iter_volts().map(|v| v as f32).collect();

                    // Sincronización inteligente en hardware (evita trazo cortado/fragmentado en cortocircuito):
                    // Si V_dut en CH1 es casi nulo (<0.35V) y hay corriente en CH2, la FPGA debe sincronizar en CH2
                    if !link.config.roll && !v_samples.is_empty() && !i_samples.is_empty() {
                        let v_max = v_samples.iter().cloned().fold(f32::MIN, f32::max);
                        let v_min = v_samples.iter().cloned().fold(f32::MAX, f32::min);
                        let i_max = i_samples.iter().cloned().fold(f32::MIN, f32::max);
                        let i_min = i_samples.iter().cloned().fold(f32::MAX, f32::min);
                        let v_vpp = v_max - v_min;
                        let i_vpp = i_max - i_min;

                        if v_vpp < 0.35 && i_vpp >= 0.5 {
                            if link.config.trigger.source != 1 {
                                link.config.trigger.source = 1;
                                link.dirty = true;
                            }
                        } else if v_vpp >= 0.6 {
                            if link.config.trigger.source != 0 {
                                link.config.trigger.source = 0;
                                link.dirty = true;
                            }
                        }
                    }

                    state.vi_tracer.process_samples(&v_samples, &i_samples);
                }
            }
        }
        ctx.request_repaint();
    }

    // 3. Lienzo principal exclusivo del Trazador VI: Cuadrícula cuadrada a la izquierda + Acciones a la derecha
    egui::Area::new("vi_tracer_canvas".into())
        .fixed_pos(plot_rect.min)
        .show(ctx, |ui| {
            ui.set_width(plot_rect.width());
            ui.set_height(plot_rect.height());

            egui::Frame::new()
                .fill(egui::Color32::from_rgb(10, 13, 18))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // Cabecera superior compacta con Selector y CRUD de Equipos Trazadores
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("⚡ TRAZADOR DE CURVAS V-I")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 210, 50)),
                        );

                        ui.separator();

                        // Selector de Equipo Trazador
                        let active_id = state.vi_tracer.equipment_store.active_id.clone();
                        let active_name = state.vi_tracer.equipment_store.active_equipment().name.clone();

                        let mut new_active_id = None;
                        egui::ComboBox::from_id_salt("tracer_eq_selector")
                            .selected_text(egui::RichText::new(format!("📌 {}", active_name)).strong().color(egui::Color32::from_rgb(120, 200, 255)))
                            .show_ui(ui, |ui| {
                                for eq in &state.vi_tracer.equipment_store.equipments {
                                    let is_active = eq.id == active_id;
                                    let label = format!("{} (±{:.0}V, ±{:.0}mA)", eq.name, eq.eff_ch1_vdiv() * 5.0, eq.eff_ch2_idiv_ma() * 5.0);
                                    if ui.selectable_label(is_active, label).clicked() {
                                        new_active_id = Some(eq.id.clone());
                                    }
                                }
                            });

                        if let Some(id) = new_active_id {
                            state.vi_tracer.equipment_store.set_active(&id);
                            let eq = state.vi_tracer.equipment_store.active_equipment().clone();
                            apply_tracer_equipment_to_link(link, state, &eq);
                        }

                        // Botones CRUD: Crear / Editar / Borrar
                        if ui.button("➕ Nuevo").on_hover_text("Crear un nuevo perfil de equipo trazador").clicked() {
                            state.vi_tracer.is_creating_new = true;
                            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
                            let mut new_eq = TracerEquipment::new_custom("Nuevo Tracer", 0.5, 10.0, 0.5, 10.0, 10.0e-3, true, 1000.0);
                            new_eq.id = format!("eq_{ts}");
                            state.vi_tracer.editor_equipment = Some(new_eq);
                        }

                        if ui.button("✏ Editar").on_hover_text("Editar los ajustes de este equipo trazador").clicked() {
                            let eq = state.vi_tracer.equipment_store.active_equipment().clone();
                            state.vi_tracer.is_creating_new = false;
                            state.vi_tracer.editor_equipment = Some(eq);
                        }

                        if state.vi_tracer.equipment_store.equipments.len() > 1 {
                            if ui.button("🗑 Borrar").on_hover_text("Borrar este equipo trazador").clicked() {
                                let id_del = state.vi_tracer.equipment_store.active_id.clone();
                                state.vi_tracer.equipment_store.delete_equipment(&id_del);
                                let eq = state.vi_tracer.equipment_store.active_equipment().clone();
                                apply_tracer_equipment_to_link(link, state, &eq);
                            }
                        }

                        ui.separator();

                        // Estado de captura
                        let frozen = state.vi_tracer.frozen;
                        ui.label(
                            egui::RichText::new(if frozen { "❄ CONGELADO" } else { "🟢 EN VIVO" })
                                .size(11.0)
                                .strong()
                                .color(if frozen { egui::Color32::from_rgb(80, 180, 255) } else { egui::Color32::from_rgb(80, 235, 120) }),
                        );

                        // Botón de ventana de canales
                        let ms_open = state.vi_tracer.mini_scope_open;
                        if ui.button(if ms_open { "📺 Ocultar Canales" } else { "📺 Ver Canales" }).clicked() {
                            state.vi_tracer.mini_scope_open = !ms_open;
                        }

                        // Conmutador rápido Vectores / Puntos
                        let dots_mode = state.vi_tracer.draw_dots;
                        if ui.selectable_label(dots_mode, if dots_mode { "⠂ Puntos" } else { "📈 Vectores" })
                            .on_hover_text("Alternar visualización de curva: por puntos discretos o por vectores continuos")
                            .clicked()
                        {
                            state.vi_tracer.draw_dots = !dots_mode;
                        }

                        // Botón de Setup General
                        if ui.button("⚙ Setup...").on_hover_text("Configurar equipos trazadores, canales, escalas y visualización").clicked() {
                            state.vi_tracer.setup_dialog_open = true;
                        }

                        // Mensaje de estado temporal (captura guardada, etc.)
                        if let Some((ref msg, _)) = state.vi_tracer.status_message {
                            ui.separator();
                            ui.label(
                                egui::RichText::new(msg)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(100, 240, 150)),
                            );
                        }
                    });

                    ui.add_space(6.0);

                    // Cuerpo principal dividido en dos: Cuadrícula CUADRADA a la izquierda y Columna de Acciones a la derecha
                    let total_avail = ui.available_size();
                    let sidebar_width = 330.0;
                    let gap = 12.0;

                    let max_sq_w = (total_avail.x - sidebar_width - gap).max(240.0);
                    let max_sq_h = (total_avail.y - 6.0).max(240.0);
                    let sq_size = max_sq_w.min(max_sq_h);

                    ui.horizontal(|ui| {
                        // ==========================================
                        // COLUMNA IZQUIERDA: Cuadrícula Cuadrada (1:1)
                        // ==========================================
                        let (canvas_rect, _) = ui.allocate_exact_size(egui::vec2(sq_size, sq_size), egui::Sense::hover());
                        let painter = ui.painter_at(canvas_rect);

                        painter.rect_filled(canvas_rect, 4.0, egui::Color32::from_rgb(6, 8, 12));
                        painter.rect_stroke(canvas_rect, 4.0, egui::Stroke::new(1.2, egui::Color32::from_rgb(38, 48, 68)), egui::StrokeKind::Inside);

                        let center = canvas_rect.center();

                        // Escalas efectivas derivadas de los canales de hardware y R_sense:
                        // 1 división horizontal = ch1_eff_vdiv (Voltios)
                        // 1 división vertical = ch2_eff_idiv_ma (miliamperios)
                        let ch1_eff_vdiv = (link.config.channels[0].volts_div * link.config.channels[0].probe).max(0.001);
                        let ch2_eff_vdiv = (link.config.channels[1].volts_div * link.config.channels[1].probe).max(0.001);
                        let r_sense = (state.vi_tracer.r_sense_ohms as f64).max(0.1);
                        let ch2_eff_idiv_ma = (ch2_eff_vdiv / r_sense) * 1000.0;

                        // Cantidad configurable de divisiones (por defecto 4x4)
                        let divs_h = state.vi_tracer.grid_divs_h.max(2);
                        let divs_v = state.vi_tracer.grid_divs_v.max(2);
                        let half_h = (divs_h / 2) as i32;
                        let half_v = (divs_v / 2) as i32;
                        let div_step_x = sq_size / divs_h as f32;
                        let div_step_y = sq_size / divs_v as f32;

                        // Rejilla de divisiones verticales (Eje X de Tensión: -half_h a +half_h divisiones)
                        for k in -half_h..=half_h {
                            let x = center.x + k as f32 * div_step_x;
                            let is_axis = k == 0;
                            let stroke = if is_axis {
                                egui::Stroke::new(1.4, egui::Color32::from_rgb(70, 95, 130))
                            } else {
                                egui::Stroke::new(0.6, egui::Color32::from_rgb(28, 36, 52))
                            };
                            painter.line_segment([egui::pos2(x, canvas_rect.min.y), egui::pos2(x, canvas_rect.max.y)], stroke);

                            // Etiqueta de voltaje de referencia calculada en cada división
                            let v_val = k as f64 * ch1_eff_vdiv;
                            let v_str = if v_val.abs() >= 10.0 {
                                format!("{:.0}V", v_val)
                            } else if v_val.abs() >= 1.0 {
                                format!("{:.1}V", v_val)
                            } else if k == 0 {
                                "0V".to_string()
                            } else {
                                format!("{:.2}V", v_val)
                            };

                            let text_y = center.y + 4.0;
                            painter.text(
                                egui::pos2(x, text_y),
                                egui::Align2::CENTER_TOP,
                                v_str,
                                egui::FontId::monospace(9.0),
                                if is_axis { egui::Color32::from_rgb(210, 225, 245) } else { egui::Color32::from_rgb(130, 145, 170) },
                            );
                        }

                        // Rejilla de divisiones horizontales (Eje Y de Corriente: -half_v a +half_v divisiones)
                        for m in -half_v..=half_v {
                            let y = center.y - m as f32 * div_step_y;
                            let is_axis = m == 0;
                            let stroke = if is_axis {
                                egui::Stroke::new(1.4, egui::Color32::from_rgb(70, 95, 130))
                            } else {
                                egui::Stroke::new(0.6, egui::Color32::from_rgb(28, 36, 52))
                            };
                            painter.line_segment([egui::pos2(canvas_rect.min.x, y), egui::pos2(canvas_rect.max.x, y)], stroke);

                            // Etiqueta de corriente de referencia en mA
                            if m != 0 {
                                let i_val = m as f64 * ch2_eff_idiv_ma;
                                let i_str = if i_val.abs() >= 10.0 {
                                    format!("{:.0}mA", i_val)
                                } else {
                                    format!("{:.1}mA", i_val)
                                };
                                painter.text(
                                    egui::pos2(center.x + 6.0, y - 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    i_str,
                                    egui::FontId::monospace(9.0),
                                    egui::Color32::from_rgb(130, 145, 170),
                                );
                            }
                        }



                        // Curva de Referencia (Golden Sample) en verde esmeralda
                        if let Some(ref ref_pts) = state.vi_tracer.reference_points {
                            if state.vi_tracer.draw_dots {
                                for pt in ref_pts {
                                    let pos = egui::pos2(
                                        (center.x + (pt.0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                        (center.y - (pt.1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                    );
                                    painter.circle_filled(pos, 1.6, egui::Color32::from_rgba_unmultiplied(80, 235, 120, 160));
                                }
                            } else {
                                for window in ref_pts.windows(2) {
                                    let p0 = egui::pos2(
                                        (center.x + (window[0].0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                        (center.y - (window[0].1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                    );
                                    let p1 = egui::pos2(
                                        (center.x + (window[1].0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                        (center.y - (window[1].1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                    );
                                    if (p1 - p0).length() < canvas_rect.width() * 0.40 {
                                        painter.line_segment([p0, p1], egui::Stroke::new(1.6, egui::Color32::from_rgba_unmultiplied(80, 235, 120, 160)));
                                    }
                                }
                                if ref_pts.len() >= 3 {
                                    let p_last = egui::pos2(
                                        (center.x + (ref_pts[ref_pts.len() - 1].0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                        (center.y - (ref_pts[ref_pts.len() - 1].1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                    );
                                    let p_first = egui::pos2(
                                        (center.x + (ref_pts[0].0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                        (center.y - (ref_pts[0].1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                    );
                                    if (p_first - p_last).length() < canvas_rect.width() * 0.40 {
                                        painter.line_segment([p_last, p_first], egui::Stroke::new(1.6, egui::Color32::from_rgba_unmultiplied(80, 235, 120, 160)));
                                    }
                                }
                            }
                        }

                        // Curva V-I En Vivo (DUT) en cian brillante
                        let live_pts = &state.vi_tracer.current_points;
                        if state.vi_tracer.draw_dots {
                            for pt in live_pts {
                                let pos = egui::pos2(
                                    (center.x + (pt.0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                    (center.y - (pt.1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                );
                                painter.circle_filled(pos, 1.3, egui::Color32::from_rgb(40, 215, 255));
                            }
                        } else if live_pts.len() >= 2 {
                            for window in live_pts.windows(2) {
                                let p0 = egui::pos2(
                                    (center.x + (window[0].0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                    (center.y - (window[0].1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                );
                                let p1 = egui::pos2(
                                    (center.x + (window[1].0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                    (center.y - (window[1].1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                );
                                // Evitar trazo diagonal de retorno que cruce la pantalla completa en caso de wrap-around
                                if (p1 - p0).length() > canvas_rect.width() * 0.40 {
                                    continue;
                                }
                                painter.line_segment([p0, p1], egui::Stroke::new(1.8, egui::Color32::from_rgb(40, 215, 255)));
                            }
                            // Cerrar el bucle uniendo el último punto con el primero para evitar la hendidura/corte central al paso por cero
                            if live_pts.len() >= 3 {
                                let first = &live_pts[0];
                                let last = &live_pts[live_pts.len() - 1];
                                let p_last = egui::pos2(
                                    (center.x + (last.0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                    (center.y - (last.1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                );
                                let p_first = egui::pos2(
                                    (center.x + (first.0 / ch1_eff_vdiv as f32) * div_step_x).clamp(canvas_rect.min.x, canvas_rect.max.x),
                                    (center.y - (first.1 / ch2_eff_idiv_ma as f32) * div_step_y).clamp(canvas_rect.min.y, canvas_rect.max.y),
                                );
                                if (p_first - p_last).length() < canvas_rect.width() * 0.40 {
                                    painter.line_segment([p_last, p_first], egui::Stroke::new(1.8, egui::Color32::from_rgb(40, 215, 255)));
                                }
                            }
                        }

                        // Badges en las esquinas de la cuadrícula
                        // 1. Esquina superior izquierda: Estado En Vivo / Congelado
                        let status_badge_rect = egui::Rect::from_min_size(canvas_rect.min + egui::vec2(8.0, 8.0), egui::vec2(140.0, 20.0));
                        let (badge_text, bg_color, txt_color) = if state.vi_tracer.frozen {
                            ("❄ CONGELADO".to_string(), egui::Color32::from_rgb(20, 45, 75), egui::Color32::from_rgb(140, 210, 255))
                        } else {
                            let n = state.vi_tracer.recent_cycles.len().max(1);
                            (format!("🎯 RÁFAGA SÍNCRONA (N={n})"), egui::Color32::from_rgb(15, 60, 40), egui::Color32::from_rgb(80, 245, 160))
                        };
                        painter.rect_filled(status_badge_rect, 3.0, bg_color);
                        painter.text(
                            status_badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &badge_text,
                            egui::FontId::monospace(9.5),
                            txt_color,
                        );

                        // 2. Badge del Algoritmo de Tolerancia Cuantitativa Automática
                        if let Some(ref tol) = state.vi_tracer.tolerance_channel {
                            let tol_badge_rect = egui::Rect::from_min_size(canvas_rect.min + egui::vec2(8.0, 32.0), egui::vec2(250.0, 18.0));
                            let (bg_col, txt_col, badge_msg) = if let Some(in_tol) = tol.in_tolerance_percent {
                                let (bg, txt, icon) = if in_tol >= 90.0 {
                                    (egui::Color32::from_rgb(18, 55, 30), egui::Color32::from_rgb(90, 240, 140), "✅")
                                } else if in_tol >= 75.0 {
                                    (egui::Color32::from_rgb(55, 45, 15), egui::Color32::from_rgb(240, 200, 70), "⚠️")
                                } else {
                                    (egui::Color32::from_rgb(60, 20, 20), egui::Color32::from_rgb(255, 90, 90), "❌")
                                };
                                (bg, txt, format!("{icon} TOL: {in_tol:.1}% | ±{:.0}mV, ±{:.2}mA", tol.avg_dispersion_mv, tol.avg_dispersion_ma))
                            } else {
                                (egui::Color32::from_rgb(12, 28, 45), egui::Color32::from_rgb(90, 210, 255), format!("📊 ESTABILIDAD: {:.0}% | ±{:.0}mV, ±{:.2}mA", tol.stability_percent, tol.avg_dispersion_mv, tol.avg_dispersion_ma))
                            };
                            painter.rect_filled(tol_badge_rect, 3.0, bg_col);
                            painter.text(
                                tol_badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                badge_msg,
                                egui::FontId::monospace(9.5),
                                txt_col,
                            );
                        }

                        // 2. Esquina superior derecha: Referencia fijada
                        if state.vi_tracer.reference_points.is_some() {
                            let ref_label = format!("📌 Ref: {}", state.vi_tracer.reference_name);
                            painter.text(
                                egui::pos2(canvas_rect.max.x - 10.0, canvas_rect.min.y + 16.0),
                                egui::Align2::RIGHT_CENTER,
                                ref_label,
                                egui::FontId::monospace(10.0),
                                egui::Color32::from_rgb(90, 235, 130),
                            );
                        }

                        // 3. Esquina inferior izquierda: Comparador de similitud
                        if let Some(sim) = state.vi_tracer.similarity_percent {
                            let (badge_color, badge_text) = if sim >= 90.0 {
                                (egui::Color32::from_rgb(20, 60, 35), format!("✅ PINES IDÉNTICOS ({sim:.1}%)"))
                            } else if sim >= 75.0 {
                                (egui::Color32::from_rgb(70, 55, 15), format!("⚠️ LEVE DESVIACIÓN ({sim:.1}%)"))
                            } else {
                                (egui::Color32::from_rgb(75, 25, 25), format!("❌ PIN DIFERENTE ({sim:.1}%)"))
                            };
                            let sim_rect = egui::Rect::from_min_size(canvas_rect.left_bottom() + egui::vec2(8.0, -28.0), egui::vec2(190.0, 20.0));
                            painter.rect_filled(sim_rect, 3.0, badge_color);
                            painter.text(
                                sim_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                badge_text,
                                egui::FontId::monospace(10.0),
                                egui::Color32::WHITE,
                            );
                        }

                        // 4. Esquina inferior derecha: Escalas activas sincronizadas con canales
                        let r_str = if r_sense >= 1000.0 { format!("{:.0}kΩ", r_sense / 1000.0) } else { format!("{:.0}Ω", r_sense) };
                        let total_max_v = ch1_eff_vdiv * half_h as f64;
                        let total_max_i = ch2_eff_idiv_ma * half_v as f64;
                        let info_str = format!("R: {r_str} | {:.1}V/div, {:.1}mA/div (±{total_max_v:.0}V, ±{total_max_i:.0}mA) [{divs_h}x{divs_v} div]", ch1_eff_vdiv, ch2_eff_idiv_ma);
                        painter.text(
                            egui::pos2(canvas_rect.max.x - 8.0, canvas_rect.max.y - 12.0),
                            egui::Align2::RIGHT_CENTER,
                            info_str,
                            egui::FontId::monospace(9.0),
                            egui::Color32::from_rgb(140, 160, 190),
                        );

                        ui.add_space(gap);

                        // ==========================================
                        // COLUMNA DERECHA: Acciones y Herramientas
                        // ==========================================
                        ui.allocate_ui_with_layout(
                            egui::vec2(sidebar_width, total_avail.y),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.set_width(sidebar_width - 8.0);

                                        // ----------------------------------------------------
                                        // GRUPO 1: Congelación y Captura de Imagen
                                        // ----------------------------------------------------
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(14, 18, 26))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 45, 65)))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("⚡ EJECUCIÓN Y CAPTURA")
                                                        .size(11.0)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(255, 215, 60)),
                                                );
                                                ui.add_space(4.0);

                                                ui.horizontal(|ui| {
                                                    // Botón de Congelar / Reanudar
                                                    let is_f = state.vi_tracer.frozen;
                                                    let freeze_btn = egui::Button::new(
                                                        egui::RichText::new(if is_f { "▶ REANUDAR" } else { "❄ CONGELAR" })
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(if is_f { egui::Color32::from_rgb(25, 90, 50) } else { egui::Color32::from_rgb(30, 65, 105) })
                                                    .min_size(egui::vec2(130.0, 26.0));

                                                    if ui.add(freeze_btn).on_hover_text("Pausar / Congelar la curva en pantalla para inspección estática").clicked() {
                                                        state.vi_tracer.toggle_freeze();
                                                    }

                                                    // Botón de Captura de Imagen PNG
                                                    let snap_btn = egui::Button::new(
                                                        egui::RichText::new("📸 CAPTURAR PNG")
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(egui::Color32::from_rgb(38, 50, 70))
                                                    .min_size(egui::vec2(130.0, 26.0));

                                                    if ui.add(snap_btn).on_hover_text("Guardar imagen PNG de la curva en ./Capturas/Tracer_VI/").clicked() {
                                                        match save_vi_snapshot_png(
                                                            &state.vi_tracer.current_points,
                                                            state.vi_tracer.reference_points.as_deref(),
                                                            ch1_eff_vdiv as f32,
                                                            ch2_eff_idiv_ma as f32,
                                                            &state.vi_tracer.reference_name,
                                                        ) {
                                                            Ok(path) => {
                                                                state.vi_tracer.status_message = Some((format!("Guardado: {path}"), 0.0));
                                                            }
                                                            Err(e) => {
                                                                state.vi_tracer.status_message = Some((format!("Error: {e}"), 0.0));
                                                            }
                                                        }
                                                    }

                                                    // Guardar firma en Base de Datos (con ajustes HW y notas)
                                                    let save_db_btn = egui::Button::new(
                                                        egui::RichText::new("💾 GUARDAR EN BD")
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(egui::Color32::from_rgb(25, 80, 50))
                                                    .min_size(egui::vec2(130.0, 26.0));
                                                    if ui.add(save_db_btn).on_hover_text("Guardar firma actual en la Base de Datos con metadatos, notas y ajustes de hardware").clicked() {
                                                        state.vi_capture.is_save_dialog_open = true;
                                                    }

                                                    // Cargar firma desde Base de Datos
                                                    let load_db_btn = egui::Button::new(
                                                        egui::RichText::new("📂 BIBLIOTECA BD")
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(egui::Color32::from_rgb(30, 65, 105))
                                                    .min_size(egui::vec2(130.0, 26.0));
                                                    if ui.add(load_db_btn).on_hover_text("Abrir biblioteca de modelos/firmas guardadas y usar como Golden Sample con tolerancia cuantitativa").clicked() {
                                                        state.vi_capture.db = picowon_auto::ViModelDatabase::load_or_default();
                                                        state.vi_capture.is_load_db_dialog_open = true;
                                                    }
                                                });
                                            });

                                        ui.add_space(6.0);

                                        // ----------------------------------------------------
                                        // GRUPO 2: Comparador de Pines (ECU Golden Sample)
                                        // ----------------------------------------------------
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(14, 18, 26))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 45, 65)))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("🎯 COMPARADOR DE PINES (ECU)")
                                                        .size(11.0)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(80, 230, 150)),
                                                );
                                                ui.label(
                                                    egui::RichText::new("Guarde la firma de un pin bueno (ej. Inyector 1) y compare los demás inyectores en vivo.")
                                                        .size(9.5)
                                                        .color(egui::Color32::from_gray(160)),
                                                );
                                                ui.add_space(4.0);

                                                ui.horizontal(|ui| {
                                                    ui.label("Pin Ref:");
                                                    ui.text_edit_singleline(&mut state.vi_tracer.reference_name);
                                                });

                                                ui.add_space(4.0);

                                                ui.horizontal(|ui| {
                                                    if ui.button("📌 Fijar como Ref").on_hover_text("Guardar curva actual como Golden Sample de referencia").clicked() {
                                                        state.vi_tracer.capture_reference();
                                                    }

                                                    if state.vi_tracer.reference_points.is_some() {
                                                        if ui.button("✕ Limpiar Ref").on_hover_text("Borrar curva de referencia").clicked() {
                                                            state.vi_tracer.clear_reference();
                                                        }
                                                    }
                                                });

                                                // Barra de similitud si hay referencia activa
                                                if let Some(sim) = state.vi_tracer.similarity_percent {
                                                    ui.add_space(6.0);
                                                    let progress_color = if sim >= 90.0 {
                                                        egui::Color32::from_rgb(60, 220, 100)
                                                    } else if sim >= 75.0 {
                                                        egui::Color32::from_rgb(240, 190, 40)
                                                    } else {
                                                        egui::Color32::from_rgb(240, 60, 60)
                                                    };

                                                    ui.horizontal(|ui| {
                                                        ui.label("Coincidencia:");
                                                        ui.label(
                                                            egui::RichText::new(format!("{:.1}%", sim))
                                                                .strong()
                                                                .color(progress_color),
                                                        );
                                                    });

                                                    ui.add(
                                                        egui::ProgressBar::new(sim / 100.0)
                                                            .fill(progress_color)
                                                            .animate(false),
                                                    );

                                                    let diag_str = if sim >= 90.0 {
                                                        "✅ Pin idéntico al pin de referencia"
                                                    } else if sim >= 75.0 {
                                                        "⚠️ Desviación leve (tolerancia de carga)"
                                                    } else {
                                                        "❌ Pin defectuoso / Driver o clamp dañado"
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(diag_str)
                                                            .size(10.0)
                                                            .color(progress_color),
                                                    );
                                                }
                                            });

                                        ui.add_space(6.0);

                                        // ----------------------------------------------------
                                        // GRUPO 3: Circuito Equivalente Estimado
                                        // ----------------------------------------------------
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(14, 18, 26))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 45, 65)))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("🔍 CIRCUITO EQUIVALENTE")
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(120, 200, 255)),
                                                    );
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.button("🔍 Estimar").on_hover_text("Calcular parámetros de R, C, L o Semiconductores").clicked() {
                                                            state.vi_tracer.estimate_equivalent_circuit();
                                                        }
                                                    });
                                                });

                                                if let Some(ref eq) = state.vi_tracer.equivalent_circuit {
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        egui::RichText::new(&eq.summary)
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(255, 215, 80)),
                                                    );
                                                    for detail in &eq.details {
                                                        ui.label(
                                                            egui::RichText::new(format!("• {detail}"))
                                                                .size(9.5)
                                                                .color(egui::Color32::from_gray(180)),
                                                        );
                                                    }
                                                } else {
                                                    ui.label(
                                                        egui::RichText::new("Pulse 'Estimar' para modelar el componente (R, C, L, Diodo, Zener)")
                                                            .size(9.5)
                                                            .color(egui::Color32::from_gray(140)),
                                                    );
                                                }
                                            });

                                        ui.add_space(6.0);

                                        // ----------------------------------------------------
                                        // GRUPO 4: Base de Datos de Patrones Similares
                                        // ----------------------------------------------------
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(14, 18, 26))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 45, 65)))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("📚 PATRONES SIMILARES")
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(220, 160, 255)),
                                                    );
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.button("📚 Buscar").on_hover_text("Buscar en la base de datos de firmas automotrices").clicked() {
                                                            state.vi_tracer.search_pattern_library();
                                                        }
                                                    });
                                                });

                                                if !state.vi_tracer.pattern_matches.is_empty() {
                                                    ui.add_space(4.0);
                                                    for m in &state.vi_tracer.pattern_matches {
                                                        ui.group(|ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.label(
                                                                    egui::RichText::new(m.name)
                                                                        .size(10.0)
                                                                        .strong()
                                                                        .color(egui::Color32::WHITE),
                                                                );
                                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(format!("{:.0}%", m.confidence_percent))
                                                                            .size(10.0)
                                                                            .strong()
                                                                            .color(egui::Color32::from_rgb(100, 240, 140)),
                                                                    );
                                                                });
                                                            });
                                                            ui.label(
                                                                egui::RichText::new(&m.description)
                                                                    .size(9.0)
                                                                    .color(egui::Color32::from_gray(160)),
                                                            );
                                                            ui.label(
                                                                egui::RichText::new(format!("Uso: {}", m.typical_application))
                                                                    .size(9.0)
                                                                    .color(egui::Color32::from_rgb(120, 180, 230)),
                                                            );
                                                        });
                                                    }
                                                } else {
                                                    ui.label(
                                                        egui::RichText::new("Pulse 'Buscar' para cotejar con firmas de drivers de inyectores, diodos TVS y bobinas.")
                                                            .size(9.5)
                                                            .color(egui::Color32::from_gray(140)),
                                                    );
                                                }
                                            });

                                        ui.add_space(6.0);

                                        // ----------------------------------------------------
                                        // GRUPO 5: Configuración Rápida de Escala y Hardware
                                        // ----------------------------------------------------
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(14, 18, 26))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 45, 65)))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("⚙ ESCALAS Y HARDWARE")
                                                        .size(11.0)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(220, 220, 220)),
                                                );
                                                ui.add_space(4.0);

                                                ui.horizontal(|ui| {
                                                    ui.label("R_sense:");
                                                    let r_val = state.vi_tracer.r_sense_ohms;
                                                    let r_str = if r_val >= 1000.0 { format!("{} kΩ", r_val / 1000.0) } else { format!("{} Ω", r_val) };
                                                    if ui.button(egui::RichText::new(r_str).strong()).on_hover_text("Cambiar resistencia de sensado en serie (100Ω, 1kΩ, 10kΩ)").clicked() {
                                                        state.vi_tracer.r_sense_ohms = match r_val as u32 {
                                                            100 => 1000.0,
                                                            1000 => 10000.0,
                                                            _ => 100.0,
                                                        };
                                                        state.vi_tracer.reprocess_last();
                                                    }

                                                    ui.separator();

                                                    if ui.checkbox(&mut state.vi_tracer.invert_vertical, "Invertir Y (±I)")
                                                        .on_hover_text("Invertir polaridad vertical según conexionado de sensado de corriente")
                                                        .changed()
                                                    {
                                                        state.vi_tracer.reprocess_last();
                                                    }
                                                });

                                                 // CH1 Sensibilidad y Escala Real por Cuadrícula
                                                 ui.horizontal(|ui| {
                                                     ui.label(egui::RichText::new("CH1 X (Tensión):").size(10.5).strong().color(egui::Color32::from_rgb(255, 215, 0)));
                                                     if ui.small_button("◀").on_hover_text("Bajar escala de tensión").clicked() {
                                                         crate::view::zoom_channel(link, 0, false);
                                                        let new_v = link.config.channels[0].volts_div * link.config.channels[0].probe;
                                                        state.vi_tracer.max_voltage_scale = (new_v * (state.vi_tracer.grid_divs_h as f64 / 2.0)) as f32;
                                                        state.vi_tracer.reprocess_last();
                                                     }
                                                     let p1 = link.config.channels[0].probe;
                                                     let hw1 = link.config.channels[0].volts_div;
                                                     ui.label(format!("{:.2}V (x{:.0})", hw1, p1));
                                                     if ui.small_button("▶").on_hover_text("Subir escala de tensión").clicked() {
                                                         crate::view::zoom_channel(link, 0, true);
                                                        let new_v = link.config.channels[0].volts_div * link.config.channels[0].probe;
                                                        state.vi_tracer.max_voltage_scale = (new_v * (state.vi_tracer.grid_divs_h as f64 / 2.0)) as f32;
                                                        state.vi_tracer.reprocess_last();
                                                     }
                                                 });
                                                 ui.horizontal(|ui| {
                                                     ui.label(egui::RichText::new("  ↳ Escala Real/Div:").size(10.0).color(egui::Color32::from_gray(180)));
                                                     ui.label(
                                                         egui::RichText::new(format!("{:.2} V / div", ch1_eff_vdiv))
                                                             .strong()
                                                             .color(egui::Color32::from_rgb(255, 220, 80)),
                                                     );
                                                     let max_span_v = ch1_eff_vdiv * (state.vi_tracer.grid_divs_h as f64 / 2.0);
                                                     ui.label(egui::RichText::new(format!("(±{:.1}V)", max_span_v)).size(9.0).color(egui::Color32::from_gray(140)));
                                                 });

                                                 // CH2 Sensibilidad y Escala Real por Cuadrícula
                                                 ui.horizontal(|ui| {
                                                     ui.label(egui::RichText::new("CH2 Y (Corriente):").size(10.5).strong().color(egui::Color32::from_rgb(0, 200, 255)));
                                                     if ui.small_button("◀").on_hover_text("Bajar escala de corriente").clicked() {
                                                         crate::view::zoom_channel(link, 1, false);
                                                        let new_v = link.config.channels[1].volts_div * link.config.channels[1].probe;
                                                        let new_i_ma = (new_v / (state.vi_tracer.r_sense_ohms as f64).max(0.1)) * 1000.0;
                                                        state.vi_tracer.max_current_scale_ma = (new_i_ma * (state.vi_tracer.grid_divs_v as f64 / 2.0)) as f32;
                                                        state.vi_tracer.reprocess_last();
                                                     }
                                                     let p2 = link.config.channels[1].probe;
                                                     let hw2 = link.config.channels[1].volts_div;
                                                     ui.label(format!("{:.2}V (x{:.0})", hw2, p2));
                                                     if ui.small_button("▶").on_hover_text("Subir escala de corriente").clicked() {
                                                         crate::view::zoom_channel(link, 1, true);
                                                        let new_v = link.config.channels[1].volts_div * link.config.channels[1].probe;
                                                        let new_i_ma = (new_v / (state.vi_tracer.r_sense_ohms as f64).max(0.1)) * 1000.0;
                                                        state.vi_tracer.max_current_scale_ma = (new_i_ma * (state.vi_tracer.grid_divs_v as f64 / 2.0)) as f32;
                                                        state.vi_tracer.reprocess_last();
                                                     }
                                                 });
                                                 ui.horizontal(|ui| {
                                                     ui.label(egui::RichText::new("  ↳ Escala Real/Div:").size(10.0).color(egui::Color32::from_gray(180)));
                                                     ui.label(
                                                         egui::RichText::new(format!("{:.2} mA / div", ch2_eff_idiv_ma))
                                                             .strong()
                                                             .color(egui::Color32::from_rgb(100, 225, 255)),
                                                     );
                                                     let max_span_i = ch2_eff_idiv_ma * (state.vi_tracer.grid_divs_v as f64 / 2.0);
                                                     ui.label(egui::RichText::new(format!("(±{:.1}mA | {:.2}V/div)", max_span_i, ch2_eff_vdiv)).size(9.0).color(egui::Color32::from_gray(140)));
                                                 });

                                                 ui.horizontal(|ui| {
                                                     ui.label("Divs:");
                                                     if ui.selectable_label(state.vi_tracer.grid_divs_h == 6 && state.vi_tracer.grid_divs_v == 6, "6x6").clicked() {
                                                         state.vi_tracer.grid_divs_h = 6;
                                                         state.vi_tracer.grid_divs_v = 6;
                                                     }
                                                     if ui.selectable_label(state.vi_tracer.grid_divs_h == 8 && state.vi_tracer.grid_divs_v == 8, "8x8 (Defecto)").clicked() {
                                                         state.vi_tracer.grid_divs_h = 8;
                                                         state.vi_tracer.grid_divs_v = 8;
                                                     }
                                                     if ui.selectable_label(state.vi_tracer.grid_divs_h == 10 && state.vi_tracer.grid_divs_v == 10, "10x10").clicked() {
                                                         state.vi_tracer.grid_divs_h = 10;
                                                         state.vi_tracer.grid_divs_v = 10;
                                                     }
                                                 });

                                                 ui.horizontal(|ui| {
                                                     ui.label("Filtro:");
                                                     let sm = state.vi_tracer.smooth_samples;
                                                     if ui.selectable_label(sm <= 1, "Off").on_hover_text("Sin filtro (curva cruda)").clicked() {
                                                         state.vi_tracer.smooth_samples = 0;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(sm == 3, "Leve").on_hover_text("Suavizado de 3 muestras").clicked() {
                                                         state.vi_tracer.smooth_samples = 3;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(sm == 5, "Medio").on_hover_text("Suavizado de 5 muestras (Recomendado)").clicked() {
                                                         state.vi_tracer.smooth_samples = 5;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(sm == 9, "Fuerte").on_hover_text("Suavizado de 9 muestras").clicked() {
                                                         state.vi_tracer.smooth_samples = 9;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                 });

                                                 ui.horizontal(|ui| {
                                                     ui.label("Ráfaga:");
                                                     let bc = state.vi_tracer.burst_count;
                                                     if ui.selectable_label(bc == 1, "Off").on_hover_text("Sin promediado de ráfaga").clicked() {
                                                         state.vi_tracer.burst_count = 1;
                                                         state.vi_tracer.recent_cycles.clear();
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(bc == 4, "4").clicked() {
                                                         state.vi_tracer.burst_count = 4;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(bc == 8, "8").clicked() {
                                                         state.vi_tracer.burst_count = 8;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(bc == 16, "16 (Defecto)").on_hover_text("Ráfaga de 16 ciclos (Por defecto)").clicked() {
                                                         state.vi_tracer.burst_count = 16;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                     if ui.selectable_label(bc == 32, "32").on_hover_text("Ráfaga de 32 ciclos (Ultra alta estabilidad)").clicked() {
                                                         state.vi_tracer.burst_count = 32;
                                                         state.vi_tracer.reprocess_last();
                                                     }
                                                 });

                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new("Tolerancia:").size(10.5).strong().color(egui::Color32::from_rgb(100, 235, 140)));
                                                    ui.add(egui::Slider::new(&mut state.vi_tracer.tolerance_percent, 1.0..=25.0).step_by(0.5).suffix("%"))
                                                        .on_hover_text("Umbral porcentual de tolerancia cuantitativa para cotejo automático con el Golden Sample");
                                                });

                                                ui.add_space(4.0);
                                                if ui.add_sized([ui.available_width(), 22.0], egui::Button::new("🛠 Editar Parámetros del Tracer...")).clicked() {
                                                    let eq = state.vi_tracer.equipment_store.active_equipment().clone();
                                                    state.vi_tracer.is_creating_new = false;
                                                    state.vi_tracer.editor_equipment = Some(eq);
                                                }
                                            });

                                        ui.add_space(6.0);

                                        // GRUPO 6: SINCRONIZACIÓN Y DISPARO SÍNCRONO
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(14, 18, 26))
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 45, 65)))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("🎯 DISPARO SÍNCRONO")
                                                            .size(11.0)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(220, 220, 220)),
                                                    );
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.button(egui::RichText::new("⚡ Forzar Disparo").strong().color(egui::Color32::from_rgb(255, 215, 60)))
                                                            .on_hover_text("Fuerza la adquisición inmediata de una trama en la FPGA")
                                                            .clicked()
                                                        {
                                                            link.force_trigger();
                                                        }
                                                    });
                                                });
                                                ui.add_space(2.0);
                                                ui.label(
                                                    egui::RichText::new("• Disparo síncrono en CH1 por cruce por cero (0.0 V, flanco +)\n• Extracción de ciclos completos (0°→360°) y promedio en fase")
                                                        .size(9.0)
                                                        .color(egui::Color32::from_gray(160)),
                                                );
                                            });
                                    });
                            },
                        );
                    });
                });
        });

    // 4. Ventana modal / emergente de Setup y Editor de Equipos Trazadores VI
    show_vi_tracer_setup_dialog(ctx, link, state);
    show_tracer_equipment_editor_dialog(ctx, link, state);

    // 5. Cuadrícula flotante de señales en canales CH1/CH2 (mini-osciloscopio)
    if state.vi_tracer.mini_scope_open {
        let win_pos = egui::pos2(plot_rect.max.x - 310.0, plot_rect.min.y + 35.0);
        egui::Window::new("📺 Señales en Canales (CH1 / CH2)")
            .default_pos(win_pos)
            .default_size(egui::vec2(290.0, 160.0))
            .collapsible(true)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("CH1: V_dut").size(10.0).strong().color(egui::Color32::from_rgb(255, 215, 0)));
                    ui.separator();
                    ui.label(egui::RichText::new("CH2: I_shunt").size(10.0).strong().color(egui::Color32::from_rgb(0, 200, 255)));
                });

                let avail = ui.available_size();
                if avail.x > 60.0 && avail.y > 40.0 {
                    let (grid_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                    let p = ui.painter_at(grid_rect);
                    p.rect_filled(grid_rect, 3.0, egui::Color32::from_rgb(6, 8, 12));
                    p.rect_stroke(grid_rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 48, 65)), egui::StrokeKind::Inside);

                    // Cuadrícula 4x4
                    let step_x = grid_rect.width() / 4.0;
                    let step_y = grid_rect.height() / 4.0;
                    let g_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(18, 24, 34));
                    for i in 1..4 {
                        let x = grid_rect.min.x + i as f32 * step_x;
                        p.line_segment([egui::pos2(x, grid_rect.min.y), egui::pos2(x, grid_rect.max.y)], g_stroke);
                        let y = grid_rect.min.y + i as f32 * step_y;
                        p.line_segment([egui::pos2(grid_rect.min.x, y), egui::pos2(grid_rect.max.x, y)], g_stroke);
                    }

                    // Línea horizontal de referencia central (0V)
                    let mid_y = grid_rect.center().y;
                    p.line_segment([egui::pos2(grid_rect.min.x, mid_y), egui::pos2(grid_rect.max.x, mid_y)], egui::Stroke::new(0.8, egui::Color32::from_rgb(50, 65, 85)));

                    let max_disp_v = state.vi_tracer.max_voltage_scale.max(0.5);
                    let v_scale = (grid_rect.height() * 0.40) / max_disp_v;

                    // Trazo CH1 (Voltios en Amarillo)
                    let pts1 = &state.vi_tracer.raw_ch1;
                    if pts1.len() >= 2 {
                        let dx = grid_rect.width() / (pts1.len() - 1) as f32;
                        for i in 0..(pts1.len() - 1) {
                            let x0 = grid_rect.min.x + i as f32 * dx;
                            let y0 = (mid_y - pts1[i] * v_scale).clamp(grid_rect.min.y, grid_rect.max.y);
                            let x1 = grid_rect.min.x + (i + 1) as f32 * dx;
                            let y1 = (mid_y - pts1[i + 1] * v_scale).clamp(grid_rect.min.y, grid_rect.max.y);
                            p.line_segment([egui::pos2(x0, y0), egui::pos2(x1, y1)], egui::Stroke::new(1.3, egui::Color32::from_rgb(255, 215, 0)));
                        }
                    }

                    // Trazo CH2 (Corriente en Cian)
                    let pts2 = &state.vi_tracer.raw_ch2;
                    if pts2.len() >= 2 {
                        let dx = grid_rect.width() / (pts2.len() - 1) as f32;
                        for i in 0..(pts2.len() - 1) {
                            let x0 = grid_rect.min.x + i as f32 * dx;
                            let y0 = (mid_y - pts2[i] * v_scale).clamp(grid_rect.min.y, grid_rect.max.y);
                            let x1 = grid_rect.min.x + (i + 1) as f32 * dx;
                            let y1 = (mid_y - pts2[i + 1] * v_scale).clamp(grid_rect.min.y, grid_rect.max.y);
                            p.line_segment([egui::pos2(x0, y0), egui::pos2(x1, y1)], egui::Stroke::new(1.3, egui::Color32::from_rgb(0, 200, 255)));
                        }
                    }
                }
            });
    }
}

/// Diálogo modal de configuración completa de Equipos Trazadores y Hardware del Trazador VI
fn show_vi_tracer_setup_dialog(
    ctx: &egui::Context,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
) {
    if !state.vi_tracer.setup_dialog_open {
        return;
    }

    let mut dialog_open = state.vi_tracer.setup_dialog_open;
    let mut should_close = false;

    egui::Window::new("⚙ Equipos Trazadores VI & Ajustes de Hardware")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(false)
        .default_size(egui::vec2(480.0, 520.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Equipos Trazadores Registrados")
                        .strong()
                        .color(egui::Color32::from_rgb(255, 215, 50)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Crear Nuevo Equipo").clicked() {
                        state.vi_tracer.is_creating_new = true;
                        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
                        let mut new_eq = TracerEquipment::new_custom("Nuevo Tracer", 0.5, 10.0, 0.5, 10.0, 10.0e-3, true, 1000.0);
                        new_eq.id = format!("eq_{ts}");
                        state.vi_tracer.editor_equipment = Some(new_eq);
                    }
                });
            });
            ui.add_space(4.0);

            let active_id = state.vi_tracer.equipment_store.active_id.clone();
            let mut eq_to_activate = None;
            let mut eq_to_edit = None;
            let mut eq_to_delete = None;

            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                for eq in &state.vi_tracer.equipment_store.equipments {
                    let is_active = eq.id == active_id;
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&eq.name)
                                    .strong()
                                    .color(if is_active { egui::Color32::from_rgb(255, 215, 50) } else { egui::Color32::WHITE }),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !eq.is_factory && state.vi_tracer.equipment_store.equipments.len() > 1 {
                                    if ui.small_button("🗑").on_hover_text("Borrar este equipo").clicked() {
                                        eq_to_delete = Some(eq.id.clone());
                                    }
                                }
                                if ui.small_button("✏").on_hover_text("Editar parámetros").clicked() {
                                    eq_to_edit = Some(eq.clone());
                                }
                                if ui.button(if is_active { "✓ Activo" } else { "⚡ Cargar" }).clicked() {
                                    eq_to_activate = Some(eq.clone());
                                }
                            });
                        });
                        ui.label(
                            egui::RichText::new(&eq.description)
                                .size(10.0)
                                .color(egui::Color32::from_gray(160)),
                        );
                        let v_eff = eq.eff_ch1_vdiv();
                        let i_eff = eq.eff_ch2_idiv_ma();
                        let max_v = v_eff * (eq.grid_divs_h.max(2) as f64 / 2.0);
                        let max_i = i_eff * (eq.grid_divs_v.max(2) as f64 / 2.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Escalas: {:.1} V/div (±{:.0}V) | {:.1} mA/div (±{:.0}mA) [{}x{} div] | R: {:.0}Ω | Base: {:.0}ms",
                                v_eff, max_v, i_eff, max_i, eq.grid_divs_h.max(2), eq.grid_divs_v.max(2), eq.r_sense_ohms, eq.timebase * 1000.0
                            ))
                            .size(9.5)
                            .color(egui::Color32::from_rgb(120, 200, 255)),
                        );
                    });
                    ui.add_space(2.0);
                }
            });

            if let Some(eq) = eq_to_activate {
                state.vi_tracer.equipment_store.set_active(&eq.id);
                apply_tracer_equipment_to_link(link, state, &eq);
            }
            if let Some(eq) = eq_to_edit {
                state.vi_tracer.is_creating_new = false;
                state.vi_tracer.editor_equipment = Some(eq);
            }
            if let Some(id) = eq_to_delete {
                state.vi_tracer.equipment_store.delete_equipment(&id);
                let eq = state.vi_tracer.equipment_store.active_equipment().clone();
                apply_tracer_equipment_to_link(link, state, &eq);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(
                egui::RichText::new("Ajustes Rápidos de Canales y Sondas")
                    .strong()
                    .color(egui::Color32::from_rgb(120, 200, 255)),
            );
            ui.add_space(4.0);

            egui::Grid::new("vi_tracer_setup_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Resistencia Sensado R_sense:");
                    let mut r_idx = match state.vi_tracer.r_sense_ohms as u32 {
                        100 => 0,
                        10000 => 2,
                        _ => 1,
                    };
                    egui::ComboBox::from_id_salt("r_sense_combo")
                        .selected_text(match r_idx {
                            0 => "100 Ω",
                            2 => "10 kΩ",
                            _ => "1 kΩ (Típico)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut r_idx, 0, "100 Ω");
                            ui.selectable_value(&mut r_idx, 1, "1 kΩ (Típico Trafo)");
                            ui.selectable_value(&mut r_idx, 2, "10 kΩ");
                        });
                    state.vi_tracer.r_sense_ohms = match r_idx {
                        0 => 100.0,
                        2 => 10000.0,
                        _ => 1000.0,
                    };
                    ui.end_row();

                    ui.label("Polaridad CH2 (Corriente):");
                    ui.checkbox(&mut state.vi_tracer.invert_vertical, "Invertir Canal 2");
                    ui.end_row();

                    ui.label("Canal 1 (Tensión DUT):");
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sonda:");
                            if ui.selectable_label(link.config.channels[0].probe == 1.0, "x1").clicked() {
                                link.config.channels[0].probe = 1.0;
                                link.dirty = true;
                            }
                            if ui.selectable_label(link.config.channels[0].probe == 10.0, "x10").clicked() {
                                link.config.channels[0].probe = 10.0;
                                link.dirty = true;
                            }
                            if ui.selectable_label(link.config.channels[0].probe == 100.0, "x100").clicked() {
                                link.config.channels[0].probe = 100.0;
                                link.dirty = true;
                            }
                        });
                        let ch1_real_vdiv = link.config.channels[0].volts_div * link.config.channels[0].probe;
                        let max_v = ch1_real_vdiv * (state.vi_tracer.grid_divs_h.max(2) as f64 / 2.0);
                        ui.label(
                            egui::RichText::new(format!("↳ Escala Real: {:.2} V / div (±{:.1} V en {} div)", ch1_real_vdiv, max_v, state.vi_tracer.grid_divs_h))
                                .strong()
                                .color(egui::Color32::from_rgb(255, 215, 80)),
                        );
                    });
                    ui.end_row();

                    ui.label("Canal 2 (Corriente DUT):");
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sonda:");
                            if ui.selectable_label(link.config.channels[1].probe == 1.0, "x1").clicked() {
                                link.config.channels[1].probe = 1.0;
                                link.dirty = true;
                            }
                            if ui.selectable_label(link.config.channels[1].probe == 10.0, "x10").clicked() {
                                link.config.channels[1].probe = 10.0;
                                link.dirty = true;
                            }
                            if ui.selectable_label(link.config.channels[1].probe == 100.0, "x100").clicked() {
                                link.config.channels[1].probe = 100.0;
                                link.dirty = true;
                            }
                        });
                        let ch2_real_vdiv = link.config.channels[1].volts_div * link.config.channels[1].probe;
                        let r_s = (state.vi_tracer.r_sense_ohms as f64).max(0.1);
                        let ch2_real_idiv_ma = (ch2_real_vdiv / r_s) * 1000.0;
                        let max_i = ch2_real_idiv_ma * (state.vi_tracer.grid_divs_v.max(2) as f64 / 2.0);
                        ui.label(
                            egui::RichText::new(format!("↳ Escala Real: {:.2} mA / div (±{:.1} mA en {} div)", ch2_real_idiv_ma, max_i, state.vi_tracer.grid_divs_v))
                                .strong()
                                .color(egui::Color32::from_rgb(100, 220, 255)),
                        );
                    });
                    ui.end_row();

                    ui.label("Base de tiempo:");
                    let mut tb_ms = crate::view::timebase(link) * 1000.0;
                    if ui.add(egui::Slider::new(&mut tb_ms, 1.0..=50.0).suffix(" ms/div")).changed() {
                        crate::view::set_timebase(link, tb_ms / 1000.0);
                        link.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Divisiones Cuadrícula (H x V):");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(state.vi_tracer.grid_divs_h == 6 && state.vi_tracer.grid_divs_v == 6, "6 x 6").clicked() {
                            state.vi_tracer.grid_divs_h = 6;
                            state.vi_tracer.grid_divs_v = 6;
                        }
                        if ui.selectable_label(state.vi_tracer.grid_divs_h == 8 && state.vi_tracer.grid_divs_v == 8, "8 x 8 (Defecto)").clicked() {
                            state.vi_tracer.grid_divs_h = 8;
                            state.vi_tracer.grid_divs_v = 8;
                        }
                        if ui.selectable_label(state.vi_tracer.grid_divs_h == 10 && state.vi_tracer.grid_divs_v == 10, "10 x 10").clicked() {
                            state.vi_tracer.grid_divs_h = 10;
                            state.vi_tracer.grid_divs_v = 10;
                        }
                    });
                    ui.end_row();

                    ui.label("Ajuste Manual Divs (H / V):");
                    ui.horizontal(|ui| {
                        ui.label("H:");
                        let mut dh = state.vi_tracer.grid_divs_h;
                        if ui.add(egui::DragValue::new(&mut dh).range(2..=20).speed(0.1)).changed() {
                            state.vi_tracer.grid_divs_h = dh.max(2);
                        }
                        ui.label("V:");
                        let mut dv = state.vi_tracer.grid_divs_v;
                        if ui.add(egui::DragValue::new(&mut dv).range(2..=20).speed(0.1)).changed() {
                            state.vi_tracer.grid_divs_v = dv.max(2);
                        }
                    });
                    ui.end_row();

                    ui.separator();
                    ui.separator();
                    ui.end_row();

                    ui.label("Modo Trazado de Curva:");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(!state.vi_tracer.draw_dots, "📈 Vectores Continuos").clicked() {
                            state.vi_tracer.draw_dots = false;
                        }
                        if ui.selectable_label(state.vi_tracer.draw_dots, "⠂ Puntos Discretos").clicked() {
                            state.vi_tracer.draw_dots = true;
                        }
                    });
                    ui.end_row();

                    ui.label("Suavizado y Reducción Ruido:");
                    ui.horizontal(|ui| {
                        let sm = state.vi_tracer.smooth_samples;
                        if ui.selectable_label(sm <= 1, "Off (Crudo)").clicked() {
                            state.vi_tracer.smooth_samples = 0;
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_label(sm == 3, "Leve (3)").clicked() {
                            state.vi_tracer.smooth_samples = 3;
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_label(sm == 5, "Medio (5)").clicked() {
                            state.vi_tracer.smooth_samples = 5;
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_label(sm == 9, "Fuerte (9)").clicked() {
                            state.vi_tracer.smooth_samples = 9;
                            state.vi_tracer.reprocess_last();
                        }
                    });
                    ui.end_row();

                    ui.label("Ráfaga de Promediado:");
                    ui.horizontal(|ui| {
                        let mut bc = state.vi_tracer.burst_count;
                        if ui.selectable_value(&mut bc, 1, "Off (1)").clicked() {
                            state.vi_tracer.burst_count = 1;
                            state.vi_tracer.recent_cycles.clear();
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_value(&mut bc, 4, "4").clicked() {
                            state.vi_tracer.burst_count = 4;
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_value(&mut bc, 8, "8").clicked() {
                            state.vi_tracer.burst_count = 8;
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_value(&mut bc, 16, "16 (Defecto)").clicked() {
                            state.vi_tracer.burst_count = 16;
                            state.vi_tracer.reprocess_last();
                        }
                        if ui.selectable_value(&mut bc, 32, "32").clicked() {
                            state.vi_tracer.burst_count = 32;
                            state.vi_tracer.reprocess_last();
                        }
                    });
                    ui.end_row();

                    ui.label("Tolerancia Cuantitativa:");
                    ui.horizontal(|ui| {
                        if ui.add(egui::Slider::new(&mut state.vi_tracer.tolerance_percent, 1.0..=25.0).step_by(0.5).suffix("%")).changed() {
                            state.vi_tracer.reprocess_last();
                        }
                        ui.label(egui::RichText::new("Cotejo automático de similitud con Golden Sample").small().color(egui::Color32::from_gray(160)));
                    });
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Cerrar").clicked() {
                    should_close = true;
                }
            });
        });

    if should_close {
        dialog_open = false;
    }
    state.vi_tracer.setup_dialog_open = dialog_open;
}

/// Diálogo modal para Crear o Editar un Equipo Trazador VI
fn show_tracer_equipment_editor_dialog(
    ctx: &egui::Context,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
) {
    if state.vi_tracer.editor_equipment.is_none() {
        return;
    }

    let mut close = false;
    let mut save_and_apply = false;
    let mut eq = state.vi_tracer.editor_equipment.clone().unwrap();
    let is_new = state.vi_tracer.is_creating_new;

    let title = if is_new {
        "➕ Crear Nuevo Equipo Trazador VI"
    } else {
        "✏ Editar Equipo Trazador VI"
    };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .default_size(egui::vec2(440.0, 480.0))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Ajustes de Hardware y Escalas del Equipo")
                    .strong()
                    .color(egui::Color32::from_rgb(255, 215, 50)),
            );
            ui.add_space(4.0);

            egui::Grid::new("eq_editor_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Nombre del Equipo:");
                    ui.text_edit_singleline(&mut eq.name);
                    ui.end_row();

                    ui.label("Descripción:");
                    ui.text_edit_singleline(&mut eq.description);
                    ui.end_row();

                    ui.separator();
                    ui.separator();
                    ui.end_row();

                    // CH1 - Tensión DUT
                    ui.label("CH1 Sensibilidad (BNC):");
                    egui::ComboBox::from_id_salt("eq_ch1_vdiv")
                        .selected_text(format!("{:.2} V/div (Real: {:.1} V/div)", eq.ch1_vdiv, eq.eff_ch1_vdiv()))
                        .show_ui(ui, |ui| {
                            for &v in &[0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0] {
                                let label = format!("{:.2} V/div  ↳ Real: {:.1} V/div (con x{:.0})", v, v * eq.ch1_probe, eq.ch1_probe);
                                if ui.selectable_label((eq.ch1_vdiv - v).abs() < 0.001, label).clicked() {
                                    eq.ch1_vdiv = v;
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("CH1 Sonda Multiplicadora:");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(eq.ch1_probe == 1.0, "x1").clicked() { eq.ch1_probe = 1.0; }
                        if ui.selectable_label(eq.ch1_probe == 10.0, "x10").clicked() { eq.ch1_probe = 10.0; }
                        if ui.selectable_label(eq.ch1_probe == 100.0, "x100").clicked() { eq.ch1_probe = 100.0; }
                    });
                    ui.end_row();

                    let eff_vdiv = eq.eff_ch1_vdiv();
                    let max_v = eff_vdiv * (eq.grid_divs_h.max(2) as f64 / 2.0);
                    ui.label("CH1 Escala Real Cuadrícula:");
                    ui.label(egui::RichText::new(format!("{:.2} V / div  (±{:.1} V en {} div)", eff_vdiv, max_v, eq.grid_divs_h)).strong().color(egui::Color32::from_rgb(255, 220, 80)));
                    ui.end_row();

                    ui.separator();
                    ui.separator();
                    ui.end_row();

                    // CH2 - Corriente DUT vía R_sense
                    ui.label("CH2 Sensibilidad (BNC):");
                    egui::ComboBox::from_id_salt("eq_ch2_vdiv")
                        .selected_text(format!("{:.2} V/div (Real: {:.1} mA/div)", eq.ch2_vdiv, eq.eff_ch2_idiv_ma()))
                        .show_ui(ui, |ui| {
                            for &v in &[0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0] {
                                let eff_idiv = (v * eq.ch2_probe / eq.r_sense_ohms.max(1.0) as f64) * 1000.0;
                                let label = format!("{:.2} V/div  ↳ Real: {:.1} mA/div (con x{:.0})", v, eff_idiv, eq.ch2_probe);
                                if ui.selectable_label((eq.ch2_vdiv - v).abs() < 0.001, label).clicked() {
                                    eq.ch2_vdiv = v;
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("CH2 Sonda Multiplicadora:");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(eq.ch2_probe == 1.0, "x1").clicked() { eq.ch2_probe = 1.0; }
                        if ui.selectable_label(eq.ch2_probe == 10.0, "x10").clicked() { eq.ch2_probe = 10.0; }
                        if ui.selectable_label(eq.ch2_probe == 100.0, "x100").clicked() { eq.ch2_probe = 100.0; }
                    });
                    ui.end_row();

                    ui.label("Resistencia R_sense:");
                    egui::ComboBox::from_id_salt("eq_rsense")
                        .selected_text(if eq.r_sense_ohms >= 1000.0 { format!("{:.0} kΩ", eq.r_sense_ohms / 1000.0) } else { format!("{:.0} Ω", eq.r_sense_ohms) })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label((eq.r_sense_ohms - 100.0).abs() < 1.0, "100 Ω").clicked() { eq.r_sense_ohms = 100.0; }
                            if ui.selectable_label((eq.r_sense_ohms - 1000.0).abs() < 1.0, "1 kΩ (Típico)").clicked() { eq.r_sense_ohms = 1000.0; }
                            if ui.selectable_label((eq.r_sense_ohms - 10000.0).abs() < 1.0, "10 kΩ").clicked() { eq.r_sense_ohms = 10000.0; }
                        });
                    ui.end_row();

                    let eff_idiv = eq.eff_ch2_idiv_ma();
                    let max_i = eff_idiv * (eq.grid_divs_v.max(2) as f64 / 2.0);
                    ui.label("CH2 Escala Real Cuadrícula:");
                    ui.label(egui::RichText::new(format!("{:.2} mA / div  (±{:.1} mA en {} div)", eff_idiv, max_i, eq.grid_divs_v)).strong().color(egui::Color32::from_rgb(100, 220, 255)));
                    ui.end_row();

                    ui.label("Polaridad Vertical CH2:");
                    ui.checkbox(&mut eq.invert_vertical, "Invertir Canal 2");
                    ui.end_row();

                    ui.separator();
                    ui.separator();
                    ui.end_row();

                    // Base de tiempo
                    ui.label("Base de Tiempo:");
                    let mut tb_ms = eq.timebase * 1000.0;
                    if ui.add(egui::Slider::new(&mut tb_ms, 1.0..=50.0).suffix(" ms/div")).changed() {
                        eq.timebase = tb_ms / 1000.0;
                    }
                    ui.end_row();

                    ui.label("Divisiones Cuadrícula (H x V):");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(eq.grid_divs_h == 6 && eq.grid_divs_v == 6, "6 x 6").clicked() {
                            eq.grid_divs_h = 6;
                            eq.grid_divs_v = 6;
                        }
                        if ui.selectable_label(eq.grid_divs_h == 8 && eq.grid_divs_v == 8, "8 x 8 (Defecto)").clicked() {
                            eq.grid_divs_h = 8;
                            eq.grid_divs_v = 8;
                        }
                        if ui.selectable_label(eq.grid_divs_h == 10 && eq.grid_divs_v == 10, "10 x 10").clicked() {
                            eq.grid_divs_h = 10;
                            eq.grid_divs_v = 10;
                        }
                    });
                    ui.end_row();

                    ui.label("Ajuste Manual Divs (H / V):");
                    ui.horizontal(|ui| {
                        ui.label("H:");
                        let mut dh = eq.grid_divs_h;
                        if ui.add(egui::DragValue::new(&mut dh).range(2..=20).speed(0.1)).changed() {
                            eq.grid_divs_h = dh.max(2);
                        }
                        ui.label("V:");
                        let mut dv = eq.grid_divs_v;
                        if ui.add(egui::DragValue::new(&mut dv).range(2..=20).speed(0.1)).changed() {
                            eq.grid_divs_v = dv.max(2);
                        }
                    });
                    ui.end_row();

                    ui.separator();
                    ui.separator();
                    ui.end_row();

                    ui.label("Modo Trazado de Curva:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut eq.draw_dots, false, "📈 Vectores Continuos");
                        ui.selectable_value(&mut eq.draw_dots, true, "⠂ Puntos Discretos");
                    });
                    ui.end_row();

                    ui.label("Suavizado y Reducción Ruido:");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(eq.smooth_samples <= 1, "Off").clicked() {
                            eq.smooth_samples = 0;
                        }
                        if ui.selectable_label(eq.smooth_samples == 3, "Leve (3)").clicked() {
                            eq.smooth_samples = 3;
                        }
                        if ui.selectable_label(eq.smooth_samples == 5, "Medio (5)").clicked() {
                            eq.smooth_samples = 5;
                        }
                        if ui.selectable_label(eq.smooth_samples == 9, "Fuerte (9)").clicked() {
                            eq.smooth_samples = 9;
                        }
                    });
                    ui.end_row();

                    ui.label("Ráfaga de Promediado:");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(eq.burst_count == 1, "Off (1)").clicked() { eq.burst_count = 1; }
                        if ui.selectable_label(eq.burst_count == 4, "4").clicked() { eq.burst_count = 4; }
                        if ui.selectable_label(eq.burst_count == 8, "8").clicked() { eq.burst_count = 8; }
                        if ui.selectable_label(eq.burst_count == 16, "16 (Defecto)").clicked() { eq.burst_count = 16; }
                        if ui.selectable_label(eq.burst_count == 32, "32").clicked() { eq.burst_count = 32; }
                    });
                    ui.end_row();

                    ui.label("Tolerancia Cuantitativa:");
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut eq.tolerance_percent, 1.0..=25.0).step_by(0.5).suffix("%"));
                        ui.label(egui::RichText::new("Cotejo con Golden Sample").small().color(egui::Color32::from_gray(160)));
                    });
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("💾 Guardar y Aplicar")
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(30, 95, 55));

                if ui.add(save_btn).clicked() {
                    save_and_apply = true;
                }

                if ui.button("Cancelar").clicked() {
                    close = true;
                }
            });
        });

    if save_and_apply {
        if is_new {
            state.vi_tracer.equipment_store.add_equipment(eq.clone());
        } else {
            state.vi_tracer.equipment_store.update_equipment(eq.clone());
        }
        state.vi_tracer.equipment_store.set_active(&eq.id);
        apply_tracer_equipment_to_link(link, state, &eq);
        state.vi_tracer.editor_equipment = None;
    } else if close {
        state.vi_tracer.editor_equipment = None;
    } else {
        state.vi_tracer.editor_equipment = Some(eq);
    }
}

/// Ventana flotante de configuración de Favoritos (2 columnas del riel izquierdo)
pub fn show_favorite_setup_window(ctx: &egui::Context, state: &mut AutomotiveUiState) {
    if !state.favorite_setup_open {
        return;
    }
    egui::Window::new("⚙ Configuración de Accesos Rápidos Favoritos")
        .open(&mut state.favorite_setup_open)
        .collapsible(false)
        .resizable(false)
        .default_size(egui::vec2(320.0, 360.0))
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Personaliza los 8 botones de la botonera izquierda:").strong());
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            for slot_idx in 0..8 {
                ui.horizontal(|ui| {
                    ui.label(format!("Botón #{}:", slot_idx + 1));
                    let current = state.favorite_slots[slot_idx];
                    egui::ComboBox::from_id_salt(format!("fav_slot_{}", slot_idx))
                        .selected_text(current.label())
                        .show_ui(ui, |ui| {
                            for f in FavoriteTool::ALL {
                                if ui.selectable_label(current == f, f.label()).clicked() {
                                    state.favorite_slots[slot_idx] = f;
                                }
                            }
                        });
                });
                ui.add_space(2.0);
            }

            ui.add_space(8.0);
            if ui.button("Restablecer Valores Predeterminados").clicked() {
                state.favorite_slots = [
                    FavoriteTool::Automotive,
                    FavoriteTool::Measures,
                    FavoriteTool::Cursors,
                    FavoriteTool::SerialDecode,
                    FavoriteTool::Datalogger,
                    FavoriteTool::ActuatorTest,
                    FavoriteTool::ViTracer,
                    FavoriteTool::Math,
                ];
            }
        });
}

/// Ventana flotante de procesamiento y acondicionamiento DSP por canal
pub fn show_dsp_floating_window(
    ctx: &egui::Context,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
) {
    let Some(ch) = state.dsp_floating_ch else {
        return;
    };

    if ch >= 2 {
        state.dsp_floating_ch = None;
        return;
    }

    let mut open = true;
    let ch_color = crate::ui::widgets::channel_color(ch);

    egui::Window::new(format!("🎛 Procesamiento DSP - Canal {}", ch + 1))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(360.0, 520.0))
        .default_pos(egui::pos2(260.0, 90.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;

            // Encabezado con estado del canal
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("CANAL {}", ch + 1))
                        .strong()
                        .size(13.0)
                        .color(ch_color),
                );
                let v_div = link.config.channels[ch].volts_div * link.config.channels[ch].probe;
                let probe = link.config.channels[ch].probe;
                let coup_str = match link.config.channels[ch].coupling {
                    neowon_core::Coupling::Dc => "DC",
                    neowon_core::Coupling::Ac => "AC",
                    neowon_core::Coupling::Gnd => "GND",
                };
                ui.label(
                    egui::RichText::new(format!("({}/div · x{:.0} · {})", crate::derived::fmt_si(v_div, "V"), probe, coup_str))
                        .weak()
                        .size(11.0),
                );
            });

            ui.separator();

            let dsp = &mut state.dsp[ch];

            // 1. FILTRADO PASA-BAJOS DIGITAL
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut dsp.lpf_enabled,
                            egui::RichText::new("FILTRO PASA-BAJOS DIGITAL (LPF)").strong().color(egui::Color32::from_rgb(100, 220, 255)),
                        );
                        if dsp.lpf_enabled {
                            ui.colored_label(egui::Color32::from_rgb(80, 255, 140), "● ACTIVO");
                        }
                    });

                    if dsp.lpf_enabled {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Algoritmo de Filtrado (Selección sin distorsión):").size(10.5).strong());

                        // Selector de algoritmo
                        for algo in [
                            picowon_auto::LpfAlgorithm::Gaussian,
                            picowon_auto::LpfAlgorithm::SavitzkyGolay,
                            picowon_auto::LpfAlgorithm::EdgePreserving,
                            picowon_auto::LpfAlgorithm::Butterworth,
                        ] {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(dsp.filter_algorithm == algo, algo.name()).clicked() {
                                    dsp.filter_algorithm = algo;
                                }
                            });
                        }

                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(dsp.filter_algorithm.description())
                                .weak()
                                .size(9.5)
                                .color(egui::Color32::from_rgb(170, 185, 210)),
                        );

                        // Sensibilidad para filtro bilateral
                        if dsp.filter_algorithm == picowon_auto::LpfAlgorithm::EdgePreserving {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Umbral de Borde:").size(10.0));
                                let mut pct = (dsp.edge_threshold_frac * 100.0).clamp(1.0, 25.0);
                                if ui.add(egui::Slider::new(&mut pct, 1.0..=20.0).suffix("% escala")).changed() {
                                    dsp.edge_threshold_frac = pct / 100.0;
                                }
                            });
                        }

                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Frecuencia de Corte (fc):").size(10.5).strong());

                        // Presets de corte rápido
                        ui.horizontal_wrapped(|ui| {
                            for &(freq, label) in &picowon_auto::AUTOMOTIVE_LPF_PRESETS {
                                let is_active = (dsp.lpf_cutoff_hz - freq).abs() < 1.0;
                                let short_label = if freq >= 1000.0 {
                                    format!("{:.0} kHz", freq / 1000.0)
                                } else {
                                    format!("{:.0} Hz", freq)
                                };
                                if ui.selectable_label(is_active, short_label).on_hover_text(label).clicked() {
                                    dsp.lpf_cutoff_hz = freq;
                                }
                            }
                        });

                        // Deslizador continuo
                        ui.horizontal(|ui| {
                            ui.label("Ajuste fino:");
                            let mut fc = dsp.lpf_cutoff_hz;
                            if ui.add(egui::Slider::new(&mut fc, 100.0..=1_000_000.0).logarithmic(true).custom_formatter(|n, _| {
                                crate::derived::fmt_si(n, "Hz")
                            })).changed() {
                                dsp.lpf_cutoff_hz = fc;
                            }
                        });
                    }
                });

            // 2. MEJORA DE RESOLUCIÓN (ENOB BOOST)
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("MEJORA DE RESOLUCIÓN (ENOB)").strong().color(egui::Color32::from_rgb(100, 220, 255)));
                        if dsp.res_enhance_bits > 0.0 {
                            let total_bits = 8.0 + dsp.res_enhance_bits;
                            ui.colored_label(egui::Color32::from_rgb(100, 200, 255), format!("{total_bits:.1} bits"));
                        } else {
                            ui.label(egui::RichText::new("(8 bits nativo)").weak().size(10.0));
                        }
                    });
                    ui.label(
                        egui::RichText::new("Promedia ruido de cuantización para revelar micro-ondulaciones sin alterar amplitud.")
                            .weak()
                            .size(9.5),
                    );

                    ui.horizontal_wrapped(|ui| {
                        for &(bits, label) in &picowon_auto::RES_ENHANCE_PRESETS {
                            let is_sel = (dsp.res_enhance_bits - bits).abs() < 0.1;
                            let short_text = if bits == 0.0 { "Off" } else { label.split(' ').next().unwrap_or(label) };
                            if ui.selectable_label(is_sel, short_text).on_hover_text(label).clicked() {
                                dsp.res_enhance_bits = bits;
                            }
                        }
                    });
                });

            // 3. INVERSIÓN Y TARA / AUTO-ZERO
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut dsp.invert, egui::RichText::new("Invertir Polaridad (Invert)").strong());
                    });

                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        if ui.button("🎯 Calibrar Cero / Tara").on_hover_text("Resta la desviación DC residual (esencial para pinzas amperimétricas y sensores)").clicked() {
                            if let Some(frame) = &link.latest {
                                if let Some(cap) = frame.channels.iter().find(|c| c.ch == ch) {
                                    if !cap.data.is_empty() {
                                        let sum: f32 = cap.data.iter().sum();
                                        let avg_counts = sum / cap.data.len() as f32;
                                        dsp.tare_offset_v = cap.cal.volts_i(avg_counts);
                                    }
                                }
                            }
                        }

                        if dsp.tare_offset_v.abs() > 1e-5 {
                            let t_str = crate::derived::fmt_si(dsp.tare_offset_v, "V");
                            ui.colored_label(egui::Color32::from_rgb(220, 180, 100), format!("Tara: {t_str}"));
                            if ui.small_button("✕").on_hover_text("Restablecer tara a 0 V").clicked() {
                                dsp.tare_offset_v = 0.0;
                            }
                        }
                    });
                });

            // Acciones inferiores
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Restablecer DSP (Bypass)").on_hover_text("Desactiva todos los filtros y acondicionamientos para este canal").clicked() {
                    *dsp = picowon_auto::ChannelDspConfig::default();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cerrar").clicked() {
                        state.dsp_floating_ch = None;
                    }
                });
            });
        });

    if !open {
        state.dsp_floating_ch = None;
    }
}

/// HUD y Panel de Diagnóstico Físico CAN Bus (ISO 11898-2)
pub fn show_can_diff_hud(
    ctx: &egui::Context,
    layout: &Layout,
    link: &mut crate::Link,
    math: &mut crate::derived::MathState,
    state: &mut AutomotiveUiState,
) {
    if state.app_mode != AppMode::CanLinWaveform {
        return;
    }

    // Ingestar tramas de CH1 y CH2 para análisis de capa física
    for f in &link.arrived {
        let ch1 = f.channels.iter().find(|c| c.ch == 0).map(|c| &c.data[..]).unwrap_or(&[]);
        let ch2 = f.channels.iter().find(|c| c.ch == 1).map(|c| &c.data[..]).unwrap_or(&[]);
        if !ch1.is_empty() && !ch2.is_empty() {
            let v_div1 = (link.config.channels[0].volts_div * link.config.channels[0].probe * 5.0) as f32;
            let v_div2 = (link.config.channels[1].volts_div * link.config.channels[1].probe * 5.0) as f32;
            let v_h: Vec<f32> = ch1.iter().map(|&b| (b as f32 - 128.0) / 128.0 * v_div1).collect();
            let v_l: Vec<f32> = ch2.iter().map(|&b| (b as f32 - 128.0) / 128.0 * v_div2).collect();
            state.can_diag = picowon_auto::analyze_can_physical_layer(&v_h, &v_l);
        }
    }

    let plot_rect = layout.points(layout.plot);
    if plot_rect.width() < 100.0 {
        return;
    }

    let hud_pos = egui::pos2(plot_rect.min.x + 12.0, plot_rect.min.y + 10.0);
    egui::Window::new("🚗 Diagnóstico Capa Física CAN Bus (ISO 11898)")
        .default_pos(hud_pos)
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let math_active = math.enabled && math.op == neowon_dsp::MathOp::Sub;
                let btn = egui::Button::new(
                    egui::RichText::new(if math_active { "✔ CAN Diferencial (A - B) ACTIVO" } else { "⚡ Activar CAN Diferencial (A - B)" })
                        .strong()
                        .color(if math_active { egui::Color32::BLACK } else { egui::Color32::WHITE }),
                )
                .fill(if math_active { egui::Color32::from_rgb(80, 220, 140) } else { egui::Color32::from_rgb(220, 140, 20) });

                if ui.add(btn).on_hover_text("Configura 1-Click: CH1 (CAN_H), CH2 (CAN_L), 20 µs/div y Canal Matemático M = A - B").clicked() {
                    link.config.channels[0].enabled = true;
                    link.config.channels[0].coupling = neowon_core::Coupling::Dc;
                    link.config.channels[0].volts_div = 1.0;
                    link.config.channels[0].offset = 0.0;

                    link.config.channels[1].enabled = true;
                    link.config.channels[1].coupling = neowon_core::Coupling::Dc;
                    link.config.channels[1].volts_div = 1.0;
                    link.config.channels[1].offset = 0.0;

                    crate::view::set_timebase(link, 20e-6);

                    math.enabled = true;
                    math.op = neowon_dsp::MathOp::Sub;
                    math.rescale = true;
                    link.dirty = true;
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Estado de Salud ISO 11898
            let diag = state.can_diag;
            let (badge_color, text_color) = match diag.health {
                picowon_auto::CanBusHealth::Healthy => (egui::Color32::from_rgb(25, 100, 50), egui::Color32::from_rgb(120, 255, 160)),
                picowon_auto::CanBusHealth::NoTraffic | picowon_auto::CanBusHealth::InsufficientData => (egui::Color32::from_rgb(45, 55, 75), egui::Color32::from_rgb(180, 195, 215)),
                _ => (egui::Color32::from_rgb(130, 40, 20), egui::Color32::from_rgb(255, 190, 180)),
            };

            let mut f = egui::Frame::new();
            f.fill = badge_color;
            f.inner_margin = egui::Margin::symmetric(8, 4);
            f.show(ui, |ui| {
                ui.label(egui::RichText::new(diag.health.badge_text()).strong().color(text_color));
            });

            ui.add_space(4.0);

            // Tabla comparativa de niveles de tensión
            egui::Grid::new("can_diff_metrics_grid")
                .striped(true)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Línea / Trazo").strong().small());
                    ui.label(egui::RichText::new("Recesivo").strong().small());
                    ui.label(egui::RichText::new("Dominante").strong().small());
                    ui.label(egui::RichText::new("Medido Ahora").strong().small());
                    ui.end_row();

                    ui.label(egui::RichText::new("CH1: CAN_H").color(egui::Color32::from_rgb(255, 220, 60)));
                    ui.label("~2.50 V");
                    ui.label("~3.50 V");
                    ui.label(format!("{:.2} V / {:.2} V", diag.v_h_rec, diag.v_h_dom));
                    ui.end_row();

                    ui.label(egui::RichText::new("CH2: CAN_L").color(egui::Color32::from_rgb(80, 180, 255)));
                    ui.label("~2.50 V");
                    ui.label("~1.50 V");
                    ui.label(format!("{:.2} V / {:.2} V", diag.v_l_rec, diag.v_l_dom));
                    ui.end_row();

                    ui.label(egui::RichText::new("MATH: V_diff (A - B)").color(egui::Color32::from_rgb(255, 120, 220)).strong());
                    ui.label("~0.00 V");
                    ui.label("~2.00 V");
                    ui.label(egui::RichText::new(format!("{:.2} V (ΔV: {:.2} V)", diag.v_diff_dom, diag.delta_v)).strong().color(
                        if diag.health == picowon_auto::CanBusHealth::Healthy { egui::Color32::from_rgb(100, 255, 150) } else { egui::Color32::YELLOW }
                    ));
                    ui.end_row();

                    ui.label("Modo Común (V_cm)");
                    ui.label("~2.50 V");
                    ui.label("~2.50 V");
                    ui.label(format!("{:.2} V", diag.v_common_mode));
                    ui.end_row();
                });
        });
}

// ============================================================================
// GESTIÓN DE CAPTURAS, GRABACIÓN Y METADATOS EN CARPETAS DEDICADAS
// ============================================================================

/// Exporta un conjunto seleccionado de tramas a formato CSV con voltajes calibrados
fn export_subset_frames_csv(path: &std::path::Path, frames: &[neowon_core::SharedFrame]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = neowon_core::atomic_file::AtomicFile::create(path)?;
    let rate = frames.last().map(|f| f.sample_rate).unwrap_or(48_000.0).max(1.0);
    writeln!(f, "t_segundos,ch1_voltios,ch2_voltios")?;
    let mut i = 0usize;
    for frame in frames {
        let c0 = frame.channels.iter().find(|c| c.ch == 0);
        let c1 = frame.channels.iter().find(|c| c.ch == 1);
        let n = c0.or(c1).map_or(0, |c| c.data.len());
        for k in 0..n {
            let v = |c: Option<&neowon_core::ChannelCapture>| {
                c.and_then(|c| c.data.get(k).map(|&r| c.cal.volts_i(r)))
                    .map_or(String::new(), |val| format!("{val:.6}"))
            };
            writeln!(f, "{:.9},{},{}", i as f64 / rate, v(c0), v(c1))?;
            i += 1;
        }
    }
    f.commit()
}

/// Muestra la ventana modal de Captura y Grabación de Osciloscopio con notas de operario y selección de tramas
pub fn show_scope_capture_dialog(
    ctx: &egui::Context,
    link: &mut crate::Link,
    rec: &mut crate::record::Recorder,
    script: &mut crate::script::Script,
    state: &mut AutomotiveUiState,
) {
    if !state.scope_capture.dialog_open {
        return;
    }

    let mut open = true;
    egui::Window::new("📸 Captura y Grabación de Osciloscopio")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(440.0, 520.0))
        .default_pos(egui::pos2(320.0, 100.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;

            // 1. CARPETA Y NOMBRE DE ARCHIVO
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("📁 DESTINO Y NOMBRE").strong().color(egui::Color32::from_rgb(100, 200, 255)));
                    
                    ui.horizontal(|ui| {
                        ui.label("Carpeta:");
                        ui.text_edit_singleline(&mut state.scope_capture.folder);
                        if ui.button("📁 Examinar...").on_hover_text("Seleccionar carpeta con el explorador nativo").clicked() {
                            state.is_dialog_waiting = true;
                            request_pick_folder(
                                state.scope_capture.folder.clone(),
                                state.pending_dialog_result.clone(),
                                NativeDialogResult::ScopeFolder,
                            );
                        }
                        if ui.button("📂 Abrir").on_hover_text("Abrir esta carpeta en el Explorador de Windows").clicked() {
                            open_folder_in_explorer(&state.scope_capture.folder);
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("🔍 Ver Capturas Guardadas en Disco...").on_hover_text("Abrir gestor y visor de capturas anteriores").clicked() {
                            state.capture_browser_open = true;
                            state.capture_browser_tab = 0;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Prefijo:");
                        ui.text_edit_singleline(&mut state.scope_capture.filename_prefix);
                    });

                    let ts = picowon_auto::current_timestamp_str();
                    let clean_pfx: String = state.scope_capture.filename_prefix
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
                        .collect();
                    ui.label(
                        egui::RichText::new(format!("Vista previa: {}/{}_{}.[png/csv/nwc/json]", state.scope_capture.folder, clean_pfx, ts))
                            .weak()
                            .size(10.0)
                            .color(egui::Color32::from_rgb(160, 200, 230)),
                    );
                });

            // 2. ALCANCE DE LA GRABACIÓN (FRAMES)
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("⏱ ALCANCE DE LA GRABACIÓN").strong().color(egui::Color32::from_rgb(255, 200, 100)));
                    
                    let total_in_ram = rec.frames.len();
                    ui.label(egui::RichText::new(format!("Tramas en búfer RAM: {total_in_ram}")).small().weak());

                    ui.radio_value(&mut state.scope_capture.export_range, picowon_auto::ScopeExportRange::CurrentFrame, "Trama actual en pantalla (Frame bajo cursor)");
                    ui.radio_value(&mut state.scope_capture.export_range, picowon_auto::ScopeExportRange::AllFrames, format!("Todas las tramas en RAM ({total_in_ram} tramas)"));
                    
                    let is_last_n = matches!(state.scope_capture.export_range, picowon_auto::ScopeExportRange::LastN(_));
                    let sel_last_n = is_last_n;
                    if ui.radio(sel_last_n, "Últimas N tramas:").clicked() {
                        state.scope_capture.export_range = picowon_auto::ScopeExportRange::LastN(state.scope_capture.custom_n_frames);
                    }
                    if is_last_n {
                        ui.horizontal(|ui| {
                            for &n in &[1usize, 5, 10, 20, 50] {
                                let active = match state.scope_capture.export_range {
                                    picowon_auto::ScopeExportRange::LastN(cur_n) => cur_n == n,
                                    _ => false,
                                };
                                if ui.selectable_label(active, format!("{n}")).clicked() {
                                    state.scope_capture.custom_n_frames = n;
                                    state.scope_capture.export_range = picowon_auto::ScopeExportRange::LastN(n);
                                }
                            }
                            ui.add(egui::Slider::new(&mut state.scope_capture.custom_n_frames, 1..=50).text("tramas"));
                            state.scope_capture.export_range = picowon_auto::ScopeExportRange::LastN(state.scope_capture.custom_n_frames);
                        });
                    }
                });

            // 3. DATOS Y NOTAS DEL OPERARIO
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("📝 FICHA Y NOTAS DEL OPERARIO").strong().color(egui::Color32::from_rgb(120, 240, 160)));
                    
                    ui.horizontal(|ui| {
                        ui.label("Técnico / Operario:");
                        ui.text_edit_singleline(&mut state.scope_capture.operator_name);
                    });

                    ui.label("Notas y detalles de la medición (Vehículo, sensor, condiciones de ensayo):");
                    ui.add(
                        egui::TextEdit::multiline(&mut state.scope_capture.operator_notes)
                            .desired_rows(3)
                            .desired_width(ui.available_width()),
                    );
                });

            // 4. FORMATOS A GENERAR
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(18, 22, 30))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("📦 FORMATOS DE EXPORTACIÓN").strong().color(egui::Color32::from_rgb(200, 180, 255)));
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut state.scope_capture.export_png, "Imagen PNG");
                        ui.checkbox(&mut state.scope_capture.export_csv, "Datos CSV");
                        ui.checkbox(&mut state.scope_capture.export_nwc, "Nativo NWC");
                        ui.label(egui::RichText::new("● Ficha Técnica JSON (Siempre)").weak().small());
                    });
                });

            ui.add_space(4.0);

            // BOTONES DE ACCIÓN
            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("💾 GUARDAR EN DISCO").strong().color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(25, 95, 55))
                .min_size(egui::vec2(150.0, 26.0));

                if ui.add(save_btn).clicked() {
                    execute_scope_save(link, rec, script, state);
                }

                // Si ya se guardó antes con este prefijo, botón de confirmación rápida con 1 clic
                let same_prefix = state.scope_capture.filename_prefix == state.scope_capture.last_prefix;
                if same_prefix && state.scope_capture.last_saved_info.is_some() {
                    let quick_btn = egui::Button::new(
                        egui::RichText::new("⚡ MISMO NOMBRE (NUEVO TIMESTAMP)").color(egui::Color32::from_rgb(255, 220, 100)),
                    )
                    .fill(egui::Color32::from_rgb(40, 50, 70))
                    .min_size(egui::vec2(180.0, 26.0));
                    if ui.add(quick_btn).clicked() {
                        execute_scope_save(link, rec, script, state);
                    }
                }

                if ui.button("Cancelar").clicked() {
                    state.scope_capture.dialog_open = false;
                }
            });
        });

    if !open {
        state.scope_capture.dialog_open = false;
    }
}

fn execute_scope_save(
    link: &mut crate::Link,
    rec: &mut crate::record::Recorder,
    script: &mut crate::script::Script,
    state: &mut AutomotiveUiState,
) {
    let (base_path, _stem) = state.scope_capture.generate_base_path();

    // 1. Obtener frames según rango seleccionado
    let frames_to_save: Vec<neowon_core::SharedFrame> = match state.scope_capture.export_range {
        picowon_auto::ScopeExportRange::CurrentFrame => {
            link.latest.clone().into_iter().collect()
        }
        picowon_auto::ScopeExportRange::AllFrames => {
            if !rec.frames.is_empty() {
                rec.frames.clone()
            } else {
                link.latest.clone().into_iter().collect()
            }
        }
        picowon_auto::ScopeExportRange::LastN(n) => {
            if !rec.frames.is_empty() {
                let start = rec.frames.len().saturating_sub(n.max(1));
                rec.frames[start..].to_vec()
            } else {
                link.latest.clone().into_iter().collect()
            }
        }
    };

    // 2. Exportar NWC si está activo
    if state.scope_capture.export_nwc && !frames_to_save.is_empty() {
        let nwc_path = base_path.with_extension("nwc");
        let _ = neowon_core::nwc::write(&nwc_path, &frames_to_save);
    }

    // 3. Exportar CSV si está activo
    if state.scope_capture.export_csv && !frames_to_save.is_empty() {
        let csv_path = base_path.with_extension("csv");
        let _ = export_subset_frames_csv(&csv_path, &frames_to_save);
    }

    // 4. Exportar PNG de la gráfica si está activo
    if state.scope_capture.export_png {
        let png_path = base_path.with_extension("png");
        script.inject(crate::script::Action::ShotPlot {
            path: png_path.display().to_string(),
            roi: None,
        });
    }

    // 5. Guardar Ficha de Metadatos JSON con ajustes de HW completos
    let meta_path = base_path.with_extension("json");
    let record_len = crate::view::record_len(link);
    let timebase_per_div = record_len as f64 / link.config.sample_rate.max(1.0) / crate::ui::layout::H_DIVS as f64;
    let timebase_str = format!("{}/div", crate::derived::fmt_si(timebase_per_div, "s"));

    let ch0 = &link.config.channels[0];
    let ch1 = &link.config.channels[1];

    let metadata = picowon_auto::ScopeCaptureMetadata {
        title: state.scope_capture.filename_prefix.clone(),
        timestamp: picowon_auto::current_timestamp_str(),
        operator_name: state.scope_capture.operator_name.clone(),
        operator_notes: state.scope_capture.operator_notes.clone(),
        export_range: state.scope_capture.export_range.label(),
        channels: [
            picowon_auto::SavedChannelConfig::new(
                ch0.enabled,
                ch0.volts_div,
                ch0.probe,
                matches!(ch0.coupling, neowon_core::Coupling::Dc),
                ch0.offset,
            ),
            picowon_auto::SavedChannelConfig::new(
                ch1.enabled,
                ch1.volts_div,
                ch1.probe,
                matches!(ch1.coupling, neowon_core::Coupling::Dc),
                ch1.offset,
            ),
        ],
        timebase_sec_div: timebase_per_div,
        timebase_str,
        trigger_source: link.config.trigger.source,
        trigger_sweep: format!("{:?}", link.config.trigger.sweep),
        trigger_slope: match link.config.trigger.kind {
            neowon_core::TriggerKind::Edge { slope } => format!("{slope:?}"),
            _ => "None".to_string(),
        },
        trigger_level_v: link.config.trigger.level,
        dsp_configs: [Some(state.dsp[0]), Some(state.dsp[1])],
        sample_rate_hz: frames_to_save.last().map(|f| f.sample_rate).unwrap_or(0.0),
        total_samples: frames_to_save.iter().map(|f| f.channels.first().map_or(0, |c| c.data.len())).sum(),
        frame_count: frames_to_save.len(),
    };
    if let Ok(json_str) = serde_json::to_string_pretty(&metadata) {
        let _ = std::fs::write(&meta_path, json_str);
    }

    state.scope_capture.last_saved_info = Some((base_path.display().to_string(), 0.0));
    state.banner_message = Some(format!("Captura guardada en: {} ({})", base_path.display(), state.scope_capture.export_range.label()));
    state.scope_capture.dialog_open = false;
}

/// Muestra los diálogos modales de Inicio y Confirmación del Datalogger
pub fn show_datalogger_session_dialogs(
    ctx: &egui::Context,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
    now: f64,
) {
    // 1. DIÁLOGO DE INICIO DE GRABACIÓN
    if state.datalogger_session.is_starting_dialog_open {
        let mut open = true;
        egui::Window::new("🔴 Iniciar Grabación - Datalogger")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size(egui::vec2(400.0, 320.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                ui.label(egui::RichText::new("Configuración de Sesión de Registro").strong().color(egui::Color32::from_rgb(100, 200, 255)));

                ui.horizontal(|ui| {
                    ui.label("Carpeta:");
                    ui.text_edit_singleline(&mut state.datalogger_session.folder);
                    if ui.button("📁 Examinar...").on_hover_text("Seleccionar carpeta con el explorador nativo").clicked() {
                        state.is_dialog_waiting = true;
                        request_pick_folder(
                            state.datalogger_session.folder.clone(),
                            state.pending_dialog_result.clone(),
                            NativeDialogResult::DataloggerFolder,
                        );
                    }
                    if ui.button("📂 Abrir").on_hover_text("Abrir esta carpeta en el Explorador de Windows").clicked() {
                        open_folder_in_explorer(&state.datalogger_session.folder);
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("🔍 Ver Registros Guardados en Disco...").on_hover_text("Abrir gestor y visor de sesiones de datalogger").clicked() {
                        state.capture_browser_open = true;
                        state.capture_browser_tab = 1;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Nombre / Prefijo:");
                    ui.text_edit_singleline(&mut state.datalogger_session.filename_prefix);
                });

                let ts = picowon_auto::current_timestamp_str();
                let clean_pfx: String = state.datalogger_session.filename_prefix
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
                    .collect();
                ui.label(
                    egui::RichText::new(format!("Destino: {}/{}_{}.csv", state.datalogger_session.folder, clean_pfx, ts))
                        .weak()
                        .size(10.0),
                );

                ui.horizontal(|ui| {
                    ui.label("Técnico / Operario:");
                    ui.text_edit_singleline(&mut state.datalogger_session.operator_name);
                });

                ui.label("Notas iniciales del ensayo:");
                ui.add(
                    egui::TextEdit::multiline(&mut state.datalogger_session.operator_notes)
                        .desired_rows(3)
                        .desired_width(ui.available_width()),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let start_btn = egui::Button::new(
                        egui::RichText::new("▶ COMENZAR REGISTRO").strong().color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(160, 40, 30))
                    .min_size(egui::vec2(150.0, 24.0));

                    if ui.add(start_btn).clicked() {
                        let (base_path, stem) = state.datalogger_session.prepare_start();
                        let csv_path = base_path.with_extension("csv");
                        state.datalogger_session.session_start_time = now;
                        let _ = state.strip_chart.start_recording(csv_path, now);
                        state.datalogger_session.is_starting_dialog_open = false;
                        state.banner_message = Some(format!("Datalogger grabando: {stem}"));
                    }

                    if ui.button("Cancelar").clicked() {
                        state.datalogger_session.is_starting_dialog_open = false;
                    }
                });
            });

        if !open {
            state.datalogger_session.is_starting_dialog_open = false;
        }
    }

    // 2. DIÁLOGO DE CONFIRMACIÓN AL DETENER GRABACIÓN
    if state.datalogger_session.is_stopping_dialog_open {
        let mut open = true;
        egui::Window::new("⏹ Confirmación de Grabación - Datalogger")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size(egui::vec2(420.0, 380.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                ui.label(egui::RichText::new("Resumen de Sesión Registrada").strong().color(egui::Color32::from_rgb(255, 200, 80)));

                let dur = state.datalogger_session.recorded_duration_secs;
                let mins = (dur / 60.0).floor() as u32;
                let secs = (dur % 60.0).floor() as u32;
                let pts = state.datalogger_session.recorded_samples;

                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(18, 22, 30))
                    .inner_margin(8.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                    .show(ui, |ui| {
                        ui.label(format!("⏱ Duración Total: {mins:02}:{secs:02} ({dur:.1} s)"));
                        ui.label(format!("📊 Muestras Recopiladas: {pts} registros"));
                        if let Some(base) = &state.datalogger_session.active_file_base {
                            ui.label(format!("📁 Archivo: {}", base.with_extension("csv").display()));
                        }
                    });

                ui.label("Notas finales / Conclusiones del ensayo:");
                ui.add(
                    egui::TextEdit::multiline(&mut state.datalogger_session.operator_notes)
                        .desired_rows(3)
                        .desired_width(ui.available_width()),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let confirm_btn = egui::Button::new(
                        egui::RichText::new("💾 CONFIRMAR Y GUARDAR EN DISCO").strong().color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(25, 95, 55))
                    .min_size(egui::vec2(180.0, 26.0));

                    if ui.add(confirm_btn).clicked() {
                        // Guardar archivo JSON de metadatos de sesión
                        if let Some(base) = &state.datalogger_session.active_file_base {
                            let meta_path = base.with_extension("json");
                            let meta_info = serde_json::json!({
                                "titulo": state.datalogger_session.filename_prefix,
                                "operario": state.datalogger_session.operator_name,
                                "notas": state.datalogger_session.operator_notes,
                                "duracion_segundos": dur,
                                "muestras_registradas": pts,
                                "timestamp": picowon_auto::current_timestamp_str(),
                                "ch1_vdiv": link.config.channels[0].volts_div * link.config.channels[0].probe,
                                "ch2_vdiv": link.config.channels[1].volts_div * link.config.channels[1].probe,
                            });
                            let _ = std::fs::write(&meta_path, serde_json::to_string_pretty(&meta_info).unwrap_or_default());
                            state.banner_message = Some(format!("Grabación guardada con éxito en: {}", base.display()));
                        }
                        state.datalogger_session.is_stopping_dialog_open = false;
                    }

                    let discard_btn = egui::Button::new(
                        egui::RichText::new("🗑 Descartar Grabación").color(egui::Color32::from_rgb(255, 120, 120)),
                    )
                    .fill(egui::Color32::from_rgb(60, 25, 25))
                    .min_size(egui::vec2(130.0, 26.0));

                    if ui.add(discard_btn).on_hover_text("Eliminar archivo y descartar datos de esta sesión").clicked() {
                        if let Some(base) = &state.datalogger_session.active_file_base {
                            let csv_path = base.with_extension("csv");
                            let _ = std::fs::remove_file(csv_path);
                        }
                        state.banner_message = Some("Grabación descartada (archivo no guardado).".to_string());
                        state.datalogger_session.is_stopping_dialog_open = false;
                    }
                });
            });

        if !open {
            state.datalogger_session.is_stopping_dialog_open = false;
        }
    }
}

/// Muestra los diálogos modales de Guardar y Cargar Firmas en BD del Tracer VI
pub fn show_vi_capture_and_db_dialogs(
    ctx: &egui::Context,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
) {
    // 1. MODAL GUARDAR FIRMA EN BASE DE DATOS
    if state.vi_capture.is_save_dialog_open {
        let mut open = true;
        egui::Window::new("💾 Guardar Firma en Base de Datos - Tracer VI")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size(egui::vec2(440.0, 480.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                ui.label(egui::RichText::new("Ficha del Componente / Pin a Guardar").strong().color(egui::Color32::from_rgb(100, 220, 255)));

                ui.horizontal(|ui| {
                    ui.label("Carpeta:");
                    ui.text_edit_singleline(&mut state.vi_capture.folder);
                    if ui.button("📁 Examinar...").on_hover_text("Seleccionar carpeta con el explorador nativo").clicked() {
                        state.is_dialog_waiting = true;
                        request_pick_folder(
                            state.vi_capture.folder.clone(),
                            state.pending_dialog_result.clone(),
                            NativeDialogResult::TracerFolder,
                        );
                    }
                    if ui.button("📂 Abrir").on_hover_text("Abrir esta carpeta en el Explorador de Windows").clicked() {
                        open_folder_in_explorer(&state.vi_capture.folder);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Nombre / Pin:");
                    ui.text_edit_singleline(&mut state.vi_capture.filename_prefix);
                });

                ui.horizontal(|ui| {
                    ui.label("Categoría:");
                    for cat in &["Inyectores", "Sensores", "Semiconductores", "Actuadores", "General"] {
                        if ui.selectable_label(state.vi_capture.category == *cat, *cat).clicked() {
                            state.vi_capture.category = cat.to_string();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Técnico:");
                    ui.text_edit_singleline(&mut state.vi_capture.operator_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Tolerancia Cuantitativa:");
                    ui.add(egui::Slider::new(&mut state.vi_capture.tolerance_percent, 1.0..=20.0).suffix("%"));
                });

                ui.label("Notas de operario (Detalle de circuito, estado de pieza sana/sospechosa):");
                ui.add(
                    egui::TextEdit::multiline(&mut state.vi_capture.operator_notes)
                        .desired_rows(3)
                        .desired_width(ui.available_width()),
                );

                // Resumen de parámetros de hardware que se guardan con la firma
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(16, 20, 28))
                    .inner_margin(6.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 50, 70)))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Ajustes de Hardware que se vincularán al modelo:").small().strong().color(egui::Color32::from_rgb(200, 215, 230)));
                        let ch1_v = link.config.channels[0].volts_div * link.config.channels[0].probe;
                        let ch2_v = link.config.channels[1].volts_div * link.config.channels[1].probe;
                        ui.label(format!("• CH1 (V): {:.2} V/div · CH2 (I): {:.2} V/div · R_sense: {:.0} Ω", ch1_v, ch2_v, state.vi_tracer.r_sense_ohms));
                        ui.label(format!("• Ráfaga: {} ciclos · Cuadrícula: {}x{} · Invertir Y: {}", state.vi_tracer.burst_count, state.vi_tracer.grid_divs_h, state.vi_tracer.grid_divs_v, state.vi_tracer.invert_vertical));
                    });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(
                        egui::RichText::new("💾 GUARDAR EN BD Y DISCO").strong().color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(25, 95, 55))
                    .min_size(egui::vec2(160.0, 26.0));

                    if ui.add(save_btn).clicked() {
                        let ch1_v = link.config.channels[0].volts_div;
                        let ch2_v = link.config.channels[1].volts_div;
                        let ch1_p = link.config.channels[0].probe;
                        let ch2_p = link.config.channels[1].probe;
                        let r_sense = state.vi_tracer.r_sense_ohms as f64;
                        let inv_v = state.vi_tracer.invert_vertical;
                        let burst = state.vi_tracer.burst_count;
                        let grid_h = state.vi_tracer.grid_divs_h;
                        let grid_v = state.vi_tracer.grid_divs_v;
                        let equiv = state.vi_tracer.equivalent_circuit.as_ref().map(|c| c.summary.clone());

                        let points = if let Some(ref q) = state.vi_tracer.quasi_stationary_curve {
                            q.clone()
                        } else {
                            state.vi_tracer.current_points.clone()
                        };

                        let name = state.vi_capture.filename_prefix.clone();
                        let cat = state.vi_capture.category.clone();
                        let desc = state.vi_capture.operator_notes.clone();
                        let op = state.vi_capture.operator_name.clone();
                        let tol = state.vi_capture.tolerance_percent;

                        let record_len = crate::view::record_len(link);
                        let timebase_per_div = record_len as f64 / link.config.sample_rate.max(1.0) / crate::ui::layout::H_DIVS as f64;

                        match state.vi_capture.save_current_signature(
                            &name, &cat, &desc, &op, tol, points.clone(),
                            r_sense, inv_v, ch1_v, ch2_v, ch1_p, ch2_p,
                            timebase_per_div, burst, grid_h, grid_v, equiv,
                        ) {
                            Ok(base_path) => {
                                // Generar también snapshot PNG con el mismo nombre base
                                let png_path = base_path.with_extension("png");
                                let ch1_eff_v = ch1_v * ch1_p;
                                let ch2_eff_i = (ch2_v * ch2_p / r_sense.max(1.0)) * 1000.0;
                                let _ = save_vi_snapshot_png_to_path(&points, None, ch1_eff_v as f32, ch2_eff_i as f32, &png_path);

                                // Fijar como referencia inmediata en pantalla
                                state.vi_tracer.capture_reference_with_name(&name);
                                state.vi_tracer.tolerance_percent = tol;

                                state.banner_message = Some(format!("Firma guardada en BD: {}", name));
                                state.vi_capture.is_save_dialog_open = false;
                            }
                            Err(e) => {
                                state.banner_message = Some(format!("Error al guardar firma: {e}"));
                            }
                        }
                    }

                    if ui.button("Cancelar").clicked() {
                        state.vi_capture.is_save_dialog_open = false;
                    }
                });
            });

        if !open {
            state.vi_capture.is_save_dialog_open = false;
        }
    }

    // 2. MODAL BIBLIOTECA / BASE DE DATOS DE FIRMAS (CARGAR COMO REFERENCIA)
    if state.vi_capture.is_load_db_dialog_open {
        let mut open = true;
        egui::Window::new("📂 Biblioteca de Firmas V-I (Base de Datos)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(egui::vec2(580.0, 480.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Firmas y Modelos Guardados").strong().color(egui::Color32::from_rgb(100, 200, 255)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut state.vi_capture.restore_hardware_on_load, "Restaurar ajustes de HW al cargar");
                    });
                });

                ui.separator();

                let mut to_delete: Option<String> = None;
                let mut to_load: Option<picowon_auto::ViModelEntry> = None;

                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        if state.vi_capture.db.models.is_empty() {
                            ui.label("No hay firmas guardadas en la base de datos.");
                        }

                        for entry in &state.vi_capture.db.models {
                            let is_selected = state.vi_capture.selected_db_id.as_deref() == Some(&entry.id);
                            
                            egui::Frame::new()
                                .fill(if is_selected { egui::Color32::from_rgb(26, 36, 50) } else { egui::Color32::from_rgb(16, 20, 26) })
                                .stroke(egui::Stroke::new(1.0, if is_selected { egui::Color32::from_rgb(80, 160, 240) } else { egui::Color32::from_rgb(35, 45, 60) }))
                                .inner_margin(8.0)
                                .corner_radius(4.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&entry.name).strong().size(13.0).color(egui::Color32::WHITE));
                                                ui.colored_label(egui::Color32::from_rgb(100, 200, 255), format!("[{}]", entry.category));
                                                ui.label(egui::RichText::new(format!("Tol: ±{:.1}%", entry.tolerance_percent)).small().color(egui::Color32::from_rgb(120, 240, 150)));
                                            });

                                            if !entry.description.is_empty() {
                                                ui.label(egui::RichText::new(&entry.description).weak().small().color(egui::Color32::from_gray(180)));
                                            }

                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "HW: CH1 {:.1}V · CH2 {:.1}V · R_sense {:.0}Ω | Técnico: {} ({})",
                                                    entry.ch1_vdiv * entry.ch1_probe,
                                                    entry.ch2_vdiv * entry.ch2_probe,
                                                    entry.r_sense_ohms,
                                                    entry.operator_name,
                                                    entry.created_at
                                                ))
                                                .size(9.5)
                                                .color(egui::Color32::from_gray(150)),
                                            );
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("🗑").on_hover_text("Eliminar esta firma de la BD").clicked() {
                                                to_delete = Some(entry.id.clone());
                                            }

                                            let load_btn = egui::Button::new(
                                                egui::RichText::new("🎯 USAR COMO REF").strong().color(egui::Color32::WHITE),
                                            )
                                            .fill(egui::Color32::from_rgb(25, 95, 55));

                                            if ui.add(load_btn).on_hover_text("Cargar como curva Golden Sample de referencia e iniciar comparación cuantitativa").clicked() {
                                                to_load = Some(entry.clone());
                                            }
                                        });
                                    });
                                });
                            ui.add_space(3.0);
                        }
                    });

                if let Some(del_id) = to_delete {
                    state.vi_capture.db.remove(&del_id);
                }

                if let Some(entry) = to_load {
                    // Cargar curva como referencia en vivo
                    state.vi_tracer.reference_points = Some(entry.points.clone());
                    state.vi_tracer.reference_name = entry.name.clone();
                    state.vi_tracer.tolerance_percent = entry.tolerance_percent;

                    // Restaurar ajustes de hardware si la opción está activa
                    if state.vi_capture.restore_hardware_on_load {
                        link.config.channels[0].volts_div = entry.ch1_vdiv;
                        link.config.channels[1].volts_div = entry.ch2_vdiv;
                        link.config.channels[0].probe = entry.ch1_probe;
                        link.config.channels[1].probe = entry.ch2_probe;
                        state.vi_tracer.r_sense_ohms = entry.r_sense_ohms as f32;
                        state.vi_tracer.invert_vertical = entry.invert_vertical;
                        state.vi_tracer.burst_count = entry.burst_count;
                        state.vi_tracer.grid_divs_h = entry.grid_divs_h;
                        state.vi_tracer.grid_divs_v = entry.grid_divs_v;
                        link.dirty = true;
                    }

                    // Calcular inmediatamente la comparación cuantitativa
                    state.vi_tracer.compute_similarity_with_reference();
                    state.banner_message = Some(format!("Firma cargada como Golden Sample: {}", entry.name));
                    state.vi_capture.is_load_db_dialog_open = false;
                }

                ui.separator();
                if ui.button("Cerrar").clicked() {
                    state.vi_capture.is_load_db_dialog_open = false;
                }
            });

        if !open {
            state.vi_capture.is_load_db_dialog_open = false;
        }
    }
}

/// Abre una carpeta en el Explorador de archivos de Windows
pub fn open_folder_in_explorer(folder: &str) {
    let p = std::path::Path::new(folder);
    if !p.exists() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::process::Command::new("explorer").arg(folder).spawn();
}

/// Lanza el diálogo nativo de selección de carpeta de Windows en un hilo de fondo (sin congelar la UI)
pub fn request_pick_folder<F: FnOnce(String) -> NativeDialogResult + Send + 'static>(
    initial_dir: String,
    slot: std::sync::Arc<std::sync::Mutex<Option<NativeDialogResult>>>,
    maker: F,
) {
    std::thread::spawn(move || {
        let clean_dir = initial_dir.replace('\\', "\\\\").replace('\'', "''");
        let script = format!(
            "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; \
            $f = New-Object System.Windows.Forms.FolderBrowserDialog; \
            $f.Description = 'Seleccionar Carpeta de Almacenamiento'; \
            $f.SelectedPath = '{clean_dir}'; \
            if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ \
                [Console]::WriteLine($f.SelectedPath) \
            }}"
        );
        if let Ok(out) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
        {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    if let Ok(mut lock) = slot.lock() {
                        *lock = Some(maker(path));
                    }
                }
            }
        }
    });
}

/// Lanza el diálogo nativo de selección de archivo de Windows en un hilo de fondo (sin congelar la UI)
pub fn request_pick_file<F: FnOnce(std::path::PathBuf) -> NativeDialogResult + Send + 'static>(
    filter: &'static str,
    slot: std::sync::Arc<std::sync::Mutex<Option<NativeDialogResult>>>,
    maker: F,
) {
    std::thread::spawn(move || {
        let script = format!(
            "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; \
            $f = New-Object System.Windows.Forms.OpenFileDialog; \
            $f.Title = 'Seleccionar Captura Almacenada'; \
            $f.Filter = '{filter}'; \
            if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ \
                [Console]::WriteLine($f.FileName) \
            }}"
        );
        if let Ok(out) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
        {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    if let Ok(mut lock) = slot.lock() {
                        *lock = Some(maker(std::path::PathBuf::from(path)));
                    }
                }
            }
        }
    });
}

/// Procesa las respuestas de diálogos nativos pendientes
pub fn process_pending_dialogs(
    link: &mut crate::Link,
    rec: &mut crate::record::Recorder,
    state: &mut AutomotiveUiState,
) {
    let result = if let Ok(mut lock) = state.pending_dialog_result.lock() {
        lock.take()
    } else {
        None
    };

    if let Some(res) = result {
        state.is_dialog_waiting = false;
        match res {
            NativeDialogResult::ScopeFolder(p) => {
                state.scope_capture.folder = p.clone();
                state.banner_message = Some(format!("Carpeta de Osciloscopio actualizada: {p}"));
            }
            NativeDialogResult::DataloggerFolder(p) => {
                state.datalogger_session.folder = p.clone();
                state.banner_message = Some(format!("Carpeta de Datalogger actualizada: {p}"));
            }
            NativeDialogResult::TracerFolder(p) => {
                state.vi_capture.folder = p.clone();
                state.banner_message = Some(format!("Carpeta de Tracer VI actualizada: {p}"));
            }
            NativeDialogResult::OpenCaptureFile(p) => {
                load_capture_from_file_path(&p, link, rec, state);
            }
        }
    }
}

/// Carga un archivo de captura suelto desde disco, restaurando datos y configuración de hardware
pub fn load_capture_from_file_path(
    path: &std::path::Path,
    link: &mut crate::Link,
    rec: &mut crate::record::Recorder,
    state: &mut AutomotiveUiState,
) {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "json" => {
            // Intentar cargar como metadatos de osciloscopio
            if let Ok(meta) = picowon_auto::ScopeCaptureMetadata::load_from_file(path) {
                // 1. Restaurar configuración de canales
                link.config.channels[0].enabled = meta.channels[0].enabled;
                link.config.channels[0].volts_div = meta.channels[0].volts_div;
                link.config.channels[0].probe = meta.channels[0].probe;
                link.config.channels[0].coupling = if meta.channels[0].coupling_dc {
                    neowon_core::Coupling::Dc
                } else {
                    neowon_core::Coupling::Ac
                };
                link.config.channels[0].offset = meta.channels[0].offset;

                link.config.channels[1].enabled = meta.channels[1].enabled;
                link.config.channels[1].volts_div = meta.channels[1].volts_div;
                link.config.channels[1].probe = meta.channels[1].probe;
                link.config.channels[1].coupling = if meta.channels[1].coupling_dc {
                    neowon_core::Coupling::Dc
                } else {
                    neowon_core::Coupling::Ac
                };
                link.config.channels[1].offset = meta.channels[1].offset;

                // 2. Restaurar disparo
                link.config.trigger.source = meta.trigger_source;
                link.config.trigger.level = meta.trigger_level_v;

                // 3. Restaurar DSP si estaban configurados
                if let Some(d0) = meta.dsp_configs[0] {
                    state.dsp[0] = d0;
                }
                if let Some(d1) = meta.dsp_configs[1] {
                    state.dsp[1] = d1;
                }

                // 4. Cargar archivo NWC binario de tramas si existe
                let nwc_path = path.with_extension("nwc");
                let mut loaded_frames_count = 0;
                if nwc_path.exists() {
                    if let Ok(count) = rec.load_capture(&nwc_path) {
                        loaded_frames_count = count;
                    }
                }

                // 5. Congelar adquisición en vivo para inspección estática
                link.config.running = false;
                link.config.sample_rate = meta.sample_rate_hz;
                link.latest = rec.frames.last().cloned();
                link.dirty = true;

                let ch0_v = meta.channels[0].volts_div * meta.channels[0].probe;
                let ch1_v = meta.channels[1].volts_div * meta.channels[1].probe;
                state.loaded_capture_info = Some(LoadedCaptureBannerInfo {
                    title: meta.title.clone(),
                    timestamp: meta.timestamp.clone(),
                    operator: meta.operator_name.clone(),
                    notes: meta.operator_notes.clone(),
                    details: format!(
                        "CH1: {:.2}V/div · CH2: {:.2}V/div · Base: {} · Disparo: C{} ({:.2}V) · {} tramas cargadas",
                        ch0_v, ch1_v, meta.timebase_str, meta.trigger_source + 1, meta.trigger_level_v, loaded_frames_count
                    ),
                    file_path: path.display().to_string(),
                });

                state.banner_message = Some(format!(
                    "Captura cargada: {} (Ajustes de HW y DSP restaurados. Osciloscopio en pausa para inspección)",
                    meta.title
                ));
                return;
            }

            // Intentar cargar como modelo de curva V-I
            if let Ok(mut f) = std::fs::File::open(path) {
                let mut content = String::new();
                use std::io::Read;
                if f.read_to_string(&mut content).is_ok() {
                    if let Ok(entry) = serde_json::from_str::<picowon_auto::ViModelEntry>(&content) {
                        state.vi_tracer.reference_points = Some(entry.points.clone());
                        state.vi_tracer.reference_name = entry.name.clone();
                        state.vi_tracer.tolerance_percent = entry.tolerance_percent;
                        state.banner_message = Some(format!("Firma VI cargada desde JSON: {}", entry.name));
                        state.switch_app_mode(AppMode::ViCurveTracer, link);
                        return;
                    }
                }
            }
        }
        "nwc" | "cap" => {
            if let Ok(count) = rec.load_capture(path) {
                // Intentar buscar JSON adyacente para restaurar notas
                let json_path = path.with_extension("json");
                if json_path.exists() {
                    if let Ok(meta) = picowon_auto::ScopeCaptureMetadata::load_from_file(&json_path) {
                        link.config.channels[0].volts_div = meta.channels[0].volts_div;
                        link.config.channels[0].probe = meta.channels[0].probe;
                        link.config.channels[0].enabled = meta.channels[0].enabled;
                        link.config.channels[0].offset = meta.channels[0].offset;
                        link.config.channels[1].volts_div = meta.channels[1].volts_div;
                        link.config.channels[1].probe = meta.channels[1].probe;
                        link.config.channels[1].enabled = meta.channels[1].enabled;
                        link.config.channels[1].offset = meta.channels[1].offset;
                        link.config.trigger.source = meta.trigger_source;
                        link.config.trigger.level = meta.trigger_level_v;
                        if let Some(d0) = meta.dsp_configs[0] { state.dsp[0] = d0; }
                        if let Some(d1) = meta.dsp_configs[1] { state.dsp[1] = d1; }

                        state.loaded_capture_info = Some(LoadedCaptureBannerInfo {
                            title: meta.title.clone(),
                            timestamp: meta.timestamp.clone(),
                            operator: meta.operator_name.clone(),
                            notes: meta.operator_notes.clone(),
                            details: format!("Archivo: {} ({} tramas)", path.file_name().and_then(|n| n.to_str()).unwrap_or(""), count),
                            file_path: path.display().to_string(),
                        });
                    }
                }
                link.config.running = false;
                link.latest = rec.frames.last().cloned();
                link.dirty = true;
                state.banner_message = Some(format!("Archivo {} cargado con éxito ({} tramas)", path.display(), count));
            }
        }
        "csv" => {
            // Si es CSV de Datalogger o de Trazador VI
            if let Ok(content) = std::fs::read_to_string(path) {
                let first_line = content.lines().next().unwrap_or("");
                if first_line.contains("Voltaje_V") || first_line.contains("Corriente_mA") {
                    let mut pts = Vec::new();
                    for line in content.lines().skip(1) {
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 2 {
                            if let (Ok(v), Ok(i)) = (parts[0].trim().parse::<f32>(), parts[1].trim().parse::<f32>()) {
                                pts.push((v, i));
                            }
                        }
                    }
                    if !pts.is_empty() {
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Curva_CSV");
                        state.vi_tracer.reference_points = Some(pts);
                        state.vi_tracer.reference_name = stem.to_string();
                        state.banner_message = Some(format!("Curva CSV cargada como referencia: {stem}"));
                        state.switch_app_mode(AppMode::ViCurveTracer, link);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Ventana modal interactiva: Gestor y Explorador de Capturas Almacenadas con Buscador Filtrable
pub fn show_capture_browser_dialog(
    ctx: &egui::Context,
    link: &mut crate::Link,
    rec: &mut crate::record::Recorder,
    state: &mut AutomotiveUiState,
    _now: f64,
) {
    // Procesar cualquier respuesta asíncrona de diálogos de Windows
    process_pending_dialogs(link, rec, state);

    if !state.capture_browser_open {
        return;
    }

    let mut open = true;
    egui::Window::new("📂 Explorador y Gestor de Capturas Almacenadas")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(760.0, 560.0))
        .default_pos(egui::pos2(240.0, 60.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;

            // 1. BARRA SUPERIOR: PESTAÑAS DE MÓDULO Y ACCIONES DE DISCO
            ui.horizontal(|ui| {
                let tab0 = ui.selectable_label(state.capture_browser_tab == 0, "⚡ Capturas Osciloscopio");
                if tab0.clicked() { state.capture_browser_tab = 0; }

                let tab1 = ui.selectable_label(state.capture_browser_tab == 1, "📈 Registros Datalogger");
                if tab1.clicked() { state.capture_browser_tab = 1; }

                let tab2 = ui.selectable_label(state.capture_browser_tab == 2, "📉 Modelos Tracer V-I");
                if tab2.clicked() { state.capture_browser_tab = 2; }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let folder_to_open = match state.capture_browser_tab {
                        0 => &state.scope_capture.folder,
                        1 => &state.datalogger_session.folder,
                        _ => &state.vi_capture.folder,
                    };

                    if ui.button("📂 Abrir Carpeta en Windows").on_hover_text("Ver los archivos en el Explorador de Windows").clicked() {
                        open_folder_in_explorer(folder_to_open);
                    }

                    if ui.button("📁 Cargar Archivo Externo...").on_hover_text("Seleccionar cualquier archivo suelto desde disco").clicked() {
                        state.is_dialog_waiting = true;
                        let filter = match state.capture_browser_tab {
                            0 => "Capturas Osciloscopio (*.json;*.nwc;*.cap)|*.json;*.nwc;*.cap|Todos (*.*)|*.*",
                            1 => "Registros Datalogger (*.csv;*.json)|*.csv;*.json|Todos (*.*)|*.*",
                            _ => "Firmas Tracer VI (*.json;*.csv)|*.json;*.csv|Todos (*.*)|*.*",
                        };
                        request_pick_file(
                            filter,
                            state.pending_dialog_result.clone(),
                            NativeDialogResult::OpenCaptureFile,
                        );
                    }
                });
            });

            // 2. BUSCADOR REACTIVO EN TIEMPO REAL
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍 Filtro:").strong().color(egui::Color32::from_rgb(100, 200, 255)));
                ui.add(
                    egui::TextEdit::singleline(&mut state.capture_browser_search)
                        .desired_width(ui.available_width() - 40.0)
                        .hint_text("Buscar por nombre, técnico, notas o fecha..."),
                );
                if !state.capture_browser_search.is_empty() {
                    if ui.small_button("✖").on_hover_text("Limpiar filtro de búsqueda").clicked() {
                        state.capture_browser_search.clear();
                    }
                }
            });
            ui.separator();

            // 3. CONTENIDO DE PESTAÑA SELECCIONADA
            match state.capture_browser_tab {
                0 => {
                    // PESTAÑA OSCILOSCOPIO
                    let mut captures = picowon_auto::scan_scope_captures(&state.scope_capture.folder);
                    if !state.capture_browser_search.is_empty() {
                        let query = state.capture_browser_search.to_lowercase();
                        captures.retain(|c| {
                            c.display_title.to_lowercase().contains(&query)
                                || c.operator_name.to_lowercase().contains(&query)
                                || c.operator_notes.to_lowercase().contains(&query)
                                || c.timestamp_str.contains(&query)
                                || c.stem.to_lowercase().contains(&query)
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("📁 Carpeta: {} ({} capturas encontradas)", state.scope_capture.folder, captures.len())).small().weak());
                    });

                    let mut to_load_entry: Option<picowon_auto::SavedScopeCaptureEntry> = None;
                    let mut to_delete_entry: Option<picowon_auto::SavedScopeCaptureEntry> = None;

                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if captures.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(20.0);
                                    ui.label(egui::RichText::new("No se encontraron capturas guardadas en esta carpeta.").weak());
                                    ui.label(egui::RichText::new("Guarde una captura con el botón [📸 CAPTURAR / GRABAR] en el panel izquierdo.").small().weak());
                                });
                            }

                            for entry in captures {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(18, 22, 30))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                                    .inner_margin(8.0)
                                    .corner_radius(4.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&entry.display_title).strong().size(13.0).color(egui::Color32::WHITE));
                                                    if !entry.timestamp_str.is_empty() {
                                                        ui.colored_label(egui::Color32::from_rgb(100, 200, 255), format!("⏱ {}", entry.timestamp_str));
                                                    }
                                                    ui.colored_label(egui::Color32::from_rgb(140, 220, 160), format!("👤 {}", entry.operator_name));
                                                });

                                                ui.label(
                                                    egui::RichText::new(&entry.technical_summary)
                                                        .size(10.0)
                                                        .color(egui::Color32::from_rgb(200, 215, 230)),
                                                );

                                                if !entry.operator_notes.is_empty() {
                                                    egui::Frame::new()
                                                        .fill(egui::Color32::from_rgb(12, 15, 22))
                                                        .inner_margin(4.0)
                                                        .corner_radius(2.0)
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(format!("📝 Notas: {}", entry.operator_notes))
                                                                    .small()
                                                                    .color(egui::Color32::from_rgb(255, 230, 150)),
                                                            );
                                                        });
                                                }
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.small_button("🗑").on_hover_text("Eliminar los archivos de esta captura").clicked() {
                                                    to_delete_entry = Some(entry.clone());
                                                }

                                                let load_btn = egui::Button::new(
                                                    egui::RichText::new("🎯 CARGAR Y RESTAURAR").strong().color(egui::Color32::WHITE),
                                                )
                                                .fill(egui::Color32::from_rgb(25, 95, 55));

                                                if ui.add(load_btn).on_hover_text("Cargar tramas en pantalla y restaurar exactamente los ajustes de hardware y DSP").clicked() {
                                                    to_load_entry = Some(entry.clone());
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });

                    if let Some(del) = to_delete_entry {
                        picowon_auto::delete_scope_capture_files(&del);
                    }

                    if let Some(entry) = to_load_entry {
                        if let Some(ref json_p) = entry.json_path {
                            load_capture_from_file_path(json_p, link, rec, state);
                        } else if let Some(ref nwc_p) = entry.nwc_path {
                            load_capture_from_file_path(nwc_p, link, rec, state);
                        }
                        state.capture_browser_open = false;
                    }
                }
                1 => {
                    // PESTAÑA DATALOGGER
                    let mut sessions = picowon_auto::scan_datalogger_sessions(&state.datalogger_session.folder);
                    if !state.capture_browser_search.is_empty() {
                        let query = state.capture_browser_search.to_lowercase();
                        sessions.retain(|s| {
                            s.title.to_lowercase().contains(&query)
                                || s.operator_name.to_lowercase().contains(&query)
                                || s.operator_notes.to_lowercase().contains(&query)
                                || s.timestamp_str.contains(&query)
                                || s.stem.to_lowercase().contains(&query)
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("📁 Carpeta: {} ({} sesiones encontradas)", state.datalogger_session.folder, sessions.len())).small().weak());
                    });

                    let mut to_delete_sess: Option<picowon_auto::SavedDataloggerSessionEntry> = None;

                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if sessions.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(20.0);
                                    ui.label(egui::RichText::new("No se encontraron sesiones de datalogger en esta carpeta.").weak());
                                });
                            }

                            for sess in sessions {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(18, 22, 30))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                                    .inner_margin(8.0)
                                    .corner_radius(4.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&sess.title).strong().size(13.0).color(egui::Color32::WHITE));
                                                    if !sess.timestamp_str.is_empty() {
                                                        ui.colored_label(egui::Color32::from_rgb(100, 200, 255), format!("⏱ {}", sess.timestamp_str));
                                                    }
                                                    ui.colored_label(egui::Color32::from_rgb(140, 220, 160), format!("👤 {}", sess.operator_name));
                                                });

                                                let mins = (sess.duration_secs / 60.0).floor() as u32;
                                                let secs = (sess.duration_secs % 60.0).floor() as u32;
                                                ui.label(
                                                    egui::RichText::new(format!("Duración: {mins:02}:{secs:02} · Muestras: {} · Archivo: {}", sess.sample_count, sess.csv_path.display()))
                                                        .size(10.0)
                                                        .color(egui::Color32::from_rgb(200, 215, 230)),
                                                );

                                                if !sess.operator_notes.is_empty() {
                                                    egui::Frame::new()
                                                        .fill(egui::Color32::from_rgb(12, 15, 22))
                                                        .inner_margin(4.0)
                                                        .corner_radius(2.0)
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(format!("📝 {}", sess.operator_notes))
                                                                    .small()
                                                                    .color(egui::Color32::from_rgb(255, 230, 150)),
                                                            );
                                                        });
                                                }
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.small_button("🗑").on_hover_text("Eliminar los archivos de esta sesión").clicked() {
                                                    to_delete_sess = Some(sess.clone());
                                                }

                                                if ui.button("📂 Abrir CSV").on_hover_text("Abrir archivo CSV directamente").clicked() {
                                                    let _ = std::process::Command::new("explorer").arg(&sess.csv_path).spawn();
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });

                    if let Some(del) = to_delete_sess {
                        picowon_auto::delete_datalogger_session_files(&del);
                    }
                }
                _ => {
                    // PESTAÑA TRACER V-I
                    let mut models = state.vi_capture.db.models.clone();
                    if !state.capture_browser_search.is_empty() {
                        let query = state.capture_browser_search.to_lowercase();
                        models.retain(|m| {
                            m.name.to_lowercase().contains(&query)
                                || m.category.to_lowercase().contains(&query)
                                || m.description.to_lowercase().contains(&query)
                                || m.operator_name.to_lowercase().contains(&query)
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("📚 Biblioteca de Modelos Tracer VI ({} firmas)", models.len())).small().weak());
                    });

                    let mut to_delete_id: Option<String> = None;
                    let mut to_load_model: Option<picowon_auto::ViModelEntry> = None;

                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for m in models {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(18, 22, 30))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)))
                                    .inner_margin(8.0)
                                    .corner_radius(4.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&m.name).strong().size(13.0).color(egui::Color32::WHITE));
                                                    ui.colored_label(egui::Color32::from_rgb(100, 200, 255), format!("[{}]", m.category));
                                                    ui.colored_label(egui::Color32::from_rgb(120, 240, 150), format!("Tol: ±{:.1}%", m.tolerance_percent));
                                                });

                                                if !m.description.is_empty() {
                                                    ui.label(egui::RichText::new(&m.description).small().weak());
                                                }

                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "HW: CH1 {:.1}V · CH2 {:.1}V · R_sense {:.0}Ω | Técnico: {} ({})",
                                                        m.ch1_vdiv * m.ch1_probe,
                                                        m.ch2_vdiv * m.ch2_probe,
                                                        m.r_sense_ohms,
                                                        m.operator_name,
                                                        m.created_at
                                                    ))
                                                    .size(9.5)
                                                    .color(egui::Color32::from_gray(150)),
                                                );
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.small_button("🗑").clicked() {
                                                    to_delete_id = Some(m.id.clone());
                                                }

                                                let load_btn = egui::Button::new(
                                                    egui::RichText::new("🎯 CARGAR COMO REF").strong().color(egui::Color32::WHITE),
                                                )
                                                .fill(egui::Color32::from_rgb(25, 95, 55));

                                                if ui.add(load_btn).clicked() {
                                                    to_load_model = Some(m.clone());
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });

                    if let Some(id) = to_delete_id {
                        state.vi_capture.db.remove(&id);
                    }

                    if let Some(entry) = to_load_model {
                        state.vi_tracer.reference_points = Some(entry.points.clone());
                        state.vi_tracer.reference_name = entry.name.clone();
                        state.vi_tracer.tolerance_percent = entry.tolerance_percent;
                        link.config.channels[0].volts_div = entry.ch1_vdiv;
                        link.config.channels[1].volts_div = entry.ch2_vdiv;
                        link.config.channels[0].probe = entry.ch1_probe;
                        link.config.channels[1].probe = entry.ch2_probe;
                        state.vi_tracer.r_sense_ohms = entry.r_sense_ohms as f32;
                        state.vi_tracer.invert_vertical = entry.invert_vertical;
                        state.vi_tracer.burst_count = entry.burst_count;
                        state.vi_tracer.grid_divs_h = entry.grid_divs_h;
                        state.vi_tracer.grid_divs_v = entry.grid_divs_v;
                        link.dirty = true;
                        state.switch_app_mode(AppMode::ViCurveTracer, link);
                        state.vi_tracer.compute_similarity_with_reference();
                        state.banner_message = Some(format!("Firma cargada como referencia: {}", entry.name));
                        state.capture_browser_open = false;
                    }
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cerrar").clicked() {
                    state.capture_browser_open = false;
                }
            });
        });

    if !open {
        state.capture_browser_open = false;
    }
}

/// Dibuja el banner superior con las notas del operario y estado de la captura cargada en pantalla
pub fn show_active_capture_banner(
    ctx: &egui::Context,
    l: &crate::ui::layout::Layout,
    link: &mut crate::Link,
    state: &mut AutomotiveUiState,
) {
    let info = match state.loaded_capture_info {
        Some(ref i) => i.clone(),
        None => return,
    };

    if state.app_mode != AppMode::Oscilloscope {
        return;
    }

    let rect = l.points(crate::ui::layout::Roi::Plot.rect(l));
    let banner_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + 8.0, rect.min.y + 8.0),
        egui::vec2((rect.width() - 16.0).max(200.0), 48.0),
    );

    egui::Area::new("active_capture_banner".into())
        .fixed_pos(banner_rect.min)
        .show(ctx, |ui| {
            ui.set_width(banner_rect.width());
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_premultiplied(16, 26, 42, 240))
                .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 160, 255)))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("📂 REVISANDO CAPTURA HISTÓRICA:").strong().size(11.0).color(egui::Color32::from_rgb(100, 200, 255)));
                                ui.label(egui::RichText::new(&info.title).strong().size(11.5).color(egui::Color32::WHITE));
                                ui.colored_label(egui::Color32::from_rgb(180, 220, 255), format!("({})", info.timestamp));
                                ui.colored_label(egui::Color32::from_rgb(140, 240, 160), format!("👤 Técnico: {}", info.operator));
                            });
                            if !info.notes.is_empty() {
                                ui.label(egui::RichText::new(format!("📝 Notas: {}", info.notes)).small().color(egui::Color32::from_rgb(255, 235, 160)));
                            } else {
                                ui.label(egui::RichText::new(&info.details).small().color(egui::Color32::from_gray(180)));
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").on_hover_text("Cerrar este banner").clicked() {
                                state.loaded_capture_info = None;
                            }

                            let resume_btn = egui::Button::new(
                                egui::RichText::new("▶ REANUDAR SEÑAL EN VIVO").strong().color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(25, 110, 60));

                            if ui.add(resume_btn).on_hover_text("Reanudar la adquisición del osciloscopio en tiempo real").clicked() {
                                link.config.running = true;
                                link.dirty = true;
                                state.loaded_capture_info = None;
                                state.banner_message = Some("Adquisición en tiempo real reanudada.".to_string());
                            }
                        });
                    });
                });
        });
}
