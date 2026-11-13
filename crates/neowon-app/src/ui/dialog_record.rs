//! Record / Export / History section: capture the record stream to memory,
//! scrub back through it, and write it out — `.nwc` (lossless, reloadable),
//! WAV (audio, CH1 = L / CH2 = R), CSV, raw i8, or a PNG of the plot.

use bevy_egui::egui;

use crate::Link;
use crate::derived::fmt_si;
use crate::record::{History, Recorder, default_stem, export_dir};
use crate::script::{Action, Script};

pub fn show(
    ui: &mut egui::Ui,
    link: &mut Link,
    rec: &mut Recorder,
    hist: &mut History,
    script: &mut Script,
) {
    ui.horizontal(|ui| {
        // The scrollback ring is always capturing unless paused; the
        // History slider below scrubs it, like terminal scrollback.
        let label = if rec.on {
            "⏺ Capturing"
        } else {
            "⏸ Paused"
        };
        let color = if rec.on {
            egui::Color32::from_rgb(150, 45, 40)
        } else {
            egui::Color32::from_rgb(28, 30, 36)
        };
        if ui.add(egui::Button::new(label).fill(color)).clicked() {
            rec.on = !rec.on;
        }
        if ui.button("Clear").clicked() {
            rec.clear();
            hist.live(link);
        }
    });
    ui.label(
        // Counts right-aligned in fixed fields — they tick up while
        // recording and must not push the line wider digit by digit.
        egui::RichText::new(format!(
            "{:>6} records · {:>9} samples/ch · {}",
            rec.frames.len(),
            rec.samples_per_channel(),
            fmt_si(rec.seconds(), "s"),
        ))
        .monospace()
        .small(),
    );

    ui.separator();
    let n = rec.frames.len();
    ui.horizontal(|ui| {
        ui.label("History");
        let at = hist.active;
        let mut idx = at.unwrap_or(n.saturating_sub(1));
        let slider = ui.add_enabled(
            n > 1,
            egui::Slider::new(&mut idx, 0..=n.saturating_sub(1).max(1)).show_value(false),
        );
        if slider.changed() {
            hist.show(link, rec, idx);
        }
        if ui.add_enabled(n > 0, egui::Button::new("◀")).clicked() {
            hist.show(link, rec, at.unwrap_or(n - 1).saturating_sub(1));
        }
        if ui.add_enabled(n > 0, egui::Button::new("▶")).clicked() {
            hist.show(link, rec, at.map_or(n - 1, |i| i + 1));
        }
        if ui
            .add_enabled(at.is_some(), egui::Button::new("Live"))
            .clicked()
        {
            hist.live(link);
        }
    });
    ui.label(
        egui::RichText::new(match hist.active {
            Some(i) => format!("{:>6} / {n:<6}", i + 1),
            None => format!("  live / {n:<6}"),
        })
        .monospace()
        .small(),
    );
    if ui
        .button(
            egui::RichText::new("🖼 Abrir Galería Visual del Búfer (Cap. 6.1)")
                .strong()
                .color(egui::Color32::from_rgb(255, 205, 85)),
        )
        .on_hover_text("Inspecciona miniaturas de todas las tramas en RAM y filtra anomalías de máscara")
        .clicked()
    {
        hist.gallery_open = true;
    }

    ui.separator();
    let stem = default_stem();
    let dir = export_dir();
    ui.horizontal(|ui| {
        let disabled = rec.frames.is_empty();
        let export = |name: &str| dir.join(format!("{stem}.{name}"));
        if ui
            .add_enabled(!disabled, egui::Button::new("Save .nwc"))
            .clicked()
        {
            let path = export("nwc");
            match rec.save_nwc(&path) {
                Ok(()) => rec.last_export = Some(path.display().to_string()),
                Err(e) => rec.last_export = Some(format!("failed: {e}")),
            }
        }
        if ui
            .add_enabled(!disabled, egui::Button::new("Export WAV"))
            .clicked()
        {
            let path = export("wav");
            match rec.export_wav(&path) {
                Ok(()) => rec.last_export = Some(path.display().to_string()),
                Err(e) => rec.last_export = Some(format!("failed: {e}")),
            }
        }
        if ui
            .add_enabled(!disabled, egui::Button::new("CSV"))
            .clicked()
        {
            let path = export("csv");
            match rec.export_csv(&path) {
                Ok(()) => rec.last_export = Some(path.display().to_string()),
                Err(e) => rec.last_export = Some(format!("failed: {e}")),
            }
        }
        if ui
            .add_enabled(!disabled, egui::Button::new("Raw i8"))
            .clicked()
        {
            match rec.export_raw(&export("raw")) {
                Ok(files) => rec.last_export = files.first().cloned(),
                Err(e) => rec.last_export = Some(format!("failed: {e}")),
            }
        }
        if ui.button("PNG").clicked() {
            // Goes through the script queue: the plot readback is a GPU
            // round-trip, and a capture export is the plot, not the window.
            script.inject(Action::ShotPlot {
                path: export("png").display().to_string(),
                roi: None,
            });
            rec.last_export = Some(export("png").display().to_string());
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Load").clicked() {
            let path = rec.load_path.clone();
            script.inject(Action::CapLoad(path));
        }
        ui.add(
            egui::TextEdit::singleline(&mut rec.load_path)
                .hint_text("capture.nwc or vendor .cap")
                .desired_width(ui.available_width()),
        );
    });
    if let Some(last) = &rec.last_export {
        ui.label(egui::RichText::new(last.clone()).small().weak());
    } else {
        ui.label(
            egui::RichText::new(format!("→ {}", dir.display()))
                .small()
                .weak(),
        );
    }
}

