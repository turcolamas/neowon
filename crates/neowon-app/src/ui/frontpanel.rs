//! Left-rail panel (PicoScope 7 & Traditional Scope style):
//! Vertical channel cards CH1/CH2 with compact stepper [◀ V/div ▶],
//! Horizontal timebase & delay card, Trigger controls card (CH1/CH2/EXT),
//! and 2-column customizable Favorite Action buttons with Setup configurator.

use bevy_egui::egui;
use neowon_core::Sweep;

use crate::Link;
use crate::derived::{MathState, MeasureState, fmt_si};
use crate::gpu::Phosphor;

use super::automotive_dock::{AppMode, AutomotiveUiState, FavoriteTool};
use super::layout::{Layout, Roi};
use super::menu::{Menu, MenuState};
use super::widgets::{StepperAction, channel_color, compact_stepper};

#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &egui::Context,
    l: &Layout,
    link: &mut Link,
    _phosphor: &mut Phosphor,
    math: &mut MathState,
    _meas: &mut MeasureState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) -> egui::Rect {
    // Secuencia de arranque seguro y persistencia de canales
    auto.tick_safety_startup(link);

    let rect = l.points(Roi::FrontPanel.rect(l));
    let resp = egui::Area::new("frontpanel".into())
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_clip_rect(rect);
            ui.set_width(rect.width());
            ui.set_max_width(rect.width());
            ui.set_height(rect.height());
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 18, 24))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 48, 60)))
                .inner_margin(egui::Margin::symmetric(2, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_max_width(ui.available_width());
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            ui.spacing_mut().item_spacing.y = 4.0;
                            ui.set_width(ui.available_width());
                            ui.set_max_width(ui.available_width());

                            // 1. CANALES (CH1 y CH2)
                            channel_card(ui, link, 0, channel_color(0), menus, auto);
                            channel_card(ui, link, 1, channel_color(1), menus, auto);

                            // 2. HORIZONTAL (Base de Tiempo y Delay)
                            horizontal_card(ui, link, menus, auto);

                            // 3. TRIGGER (Disparo Interno / Externo)
                            trigger_card(ui, link, menus);

                            // 4. FAVORITOS (Botonera 2 Columnas)
                            favorites_card(ui, link, menus, math, auto);

                            // 5. GENERADOR DE PRUEBA / CALIBRADOR
                            generator_card(ui);
                        });
                });
        });

    // Ventana modal/flotante de setup de favoritos si está abierta
    crate::ui::automotive_dock::show_favorite_setup_window(ctx, auto);

    // Ventana flotante de tratamiento de señal DSP por canal si está abierta
    crate::ui::automotive_dock::show_dsp_floating_window(ctx, link, auto);

    resp.response.rect
}

