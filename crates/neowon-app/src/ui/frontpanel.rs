//! Left-rail panel (PicoScope 7 & Traditional Scope style):
//! Vertical channel cards CH1/CH2 with compact stepper [- V/div +],
//! Horizontal timebase & delay card, Trigger controls card (CH1/CH2/EXT),
//! and 2-column customizable Favorite Action buttons with Setup configurator.

use bevy_egui::egui;
use neowon_core::Sweep;

use crate::Link;
use crate::derived::{MathState, MeasureState, fmt_si};
use crate::gpu::Phosphor;

use super::automotive_dock::AutomotiveUiState;
use super::layout::{Layout, Roi};
use super::menu::{Menu, MenuState};
use super::widgets::{ButtonTone, pro_button_widget, StepperAction, channel_color, compact_stepper};

#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &egui::Context,
    l: &Layout,
    link: &mut Link,
    _phosphor: &mut Phosphor,
    math: &mut MathState,
    meas: &mut MeasureState,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) -> egui::Rect {
    // Secuencia de arranque seguro y persistencia de canales
    auto.tick_safety_startup(link);

    // Fase 1 (Cap. 2.1): Escala Vertical Automática Segura (Auto-Range) por canal si está activa
    if !auto.safety.active && link.config.running {
        for ch in 0..2 {
            if auto.ch_ext[ch].auto_range && link.config.channels[ch].enabled {
                if let Some(m) = meas.latest.get(ch).and_then(|m| *m) {
                    let cur_vdiv = link.config.channels[ch].volts_div;
                    let factor = link.config.channels[ch].probe;
                    let next_vdiv = picowon_auto::compute_safe_auto_vdiv(m.vmin, m.vmax, factor, cur_vdiv);
                    if (next_vdiv - cur_vdiv).abs() > 1e-4 {
                        link.config.channels[ch].volts_div = next_vdiv;
                        link.dirty = true;
                    }
                }
            }
        }
    }

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

                            // 0. INDICADOR DE ESTACIÓN DE LA SUITE ACTIVA
                            workstation_header_card(ui, auto);

                            // 1. CANALES (CH1 y CH2 — Base común con Sondas, Transductores y Unidades Físicas)
                            channel_card(ui, link, meas, 0, channel_color(0), menus, auto);
                            channel_card(ui, link, meas, 1, channel_color(1), menus, auto);

                            if auto.suite.active_workstation == crate::ui::suite::SuiteWorkstation::ViTracer {
                                // En Estación Trazador V-I (X-Y puro): sin Base de Tiempo ni Trigger de Osciloscopio
                                vi_tracer_sidebar_card(ui, link, auto);
                            } else {
                                // 2. HORIZONTAL (Base de Tiempo y Delay)
                                horizontal_card(ui, link, menus, auto);

                                // 3. TRIGGER (Disparo Interno / Externo + Histéresis Fase 1)
                                trigger_card(ui, link, menus, meas, auto);

                                // 4. PANEL DE CONTROLES EXCLUSIVO DE LA ESTACIÓN ACTIVA (Sin rastros de otras estaciones)
                                workstation_specific_action_card(ui, link, menus, math, auto);

                                // 5. GENERADOR DE PRUEBA / CALIBRADOR (Solo en Osciloscopio y Testeos)
                                if matches!(
                                    auto.suite.active_workstation,
                                    crate::ui::suite::SuiteWorkstation::ScopeDatalogger
                                        | crate::ui::suite::SuiteWorkstation::AutomotiveTests
                                ) {
                                    generator_card(ui);
                                }
                            }
                        });
                });
        });

    // Ventana flotante de tratamiento de señal DSP por canal si está abierta
    crate::ui::automotive_dock::show_dsp_floating_window(ctx, link, auto);

    // Ventana flotante de Fase 1: Sondas, Pinzas, Transductores y Unidades Físicas por canal
    crate::ui::suite::show_probe_transducer_window(ctx, link, auto);

    resp.response.rect
}

