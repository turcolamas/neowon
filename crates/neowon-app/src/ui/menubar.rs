//! The application bar: drop-down menus on the left, ambient status on the
//! right. Instrument *function* menus live in the always-visible dock
//! (menu.rs) — these are the app-level ones: files, which views are showing,
//! and settings that are not part of an instrument setup.

use bevy_egui::egui;
use neowon_core::{AcqMode, Sweep};

use crate::Link;
use crate::derived::fmt_si;

use super::layout::{Layout, Roi};
use super::menu::MenuState;
use super::widgets::{ButtonTone, pro_button_widget, RUN_COLOR, STOP_COLOR, WAIT_COLOR};

pub fn run_state(link: &Link, now: f64) -> (&'static str, egui::Color32) {
    if !link.config.running {
        return ("STOP", STOP_COLOR);
    }
    let starved = matches!(link.config.trigger.sweep, Sweep::Normal | Sweep::Single)
        && now - link.last_frame_at > 0.5;
    if starved {
        ("WAIT", WAIT_COLOR)
    } else {
        ("RUN", RUN_COLOR)
    }
}

pub fn show(
    ctx: &egui::Context,
    l: &Layout,
    link: &mut Link,
    now: f64,
    _deep: &crate::deep::DeepView,
    bar: &mut BarState<'_>,
) -> egui::Rect {
    let rect = l.points(Roi::MenuBar.rect(l));
    let resp = egui::Area::new("menubar".into())
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_max_width(rect.width());
            ui.set_height(rect.height());
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 6.0;

                // 0. Botón Principal del Hub de la Suite (F2)
                let launcher_open = bar.auto.suite.launcher_open;
                let suite_btn = pro_button_widget("🏠 SUITE", ButtonTone::Warning, launcher_open, 26.0)
                    .min_size(egui::vec2(74.0, 26.0));
                if ui
                    .add(suite_btn)
                    .on_hover_text("Abrir Hub Principal de PicoWon Automotive Suite (Atajo: F2)")
                    .clicked()
                {
                    bar.auto.suite.launcher_open = !bar.auto.suite.launcher_open;
                }

                // 1. Menús superiores y Ficha del Vehículo
                ui.menu_button("📁 Archivo", |ui| {
                    if ui.button("🏠 Abrir Selector de Suite (F2)…").clicked() {
                        bar.auto.suite.launcher_open = true;
                        ui.close();
                    }
                    if ui.button("🚘 Ficha Técnica del Vehículo…").clicked() {
                        bar.auto.suite.vehicle_dialog_open = true;
                        ui.close();
                    }
                    if ui.button("🔌 Conectar Dispositivo / Modo Simulador Demo…").clicked() {
                        bar.auto.suite.device_dialog_open = true;
                        ui.close();
                    }
                    ui.separator();
                    let dir = crate::record::export_dir();
                    if ui.button("Guardar configuración (.nws)…").clicked() {
                        let p = dir.join("setup.nws");
                        bar.script.inject(crate::script::Action::SessionSave(p.display().to_string()));
                        ui.close();
                    }
                    if ui.button("Cargar configuración (.nws)…").clicked() {
                        let p = dir.join("setup.nws");
                        bar.script.inject(crate::script::Action::SessionLoad(p.display().to_string()));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Guardar captura (.nwc)…").clicked() {
                        let p = dir.join(format!("{}.nwc", crate::record::default_stem()));
                        bar.script.inject(crate::script::Action::CapSave(p.display().to_string()));
                        ui.close();
                    }
                    if ui.button("Exportar señal a CSV…").clicked() {
                        let p = dir.join(format!("{}.csv", crate::record::default_stem()));
                        bar.script.inject(crate::script::Action::Export("csv".into(), p.display().to_string()));
                        ui.close();
                    }
                });

                // Botón de Ficha Técnica del Vehículo Activo (Cap. 10.2)
                let veh_empty = bar.auto.suite.vehicle_store.active.is_empty();
                let veh_label = if veh_empty {
                    "🚘 Vehículo".to_string()
                } else {
                    let v = &bar.auto.suite.vehicle_store.active;
                    let short = format!("{} {}", v.make.trim(), v.model.trim()).trim().to_string();
                    if short.is_empty() {
                        format!("🚘 {}", v.plate_or_vin.trim())
                    } else {
                        format!("🚘 {short}")
                    }
                };
                let veh_btn = pro_button_widget(
                    veh_label,
                    if veh_empty { ButtonTone::Neutral } else { ButtonTone::Success },
                    false,
                    26.0,
                );
                if ui
                    .add(veh_btn)
                    .on_hover_text(format!(
                        "Ficha del Vehículo en Taller: {}\n(Clic para editar datos de vehículo, motor y orden de trabajo)",
                        bar.auto.suite.vehicle_store.active.summary_badge()
                    ))
                    .clicked()
                {
                    bar.auto.suite.vehicle_dialog_open = true;
                }

                // Ayuda siempre visible con margen garantizado
                let help_btn = pro_button_widget("❓ F1", ButtonTone::Neutral, bar.auto.help_window_open, 26.0);
                if ui.add(help_btn).on_hover_text("Guía rápida de navegación con ratón y atajos de teclado (F1)").clicked() {
                    bar.auto.help_window_open = !bar.auto.help_window_open;
                }

                // Selector en caliente de Dispositivo USB / Simulador Demo (Cap. 1.7 Connect Device)
                let (dev_lbl, dev_tone) = if link.is_sim_mode() {
                    (
                        format!("🧪 DEMO ({})", link.stimulus),
                        ButtonTone::Primary,
                    )
                } else if link.caps.is_some() {
                    (
                        "🔌 USB VDS1022".to_string(),
                        ButtonTone::Success,
                    )
                } else {
                    (
                        "🔌 Sin USB (Clic Demo)".to_string(),
                        ButtonTone::Warning,
                    )
                };
                let dev_btn = pro_button_widget(dev_lbl, dev_tone, false, 26.0);
                if ui
                    .add(dev_btn)
                    .on_hover_text(format!(
                        "Estado del Dispositivo: {}\nClic para conmutar en vivo entre Hardware USB OWON VDS1022 y Simulador Demo Automotriz sin reiniciar (Cap. 1.7)",
                        link.status
                    ))
                    .clicked()
                {
                    bar.auto.suite.device_dialog_open = !bar.auto.suite.device_dialog_open;
                }

                ui.separator();

                // 2. Identidad de la Estación Activa (Cada Estación se presenta como su propio Software dedicado)
                let ws = bar.auto.suite.active_workstation;
                let accent = ws.accent_color();
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(22, 26, 36))
                    .stroke(egui::Stroke::new(1.5, accent))
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(ws.card_title())
                                .size(11.0)
                                .strong()
                                .color(accent),
                        );
                    });

                ui.separator();

                // 3. RUN / STOP (Motor común de adquisición en tiempo real)
                let running = link.config.running;
                let (r_label, r_tone) = if !running {
                    ("⏹ STOP", ButtonTone::Danger)
                } else if matches!(link.config.trigger.sweep, Sweep::Normal | Sweep::Single) && now - link.last_frame_at > 0.5 {
                    ("⏳ WAIT", ButtonTone::Warning)
                } else {
                    ("▶ RUN", ButtonTone::Success)
                };
                let r_btn = pro_button_widget(r_label, r_tone, true, 26.0)
                    .min_size(egui::vec2(72.0, 26.0));
                if ui.add(r_btn).on_hover_text("Conmutar RUN / STOP (Espacio)").clicked() {
                    link.config.running = !running;
                    link.dirty = true;
                }

                ui.separator();

                // 4. Barra de herramientas exclusiva de la Estación activa (sin rastros de otras estaciones)
                match ws {
                    crate::ui::suite::SuiteWorkstation::ScopeDatalogger => {
                        let is_dl = bar.auto.strip_chart_view_active;
                        for (lbl, dl_mode) in [("📊 Vista Osciloscopio", false), ("📈 Vista Datalogger", true)] {
                            let sel = is_dl == dl_mode;
                            let b = pro_button_widget(lbl, ButtonTone::Warning, sel, 26.0);
                            if ui.add(b).clicked() {
                                bar.auto.strip_chart_view_active = dl_mode;
                                bar.auto.app_mode = if dl_mode {
                                    crate::ui::automotive_dock::AppMode::Datalogger
                                } else {
                                    crate::ui::automotive_dock::AppMode::Oscilloscope
                                };
                            }
                        }

                        if !is_dl {
                            ui.separator();
                            let autoset_btn = pro_button_widget("⚡ AutoSetup", ButtonTone::Warning, false, 26.0);
                            if ui.add(autoset_btn).on_hover_text("Configuración automática de escalas y disparo (AutoSetup)").clicked() {
                                let _ = link.sup.commands.send(neowon_backend::Command::AutoSet);
                            }

                            let is_peak = link.config.acq == AcqMode::Peak;
                            let glitch_btn = pro_button_widget(
                                if is_peak { "⚡ Glitch: ON" } else { "⚡ Glitch: OFF" },
                                if is_peak { ButtonTone::Warning } else { ButtonTone::Neutral },
                                is_peak,
                                26.0,
                            );
                            if ui.add(glitch_btn).on_hover_text("Detección de Glitches / Transitorios Rápidos (Peak Detect)").clicked() {
                                link.config.acq = if is_peak { AcqMode::Sample } else { AcqMode::Peak };
                                link.dirty = true;
                            }

                            let z_open = bar.auto.zoom_window_open;
                            let z_btn = pro_button_widget("🔍 Zoom", ButtonTone::Primary, z_open, 26.0);
                            if ui.add(z_btn).on_hover_text("Mostrar u ocultar ventana flotante de Zoom").clicked() {
                                bar.auto.zoom_window_open = !bar.auto.zoom_window_open;
                            }

                            let fft_on = bar.fft.enabled;
                            let fft_btn = pro_button_widget("📉 Espectro FFT", ButtonTone::Primary, fft_on, 26.0);
                            if ui.add(fft_btn).on_hover_text("Activar o desactivar analizador de espectro FFT").clicked() {
                                bar.fft.enabled = !bar.fft.enabled;
                            }
                        }
                    }
                    crate::ui::suite::SuiteWorkstation::ViTracer => {
                        let b1 = pro_button_widget("📌 Fijar Golden Sample", ButtonTone::Warning, false, 26.0);
                        if ui.add(b1).clicked() {
                            bar.auto.vi_tracer.capture_reference();
                        }
                        let b2 = pro_button_widget("🗑 Borrar Golden", ButtonTone::Danger, false, 26.0);
                        if ui.add(b2).clicked() {
                            bar.auto.vi_tracer.clear_reference();
                        }
                        let b3 = pro_button_widget("💾 Guardar Curva V-I", ButtonTone::Primary, false, 26.0);
                        if ui.add(b3).clicked() {
                            bar.auto.vi_capture.is_save_dialog_open = true;
                        }
                        let b4 = pro_button_widget("🗄 Biblioteca Firmas V-I", ButtonTone::Neutral, bar.auto.vi_capture.is_load_db_dialog_open, 26.0);
                        if ui.add(b4).clicked() {
                            bar.auto.vi_capture.is_load_db_dialog_open = !bar.auto.vi_capture.is_load_db_dialog_open;
                        }
                    }
                    crate::ui::suite::SuiteWorkstation::CommsMonitor => {
                        for (lbl, lay) in [
                            ("🔀 Vista Combinada", crate::ui::suite::CommsViewLayout::CombinedSplit),
                            ("📈 Solo Onda Física", crate::ui::suite::CommsViewLayout::PhysicalWaveformOnly),
                            ("🌐 Tabla Tráfico CAN/LIN", crate::ui::suite::CommsViewLayout::TrafficTableOnly),
                        ] {
                            let sel = bar.auto.suite.comms_layout == lay;
                            let b = pro_button_widget(lbl, ButtonTone::Success, sel, 26.0);
                            if ui.add(b).clicked() {
                                bar.auto.suite.comms_layout = lay;
                            }
                        }
                        ui.separator();
                        let send_btn = pro_button_widget("🔬 Enviar a Osciloscopio", ButtonTone::Warning, false, 26.0);
                        if ui.add(send_btn).clicked() {
                            crate::ui::suite::transfer_capture_to_scope_workstation(bar.auto, link, bar.math);
                        }
                    }
                    crate::ui::suite::SuiteWorkstation::ActuatorsLoop => {
                        let is_split = bar.auto.suite.actuator_layout
                            == crate::ui::suite::ActuatorViewLayout::SplitCurveAndWaveforms;
                        for (lbl, lay) in [
                            ("🔀 Ondas + Curva X-Y", crate::ui::suite::ActuatorViewLayout::SplitCurveAndWaveforms),
                            ("🎯 Solo Curva X-Y", crate::ui::suite::ActuatorViewLayout::TransferCurveXy),
                        ] {
                            let sel = (lay == crate::ui::suite::ActuatorViewLayout::SplitCurveAndWaveforms) == is_split;
                            let b = pro_button_widget(lbl, ButtonTone::Custom(accent), sel, 26.0);
                            if ui.add(b).clicked() {
                                bar.auto.suite.actuator_layout = lay;
                            }
                        }
                        let clr_btn = pro_button_widget("🗑 Limpiar Curva", ButtonTone::Danger, false, 26.0);
                        if ui.add(clr_btn).clicked() {
                            bar.auto.actuator.clear();
                        }
                        ui.separator();
                        let send_btn = pro_button_widget("🔬 Enviar a Osciloscopio", ButtonTone::Warning, false, 26.0);
                        if ui.add(send_btn).clicked() {
                            crate::ui::suite::transfer_capture_to_scope_workstation(bar.auto, link, bar.math);
                        }
                    }
                    crate::ui::suite::SuiteWorkstation::AutomotiveTests => {
                        let guide_btn = pro_button_widget("📋 Panel Guía y Conexión", ButtonTone::Primary, bar.menus.is_open(crate::ui::Menu::Automotive), 26.0);
                        if ui.add(guide_btn).clicked() {
                            bar.menus.toggle(crate::ui::Menu::Automotive);
                        }
                        let r720 = bar.auto.ruler.enabled;
                        let r_btn = pro_button_widget("📐 Regla 720° 4-Tiempos", ButtonTone::Warning, r720, 26.0);
                        if ui.add(r_btn).clicked() {
                            bar.auto.ruler.enabled = !bar.auto.ruler.enabled;
                        }
                        let autoset_btn = pro_button_widget("⚡ AutoSetup", ButtonTone::Warning, false, 26.0);
                        if ui.add(autoset_btn).clicked() {
                            let _ = link.sup.commands.send(neowon_backend::Command::AutoSet);
                        }
                        ui.separator();
                        let send_btn = pro_button_widget("🔬 Enviar a Osciloscopio", ButtonTone::Warning, false, 26.0);
                        if ui.add(send_btn).clicked() {
                            crate::ui::suite::transfer_capture_to_scope_workstation(bar.auto, link, bar.math);
                        }
                    }
                }
            });
        });
    l.pixels(resp.response.rect)
}