fn channel_card(
    ui: &mut egui::Ui,
    link: &mut Link,
    ch: usize,
    color: egui::Color32,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) {
    let dsp = auto.dsp[ch];
    let enabled = link.config.channels[ch].enabled;
    let v_div = link.config.channels[ch].volts_div * link.config.channels[ch].probe;
    let probe = link.config.channels[ch].probe;
    let coupling = link.config.channels[ch].coupling;

    egui::Frame::new()
        .fill(if enabled {
            egui::Color32::from_rgb(22, 26, 34)
        } else {
            egui::Color32::from_rgb(18, 20, 25)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if enabled {
                color
            } else {
                egui::Color32::from_gray(45)
            },
        ))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                let name = format!("CH{}", ch + 1);
                let btn = egui::Button::new(
                    egui::RichText::new(name).strong().size(11.0).color(if enabled {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::from_gray(160)
                    }),
                )
                .fill(if enabled {
                    color
                } else {
                    egui::Color32::from_rgb(32, 36, 44)
                })
                .min_size(egui::vec2(38.0, 19.0));
                if ui.add(btn).clicked() {
                    link.config.channels[ch].enabled = !enabled;
                    link.dirty = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Ajustes generales del canal (acoplamiento, ancho de banda)").clicked() {
                        menus.reveal(Menu::Channel(ch));
                    }

                    // Botón DSP que abre/cierra la ventana flotante de DSP
                    let is_dsp_open = auto.dsp_floating_ch == Some(ch);
                    let dsp_active = dsp.lpf_enabled
                        || dsp.res_enhance_bits > 0.0
                        || dsp.invert
                        || dsp.tare_offset_v.abs() > 1e-6;

                    let dsp_text = if dsp.lpf_enabled {
                        format!("🎛 DSP {}", dsp.filter_algorithm.short_name())
                    } else if dsp_active {
                        "🎛 DSP ON".to_string()
                    } else {
                        "🎛 DSP".to_string()
                    };

                    let dsp_color = if is_dsp_open {
                        egui::Color32::from_rgb(255, 215, 0)
                    } else if dsp.lpf_enabled {
                        egui::Color32::from_rgb(100, 240, 160)
                    } else if dsp_active {
                        egui::Color32::from_rgb(100, 200, 255)
                    } else {
                        egui::Color32::from_rgb(200, 215, 230)
                    };

                    let dsp_fill = if is_dsp_open {
                        egui::Color32::from_rgb(45, 42, 25)
                    } else if dsp_active {
                        egui::Color32::from_rgb(25, 45, 38)
                    } else {
                        egui::Color32::from_rgb(34, 40, 52)
                    };

                    let dsp_btn = egui::Button::new(
                        egui::RichText::new(dsp_text).size(10.0).strong().color(dsp_color),
                    )
                    .fill(dsp_fill)
                    .min_size(egui::vec2(52.0, 19.0));

                    if ui
                        .add(dsp_btn)
                        .on_hover_text("Abrir ventana flotante de Procesamiento DSP (Filtros anti-ruido sin distorsión, inversión, resolución y tara)")
                        .clicked()
                    {
                        auto.dsp_floating_ch = if auto.dsp_floating_ch == Some(ch) {
                            None
                        } else {
                            Some(ch)
                        };
                    }
                });
            });

            if enabled {
                ui.add_space(2.0);
                // Control Stepper Compacto unificado [◀ V/div ▶]
                let val_str = format!("{}/div", fmt_si(v_div, "V"));
                let action = compact_stepper(ui, &format!("ch{}_vdiv", ch), &val_str, 20.0);
                match action {
                    StepperAction::Prev => crate::view::zoom_channel(link, ch, true),
                    StepperAction::Next => crate::view::zoom_channel(link, ch, false),
                    StepperAction::ClickedValue => crate::view::zoom_channel(link, ch, false),
                    StepperAction::None => {}
                }

                ui.add_space(2.0);
                // Controles compactos: Acoplamiento, Atenuación y Cero en 1 sola fila
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let coup_str = match coupling {
                        neowon_core::Coupling::Dc => "DC",
                        neowon_core::Coupling::Ac => "AC",
                        neowon_core::Coupling::Gnd => "GND",
                    };
                    let w_btn = ((ui.available_width() - 4.0) / 3.0).floor();
                    let coup_btn = egui::Button::new(
                        egui::RichText::new(coup_str).size(9.5).strong().color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(34, 40, 52))
                    .min_size(egui::vec2(w_btn, 18.0));
                    if ui.add(coup_btn).on_hover_text("Alternar acoplamiento DC / AC").clicked() {
                        link.config.channels[ch].coupling = match coupling {
                            neowon_core::Coupling::Dc => neowon_core::Coupling::Ac,
                            _ => neowon_core::Coupling::Dc,
                        };
                        link.dirty = true;
                    }

                    let probe_str = if probe >= 90.0 {
                        "x100"
                    } else if probe >= 9.0 {
                        "x10"
                    } else {
                        "x1"
                    };
                    let is_attenuated = probe > 1.5;
                    let probe_btn = egui::Button::new(
                        egui::RichText::new(probe_str)
                            .size(9.5)
                            .strong()
                            .color(if is_attenuated {
                                egui::Color32::from_rgb(255, 215, 0)
                            } else {
                                egui::Color32::WHITE
                            }),
                    )
                    .fill(if probe >= 90.0 {
                        egui::Color32::from_rgb(60, 40, 75)
                    } else if probe >= 9.0 {
                        egui::Color32::from_rgb(38, 48, 64)
                    } else {
                        egui::Color32::from_rgb(34, 40, 52)
                    })
                    .min_size(egui::vec2(w_btn, 18.0));
                    if ui.add(probe_btn).on_hover_text("Atenuación de Sonda: x1 / x10 / x100 (conmuta rango de hardware y relés)").clicked() {
                        crate::view::cycle_channel_probe(link, ch);
                    }

                    let offset_v = link.config.channels[ch].offset * v_div * 10.0;
                    let off_str = if offset_v.abs() < 1e-4 {
                        "🎯 0V".to_string()
                    } else {
                        format!("🎯{}", fmt_si(offset_v, "V").replace(' ', ""))
                    };
                    let off_btn = egui::Button::new(
                        egui::RichText::new(off_str)
                            .size(9.0)
                            .strong()
                            .color(egui::Color32::from_rgb(180, 215, 245)),
                    )
                    .fill(egui::Color32::from_rgb(26, 32, 44))
                    .min_size(egui::vec2(w_btn, 18.0));
                    if ui.add(off_btn).on_hover_text("Centrar nivel a 0.0 V").clicked() {
                        link.config.channels[ch].offset = 0.0;
                        link.dirty = true;
                    }
                });
            } else {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Off").size(10.0).weak());
                    if ui.button(egui::RichText::new("+ Activar").size(10.0).color(color)).clicked() {
                        link.config.channels[ch].enabled = true;
                        link.dirty = true;
                    }
                });
            }
        });
}