fn inactive_channel_badge_colors(ch: usize) -> (egui::Color32, egui::Color32, egui::Color32) {
    // Amarillo / ciánico muy suave, casi gris, para distinguir el canal cuando está desactivado
    match ch {
        0 => (
            egui::Color32::from_rgb(118, 114, 96),
            egui::Color32::from_rgb(142, 138, 118),
            egui::Color32::from_rgb(28, 30, 34),
        ),
        _ => (
            egui::Color32::from_rgb(92, 112, 122),
            egui::Color32::from_rgb(116, 134, 144),
            egui::Color32::from_rgb(28, 30, 34),
        ),
    }
}

fn channel_card(
    ui: &mut egui::Ui,
    link: &mut Link,
    meas: &MeasureState,
    ch: usize,
    color: egui::Color32,
    menus: &mut MenuState,
    auto: &mut AutomotiveUiState,
) {
    let dsp = auto.dsp[ch];
    let enabled = link.config.channels[ch].enabled;
    let bnc_vdiv = link.config.channels[ch].volts_div;
    let coupling = link.config.channels[ch].coupling;
    let probe_spec = auto.ch_ext[ch].probe_spec.clone();
    let alias = auto.ch_ext[ch].alias.trim().to_string();
    let auto_range = auto.ch_ext[ch].auto_range;
    let freq_counter = auto.ch_ext[ch].freq_counter;

    egui::Frame::new()
        .fill(if enabled {
            egui::Color32::from_rgb(22, 26, 34)
        } else {
            egui::Color32::from_rgb(48, 52, 60)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if enabled {
                color
            } else {
                egui::Color32::from_rgb(90, 96, 108)
            },
        ))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                let name = if alias.is_empty() {
                    format!("CH{}", ch + 1)
                } else {
                    format!("CH{}·{}", ch + 1, alias)
                };
                let tone = if enabled {
                    if ch == 0 { ButtonTone::Ch1 } else { ButtonTone::Ch2 }
                } else {
                    ButtonTone::Neutral
                };
                let btn = pro_button_widget(name, tone, enabled, 22.0)
                    .min_size(egui::vec2(40.0, 22.0))
                    .wrap_mode(egui::TextWrapMode::Truncate);
                let tooltip = if enabled {
                    format!("Desactivar canal CH{}", ch + 1)
                } else {
                    format!("Activar canal CH{}", ch + 1)
                };
                if ui.add(btn).on_hover_text(tooltip).clicked() {
                    link.config.channels[ch].enabled = !enabled;
                    link.dirty = true;
                }
                if enabled {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ch_open = menus.is_open(Menu::Channel(ch));
                        let gear_btn = pro_button_widget("⚙", ButtonTone::Neutral, ch_open, 22.0)
                            .min_size(egui::vec2(22.0, 22.0));
                        if ui
                            .add(gear_btn)
                            .on_hover_ui(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(if ch == 0 { "🅰" } else { "🅱" }).size(16.0));
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(format!("Ajustes Canal {}", ch + 1)).strong().size(11.0).color(egui::Color32::from_rgb(108, 185, 245)));
                                        ui.label(egui::RichText::new("Acoplamiento (AC/DC/GND), atenuación, sondas y unidades").size(10.0).color(egui::Color32::from_gray(215)));
                                    });
                                });
                            })
                            .clicked()
                        {
                            menus.toggle(Menu::Channel(ch));
                        }

                        // Botón Auto-Range (Cap. 2.1)
                        let auto_btn = pro_button_widget("Auto", ButtonTone::Success, auto_range, 22.0)
                            .min_size(egui::vec2(32.0, 22.0));
                        if ui
                            .add(auto_btn)
                            .on_hover_text("Escala Vertical Automática Segura (Auto-Range)")
                            .clicked()
                        {
                            auto.ch_ext[ch].auto_range = !auto_range;
                        }

                        // Botón DSP que abre/cierra la ventana flotante de DSP
                        let is_dsp_open = auto.dsp_floating_ch == Some(ch);
                        let dsp_active = dsp.lpf_enabled
                            || dsp.res_enhance_bits > 0.0
                            || dsp.invert
                            || dsp.tare_offset_v.abs() > 1e-6;

                        let dsp_text = if dsp.lpf_enabled {
                            format!("🎛{}", dsp.filter_algorithm.short_name())
                        } else if dsp_active {
                            "🎛ON".to_string()
                        } else {
                            "🎛DSP".to_string()
                        };

                        let dsp_tone = if is_dsp_open {
                            ButtonTone::Warning
                        } else if dsp_active {
                            ButtonTone::Primary
                        } else {
                            ButtonTone::Neutral
                        };

                        let dsp_btn = pro_button_widget(dsp_text, dsp_tone, is_dsp_open, 22.0)
                            .min_size(egui::vec2(44.0, 22.0));

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
                }
            });

            if enabled {
                ui.add_space(2.0);
                // Control Stepper Compacto unificado [- Unidad/div +] usando ProbeSpec (V, A, bar, psi, kPa, °C, kV)
                let val_str = probe_spec.format_per_div(bnc_vdiv);
                let action = compact_stepper(ui, &format!("ch{}_vdiv", ch), &val_str, 22.0);
                match action {
                    StepperAction::Prev => {
                        auto.ch_ext[ch].auto_range = false;
                        crate::view::zoom_channel(link, ch, true);
                    }
                    StepperAction::Next => {
                        auto.ch_ext[ch].auto_range = false;
                        crate::view::zoom_channel(link, ch, false);
                    }
                    StepperAction::ClickedValue => {
                        auto.probe_dialog_ch = Some(ch);
                    }
                    StepperAction::None => {}
                }

                ui.add_space(2.0);
                // Controles compactos: Acoplamiento, Sonda/Transductor y Cero en 1 sola fila
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let coup_str = match coupling {
                        neowon_core::Coupling::Dc => "DC",
                        neowon_core::Coupling::Ac => "AC",
                        neowon_core::Coupling::Gnd => "GND",
                    };
                    let w_btn = ((ui.available_width() - 4.0) / 3.0).floor();
                    let coup_btn = pro_button_widget(coup_str, ButtonTone::Neutral, false, 20.0)
                        .min_size(egui::vec2(w_btn, 20.0));
                    if ui.add(coup_btn).on_hover_text("Alternar acoplamiento DC / AC").clicked() {
                        link.config.channels[ch].coupling = match coupling {
                            neowon_core::Coupling::Dc => neowon_core::Coupling::Ac,
                            _ => neowon_core::Coupling::Dc,
                        };
                        link.dirty = true;
                    }

                    // Botón de Sonda / Transductor (Cap. 2.1 / Cap. 12.6)
                    let probe_str = format!("🔌{}", probe_spec.short_badge);
                    let is_special = probe_spec.id != "x1_direct";
                    let probe_tone = if is_special { ButtonTone::Warning } else { ButtonTone::Neutral };
                    let probe_btn = pro_button_widget(probe_str, probe_tone, is_special, 20.0)
                        .min_size(egui::vec2(w_btn, 20.0));
                    let p_resp = ui.add(probe_btn).on_hover_text(format!(
                        "Sonda/Transductor activo: {} ({})\n• Clic izquierdo: Abrir biblioteca de sondas, pinzas amperimétricas y transductores\n• Clic derecho: Alternar rápido x1 / x10 / x100",
                        probe_spec.name,
                        probe_spec.unit.symbol()
                    ));
                    if p_resp.clicked() {
                        auto.probe_dialog_ch = if auto.probe_dialog_ch == Some(ch) {
                            None
                        } else {
                            Some(ch)
                        };
                    } else if p_resp.secondary_clicked() {
                        crate::view::cycle_channel_probe(link, ch);
                        let p_val = link.config.channels[ch].probe;
                        auto.ch_ext[ch].probe_spec = if p_val >= 90.0 {
                            picowon_auto::ProbeSpec::probe_x100()
                        } else if p_val >= 9.0 {
                            picowon_auto::ProbeSpec::probe_x10()
                        } else {
                            picowon_auto::ProbeSpec::probe_x1()
                        };
                    }

                    let v_div_scaled = bnc_vdiv * link.config.channels[ch].probe;
                    let offset_v = link.config.channels[ch].offset * v_div_scaled * 10.0;
                    let off_str = if offset_v.abs() < 1e-4 {
                        format!("🎯 0{}", probe_spec.unit.symbol())
                    } else {
                        format!("🎯{}", probe_spec.unit.format_value(offset_v).replace(' ', ""))
                    };
                    let off_btn = pro_button_widget(off_str, ButtonTone::Neutral, false, 20.0)
                        .min_size(egui::vec2(w_btn, 20.0));
                    if ui.add(off_btn).on_hover_text("Centrar nivel a 0").clicked() {
                        link.config.channels[ch].offset = 0.0;
                        link.dirty = true;
                    }
                });

                // Si el contador de frecuencia dedicado por canal está activo, mostrar lectura en vivo + RPM
                if freq_counter {
                    let f_hz = link
                        .latest
                        .as_ref()
                        .and_then(|f| f.channels.iter().find(|c| c.ch == ch))
                        .and_then(|c| c.freq_meter)
                        .or_else(|| meas.latest.get(ch).and_then(|m| *m).and_then(|m| m.freq));
                    let f_txt = match f_hz {
                        Some(hz) if hz > 0.1 => {
                            let rpm = hz * 60.0;
                            format!("⏱ {} ({:.0} RPM)", fmt_si(hz, "Hz"), rpm)
                        }
                        _ => "⏱ Sin frecuencia detectada".to_string(),
                    };
                    ui.label(
                        egui::RichText::new(f_txt)
                            .size(9.0)
                            .monospace()
                            .color(egui::Color32::from_rgb(120, 235, 175)),
                    );
                }
            } else {
                // Mantener la misma altura y ancho del cuadro activo en gris suave indicando inactividad
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 51.0),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.label(
                            egui::RichText::new("No disponible")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(138, 144, 154)),
                        );
                    },
                );
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
                    let h_open = menus.is_open(Menu::Horizontal);
                    let gear_btn = pro_button_widget("⚙", ButtonTone::Neutral, h_open, 22.0)
                        .min_size(egui::vec2(22.0, 22.0));
                    if ui
                        .add(gear_btn)
                        .on_hover_ui(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⏱").size(16.0));
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Ajustes Horizontales").strong().size(11.0).color(egui::Color32::from_rgb(108, 185, 245)));
                                    ui.label(egui::RichText::new("Base de tiempo, delay, frecuencia de muestreo y escala").size(10.0).color(egui::Color32::from_gray(215)));
                                });
                            });
                        })
                        .clicked()
                    {
                        menus.toggle(Menu::Horizontal);
                    }
                });
            });

            ui.add_space(2.0);

            // Stepper Time/div
            let record_len = crate::view::record_len(link);
            let per_div = record_len as f64 / link.config.sample_rate / crate::ui::layout::H_DIVS as f64;
            let tb_str = format!("{}/div", fmt_si(per_div, "s"));
            let action = compact_stepper(ui, "h_timebase", &tb_str, 22.0);
            match action {
                StepperAction::Prev => crate::view::timebase_step(link, false),
                StepperAction::Next => crate::view::timebase_step(link, true),
                StepperAction::ClickedValue => menus.reveal(Menu::Horizontal),
                StepperAction::None => {}
            }

            ui.add_space(2.0);

            // Stepper Delay / Posición Horizontal
            let record_s = record_len as f64 / link.config.sample_rate;
            let delay = (link.config.position - 0.5) * record_s;
            let delay_str = fmt_si(delay, "s");
            let dly_action = compact_stepper(ui, "h_delay", &format!("Dly: {delay_str}"), 22.0);
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