fn badge(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(64.0, 22.0), egui::Sense::hover());
    ui.painter()
        .rect(r, 4.0, color, egui::Stroke::NONE, egui::StrokeKind::Middle);
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        egui::Color32::BLACK,
    );
}

fn sdr_status(
    ui: &mut egui::Ui,
    sdr: &crate::sdr::SdrState,
    caps: Option<&neowon_backend::SdrCaps>,
    refmap: &crate::refmap::RefMap,
    status: &str,
    now: f64,
) {
    let c = &sdr.config;
    if c.running {
        badge(ui, "RUN", RUN_COLOR);
    } else {
        badge(ui, "STOP", STOP_COLOR);
    }
    let gain = match c.gain {
        neowon_backend::SdrGain::Auto => "AGC".to_string(),
        neowon_backend::SdrGain::Manual(db) => format!("{db:.1} dB"),
    };
    ui.label(
        egui::RichText::new(format!(
            "Tuned {:.4} MHz   {}   {gain}",
            sdr.tuned_hz / 1e6,
            fmt_si(c.sample_rate, "S/s")
        ))
        .monospace(),
    );
    // The band the tuned frequency is in (the most specific name), like
    // SDR++'s band overlay, but where it is always in view.
    let bands = refmap.at(sdr.tuned_hz);
    match bands.first() {
        Some(b) => {
            let all: Vec<String> = bands
                .iter()
                .map(|b| {
                    format!(
                        "{} · {} · {}",
                        b.name,
                        crate::refmap::fmt_range(b.lo_hz, b.hi_hz),
                        b.kind
                    )
                })
                .collect();
            ui.label(egui::RichText::new(&b.name).color(crate::refmap::colour(&b.kind)))
                .on_hover_text(format!(
                    "{}\n(band plan: {})",
                    all.join("\n"),
                    refmap.stem()
                ));
        }
        None => {
            ui.weak("no allocation")
                .on_hover_text(format!("band plan: {}", refmap.stem()));
        }
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let n = format!("#{}", sdr.frames_seen);
        ui.label(egui::RichText::new(format!("{n:>8}")).monospace())
            .on_hover_text("IQ frames since the SDR connected.");
        match caps {
            Some(c) => {
                ui.weak(format!("{} · {}", c.name, c.tuner));
                bar_error(ui, status, now);
            }
            None => {
                ui.weak(status);
            }
        };
    });
}