fn horizontal_card(ui: &mut egui::Ui, link: &mut Link, menus: &mut MenuState, _auto: &mut AutomotiveUiState) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 24, 30))
        .stroke(egui::Stroke::new(1.8, egui::Color32::from_rgb(255, 145, 10)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⏱ HORIZONTAL")
                        .strong()
                        .size(10.5)
                        .color(egui::Color32::from_rgb(255, 165, 30)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Abrir panel horizontal").clicked() {
                        menus.reveal(Menu::Horizontal);
                    }
                });
            });

            ui.add_space(2.0);

            // Stepper Time/div
            let record_len = crate::view::record_len(link);
            let per_div = record_len as f64 / link.config.sample_rate / crate::ui::layout::H_DIVS as f64;
            let tb_str = format!("{}/div", fmt_si(per_div, "s"));
            let action = compact_stepper(ui, "h_timebase", &tb_str, 19.0);
            match action {
                StepperAction::Prev => crate::view::timebase_step(link, true),
                StepperAction::Next => crate::view::timebase_step(link, false),
                StepperAction::ClickedValue => menus.reveal(Menu::Horizontal),
                StepperAction::None => {}
            }

            ui.add_space(2.0);

            // Stepper Delay / Posición Horizontal
            let record_s = record_len as f64 / link.config.sample_rate;
            let delay = (link.config.position - 0.5) * record_s;
            let delay_str = fmt_si(delay, "s");
            let dly_action = compact_stepper(ui, "h_delay", &format!("Dly: {delay_str}"), 19.0);
            match dly_action {
                StepperAction::Prev => {
                    link.config.position = (link.config.position - 0.05).clamp(0.0, 1.0);
                    link.dirty = true;
                }
                StepperAction::Next => {
                    link.config.position = (link.config.position + 0.05).clamp(0.0, 1.0);
                    link.dirty = true;
                }
                StepperAction::ClickedValue => {
                    link.config.position = 0.5;
                    link.dirty = true;
                }
                StepperAction::None => {}
            }
        });
}