fn trigger_50_percent_level(link: &Link, meas: &MeasureState) -> Option<f64> {
    let src = link.config.trigger.source;
    if src >= 2 || !link.config.channels[src].enabled {
        return None;
    }
    if let Some(m) = meas.latest.get(src).and_then(|m| *m) {
        return Some(0.5 * (m.vmin + m.vmax));
    }
    let cap = link
        .latest
        .as_ref()
        .and_then(|f| f.channels.iter().find(|c| c.ch == src))?;
    let (&first, rest) = cap.data.split_first()?;
    let (mn, mx) = rest
        .iter()
        .fold((first, first), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let vmin = mn as f64 * cap.cal.scale_i + cap.cal.offset_i;
    let vmax = mx as f64 * cap.cal.scale_i + cap.cal.offset_i;
    Some(0.5 * (vmin + vmax))
}

fn trigger_card(
    ui: &mut egui::Ui,
    link: &mut Link,
    menus: &mut MenuState,
    meas: &MeasureState,
    auto: &mut AutomotiveUiState,
) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 24, 30))
        .stroke(egui::Stroke::new(1.8, egui::Color32::from_rgb(245, 65, 65)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚡ TRIGGER")
                        .strong()
                        .size(10.0)
                        .color(egui::Color32::from_rgb(255, 115, 105)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let t_open = menus.is_open(Menu::Trigger);
                    let gear_btn = pro_button_widget("⚙", ButtonTone::Neutral, t_open, 22.0)
                        .min_size(egui::vec2(22.0, 22.0));
                    if ui
                        .add(gear_btn)
                        .on_hover_ui(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⚡").size(16.0));
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Ajustes de Trigger").strong().size(11.0).color(egui::Color32::from_rgb(108, 185, 245)));
                                    ui.label(egui::RichText::new("Modo de disparo, nivel de umbral, flanco e histéresis").size(10.0).color(egui::Color32::from_gray(215)));
                                });
                            });
                        })
                        .clicked()
                    {
                        menus.toggle(Menu::Trigger);
                    }
                    let target_50 = trigger_50_percent_level(link, meas);
                    let btn_50 = pro_button_widget("50%", ButtonTone::Warning, false, 20.0)
                        .min_size(egui::vec2(32.0, 20.0));
                    let resp_50 = ui
                        .add_enabled(target_50.is_some(), btn_50)
                        .on_hover_text("Ajustar nivel de trigger al 50% de la señal presente en el canal sensado")
                        .on_disabled_hover_text("Sin señal disponible en el canal sensado");
                    if resp_50.clicked() {
                        if let Some(lvl) = target_50 {
                            link.config.trigger.level = lvl;
                            link.dirty = true;
                        }
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
                    let tone = match s_idx {
                        0 => ButtonTone::Ch1,
                        1 => ButtonTone::Ch2,
                        _ => ButtonTone::Neutral,
                    };
                    let btn = pro_button_widget(s_name, tone, active, 20.0)
                        .min_size(egui::vec2(btn_w, 20.0));
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
                    let tone = if sw == Sweep::Single {
                        ButtonTone::Warning
                    } else {
                        ButtonTone::Primary
                    };
                    let btn = pro_button_widget(label, tone, active, 20.0)
                        .min_size(egui::vec2(btn_w, 20.0));
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
                let sl_btn = pro_button_widget(if slope_rising { "↗" } else { "↘" }, ButtonTone::Neutral, false, 22.0)
                    .min_size(egui::vec2(24.0, 22.0));
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
                let lvl_str = auto.ch_ext[src].probe_spec.format_scaled_value(link.config.trigger.level);
                let action = compact_stepper(ui, "trig_lvl", &lvl_str, 22.0);
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

            ui.add_space(2.0);
            // 4. Histéresis de Disparo Configurable en Hardware FPGA (Cap. 4.11)
            let hyst_str = format!("Hyst: {:.1}%", link.config.trigger.hysteresis_pct);
            let h_act = compact_stepper(ui, "trig_hyst", &hyst_str, 20.0);
            match h_act {
                StepperAction::Prev => {
                    let h = (link.config.trigger.hysteresis_pct - 0.5).clamp(0.5, 15.0);
                    link.config.trigger.hysteresis_pct = h;
                    auto.trigger_hysteresis_pct = h;
                    link.dirty = true;
                }
                StepperAction::Next => {
                    let h = (link.config.trigger.hysteresis_pct + 0.5).clamp(0.5, 15.0);
                    link.config.trigger.hysteresis_pct = h;
                    auto.trigger_hysteresis_pct = h;
                    link.dirty = true;
                }
                StepperAction::ClickedValue => {
                    link.config.trigger.hysteresis_pct = 1.5;
                    auto.trigger_hysteresis_pct = 1.5;
                    link.dirty = true;
                }
                StepperAction::None => {}
            }

            ui.add_space(2.0);
            // 5. Calificador de Disparo Automotriz Avanzado (Fase 2 — Cap. 4.3 a 4.8)
            let q_mode = auto.trigger_qualifier.mode;
            let q_str = format!("Modo: {}", q_mode.short_badge());
            let q_act = compact_stepper(ui, "trig_qual_mode", &q_str, 20.0);
            let all_modes = picowon_auto::AdvancedTriggerMode::ALL;
            let cur_idx = all_modes.iter().position(|&m| m == q_mode).unwrap_or(0);
            match q_act {
                StepperAction::Prev => {
                    let next_idx = if cur_idx == 0 {
                        all_modes.len() - 1
                    } else {
                        cur_idx - 1
                    };
                    auto.trigger_qualifier.mode = all_modes[next_idx];
                    auto.trigger_qualifier_rt = Default::default();
                }
                StepperAction::Next => {
                    let next_idx = (cur_idx + 1) % all_modes.len();
                    auto.trigger_qualifier.mode = all_modes[next_idx];
                    auto.trigger_qualifier_rt = Default::default();
                }
                StepperAction::ClickedValue => {
                    menus.reveal(Menu::Trigger);
                }
                StepperAction::None => {}
            }
            if q_mode != picowon_auto::AdvancedTriggerMode::StandardHardware {
                let matched = auto.trigger_qualifier_rt.last_result.matched;
                let badge_txt = if let Some(rpm) = auto.trigger_qualifier_rt.last_result.estimated_rpm {
                    format!("🦷 {:.0} RPM (OK:{})", rpm, auto.trigger_qualifier_rt.matched_frames)
                } else if matched {
                    format!("✔ Calificado ({})", auto.trigger_qualifier_rt.matched_frames)
                } else {
                    "⏳ Buscando evento…".to_string()
                };
                ui.label(
                    egui::RichText::new(badge_txt)
                        .size(9.0)
                        .monospace()
                        .color(if matched {
                            egui::Color32::from_rgb(115, 240, 165)
                        } else {
                            egui::Color32::from_rgb(255, 200, 95)
                        }),
                );
            }
        });
}

