use bevy_egui::egui;

use crate::Link;
use crate::cursors::{ChannelRulerMode, CursorState, PhasePreset};
use crate::derived::{MeasureState, fmt_si, fmt_si_sticky};
use crate::gpu::Phosphor;
use crate::ui::automotive_dock::AutomotiveUiState;
use crate::ui::layout::{Layout, Roi};
use crate::ui::widgets::{StepperAction, channel_color, compact_stepper};

/// Interpola linealmente el valor en Voltios de un `ChannelCapture` en la posición fraccional `frac` (`0.0..=1.0`).
pub(crate) fn sample_volts_at_frac(cap: &neowon_core::ChannelCapture, frac: f32) -> Option<f64> {
    if cap.data.is_empty() {
        return None;
    }
    let max_idx = (cap.data.len() - 1) as f32;
    let f_idx = (frac.clamp(0.0, 1.0) * max_idx).clamp(0.0, max_idx);
    let i0 = (f_idx.floor() as usize).min(cap.data.len() - 1);
    let i1 = (i0 + 1).min(cap.data.len() - 1);
    let t = (f_idx - i0 as f32) as f64;
    let raw = (cap.data[i0] as f64) * (1.0 - t) + (cap.data[i1] as f64) * t;
    Some(raw * cap.cal.scale_i + cap.cal.offset_i)
}

/// Botón segmentado estilo PicoScope 7 (`[ Off | On ]`).
fn segmented_off_on(ui: &mut egui::Ui, id_salt: &str, value: &mut bool) {
    let total_w = ui.available_width().min(168.0).max(110.0);
    let half_w = (total_w / 2.0).floor();
    let h = 25.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (is_on_btn, label) in [(false, "Off"), (true, "On")] {
            let selected = *value == is_on_btn;
            let fill = if selected {
                egui::Color32::from_rgb(185, 228, 255)
            } else {
                egui::Color32::from_rgb(16, 22, 32)
            };
            let text_col = if selected {
                egui::Color32::from_rgb(10, 18, 28)
            } else {
                egui::Color32::from_rgb(215, 228, 245)
            };
            let btn = egui::Button::new(
                egui::RichText::new(label)
                    .size(11.5)
                    .strong()
                    .color(text_col),
            )
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(95, 165, 235)))
            .corner_radius(3.0);
            if ui
                .push_id((id_salt, is_on_btn), |ui| {
                    ui.add_sized(egui::vec2(half_w, h), btn)
                })
                .inner
                .clicked()
            {
                *value = is_on_btn;
            }
        }
    });
}

/// Selector segmentado de 3 estados (`[ Off | Manual | Track ]`) por canal estilo PicoScope 7.
fn segmented_channel_ruler_mode(
    ui: &mut egui::Ui,
    ch: usize,
    ch_col: egui::Color32,
    enabled_ch: bool,
    mode: &mut ChannelRulerMode,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let lbl = if ch == 0 { "A (CH1)" } else { "B (CH2)" };
        ui.add_sized(
            egui::vec2(52.0, 24.0),
            egui::Label::new(
                egui::RichText::new(lbl)
                    .strong()
                    .size(11.0)
                    .color(if enabled_ch {
                        ch_col
                    } else {
                        egui::Color32::from_gray(120)
                    }),
            ),
        );
        ui.spacing_mut().item_spacing.x = 0.0;
        let seg_w = ((ui.available_width() - 2.0) / 3.0).floor().max(44.0);
        for (m, text) in [
            (ChannelRulerMode::Off, "Off"),
            (ChannelRulerMode::Manual, "Manual"),
            (ChannelRulerMode::Track, "Track"),
        ] {
            let selected = *mode == m;
            let fill = if selected {
                egui::Color32::from_rgb(185, 228, 255)
            } else {
                egui::Color32::from_rgb(16, 22, 32)
            };
            let text_col = if selected {
                egui::Color32::from_rgb(10, 18, 28)
            } else {
                egui::Color32::from_rgb(215, 228, 245)
            };
            let btn = egui::Button::new(
                egui::RichText::new(text)
                    .size(10.5)
                    .strong()
                    .color(text_col),
            )
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(95, 165, 235)))
            .corner_radius(2.0);
            if ui
                .add_sized(egui::vec2(seg_w, 24.0), btn)
                .clicked()
            {
                *mode = m;
                changed = true;
            }
        }
    });
    changed
}

