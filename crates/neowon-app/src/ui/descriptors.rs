//! Bottom-bar measurement strip (PicoScope 7 style):
//! Live automotive telemetry for Channel 1 & 2 (Freq, Duty, PWM, Vpp, Vrms),
//! with interactive checkboxes configurator popup.

use bevy_egui::egui;

use crate::Link;
use crate::derived::{MeasureState, fmt_si};
use crate::gpu::Phosphor;

use super::layout::{Layout, Roi};
use super::menu::MenuState;
use super::widgets::{channel_color, ButtonTone, pro_button_widget};

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

                // Barra inferior sin valores numéricos de CH1/CH2 (se muestran en la tabla/ventanas flotantes)
                let table_btn = pro_button_widget("📊 Tabla Mediciones", ButtonTone::Success, meas.window, 24.0)
                    .min_size(egui::vec2(96.0, 24.0));
                if ui
                    .add(table_btn)
                    .on_hover_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📊").size(16.0));
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Tabla Flotante de Mediciones").strong().size(11.0).color(egui::Color32::from_rgb(100, 230, 155)));
                                ui.label(egui::RichText::new("Muestra la tabla en vivo sobre la cuadrícula con opacidad/transparencia regulable").size(10.0).color(egui::Color32::from_gray(215)));
                            });
                        });
                    })
                    .clicked()
                {
                    meas.window = !meas.window;
                }

                // --- BOTÓN TARJETAS FLOTANTES CH1/CH2 ---
                let float_text = if auto.meas_floating {
                    "🪟 Ocultar Tarjetas CH1/CH2"
                } else {
                    "🪟 Tarjetas CH1/CH2"
                };
                let float_btn = pro_button_widget(float_text, ButtonTone::Primary, auto.meas_floating, 24.0)
                    .min_size(egui::vec2(88.0, 24.0));
                if ui
                    .add(float_btn)
                    .on_hover_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🪟").size(16.0));
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Tarjetas Flotantes por Canal").strong().size(11.0).color(egui::Color32::from_rgb(100, 200, 255)));
                                ui.label(egui::RichText::new("Despliega ventanas individuales arrastrables para CH1 y CH2 con fondo translúcido").size(10.0).color(egui::Color32::from_gray(215)));
                            });
                        });
                    })
                    .clicked()
                {
                    auto.meas_floating = !auto.meas_floating;
                }

                ui.separator();

                // --- BOTÓN CONFIGURAR MEDIDAS [⚙ Medidas] ---
                let cfg_btn = pro_button_widget("⚙ Medidas", ButtonTone::Warning, auto.meas_popup_open, 24.0)
                    .min_size(egui::vec2(74.0, 24.0));
                if ui
                    .add(cfg_btn)
                    .on_hover_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚙").size(16.0));
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Configuración de Mediciones").strong().size(11.0).color(egui::Color32::from_rgb(255, 215, 0)));
                                ui.label(egui::RichText::new("Selecciona qué parámetros automotrices calcular y mostrar (Freq, Vpp, Duty, PWM)").size(10.0).color(egui::Color32::from_gray(215)));
                            });
                        });
                    })
                    .clicked()
                {
                    auto.meas_popup_open = !auto.meas_popup_open;
                }

                // --- NAVEGADOR DE TRAMAS Y BOTÓN GALERÍA DE MINIATURAS (Siempre visible) ---
                ui.separator();
                let total_frames = _rec.frames.len();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🎞 Búfer:").strong().size(11.0).color(egui::Color32::from_rgb(255, 185, 55)));

                    let cur_idx = _hist.active.unwrap_or(total_frames.saturating_sub(1));
                    let can_prev = total_frames > 0 && cur_idx > 0;
                    let prev_btn = pro_button_widget("◀", ButtonTone::Neutral, false, 22.0)
                        .min_size(egui::vec2(22.0, 22.0));
                    if ui.add_enabled(can_prev, prev_btn).on_hover_text("Trama anterior").clicked() {
                        _hist.show(link, _rec, cur_idx.saturating_sub(1));
                    }

                    let is_live = _hist.active.is_none();
                    let count_str = if total_frames > 0 {
                        format!("{} / {}", cur_idx + 1, total_frames)
                    } else {
                        "0 / 0".to_string()
                    };
                    ui.label(egui::RichText::new(count_str).monospace().strong().size(11.0));

                    let can_next = total_frames > 0 && _hist.active.is_some() && cur_idx + 1 < total_frames;
                    let next_btn = pro_button_widget("▶", ButtonTone::Neutral, false, 22.0)
                        .min_size(egui::vec2(22.0, 22.0));
                    if ui.add_enabled(can_next, next_btn).on_hover_text("Trama siguiente").clicked() {
                        if cur_idx + 1 >= total_frames - 1 {
                            _hist.live(link);
                        } else {
                            _hist.show(link, _rec, cur_idx + 1);
                        }
                    }

                    if !is_live {
                        let live_btn = pro_button_widget("🔴 En Vivo", ButtonTone::Danger, false, 22.0);
                        if ui.add(live_btn).on_hover_text("Volver a captura en vivo").clicked() {
                            _hist.live(link);
                        }
                    }

                    let gal_btn = pro_button_widget("🖼 Galería Miniaturas", ButtonTone::Warning, _hist.gallery_open, 24.0)
                        .min_size(egui::vec2(120.0, 24.0));
                    if ui
                        .add(gal_btn)
                        .on_hover_ui(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🖼").size(18.0).color(egui::Color32::from_rgb(255, 215, 90)));
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Galería de Miniaturas del Búfer").strong().size(11.5).color(egui::Color32::from_rgb(255, 215, 90)));
                                    ui.label(egui::RichText::new("Abre la ventana visual con el mosaico de todas las formas de onda capturadas, reproducción automática y filtro de fallos").size(10.0).color(egui::Color32::from_gray(215)));
                                });
                            });
                        })
                        .clicked()
                    {
                        _hist.gallery_open = !_hist.gallery_open;
                    }
                });
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
                ui.separator();
                super::measure_window::opacity_slider(ui, &mut meas.window_opacity);
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
                .frame(super::measure_window::translucent_window_frame(ctx, meas.window_opacity))
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
    let alias = auto.ch_ext[ch].alias.trim();
    let probe_spec = &auto.ch_ext[ch].probe_spec;
    let unit_sym = probe_spec.unit.symbol();
    let name = if alias.is_empty() {
        format!("CH{} [{}]", ch + 1, probe_spec.short_badge)
    } else {
        format!("CH{}·{} [{}]", ch + 1, alias, probe_spec.short_badge)
    };
    let card_alpha = (meas.window_opacity.clamp(0.20, 1.0) * 235.0).round() as u8;

    egui::Frame::new()
        .fill(if c.enabled {
            egui::Color32::from_rgba_unmultiplied(22, 26, 33, card_alpha)
        } else {
            egui::Color32::from_rgba_unmultiplied(18, 20, 24, card_alpha)
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

                // Pico a Pico (en unidad física del canal: V, A, bar, psi, kPa, °C, kV)
                if auto.meas_show_vpp {
                    let vpp_str = dsp_m
                        .map(|m| probe_spec.unit.format_value(m.vpp))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("{}pp: {vpp_str}", unit_sym))
                            .size(11.0)
                            .monospace(),
                    );
                }

                // Máximo (en unidad física con offset de transductor incluido)
                if auto.meas_show_vmax {
                    let vmax_str = dsp_m
                        .map(|m| probe_spec.format_scaled_value(m.vmax))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("Max: {vmax_str}"))
                            .size(11.0)
                            .weak()
                            .monospace(),
                    );
                }

                // Mínimo (en unidad física con offset de transductor incluido)
                if auto.meas_show_vmin {
                    let vmin_str = dsp_m
                        .map(|m| probe_spec.format_scaled_value(m.vmin))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("Min: {vmin_str}"))
                            .size(11.0)
                            .weak()
                            .monospace(),
                    );
                }

                // Eficaz / RMS (en unidad física)
                if auto.meas_show_vrms {
                    let vrms_str = dsp_m
                        .and_then(|m| m.vrms)
                        .map(|v| probe_spec.unit.format_value(v))
                        .unwrap_or_else(|| "---".to_string());
                    ui.label(
                        egui::RichText::new(format!("RMS: {vrms_str}"))
                            .size(11.0)
                            .weak()
                            .monospace(),
                    );
                }
            });
        });
}