fn capture_and_load_row(ui: &mut egui::Ui, auto: &mut AutomotiveUiState, col_w: f32, gap: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let capt_btn = pro_button_widget("📸 CAPTURA", ButtonTone::Primary, false, 24.0)
            .min_size(egui::vec2(col_w, 24.0))
            .wrap_mode(egui::TextWrapMode::Truncate);
        if ui
            .add(capt_btn)
            .on_hover_text("Guardar tramas, imagen PNG, CSV y ficha del vehículo")
            .clicked()
        {
            auto.scope_capture.dialog_open = true;
        }

        let load_btn = pro_button_widget("📂 CARGAR", ButtonTone::Neutral, false, 24.0)
            .min_size(egui::vec2(col_w, 24.0))
            .wrap_mode(egui::TextWrapMode::Truncate);
        if ui
            .add(load_btn)
            .on_hover_text("Explorador y gestor de capturas almacenadas")
            .clicked()
        {
            auto.capture_browser_open = true;
            auto.capture_browser_tab = 0;
        }
    });
}

/// Tarjeta de herramientas exclusiva de la Estación de Trabajo activa (sin botones de otras estaciones)
fn workstation_specific_action_card(
    ui: &mut egui::Ui,
    link: &mut Link,
    menus: &mut MenuState,
    math: &mut MathState,
    auto: &mut AutomotiveUiState,
) {
    let ws = auto.suite.active_workstation;
    let accent = ws.accent_color();
    let gap = 3.0;
    let btn_h = 25.0;

    egui::Frame::new()
        .fill(egui::Color32::from_rgb(20, 22, 32))
        .stroke(egui::Stroke::new(1.6, accent))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let col_w = ((ui.available_width() - gap) / 2.0).floor().max(40.0);

            match ws {
                crate::ui::suite::SuiteWorkstation::ScopeDatalogger => {
                    ui.label(
                        egui::RichText::new("🛠 INSTRUMENTOS SCOPE")
                            .strong()
                            .size(10.0)
                            .color(accent),
                    );
                    ui.add_space(2.0);
                    capture_and_load_row(ui, auto, col_w, gap);
                    ui.add_space(3.0);
                }
                crate::ui::suite::SuiteWorkstation::CommsMonitor => {
                    ui.label(
                        egui::RichText::new("🌐 CONTROL CAN / LIN")
                            .strong()
                            .size(10.0)
                            .color(accent),
                    );
                    ui.add_space(2.0);
                    capture_and_load_row(ui, auto, col_w, gap);
                    ui.add_space(3.0);

                    let lay = auto.suite.comms_layout;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        let b1 = uniform_card_btn(
                            "🔀 Combinado",
                            lay == crate::ui::suite::CommsViewLayout::CombinedSplit,
                            accent,
                        );
                        if ui.add_sized(egui::vec2(col_w, btn_h), b1).clicked() {
                            auto.suite.comms_layout = crate::ui::suite::CommsViewLayout::CombinedSplit;
                        }
                        let b2 = uniform_card_btn(
                            "🌐 Tráfico",
                            lay == crate::ui::suite::CommsViewLayout::TrafficTableOnly,
                            accent,
                        );
                        if ui.add_sized(egui::vec2(col_w, btn_h), b2).clicked() {
                            auto.suite.comms_layout = crate::ui::suite::CommsViewLayout::TrafficTableOnly;
                        }
                    });
                }
                crate::ui::suite::SuiteWorkstation::ActuatorsLoop => {
                    ui.label(
                        egui::RichText::new("🎯 LAZO CERRADO")
                            .strong()
                            .size(10.0)
                            .color(accent),
                    );
                    ui.add_space(2.0);
                    capture_and_load_row(ui, auto, col_w, gap);
                    ui.add_space(3.0);

                    let is_split = auto.suite.actuator_layout
                        == crate::ui::suite::ActuatorViewLayout::SplitCurveAndWaveforms;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        let b1 = uniform_card_btn("🔀 Ondas+XY", is_split, accent);
                        if ui.add_sized(egui::vec2(col_w, btn_h), b1).clicked() {
                            auto.suite.actuator_layout =
                                crate::ui::suite::ActuatorViewLayout::SplitCurveAndWaveforms;
                        }
                        let b2 = uniform_card_btn("🎯 Solo X-Y", !is_split, accent);
                        if ui.add_sized(egui::vec2(col_w, btn_h), b2).clicked() {
                            auto.suite.actuator_layout =
                                crate::ui::suite::ActuatorViewLayout::TransferCurveXy;
                        }
                    });
                    ui.add_space(3.0);
                    if ui
                        .add_sized(
                            egui::vec2(ui.available_width(), btn_h),
                            uniform_card_btn("🗑 Reiniciar Curva X-Y", false, accent),
                        )
                        .clicked()
                    {
                        auto.actuator.clear();
                    }
                }
                crate::ui::suite::SuiteWorkstation::AutomotiveTests => {
                    ui.label(
                        egui::RichText::new("🔧 ENSAYOS GUIADOS")
                            .strong()
                            .size(10.0)
                            .color(accent),
                    );
                    ui.add_space(2.0);
                    capture_and_load_row(ui, auto, col_w, gap);
                    ui.add_space(3.0);

                }
                crate::ui::suite::SuiteWorkstation::ViTracer => {}
            }

            // Favoritas de la estación (⭐ desde "More...") + botón "More…" que abre la columna adyacente
            if ws != crate::ui::suite::SuiteWorkstation::ViTracer {
                ui.add_space(3.0);
                crate::ui::suite::favorites_grid(
                    ui, link, math, menus, auto, col_w, gap, btn_h, accent,
                );
            }
        });
}