/// How long a refusal stays in the SDR bar after it happened; the log and
/// `get status` keep it after that.
const ERROR_SHOW_S: f64 = 15.0;

/// The home for an action's refusal, in either workspace: with a device
/// connected the bar names the device, not `link.status`, so a failing verb
/// needs a place of its own. Shown while fresh, truncated to the bar with
/// the whole message on hover.
fn bar_error(ui: &mut egui::Ui, status: &str, now: f64) {
    let id = egui::Id::new("sdr-bar-error");
    let seen: Option<(String, f64)> = ui.data(|d| d.get_temp(id));
    let since = match seen {
        Some((text, at)) if text == status => at,
        _ => {
            ui.data_mut(|d| d.insert_temp(id, (status.to_string(), now)));
            now
        }
    };
    let error = status.starts_with("error:") || status.starts_with("disconnected:");
    if !error || now - since > ERROR_SHOW_S {
        return;
    }
    let short: String = if status.chars().count() > 72 {
        status.chars().take(71).chain(['…']).collect()
    } else {
        status.to_string()
    };
    ui.colored_label(ui.visuals().error_fg_color, short)
        .on_hover_text(status);
}

/// What the menus need to reach. Bundled so the bar keeps one parameter.
pub struct BarState<'a> {
    pub settings: &'a mut crate::ui::settings::Settings,
    pub script: &'a mut crate::script::Script,
    pub menus: &'a mut MenuState,
    pub math: &'a mut crate::derived::MathState,
    pub fft: &'a mut crate::derived::FftState,
    pub wf: &'a mut crate::viz::waterfall::WaterfallState,
    pub viz: &'a mut crate::viz::three_d::Viz3dState,
    pub sdr: &'a crate::sdr::SdrState,
    pub refmap: &'a crate::refmap::RefMap,
    pub auto: &'a mut crate::ui::automotive_dock::AutomotiveUiState,
    pub rec: &'a crate::record::Recorder,
    pub hist: &'a mut crate::record::History,
}