/// Ventana flotante del Navegador Visual de Búfer de Formas de Onda (PicoScope 7 Cap. 6.1).
/// Permite inspeccionar en cuadrícula de miniaturas todas las tramas almacenadas en RAM,
/// reproducirlas paso a paso y filtrar sólo aquellas que violaron la máscara Pasa/Falla.
pub fn show_buffer_gallery_window(
    ctx: &egui::Context,
    link: &mut Link,
    rec: &Recorder,
    hist: &mut History,
    pf: &crate::derived::PfState,
    now: f64,
) {
    // Índices visibles según el filtro de fallos de máscara
    let matching_indices: Vec<usize> = (0..rec.frames.len())
        .filter(|&i| {
            if hist.filter_failed_only {
                pf.failed_frame_seqs.contains(&rec.frames[i].seq)
            } else {
                true
            }
        })
        .collect();

    // Avance automático si está en modo Reproducción (Play)
    if hist.playing && !matching_indices.is_empty() {
        let period = 1.0 / (hist.play_fps.clamp(0.5, 30.0) as f64);
        if now - hist.last_step_time >= period {
            hist.last_step_time = now;
            let cur_idx = hist.active.unwrap_or(0);
            let next_pos = matching_indices
                .iter()
                .position(|&x| x > cur_idx)
                .unwrap_or(0);
            hist.show(link, rec, matching_indices[next_pos]);
        }
        ctx.request_repaint();
    }

    if !hist.gallery_open {
        return;
    }

    let mut open = hist.gallery_open;
    egui::Window::new("🎞 Navegador Visual de Búfer de Formas de Onda (Cap. 6.1)")
        .open(&mut open)
        .default_width(660.0)
        .default_height(430.0)
        .resizable(true)
        .show(ctx, |ui| {
            let total = rec.frames.len();
            let failed_in_buf = rec
                .frames
                .iter()
                .filter(|f| pf.failed_frame_seqs.contains(&f.seq))
                .count();

            ui.horizontal(|ui| {
                let can_nav = !matching_indices.is_empty();
                let cur_idx = hist.active.unwrap_or_else(|| total.saturating_sub(1));

                if ui
                    .add_enabled(can_nav, egui::Button::new("⏮ Primera"))
                    .clicked()
                {
                    hist.playing = false;
                    if let Some(&first) = matching_indices.first() {
                        hist.show(link, rec, first);
                    }
                }
                if ui
                    .add_enabled(can_nav, egui::Button::new("◀ Ant"))
                    .clicked()
                {
                    hist.playing = false;
                    if let Some(&prev) = matching_indices.iter().rev().find(|&&x| x < cur_idx) {
                        hist.show(link, rec, prev);
                    }
                }

                let play_label = if hist.playing {
                    "⏸ Pausar"
                } else {
                    "▶ Reproducir"
                };
                if ui
                    .add_enabled(can_nav, egui::Button::new(play_label))
                    .clicked()
                {
                    hist.playing = !hist.playing;
                    hist.last_step_time = now;
                }

                if ui
                    .add_enabled(can_nav, egui::Button::new("Sig ▶"))
                    .clicked()
                {
                    hist.playing = false;
                    if let Some(&next) = matching_indices.iter().find(|&&x| x > cur_idx) {
                        hist.show(link, rec, next);
                    }
                }
                if ui
                    .add_enabled(can_nav, egui::Button::new("Última ⏭"))
                    .clicked()
                {
                    hist.playing = false;
                    if let Some(&last) = matching_indices.last() {
                        hist.show(link, rec, last);
                    }
                }

                if hist.active.is_some() {
                    if ui
                        .button(
                            egui::RichText::new("🔴 En Vivo")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 95, 95)),
                        )
                        .clicked()
                    {
                        hist.live(link);
                    }
                }

                ui.separator();
                ui.label(egui::RichText::new("Vel:").size(11.0));
                ui.add(
                    egui::Slider::new(&mut hist.play_fps, 1.0..=15.0)
                        .suffix(" fps")
                        .fixed_decimals(0),
                );
            });

            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut hist.filter_failed_only,
                    format!(
                        "🔍 Mostrar sólo tramas con Fallo de Máscara ({failed_in_buf} fallidas de {total} en RAM)"
                    ),
                );
            });
            ui.separator();

            if matching_indices.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(
                        egui::RichText::new(if hist.filter_failed_only {
                            "No hay tramas con fallo de máscara en el búfer actual."
                        } else {
                            "El búfer de formas de onda está vacío."
                        })
                        .color(egui::Color32::from_gray(165)),
                    );
                    ui.add_space(30.0);
                });
                return;
            }

            let cur_active = hist.active;
            egui::ScrollArea::vertical()
                .max_height(330.0)
                .show(ui, |ui| {
                    let cols = 4usize;
                    let card_w = ((ui.available_width() - 28.0) / (cols as f32)).max(135.0);
                    let card_h = 86.0;

                    for chunk in matching_indices.chunks(cols) {
                        ui.horizontal(|ui| {
                            for &f_idx in chunk {
                                let frame = &rec.frames[f_idx];
                                let is_selected = cur_active == Some(f_idx);
                                let is_failed = pf.failed_frame_seqs.contains(&frame.seq);

                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(card_w, card_h),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);

                                let bg_col = if is_selected {
                                    egui::Color32::from_rgb(24, 34, 52)
                                } else if is_failed {
                                    egui::Color32::from_rgb(38, 16, 18)
                                } else {
                                    egui::Color32::from_rgb(12, 15, 20)
                                };
                                let border_col = if is_selected {
                                    egui::Color32::from_rgb(255, 200, 65)
                                } else if is_failed {
                                    egui::Color32::from_rgb(245, 75, 75)
                                } else if resp.hovered() {
                                    egui::Color32::from_rgb(100, 140, 195)
                                } else {
                                    egui::Color32::from_rgb(48, 56, 70)
                                };

                                painter.rect_filled(rect, 4.0, bg_col);
                                painter.rect_stroke(
                                    rect,
                                    4.0,
                                    egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_col),
                                    egui::StrokeKind::Inside,
                                );

                                // Líneas guía centrales de la miniatura
                                let plot_rect = egui::Rect::from_min_max(
                                    egui::pos2(rect.left() + 4.0, rect.top() + 20.0),
                                    egui::pos2(rect.right() - 4.0, rect.bottom() - 4.0),
                                );
                                painter.line_segment(
                                    [
                                        egui::pos2(plot_rect.left(), plot_rect.center().y),
                                        egui::pos2(plot_rect.right(), plot_rect.center().y),
                                    ],
                                    egui::Stroke::new(0.5, egui::Color32::from_gray(45)),
                                );

                                // Trazas en miniatura (CH1 amarillo, CH2 cian)
                                for cap in &frame.channels {
                                    if cap.data.is_empty() {
                                        continue;
                                    }
                                    let trace_col = if cap.ch == 0 {
                                        egui::Color32::from_rgb(255, 215, 45)
                                    } else {
                                        egui::Color32::from_rgb(55, 205, 255)
                                    };
                                    let n_pts = 56usize.min(cap.data.len());
                                    let mut pts = Vec::with_capacity(n_pts);
                                    for k in 0..n_pts {
                                        let s_idx = k * (cap.data.len() - 1) / (n_pts - 1).max(1);
                                        let x = plot_rect.left()
                                            + (k as f32 / (n_pts - 1).max(1) as f32)
                                                * plot_rect.width();
                                        let norm_y =
                                            (cap.data[s_idx].clamp(-125.0, 125.0) / 125.0) * 0.45;
                                        let y = plot_rect.center().y - norm_y * plot_rect.height();
                                        pts.push(egui::pos2(x, y));
                                    }
                                    painter.add(egui::Shape::line(
                                        pts,
                                        egui::Stroke::new(1.1, trace_col),
                                    ));
                                }

                                // Encabezado de tarjeta (#Trama y estado Pasa/Falla)
                                let title_txt = format!("#{} (Seq {})", f_idx + 1, frame.seq);
                                painter.text(
                                    egui::pos2(rect.left() + 6.0, rect.top() + 4.0),
                                    egui::Align2::LEFT_TOP,
                                    title_txt,
                                    egui::FontId::monospace(10.0),
                                    egui::Color32::from_gray(225),
                                );
                                if is_failed {
                                    painter.text(
                                        egui::pos2(rect.right() - 6.0, rect.top() + 4.0),
                                        egui::Align2::RIGHT_TOP,
                                        "✖ FALLA",
                                        egui::FontId::proportional(9.5),
                                        egui::Color32::from_rgb(255, 95, 95),
                                    );
                                }

                                if resp
                                    .on_hover_text(format!(
                                        "Trama #{} (Seq #{}) — Clic para inspeccionar en pantalla principal",
                                        f_idx + 1,
                                        frame.seq
                                    ))
                                    .clicked()
                                {
                                    hist.playing = false;
                                    hist.show(link, rec, f_idx);
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
        });
    hist.gallery_open = open;
}