fn uniform_card_btn(label: &str, active: bool, accent: egui::Color32) -> egui::Button<'static> {
    pro_button_widget(label.to_string(), ButtonTone::Custom(accent), active, 24.0)
        .wrap_mode(egui::TextWrapMode::Truncate)
}

fn workstation_header_card(ui: &mut egui::Ui, auto: &mut AutomotiveUiState) {
    let ws = auto.suite.active_workstation;
    let accent = ws.accent_color();
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(22, 26, 36))
        .stroke(egui::Stroke::new(1.2, accent))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(ws.short_title())
                        .size(9.5)
                        .strong()
                        .color(accent),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("🏠")
                        .on_hover_text("Cambiar de Estación en PicoWon Suite (F2)")
                        .clicked()
                    {
                        auto.suite.launcher_open = true;
                    }
                });
            });
        });
}

fn vi_tracer_sidebar_card(ui: &mut egui::Ui, _link: &mut Link, auto: &mut AutomotiveUiState) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(18, 26, 34))
        .stroke(egui::Stroke::new(1.6, egui::Color32::from_rgb(80, 220, 255)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("⚡ TRAZADOR V-I (X-Y)")
                    .size(10.0)
                    .strong()
                    .color(egui::Color32::from_rgb(80, 220, 255)),
            );
            ui.add_space(2.0);
            let eq = auto.vi_tracer.equipment_store.active_equipment();
            ui.label(
                egui::RichText::new(format!("Equipo: {}", eq.name))
                    .size(9.5)
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(format!(
                    "CH1 {:.1}V · CH2 {:.1}V · Rs {:.0} Ω",
                    eq.ch1_vdiv * eq.ch1_probe,
                    eq.ch2_vdiv * eq.ch2_probe,
                    eq.r_sense_ohms
                ))
                .size(9.0)
                .monospace()
                .color(egui::Color32::from_gray(185)),
            );
            ui.add_space(3.0);
            if ui
                .add_sized(
                    [ui.available_width(), 20.0],
                    egui::Button::new(
                        egui::RichText::new("📌 Fijar Golden Sample")
                            .size(9.5)
                            .strong()
                            .color(egui::Color32::from_rgb(255, 215, 70)),
                    )
                    .fill(egui::Color32::from_rgb(35, 42, 56)),
                )
                .clicked()
            {
                auto.vi_tracer.capture_reference();
            }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use neowon_backend::Backend;
    use neowon_core::{AcqMode, Acquisition, CaptureFrame, ChannelCapture, IqCal, SampleLayout};

    use super::*;

    fn test_link() -> Link {
        let sup = neowon_backend::spawn(|| -> Result<Box<dyn Backend>, String> {
            Err("no hardware in tests".into())
        });
        Link::new(sup, crate::view::startup_config())
    }

    #[test]
    fn trigger_50_percent_uses_measurements_and_respects_source_state() {
        let mut link = test_link();
        let mut meas = MeasureState::default();

        // Sin mediciones ni trama -> None
        assert_eq!(trigger_50_percent_level(&link, &meas), None);

        // Con mediciones en CH1 -> punto medio entre vmin y vmax
        let mut m = crate::derived::Measurements::default();
        m.vmin = -2.0;
        m.vmax = 10.0;
        meas.latest[0] = Some(m);
        assert_eq!(trigger_50_percent_level(&link, &meas), Some(4.0));

        // Si la fuente es EXT (2) -> None
        link.config.trigger.source = 2;
        assert_eq!(trigger_50_percent_level(&link, &meas), None);

        // Si la fuente es CH2 pero CH2 está desactivado -> None
        link.config.trigger.source = 1;
        link.config.channels[1].enabled = false;
        assert_eq!(trigger_50_percent_level(&link, &meas), None);

        // Con CH2 activado y sin MeasureState, calcula desde la trama bruta
        link.config.channels[1].enabled = true;
        let frame = CaptureFrame::new(
            1,
            None,
            1_000_000.0,
            AcqMode::Sample,
            Acquisition::Record { samples: 3 },
            SampleLayout::Real,
            vec![ChannelCapture {
                ch: 1,
                cal: IqCal::real(0.1, 1.0),
                data: vec![-10.0, 30.0, 0.0],
                clipped: false,
                freq_meter: None,
            }],
        )
        .expect("valid test frame");
        link.latest = Some(Arc::new(frame));
        // vmin = -10 * 0.1 + 1.0 = 0.0, vmax = 30 * 0.1 + 1.0 = 4.0 -> 50% = 2.0 V
        let lvl = trigger_50_percent_level(&link, &meas).expect("fallback frame level");
        assert!((lvl - 2.0).abs() < 1e-12);
    }
}