fn trigger_card(ui: &mut egui::Ui, link: &mut Link, menus: &mut MenuState) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 24, 30))
        .stroke(egui::Stroke::new(1.8, egui::Color32::from_rgb(245, 65, 65)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚡ DISPARO")
                        .strong()
                        .size(10.0)
                        .color(egui::Color32::from_rgb(255, 115, 105)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Abrir panel de disparo").clicked() {
                        menus.reveal(Menu::Trigger);
                    }
                });
            });

            ui.add_space(2.0);

            // 1. Fuente: C1 / C2 / EXT (dimensionado dinámico exacto)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let src = link.config.trigger.source;
                let btn_w = ((ui.available_width() - 4.0) / 3.0).floor();
                for (s_idx, s_name) in [(0, "C1"), (1, "C2"), (2, "EXT")] {
                    let active = src == s_idx;
                    let btn = egui::Button::new(
                        egui::RichText::new(s_name).size(10.0).strong().color(if active {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_gray(150)
                        }),
                    )
                    .fill(if active {
                        if s_idx == 0 {
                            egui::Color32::from_rgb(180, 140, 20)
                        } else if s_idx == 1 {
                            egui::Color32::from_rgb(25, 120, 180)
                        } else {
                            egui::Color32::from_rgb(100, 100, 120)
                        }
                    } else {
                        egui::Color32::from_rgb(28, 32, 40)
                    })
                    .min_size(egui::vec2(btn_w, 18.0));
                    if ui.add(btn).clicked() {
                        link.config.trigger.source = s_idx;
                        link.dirty = true;
                    }
                }
            });

            ui.add_space(2.0);

            // 2. Modo: Auto / Norm / Single (dimensionado dinámico exacto)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let sweep = link.config.trigger.sweep;
                let btn_w = ((ui.available_width() - 4.0) / 3.0).floor();
                for (sw, label) in [
                    (Sweep::Auto, "Auto"),
                    (Sweep::Normal, "Norm"),
                    (Sweep::Single, "Sngl"),
                ] {
                    let active = sweep == sw;
                    let btn = egui::Button::new(
                        egui::RichText::new(label).size(10.0).color(if active {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_gray(140)
                        }),
                    )
                    .fill(if active {
                        egui::Color32::from_rgb(45, 58, 78)
                    } else {
                        egui::Color32::from_rgb(26, 30, 38)
                    })
                    .min_size(egui::vec2(btn_w, 18.0));
                    if ui.add(btn).clicked() {
                        link.config.trigger.sweep = sw;
                        link.dirty = true;
                    }
                }
            });

            ui.add_space(2.0);

            // 3. Flanco [↗][↘] y Stepper Nivel
            let src = link.config.trigger.source.min(1);
            let slope_rising = matches!(
                link.config.trigger.kind,
                neowon_core::TriggerKind::Edge {
                    slope: neowon_core::Slope::Rising
                }
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let sl_btn = egui::Button::new(if slope_rising { "↗" } else { "↘" })
                    .min_size(egui::vec2(20.0, 20.0))
                    .fill(egui::Color32::from_rgb(32, 38, 50));
                if ui.add(sl_btn).on_hover_text("Cambiar flanco subida/bajada").clicked() {
                    link.config.trigger.kind = if slope_rising {
                        neowon_core::TriggerKind::Edge {
                            slope: neowon_core::Slope::Falling,
                        }
                    } else {
                        neowon_core::TriggerKind::Edge {
                            slope: neowon_core::Slope::Rising,
                        }
                    };
                    link.dirty = true;
                }

                let step = 0.05 * link.config.channels[src].volts_div * link.config.channels[src].probe;
                let lvl_str = fmt_si(link.config.trigger.level, "V");
                let action = compact_stepper(ui, "trig_lvl", &lvl_str, 20.0);
                match action {
                    StepperAction::Prev => {
                        link.config.trigger.level -= step;
                        link.dirty = true;
                    }
                    StepperAction::Next => {
                        link.config.trigger.level += step;
                        link.dirty = true;
                    }
                    StepperAction::ClickedValue => {
                        link.config.trigger.level = 0.0;
                        link.dirty = true;
                    }
                    StepperAction::None => {}
                }
            });
        });
}

