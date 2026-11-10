//! Bottom-bar measurement strip (PicoScope 7 style):
//! Live automotive telemetry for Channel 1 & 2 (Freq, Duty, PWM, Vpp, Vrms),
//! with interactive checkboxes configurator popup.

use bevy_egui::egui;

use crate::Link;
use crate::derived::{MeasureState, fmt_si};
use crate::gpu::Phosphor;

use super::layout::{Layout, Roi};
use super::menu::MenuState;
use super::widgets::channel_color;

pub fn show(
    ctx: &egui::Context,
    l: &Layout,
    link: &mut Link,
    _phosphor: &Phosphor,
    meas: &mut MeasureState,
    _menus: &mut MenuState,
    _rec: &crate::record::Recorder,
    _hist: &mut crate::record::History,
    auto: &mut crate::ui::automotive_dock::AutomotiveUiState,
) -> (egui::Rect, egui::Rect) {
    let rect = l.points(Roi::Descriptors.rect(l));
    let resp = egui::Area::new("descriptors".into())
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_max_width(rect.width());
            ui.set_height(rect.height());
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                // --- TARJETA DE MEDICIONES EN VIVO CH1 ---
                if !auto.meas_floating {
                    draw_live_measurements(ui, link, meas, auto, 0);
                    ui.separator();
                    draw_live_measurements(ui, link, meas, auto, 1);
                    ui.separator();
                } else {
                    ui.label(
                        egui::RichText::new("📊 Medidas Flotantes Activas (arrastrables sobre la pantalla)")
                            .weak()
                            .size(11.0),
                    );
                    ui.separator();
                }

                // --- BOTÓN MODO FLOTANTE / ACOPLADO ---
                let float_text = if auto.meas_floating { "📌 Acoplar al Pie" } else { "🪟 Ventanas Flotantes" };
                let float_btn = egui::Button::new(
                    egui::RichText::new(float_text)
                        .size(10.5)
                        .color(if auto.meas_floating {
                            egui::Color32::from_rgb(100, 200, 255)
                        } else {
                            egui::Color32::from_gray(210)
                        }),
                )
                .fill(egui::Color32::from_rgb(28, 33, 44))
                .min_size(egui::vec2(80.0, 24.0));
                if ui.add(float_btn).on_hover_text("Alternar entre barra inferior fija o tarjetas flotantes movibles").clicked() {
                    auto.meas_floating = !auto.meas_floating;
                }

                ui.separator();

                // --- BOTÓN CONFIGURAR MEDIDAS [⚙ Medidas] ---
                let cfg_btn = egui::Button::new(
                    egui::RichText::new("⚙ Medidas")
                        .size(11.0)
                        .color(if auto.meas_popup_open {
                            egui::Color32::from_rgb(255, 215, 0)
                        } else {
                            egui::Color32::WHITE
                        }),
                )
                .fill(if auto.meas_popup_open {
                    egui::Color32::from_rgb(50, 56, 70)
                } else {
                    egui::Color32::from_rgb(28, 31, 38)
                })
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 66, 80)))
                .min_size(egui::vec2(74.0, 24.0));
                if ui
                    .add(cfg_btn)
                    .on_hover_text("Abrir selector de mediciones automotrices visibles")
                    .clicked()
                {
                    auto.meas_popup_open = !auto.meas_popup_open;
                }

                // --- NAVEGADOR DE TRAMAS DEL BUFFER HISTÓRICO ---
                let total_frames = _rec.frames.len();
                if total_frames > 0 {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🎞 Trama:").strong().size(11.0).color(egui::Color32::from_rgb(255, 185, 55)));
                        let cur_idx = _hist.active.unwrap_or(total_frames.saturating_sub(1));
                        let can_prev = cur_idx > 0;
                        if ui.add_enabled(can_prev, egui::Button::new("◀").min_size(egui::vec2(18.0, 20.0))).on_hover_text("Trama anterior").clicked() {
                            _hist.show(link, _rec, cur_idx.saturating_sub(1));
                        }
                        let is_live = _hist.active.is_none();
                        let count_str = format!("{} / {}", cur_idx + 1, total_frames);
                        ui.label(egui::RichText::new(count_str).monospace().strong().size(11.0));
                        let can_next = _hist.active.is_some() && cur_idx + 1 < total_frames;
                        if ui.add_enabled(can_next, egui::Button::new("▶").min_size(egui::vec2(18.0, 20.0))).on_hover_text("Trama siguiente").clicked() {
                            if cur_idx + 1 >= total_frames - 1 {
                                _hist.live(link);
                            } else {
                                _hist.show(link, _rec, cur_idx + 1);
                            }
                        }
                        if !is_live {
                            if ui.button(egui::RichText::new("🔴 En Vivo").color(egui::Color32::from_rgb(255, 90, 90)).strong().size(10.5)).on_hover_text("Volver a captura en vivo").clicked() {
                                _hist.live(link);
                            }
                        }
                    });
                }
            });
        });

    // Ventana flotante para configurar checkboxes de mediciones
    if auto.meas_popup_open {
        egui::Window::new("⚙ Configuración de Mediciones en Pantalla")
            .open(&mut auto.meas_popup_open)
            .collapsible(false)
            .resizable(false)
            .default_pos(egui::pos2(rect.min.x + 200.0, rect.min.y - 190.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Selecciona los parámetros visibles para CH1 y CH2:")
                        .strong(),
                );
                ui.separator();
                ui.checkbox(&mut auto.meas_show_freq, "Frecuencia (Freq - Hz / kHz)");
                ui.checkbox(&mut auto.meas_show_duty, "Ciclo de Trabajo (+Duty - %)");
                ui.checkbox(
                    &mut auto.meas_show_pwm,
                    "Ancho de Pulso PWM (+Width - µs / ms)",
                );
                ui.checkbox(
                    &mut auto.meas_show_vpp,
                    "Voltaje Pico a Pico (Vpp - V / mV)",
                );
                ui.checkbox(&mut auto.meas_show_vmax, "Voltaje Máximo (Vmax)");
                ui.checkbox(&mut auto.meas_show_vmin, "Voltaje Mínimo (Vmin)");
                ui.checkbox(&mut auto.meas_show_vrms, "Voltaje Eficaz (Vrms)");
            });
    }

    // Ventanas flotantes movibles cuando meas_floating está activo
    if auto.meas_floating {
        for ch in 0..2 {
            let ch_name = format!("📊 CH{} Medidas", ch + 1);
            let default_pos = egui::pos2(rect.min.x + (ch as f32) * 280.0, rect.min.y - 80.0);
            egui::Window::new(ch_name)
                .id(egui::Id::new(format!("floating_meas_ch{}", ch)))
                .default_pos(default_pos)
                .collapsible(true)
                .resizable(false)
                .show(ctx, |ui| {
                    draw_live_measurements(ui, link, meas, auto, ch);
                });
        }
    }

    (l.pixels(resp.response.rect), egui::Rect::NOTHING)
}