/// The workspace switch: SCOPE | SDR, first in the bar, the active one
/// lit. Ctrl/⌘+1 and +2 do the same. One device claim at a time, so a
/// switch releases one instrument and claims the other (`instrument`).
fn mode_toggle(ui: &mut egui::Ui, bar: &mut BarState<'_>) {
    let is_sc = bar.auto.strip_chart_view_active;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (label, active, tip) in [
            ("📊 OSCILOSCOPIO", !is_sc, "Modo Osciloscopio automotriz en tiempo real"),
            ("📈 STRIP-CHART", is_sc, "Modo Registrador continuo Strip-Chart y Datalogger automotriz"),
        ] {
            let text = egui::RichText::new(label).strong().color(if active {
                egui::Color32::BLACK
            } else {
                egui::Color32::from_gray(210)
            });
            let b = egui::Button::new(text)
                .fill(if active {
                    egui::Color32::from_rgb(255, 175, 40)
                } else {
                    egui::Color32::from_rgb(32, 36, 44)
                })
                .min_size(egui::vec2(130.0, 24.0));
            if ui.add(b).on_hover_text(tip).clicked() {
                bar.auto.strip_chart_view_active = label.contains("STRIP");
            }
        }
    });
}

/// The drop-downs. Every item routes through a script action where one
/// exists, so the menu and a script take the same path.
fn menus(ui: &mut egui::Ui, bar: &mut BarState<'_>) {
    use crate::script::Action;
    // The menu buttons go straight into the bar's existing row: an
    // `egui::MenuBar` claims the full available width, which pushes the
    // status readouts onto a second row that the fixed-height bar clips.
    {
        mode_toggle(ui, bar);
        ui.separator();
        if ui.button(egui::RichText::new("❓ Ayuda (F1)").color(egui::Color32::from_rgb(100, 200, 255))).on_hover_text("Guía rápida de navegación con ratón y atajos de teclado (F1)").clicked() {
            bar.auto.help_window_open = !bar.auto.help_window_open;
        }
        ui.menu_button("🚗 Automotriz", |ui| {
            if ui.button("❓ Guía de Navegación y Atajos (F1)").clicked() {
                bar.auto.help_window_open = true;
                ui.close();
            }
            if ui.button("📋 Abrir Panel Automotriz").clicked() {
                bar.menus.reveal(crate::ui::Menu::Automotive);
                ui.close();
            }
            ui.separator();
            ui.label(egui::RichText::new("Categorías de Prueba:").strong().color(egui::Color32::from_rgb(255, 170, 0)));
            for cat in [
                picowon_auto::PresetCategory::Ignition,
                picowon_auto::PresetCategory::Injection,
                picowon_auto::PresetCategory::Sensors,
                picowon_auto::PresetCategory::Actuators,
                picowon_auto::PresetCategory::Networks,
                picowon_auto::PresetCategory::StartingCharging,
            ] {
                ui.menu_button(cat.label(), |ui| {
                    let presets = picowon_auto::get_all_presets();
                    for p in presets.iter().filter(|p| p.category == cat) {
                        if ui.button(p.name).on_hover_text(p.description).clicked() {
                            bar.menus.reveal(crate::ui::Menu::Automotive);
                            ui.close();
                        }
                    }
                });
            }
        });
        ui.menu_button("File", |ui| {
            let dir = crate::record::export_dir();
            if ui.button("Save setup…").clicked() {
                let p = dir.join("setup.nws");
                bar.script
                    .inject(Action::SessionSave(p.display().to_string()));
                ui.close();
            }
            if ui.button("Load setup").clicked() {
                let p = dir.join("setup.nws");
                bar.script
                    .inject(Action::SessionLoad(p.display().to_string()));
                ui.close();
            }
            ui.separator();
            if ui.button("Save capture (.nwc)").clicked() {
                let p = dir.join(format!("{}.nwc", crate::record::default_stem()));
                bar.script.inject(Action::CapSave(p.display().to_string()));
                ui.close();
            }
            if ui.button("Export WAV").clicked() {
                let p = dir.join(format!("{}.wav", crate::record::default_stem()));
                bar.script
                    .inject(Action::Export("wav".into(), p.display().to_string()));
                ui.close();
            }
            if ui.button("Export CSV").clicked() {
                let p = dir.join(format!("{}.csv", crate::record::default_stem()));
                bar.script
                    .inject(Action::Export("csv".into(), p.display().to_string()));
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            if bar.sdr.active {
                return sdr_view_menu(ui, bar);
            }
            ui.checkbox(&mut bar.fft.enabled, "Spectrum");
            ui.checkbox(&mut bar.wf.on, "Waterfall");
            let mut viz_on = bar.viz.mode != crate::viz::three_d::Viz3d::Off;
            if ui.checkbox(&mut viz_on, "3D viewport").changed() {
                bar.viz.mode = if viz_on {
                    crate::viz::three_d::Viz3d::Terrain
                } else {
                    crate::viz::three_d::Viz3d::Off
                };
            }
            ui.separator();
            for (label, m) in [
                ("Measurements", crate::ui::Menu::Measure),
                ("Cursors", crate::ui::Menu::Cursor),
                ("Decode", crate::ui::Menu::Decode),
                ("Record / Export", crate::ui::Menu::Record),
            ] {
                let mut on = bar.menus.is_open(m);
                if ui.checkbox(&mut on, label).changed() {
                    bar.menus.toggle(m);
                }
            }
        });
        if ui.button("Settings").clicked() {
            bar.settings.open = !bar.settings.open;
        }
    }
}

fn sdr_view_menu(ui: &mut egui::Ui, bar: &mut BarState<'_>) {
    for a in sdr_views(ui, bar.refmap, bar.sdr) {
        bar.script.inject(a);
    }
    sdr_view_plan(ui, bar);
}

/// The View menu's toggles, returned as the script actions they stand for
/// (script parity): the RF reference views, and the DAB receiver —
/// `sdr dab on|off`, which opens the dock's DAB section and scrolls it into
/// view, so DAB has a door outside the dock.
fn sdr_views(
    ui: &mut egui::Ui,
    rm: &crate::refmap::RefMap,
    sdr: &crate::sdr::SdrState,
) -> Vec<crate::script::Action> {
    use crate::refmap::RefMapAction as R;
    use crate::script::Action;
    use crate::sdr::{DabVerb, SdrAction};
    let mut out = Vec::new();
    for (label, on, act) in [
        ("RF map", rm.window, R::Window as fn(bool) -> R),
        ("Band strip", rm.strip, R::Strip),
        ("Minimap", rm.mini, R::Mini),
    ] {
        let mut v = on;
        if ui.checkbox(&mut v, label).changed() {
            out.push(Action::RefMap(act(v)));
        }
    }
    let mut dab = sdr.dab.on();
    if ui
        .checkbox(&mut dab, "DAB receiver")
        .on_hover_text("sdr dab on|off - decode the Band III ensemble at the hardware centre")
        .changed()
    {
        out.push(Action::Sdr(SdrAction::Dab(if dab {
            DabVerb::On
        } else {
            DabVerb::Off
        })));
    }
    out
}

fn sdr_view_plan(ui: &mut egui::Ui, bar: &mut BarState<'_>) {
    use crate::refmap::RefMapAction as R;
    use crate::script::Action;
    let rm = bar.refmap;
    ui.menu_button(format!("Band plan: {}", rm.stem()), |ui| {
        for (stem, plan) in &rm.plans {
            if ui
                .radio(
                    rm.stem() == stem,
                    format!("{stem}  ({})", plan.country_name),
                )
                .clicked()
            {
                bar.script.inject(Action::RefMap(R::Plan(stem.clone())));
                ui.close();
            }
        }
    });
    ui.separator();
    if ui.button("Catalog").clicked() {
        bar.script
            .inject(Action::Catalog(crate::catalog::CatalogAction::Window(true)));
        ui.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_egui::egui::accesskit::{self, ActionRequest, NodeId, Role, TreeId, TreeUpdate};

    /// The first node with this role and label, from one frame's AccessKit
    /// update — the tree `get uitree` renders.
    fn find(tree: &TreeUpdate, role: Role, label: &str) -> Option<(NodeId, Option<bool>)> {
        tree.nodes
            .iter()
            .find(|(_, n)| n.role() == role && n.label() == Some(label))
            .map(|(id, n)| (*id, n.toggled().map(|t| t == accesskit::Toggled::True)))
    }

    fn click(id: NodeId) -> Vec<egui::Event> {
        vec![egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::Click,
            target_tree: TreeId::ROOT,
            target_node: id,
            data: None,
        })]
    }

    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        rm: &crate::refmap::RefMap,
        sdr: &crate::sdr::SdrState,
    ) -> (Vec<String>, TreeUpdate) {
        let raw = egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        let mut acts = Vec::new();
        let out = ctx.run_ui(raw, |ui| {
            ui.menu_button("View", |ui| acts = sdr_views(ui, rm, sdr));
        });
        let tree = out.platform_output.accesskit_update.expect("accesskit on");
        let acts = acts
            .iter()
            .map(|a| match a {
                crate::script::Action::Sdr(a) => a.to_string(),
                other => format!("{other:?}"),
            })
            .collect();
        (acts, tree)
    }

    /// DAB has a door outside the dock. The SDR View menu carries a
    /// "DAB receiver" toggle that shows the receiver's state and stands for
    /// `sdr dab on|off` — the script action, so the menu and a script take
    /// the same path (script parity).
    #[test]
    fn the_sdr_view_menu_has_a_dab_receiver_toggle_bound_to_its_script_action() {
        let rm = crate::refmap::RefMap::shipped_only();
        for on in [false, true] {
            let mut sdr = crate::sdr::SdrState::default();
            if on {
                sdr.dab.rx = Some(neowon_dsp::dab::DabReceiver::new());
            }
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let (_, tree) = frame(&ctx, Vec::new(), &rm, &sdr);
            let (view, _) = find(&tree, Role::Button, "View").expect("View menu button");
            let (_, tree) = frame(&ctx, click(view), &rm, &sdr);
            let entry = find(&tree, Role::CheckBox, "DAB receiver").or_else(|| {
                find(
                    &frame(&ctx, Vec::new(), &rm, &sdr).1,
                    Role::CheckBox,
                    "DAB receiver",
                )
            });
            let (entry, toggled) = entry.expect("View menu has no DAB receiver entry");
            assert_eq!(toggled, Some(on), "the entry shows the receiver's state");
            let (acts, _) = frame(&ctx, click(entry), &rm, &sdr);
            let want = if on { "sdr dab off" } else { "sdr dab on" };
            assert_eq!(acts, vec![want.to_string()], "the entry's script action");
        }
    }
}