fn favorites_card(
    ui: &mut egui::Ui,
    link: &mut Link,
    menus: &mut MenuState,
    math: &mut MathState,
    auto: &mut AutomotiveUiState,
) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 22, 32))
        .stroke(egui::Stroke::new(1.8, egui::Color32::from_rgb(180, 110, 255)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⭐ FAVORITOS")
                        .strong()
                        .size(11.0)
                        .color(egui::Color32::from_rgb(200, 150, 255)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Setup").on_hover_text("Configurar accesos favoritos").clicked() {
                        auto.favorite_setup_open = true;
                    }
                });
            });

            ui.add_space(2.0);

            // Botones de Captura y Carga de Archivos / Gestor de Capturas
            ui.horizontal(|ui| {
                let w = ((ui.available_width() - 4.0) / 2.0).floor();
                let capt_btn = egui::Button::new(
                    egui::RichText::new("📸 CAPTURA")
                        .size(10.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(30, 68, 115))
                .min_size(egui::vec2(w, 22.0));
                if ui.add(capt_btn).on_hover_text("Captura de osciloscopio: guardar tramas (actual, todas o últimas N), imagen PNG, CSV y notas de operario").clicked() {
                    auto.scope_capture.dialog_open = true;
                }

                let load_btn = egui::Button::new(
                    egui::RichText::new("📂 CARGAR")
                        .size(10.0)
                        .strong()
                        .color(egui::Color32::from_rgb(180, 230, 255)),
                )
                .fill(egui::Color32::from_rgb(25, 45, 75))
                .min_size(egui::vec2(w, 22.0));
                if ui.add(load_btn).on_hover_text("Explorador y gestor de capturas: buscar, revisar notas y cargar tramas con sus ajustes originales").clicked() {
                    auto.capture_browser_open = true;
                    auto.capture_browser_tab = 0;
                }
            });

            ui.add_space(3.0);

            // Matriz de 2 columnas con los 8 botones favoritos extendidos
            egui::Grid::new("fav_grid")
                .spacing([3.0, 4.0])
                .show(ui, |ui| {
                    let col_w = ((ui.available_width() - 3.0) / 2.0).floor();
                    for row in 0..4 {
                        for col in 0..2 {
                            let idx = row * 2 + col;
                            let tool = auto.favorite_slots[idx];
                            let (active, label) = match tool {
                                FavoriteTool::Automotive => (menus.is_open(Menu::Automotive), tool.label()),
                                FavoriteTool::Measures => (menus.is_open(Menu::Measure), tool.label()),
                                FavoriteTool::Cursors => (menus.is_open(Menu::Cursor), tool.label()),
                                FavoriteTool::SerialDecode => (menus.is_open(Menu::Decode), tool.label()),
                                FavoriteTool::Datalogger => (auto.app_mode == AppMode::Datalogger, tool.label()),
                                FavoriteTool::ActuatorTest => (auto.app_mode == AppMode::ActuatorPwmVsSensor, tool.label()),
                                FavoriteTool::ViTracer => (auto.app_mode == AppMode::ViCurveTracer, tool.label()),
                                FavoriteTool::Math => (math.enabled, tool.label()),
                            };

                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(10.5)
                                    .strong()
                                    .color(if active { egui::Color32::WHITE } else { egui::Color32::from_rgb(220, 230, 245) }),
                            )
                            .min_size(egui::vec2(col_w, 26.0))
                            .fill(if active {
                                egui::Color32::from_rgb(75, 45, 125)
                            } else {
                                egui::Color32::from_rgb(28, 32, 44)
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if active {
                                    egui::Color32::from_rgb(205, 155, 255)
                                } else {
                                    egui::Color32::from_rgb(52, 60, 78)
                                },
                            ));

                            if ui.add(btn).clicked() {
                                match tool {
                                    FavoriteTool::Automotive => menus.toggle(Menu::Automotive),
                                    FavoriteTool::Measures => menus.toggle(Menu::Measure),
                                    FavoriteTool::Cursors => menus.toggle(Menu::Cursor),
                                    FavoriteTool::SerialDecode => menus.toggle(Menu::Decode),
                                    FavoriteTool::Datalogger => {
                                        let target = if auto.app_mode == AppMode::Datalogger {
                                            AppMode::Oscilloscope
                                        } else {
                                            AppMode::Datalogger
                                        };
                                        auto.switch_app_mode(target, link);
                                    }
                                    FavoriteTool::ActuatorTest => {
                                        let target = if auto.app_mode == AppMode::ActuatorPwmVsSensor {
                                            AppMode::Oscilloscope
                                        } else {
                                            AppMode::ActuatorPwmVsSensor
                                        };
                                        auto.switch_app_mode(target, link);
                                    }
                                    FavoriteTool::ViTracer => {
                                        let target = if auto.app_mode == AppMode::ViCurveTracer {
                                            AppMode::Oscilloscope
                                        } else {
                                            AppMode::ViCurveTracer
                                        };
                                        auto.switch_app_mode(target, link);
                                    }
                                    FavoriteTool::Math => math.enabled = !math.enabled,
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
        });
}

fn generator_card(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 24, 30))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 48, 60)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("GEN")
                        .strong()
                        .size(10.0)
                        .color(egui::Color32::from_rgb(100, 220, 160)),
                );
                ui.label(egui::RichText::new("1 kHz ±1V").size(9.5).weak());
            });
        });
}

pub fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.strong(title);
        add(ui);
    });
}

pub fn key(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let btn = egui::Button::new(egui::RichText::new(label).size(12.0))
        .fill(if active {
            egui::Color32::from_rgb(60, 64, 74)
        } else {
            egui::Color32::from_rgb(28, 30, 36)
        })
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70)))
        .min_size(egui::vec2(58.0, 30.0));
    ui.add(btn).clicked()
}

pub fn color_key(ui: &mut egui::Ui, label: String, color: egui::Color32, active: bool) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(12.0)
            .color(if active { egui::Color32::BLACK } else { color }),
    )
    .fill(if active { color } else { egui::Color32::from_rgb(28, 30, 36) })
    .stroke(egui::Stroke::new(1.0, color))
    .min_size(egui::vec2(48.0, 30.0));
    ui.add(btn).clicked()
}