fn draw_live_measurements(
    ui: &mut egui::Ui,
    link: &Link,
    meas: &MeasureState,
    auto: &crate::ui::automotive_dock::AutomotiveUiState,
    ch: usize,
) {
    let c = link.config.channels[ch];
    let col = channel_color(ch);
    let name = format!("CH{}", ch + 1);

    egui::Frame::new()
        .fill(if c.enabled {
            egui::Color32::from_rgb(22, 26, 33)
        } else {
            egui::Color32::from_rgb(18, 20, 24)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if c.enabled {
                col
            } else {
                egui::Color32::from_gray(50)
            },
        ))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Etiqueta de canal
                ui.label(
                    egui::RichText::new(name)
                        .strong()
                        .size(12.0)
                        .color(if c.enabled {
                            col
                        } else {
                            egui::Color32::from_gray(140)
                        }),
                );

                if !c.enabled {
                    ui.weak("Desactivado");
                    return;
                }

                let dsp_m = meas.latest.get(ch).and_then(|m| m.as_ref());

                // Frecuencia
                if auto.meas_show_freq {
                    let freq_str = dsp_m
                        .and_then(|m| m.freq)
                        .map(|f| fmt_si(f, "Hz"))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("F: {freq_str}"))
                            .size(11.0)
                            .strong()
                            .monospace(),
                    );
                }

                // Ciclo de trabajo (+Duty)
                if auto.meas_show_duty {
                    let duty_str = dsp_m
                        .and_then(|m| m.pduty)
                        .map(|d| format!("{:.1}%", d * 100.0))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("D: {duty_str}"))
                            .size(11.0)
                            .monospace(),
                    );
                }

                // Ancho de pulso (+Width / PWM)
                if auto.meas_show_pwm {
                    let pwm_str = dsp_m
                        .and_then(|m| m.pwidth)
                        .map(|w| fmt_si(w, "s"))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("PWM: {pwm_str}"))
                            .size(11.0)
                            .monospace(),
                    );
                }

                // Vpp
                if auto.meas_show_vpp {
                    let vpp_str = dsp_m
                        .map(|m| fmt_si(m.vpp, "V"))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("Vpp: {vpp_str}"))
                            .size(11.0)
                            .monospace(),
                    );
                }

                // Vmax
                if auto.meas_show_vmax {
                    let vmax_str = dsp_m
                        .map(|m| fmt_si(m.vmax, "V"))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("Vmax: {vmax_str}"))
                            .size(11.0)
                            .weak()
                            .monospace(),
                    );
                }

                // Vmin
                if auto.meas_show_vmin {
                    let vmin_str = dsp_m
                        .map(|m| fmt_si(m.vmin, "V"))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("Vmin: {vmin_str}"))
                            .size(11.0)
                            .weak()
                            .monospace(),
                    );
                }

                // Vrms
                if auto.meas_show_vrms {
                    let vrms_str = dsp_m
                        .and_then(|m| m.vrms).map(|v| fmt_si(v, "V"))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("Vrms: {vrms_str}"))
                            .size(11.0)
                            .weak()
                            .monospace(),
                    );
                }
            });
        });
}