/// Panel de configuración de Reglas (`Rulers`) en la columna adyacente izquierda (Imagen 2).
/// Los valores en tiempo real NO se muestran aquí, sino en la tabla flotante sobre la cuadrícula (Imagen 1).
pub fn show(
    ui: &mut egui::Ui,
    link: &Link,
    cur: &mut CursorState,
    _meas: &MeasureState,
    _deep: &crate::deep::DeepView,
    _auto: &AutomotiveUiState,
) {
    ui.spacing_mut().item_spacing.y = 6.0;

    // 1. Time rulers [ Off | On ]
    ui.label(
        egui::RichText::new("Time rulers (Reglas de Tiempo)")
            .strong()
            .size(11.5)
            .color(egui::Color32::from_gray(230)),
    );
    segmented_off_on(ui, "time_rulers_seg", &mut cur.time_on);

    ui.add_space(4.0);

    // 2. Channel rulers [ Off | Manual | Track ]
    ui.label(
        egui::RichText::new("Channel rulers (Reglas de Canal)")
            .strong()
            .size(11.5)
            .color(egui::Color32::from_gray(230)),
    );
    for ch in 0..2 {
        let enabled = link.config.channels[ch].enabled;
        let col = channel_color(ch);
        if segmented_channel_ruler_mode(ui, ch, col, enabled, &mut cur.ch_mode[ch]) {
            if cur.ch_mode[ch] == ChannelRulerMode::Track {
                cur.time_on = true;
            }
            cur.sync_amp_flag();
        }
    }

    ui.add_space(4.0);

    // 3. Phase rulers [ Off | On ]
    ui.label(
        egui::RichText::new("Phase rulers (Reglas de Fase)")
            .strong()
            .size(11.5)
            .color(egui::Color32::from_gray(230)),
    );
    segmented_off_on(ui, "phase_rulers_seg", &mut cur.phase_on);

    ui.add_space(2.0);

    // 4. Preset boundaries [ 360° | 720° | 100% ] / [ Custom ]
    ui.label(
        egui::RichText::new("Preset boundaries (Límites)")
            .size(11.0)
            .color(egui::Color32::from_gray(195)),
    );
    let col3_w = ((ui.available_width() - 4.0) / 3.0).floor().max(52.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (preset, label, upper_val, unit_str) in [
            (PhasePreset::Deg360, "360°", 360.0, "°"),
            (PhasePreset::Deg720, "720°", 720.0, "°"),
            (PhasePreset::Percent100, "100%", 100.0, "%"),
        ] {
            let sel = cur.phase_preset == preset;
            let btn = egui::Button::new(
                egui::RichText::new(label)
                    .size(11.0)
                    .strong()
                    .color(if sel {
                        egui::Color32::from_rgb(10, 18, 28)
                    } else {
                        egui::Color32::from_rgb(215, 228, 245)
                    }),
            )
            .fill(if sel {
                egui::Color32::from_rgb(165, 215, 250)
            } else {
                egui::Color32::from_rgb(18, 24, 34)
            })
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 155, 220)));
            if ui.add_sized(egui::vec2(col3_w, 24.0), btn).clicked() {
                cur.phase_preset = preset;
                cur.phase_lower = 0.0;
                cur.phase_upper = upper_val;
                cur.phase_cycle_deg = upper_val;
                cur.phase_units = unit_str.to_string();
                cur.phase_on = true;
            }
        }
    });
    ui.horizontal(|ui| {
        let sel = cur.phase_preset == PhasePreset::Custom;
        let btn = egui::Button::new(
            egui::RichText::new("Custom")
                .size(11.0)
                .strong()
                .color(if sel {
                    egui::Color32::from_rgb(10, 18, 28)
                } else {
                    egui::Color32::from_rgb(215, 228, 245)
                }),
        )
        .fill(if sel {
            egui::Color32::from_rgb(165, 215, 250)
        } else {
            egui::Color32::from_rgb(18, 24, 34)
        })
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 155, 220)));
        if ui.add_sized(egui::vec2(col3_w, 24.0), btn).clicked() {
            cur.phase_preset = PhasePreset::Custom;
            cur.phase_on = true;
        }
    });

    // 5. Lower & Upper steppers: [- 0 ° +]   [- 720 ° +]
    let half_col = ((ui.available_width() - 6.0) / 2.0).floor().max(85.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.vertical(|ui| {
            ui.set_width(half_col);
            ui.label(
                egui::RichText::new("Lower")
                    .size(10.5)
                    .color(egui::Color32::from_gray(190)),
            );
            let lo_str = format!("{:.0} {}", cur.phase_lower, cur.phase_units);
            match compact_stepper(ui, "phase_lower_step", &lo_str, 22.0) {
                StepperAction::Prev => {
                    cur.phase_lower -= 15.0;
                    cur.phase_preset = PhasePreset::Custom;
                }
                StepperAction::Next => {
                    cur.phase_lower += 15.0;
                    cur.phase_preset = PhasePreset::Custom;
                }
                StepperAction::ClickedValue => {
                    cur.phase_lower = 0.0;
                }
                StepperAction::None => {}
            }
        });
        ui.vertical(|ui| {
            ui.set_width(half_col);
            ui.label(
                egui::RichText::new("Upper")
                    .size(10.5)
                    .color(egui::Color32::from_gray(190)),
            );
            let hi_str = format!("{:.0} {}", cur.phase_upper, cur.phase_units);
            match compact_stepper(ui, "phase_upper_step", &hi_str, 22.0) {
                StepperAction::Prev => {
                    cur.phase_upper = (cur.phase_upper - 45.0).max(cur.phase_lower + 10.0);
                    cur.phase_cycle_deg = cur.phase_upper - cur.phase_lower;
                    cur.phase_preset = PhasePreset::Custom;
                }
                StepperAction::Next => {
                    cur.phase_upper += 45.0;
                    cur.phase_cycle_deg = cur.phase_upper - cur.phase_lower;
                    cur.phase_preset = PhasePreset::Custom;
                }
                StepperAction::ClickedValue => {
                    cur.phase_upper = 720.0;
                    cur.phase_cycle_deg = 720.0;
                }
                StepperAction::None => {}
            }
        });
    });

    // 6. Units
    ui.label(
        egui::RichText::new("Units")
            .size(10.5)
            .color(egui::Color32::from_gray(190)),
    );
    ui.add(
        egui::TextEdit::singleline(&mut cur.phase_units)
            .desired_width(110.0)
            .horizontal_align(egui::Align::Center),
    );

    // 7. Partitions [- 4 +]
    ui.label(
        egui::RichText::new("Partitions")
            .size(10.5)
            .color(egui::Color32::from_gray(190)),
    );
    let part_str = format!("{}", cur.phase_partitions);
    match compact_stepper(ui, "phase_partitions_step", &part_str, 22.0) {
        StepperAction::Prev => {
            cur.phase_partitions = cur.phase_partitions.saturating_sub(1).max(1);
        }
        StepperAction::Next => {
            cur.phase_partitions = (cur.phase_partitions + 1).min(12);
        }
        StepperAction::ClickedValue => {
            cur.phase_partitions = 4;
        }
        StepperAction::None => {}
    }

    // 8. Wrap [ Off | On ]
    ui.label(
        egui::RichText::new("Wrap")
            .size(10.5)
            .color(egui::Color32::from_gray(190)),
    );
    segmented_off_on(ui, "phase_wrap_seg", &mut cur.phase_wrap);

    if cur.phase_on && cur.time_on {
        ui.add_space(4.0);
        if ui
            .button("📌 Fijar límites de fase en X1..X2")
            .on_hover_text("Alinea las reglas de fase con la posición actual de los cursores de tiempo X1 y X2")
            .clicked()
        {
            let p0 = cur.pos[0].min(cur.pos[1]);
            let p1 = cur.pos[0].max(cur.pos[1]);
            cur.phase_ref = [p0, p1.max(p0 + 0.02)];
        }
    }
}

/// Dibuja sobre la cuadrícula:
/// 1. Las etiquetas flotantes en los ejes X/Y (`4.742 ms`, `40.12 ms`, `69.25`, `0.0`) donde apoyan las reglas.
/// 2. La tabla flotante compacta `Rulers` en la esquina superior derecha de la cuadrícula con los valores en vivo (Imagen 1).
#[allow(clippy::too_many_arguments)]
pub fn show_rulers_floating_overlay(
    ctx: &egui::Context,
    l: &Layout,
    link: &Link,
    phosphor: &Phosphor,
    cur: &mut CursorState,
    meas: &MeasureState,
    deep: &crate::deep::DeepView,
    auto: &AutomotiveUiState,
) {
    if !cur.any_ruler_active() {
        return;
    }

    let plot_rect = l.points(Roi::Plot.rect(l));
    let record_s = if deep.on {
        deep.span
    } else {
        crate::view::record_len(link) as f64 / meas.sample_rate.max(1.0)
    };
    let (h_center, h_span) = phosphor.hview;
    let view_frac = |pos: f32| -> f32 {
        (((pos as f64) - (h_center - h_span / 2.0)) / h_span) as f32
    };

    // --- 1. ETIQUETAS DE EJE SOBRE LOS BORDES DE LA CUADRÍCULA (Imagen 1) ---
    let overlay_painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("rulers_axis_callouts"),
    ));

    // Etiquetas en el borde inferior para Reglas de Tiempo X1 y X2
    if cur.time_on {
        for i in 0..2 {
            let xf = view_frac(cur.pos[i]);
            if (0.0..=1.0).contains(&xf) {
                let x = plot_rect.left() + xf * plot_rect.width();
                let t_val = (cur.pos[i] as f64) * record_s;
                let txt = fmt_si(t_val, "s").trim().to_string();
                let pill_rect = egui::Rect::from_center_size(
                    egui::pos2(x.clamp(plot_rect.left() + 30.0, plot_rect.right() - 30.0), plot_rect.bottom() - 9.0),
                    egui::vec2(58.0, 17.0),
                );
                overlay_painter.rect_filled(
                    pill_rect,
                    2.0,
                    egui::Color32::from_rgb(175, 222, 255),
                );
                overlay_painter.rect_stroke(
                    pill_rect,
                    2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 155, 235)),
                    egui::StrokeKind::Inside,
                );
                overlay_painter.text(
                    pill_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    txt,
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(10, 18, 28),
                );
            }
        }
    }

    // Etiquetas en el borde izquierdo para Reglas Manuales de Canal (CH1 y CH2)
    for ch in 0..2 {
        if cur.ch_mode[ch] != ChannelRulerMode::Manual && !(ch == 0 && cur.amp_on) {
            continue;
        }
        let c = link.config.channels[ch];
        let fs = c.volts_div * 10.0 * c.probe;
        let spec = &auto.ch_ext[ch].probe_spec;
        let fill_col = if ch == 0 {
            egui::Color32::from_rgb(255, 215, 55)
        } else {
            egui::Color32::from_rgb(65, 200, 255)
        };
        let fracs = if ch == 0 {
            [cur.pos[2], cur.pos[3]]
        } else {
            cur.ch2_pos
        };
        for &y_frac in &fracs {
            let y_screen = plot_rect.center().y - (y_frac / 0.8) * plot_rect.height();
            if y_screen >= plot_rect.top() && y_screen <= plot_rect.bottom() {
                let v = ((y_frac as f64) - c.offset) * fs;
                let phys = spec.scaling.v_bnc_to_physical(v / c.probe.max(1e-12));
                let txt = format!("{:.2}", phys);
                let pill_rect = egui::Rect::from_min_size(
                    egui::pos2(plot_rect.left() + 2.0, (y_screen - 8.5).clamp(plot_rect.top(), plot_rect.bottom() - 17.0)),
                    egui::vec2(44.0, 17.0),
                );
                overlay_painter.rect_filled(pill_rect, 2.0, fill_col);
                overlay_painter.text(
                    pill_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    txt,
                    egui::FontId::monospace(9.5),
                    egui::Color32::from_rgb(10, 16, 24),
                );
            }
        }
    }

    // Etiquetas de Fase y Particiones en el borde superior/inferior
    if cur.phase_on {
        let parts = cur.phase_partitions.clamp(1, 16);
        for k in 0..=parts {
            let frac = cur.phase_ref[0]
                + (cur.phase_ref[1] - cur.phase_ref[0]) * (k as f32 / parts as f32);
            let xf = view_frac(frac);
            if (0.0..=1.0).contains(&xf) {
                let x = plot_rect.left() + xf * plot_rect.width();
                let deg = cur.phase_lower
                    + (cur.phase_upper - cur.phase_lower) * (k as f32 / parts as f32);
                let is_boundary = k == 0 || k == parts;
                let pill_rect = egui::Rect::from_center_size(
                    egui::pos2(x.clamp(plot_rect.left() + 24.0, plot_rect.right() - 24.0), plot_rect.top() + 10.0),
                    egui::vec2(if is_boundary { 48.0 } else { 42.0 }, 16.0),
                );
                overlay_painter.rect_filled(
                    pill_rect,
                    2.0,
                    if is_boundary {
                        egui::Color32::from_rgb(245, 175, 45)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(60, 45, 18, 210)
                    },
                );
                overlay_painter.text(
                    pill_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0}{}", deg, cur.phase_units),
                    egui::FontId::monospace(9.5),
                    if is_boundary {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::from_rgb(255, 210, 110)
                    },
                );
            }
        }
    }

    // --- 2. TABLA FLOTANTE COMPACTA "RULERS" EN ESQUINA SUPERIOR DERECHA DE LA CUADRÍCULA (Imagen 1) ---
    let default_pos = egui::pos2(
        (plot_rect.right() - 295.0).max(plot_rect.left() + 20.0),
        plot_rect.top() + 10.0,
    );

    let mut close_all = false;
    egui::Window::new("rulers_floating_table")
        .title_bar(false)
        .resizable(false)
        .default_pos(default_pos)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(10, 14, 22, 238))
                .stroke(egui::Stroke::new(1.2, egui::Color32::from_rgb(88, 168, 238)))
                .corner_radius(3.0)
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            let table_w = 278.0;
            ui.set_width(table_w);

            // Barra de título celeste estilo PicoScope 7 ("Rulers" con — y ✕)
            let (hdr_rect, _) =
                ui.allocate_exact_size(egui::vec2(table_w, 20.0), egui::Sense::hover());
            ui.painter().rect_filled(
                hdr_rect,
                egui::CornerRadius {
                    nw: 2,
                    ne: 2,
                    sw: 0,
                    se: 0,
                },
                egui::Color32::from_rgb(108, 185, 245),
            );
            ui.painter().text(
                egui::pos2(hdr_rect.left() + 6.0, hdr_rect.center().y),
                egui::Align2::LEFT_CENTER,
                "Rulers",
                egui::FontId::proportional(11.5),
                egui::Color32::from_rgb(12, 20, 32),
            );

            // Botones [—] y [✕] en la barra de título de la tabla flotante
            let min_rect = egui::Rect::from_min_size(
                egui::pos2(hdr_rect.right() - 38.0, hdr_rect.top() + 1.0),
                egui::vec2(18.0, 18.0),
            );
            let cls_rect = egui::Rect::from_min_size(
                egui::pos2(hdr_rect.right() - 19.0, hdr_rect.top() + 1.0),
                egui::vec2(18.0, 18.0),
            );
            if ui.put(
                min_rect,
                egui::Button::new(
                    egui::RichText::new(if cur.rulers_table_minimized { "+" } else { "—" })
                        .size(10.0)
                        .strong()
                        .color(egui::Color32::BLACK),
                )
                .frame(false),
            )
            .on_hover_text("Minimizar / expandir tabla de reglas")
            .clicked()
            {
                cur.rulers_table_minimized = !cur.rulers_table_minimized;
            }
            if ui.put(
                cls_rect,
                egui::Button::new(
                    egui::RichText::new("✕")
                        .size(10.5)
                        .strong()
                        .color(egui::Color32::BLACK),
                )
                .frame(false),
            )
            .on_hover_text("Desactivar todas las reglas")
            .clicked()
            {
                close_all = true;
            }

            if cur.rulers_table_minimized {
                return;
            }

            let c_icon = 22.0;
            let c_val = 76.0;
            let c_lock = 28.0;
            let row_h = 20.0;
            let border_col = egui::Color32::from_rgb(70, 115, 165);

            // Encabezado de columnas: [   | 1 | 2 | Δ |   ]
            draw_ruler_table_row(
                ui,
                table_w,
                row_h,
                [c_icon, c_val, c_val, c_val, c_lock],
                border_col,
                None,
                ["", "1", "2", "Δ"],
                false,
                None,
            );

            // Fila de Tiempo (si time_on está activo)
            let dt = ((cur.pos[1] - cur.pos[0]).abs() as f64) * record_s;
            if cur.time_on {
                let t1 = (cur.pos[0] as f64) * record_s;
                let t2 = (cur.pos[1] as f64) * record_s;
                let s1 = fmt_si(t1, "s").trim().to_string();
                let s2 = fmt_si(t2, "s").trim().to_string();
                let sdt = fmt_si_sticky(dt, "s", &mut cur.bands[0]).trim().to_string();
                if draw_ruler_table_row(
                    ui,
                    table_w,
                    row_h,
                    [c_icon, c_val, c_val, c_val, c_lock],
                    border_col,
                    None,
                    ["", &s1, &s2, &sdt],
                    cur.lock_dt,
                    Some(cur.lock_dt),
                ) {
                    cur.lock_dt = !cur.lock_dt;
                }
            }

            // Filas de Canales CH1 y CH2 (Manual o Track)
            for ch in 0..2 {
                let mode = if ch == 0 && cur.amp_on && cur.ch_mode[0] == ChannelRulerMode::Off {
                    ChannelRulerMode::Manual
                } else {
                    cur.ch_mode[ch]
                };
                if mode == ChannelRulerMode::Off {
                    continue;
                }
                let col_badge = channel_color(ch);
                let spec = &auto.ch_ext[ch].probe_spec;

                match mode {
                    ChannelRulerMode::Manual => {
                        let c = link.config.channels[ch];
                        let fs = c.volts_div * 10.0 * c.probe;
                        let fracs = if ch == 0 {
                            [cur.pos[2], cur.pos[3]]
                        } else {
                            cur.ch2_pos
                        };
                        let v1 = ((fracs[0] as f64) - c.offset) * fs;
                        let v2 = ((fracs[1] as f64) - c.offset) * fs;
                        let s1 = spec.format_scaled_value(v1);
                        let s2 = spec.format_scaled_value(v2);
                        let sdy = spec.unit.format_value((v2 - v1).abs());
                        let locked = cur.lock_dy[ch];
                        if draw_ruler_table_row(
                            ui,
                            table_w,
                            row_h,
                            [c_icon, c_val, c_val, c_val, c_lock],
                            border_col,
                            Some(col_badge),
                            ["", &s1, &s2, &sdy],
                            locked,
                            Some(locked),
                        ) {
                            cur.lock_dy[ch] = !cur.lock_dy[ch];
                        }
                    }
                    ChannelRulerMode::Track => {
                        let (s1, s2, sdy) = if let Some(cap) = link
                            .latest
                            .as_ref()
                            .and_then(|f| f.channels.iter().find(|c| c.ch == ch))
                        {
                            if let (Some(v1), Some(v2)) = (
                                sample_volts_at_frac(cap, cur.pos[0]),
                                sample_volts_at_frac(cap, cur.pos[1]),
                            ) {
                                (
                                    spec.format_scaled_value(v1),
                                    spec.format_scaled_value(v2),
                                    spec.unit.format_value(v2 - v1),
                                )
                            } else {
                                ("---".to_string(), "---".to_string(), "---".to_string())
                            }
                        } else {
                            ("---".to_string(), "---".to_string(), "---".to_string())
                        };
                        draw_ruler_table_row(
                            ui,
                            table_w,
                            row_h,
                            [c_icon, c_val, c_val, c_val, c_lock],
                            border_col,
                            Some(col_badge),
                            ["", &s1, &s2, &sdy],
                            false,
                            None,
                        );
                    }
                    ChannelRulerMode::Off => {}
                }
            }

            // Fila de Fase (si phase_on está activo)
            if cur.phase_on {
                let deg1 = cur.pos_to_phase_deg(cur.pos[0]);
                let deg2 = cur.pos_to_phase_deg(cur.pos[1]);
                let d_deg = (deg2 - deg1).abs();
                let u = &cur.phase_units;
                let s1 = format!("{:.1}{u}", deg1);
                let s2 = format!("{:.1}{u}", deg2);
                let sd = format!("{:.1}{u}", d_deg);
                draw_ruler_table_row(
                    ui,
                    table_w,
                    row_h,
                    [c_icon, c_val, c_val, c_val, c_lock],
                    border_col,
                    Some(egui::Color32::from_rgb(245, 175, 45)),
                    ["∠", &s1, &s2, &sd],
                    false,
                    None,
                );
            }

            // Fila inferior de resumen: [ 1/Δ | 28.26 Hz | 1696 RPM ] (Imagen 1)
            if cur.time_on || cur.phase_on {
                let eff_dt = if cur.time_on {
                    dt
                } else {
                    ((cur.phase_ref[1] - cur.phase_ref[0]).abs() as f64) * record_s
                };
                let hz_str = fmt_si_sticky(1.0 / eff_dt.max(1e-12), "Hz", &mut cur.bands[1])
                    .trim()
                    .to_string();
                let revs = if cur.phase_on && (cur.phase_upper - cur.phase_lower - 720.0).abs() < 1.0 {
                    2.0
                } else {
                    1.0
                };
                let rpm_val = (revs * 60.0) / eff_dt.max(1e-9);
                let rpm_str = if eff_dt > 1e-5 && rpm_val <= 99_999.0 {
                    format!("{:.0} RPM", rpm_val)
                } else {
                    "--- RPM".to_string()
                };

                let (ftr_rect, _) =
                    ui.allocate_exact_size(egui::vec2(table_w, row_h), egui::Sense::hover());
                let p = ui.painter();
                p.rect_stroke(
                    ftr_rect,
                    0.0,
                    egui::Stroke::new(0.8, border_col),
                    egui::StrokeKind::Inside,
                );
                let x1 = ftr_rect.left() + 48.0;
                let x2 = ftr_rect.left() + 162.0;
                p.line_segment(
                    [egui::pos2(x1, ftr_rect.top()), egui::pos2(x1, ftr_rect.bottom())],
                    egui::Stroke::new(0.8, border_col),
                );
                p.line_segment(
                    [egui::pos2(x2, ftr_rect.top()), egui::pos2(x2, ftr_rect.bottom())],
                    egui::Stroke::new(0.8, border_col),
                );
                p.text(
                    egui::pos2(ftr_rect.left() + 24.0, ftr_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    "1/Δ",
                    egui::FontId::proportional(10.5),
                    egui::Color32::from_gray(220),
                );
                p.text(
                    egui::pos2((x1 + x2) * 0.5, ftr_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    hz_str,
                    egui::FontId::monospace(10.5),
                    egui::Color32::WHITE,
                );
                p.text(
                    egui::pos2((x2 + ftr_rect.right()) * 0.5, ftr_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    rpm_str,
                    egui::FontId::monospace(10.5),
                    egui::Color32::WHITE,
                );
            }
        });

    if close_all {
        cur.time_on = false;
        cur.amp_on = false;
        cur.ch_mode = [ChannelRulerMode::Off; 2];
        cur.phase_on = false;
    }
}

/// Dibuja una fila de la tabla flotante `Rulers` con celdas delimitadas y botón opcional de candado `🔒`.
#[allow(clippy::too_many_arguments)]
fn draw_ruler_table_row(
    ui: &mut egui::Ui,
    table_w: f32,
    row_h: f32,
    widths: [f32; 5],
    border_col: egui::Color32,
    badge_color: Option<egui::Color32>,
    texts: [&str; 4],
    highlight_delta: bool,
    lock_state: Option<bool>,
) -> bool {
    let mut lock_clicked = false;
    let (row_rect, _) = ui.allocate_exact_size(egui::vec2(table_w, row_h), egui::Sense::hover());
    let p = ui.painter().clone();

    p.rect_stroke(
        row_rect,
        0.0,
        egui::Stroke::new(0.8, border_col),
        egui::StrokeKind::Inside,
    );

    let mut x = row_rect.left();
    for (col_idx, &w) in widths.iter().enumerate() {
        let cell_rect = egui::Rect::from_min_size(egui::pos2(x, row_rect.top()), egui::vec2(w, row_h));
        if col_idx > 0 {
            p.line_segment(
                [cell_rect.left_top(), cell_rect.left_bottom()],
                egui::Stroke::new(0.8, border_col),
            );
        }
        if col_idx == 3 && highlight_delta {
            p.rect_filled(
                cell_rect.shrink(1.0),
                0.0,
                egui::Color32::from_rgb(35, 110, 185),
            );
        }
        if col_idx == 0 {
            if let Some(c) = badge_color {
                let sq = egui::Rect::from_center_size(cell_rect.center(), egui::vec2(9.0, 9.0));
                p.rect_filled(sq, 1.5, c);
            } else if !texts[0].is_empty() {
                p.text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    texts[0],
                    egui::FontId::proportional(10.5),
                    egui::Color32::from_gray(220),
                );
            }
        } else if col_idx <= 3 {
            p.text(
                egui::pos2(cell_rect.left() + 5.0, cell_rect.center().y),
                egui::Align2::LEFT_CENTER,
                texts[col_idx],
                egui::FontId::monospace(10.5),
                egui::Color32::WHITE,
            );
        } else if let Some(locked) = lock_state {
            let lock_btn = egui::Button::new(
                egui::RichText::new("🔒")
                    .size(10.0)
                    .color(if locked {
                        egui::Color32::from_rgb(10, 18, 28)
                    } else {
                        egui::Color32::from_gray(185)
                    }),
            )
            .fill(if locked {
                egui::Color32::from_rgb(135, 205, 255)
            } else {
                egui::Color32::TRANSPARENT
            })
            .frame(false);
            if ui
                .put(cell_rect.shrink(1.5), lock_btn)
                .on_hover_text("Bloquear intervalo Δ (mover reglas 1 y 2 juntas)")
                .clicked()
            {
                lock_clicked = true;
            }
        }
        x += w;
    }
    lock_clicked
}
