//! Herramientas de la gráfica (PicoScope 7 — Fase 4):
//! - Alerta `! Over-Range` por canal (clic = subir V/div).
//! - Pointer Tool Tip: tiempo y valor de cada canal bajo el puntero, con retícula.
//! - Zoom con historial: ⟲ / ⟳ y `1:1`.
//! - Anotaciones en la gráfica ancladas a (tiempo, amplitud), con flecha y arrastre.

use bevy_egui::egui;

use crate::Link;
use crate::cursors::CursorState;
use crate::derived::{MeasureState, fmt_si};
use crate::gpu::{Phosphor, TraceMode};
use crate::ui::automotive_dock::AutomotiveUiState;
use crate::ui::dialog_cursor::sample_volts_at_frac;
use crate::ui::layout::{Layout, Roi};
use super::widgets::{ButtonTone, pro_button_widget};

pub const ANNOT_COLORS: [egui::Color32; 4] = [
    egui::Color32::from_rgb(255, 215, 70),
    egui::Color32::from_rgb(90, 210, 255),
    egui::Color32::from_rgb(255, 120, 110),
    egui::Color32::from_rgb(130, 235, 150),
];

const MAX_ZOOM_HISTORY: usize = 32;
/// Pausa mínima entre cambios para considerar que empieza un gesto de zoom nuevo.
const GESTURE_GAP_S: f64 = 0.8;

type ViewState = (f64, f64, bool);

/// Anotación anclada: `frac_x` = fracción del registro (0..1), `frac_y` = fracción de la
/// altura de la cuadrícula medida desde el centro (-0.5..0.5, positivo = arriba).
#[derive(Debug, Clone)]
pub struct Annotation {
    pub frac_x: f32,
    pub frac_y: f32,
    pub text: String,
    pub color_idx: usize,
}

#[derive(Debug, Clone)]
pub struct PlotToolsState {
    pub pointer_tip: bool,
    pub over_range_alert: bool,
    pub annotations_visible: bool,
    pub annotations: Vec<Annotation>,
    pub zoom_undo: Vec<ViewState>,
    pub zoom_redo: Vec<ViewState>,
    last_view: Option<ViewState>,
    last_change_t: f64,
}

impl Default for PlotToolsState {
    fn default() -> Self {
        Self {
            pointer_tip: true,
            over_range_alert: true,
            annotations_visible: true,
            annotations: Vec::new(),
            zoom_undo: Vec::new(),
            zoom_redo: Vec::new(),
            last_view: None,
            last_change_t: -10.0,
        }
    }
}

fn view_differs(a: ViewState, b: ViewState) -> bool {
    a.2 != b.2 || (a.0 - b.0).abs() > 1e-9 || (a.1 - b.1).abs() > 1e-9
}

impl PlotToolsState {
    /// Registra los cambios de zoom horizontal; un gesto continuo (rueda, arrastre) cuenta como uno solo.
    pub fn track_zoom(&mut self, p: &Phosphor, now: f64) {
        let cur: ViewState = (p.hview.0, p.hview.1, p.zoom_on);
        match self.last_view {
            None => self.last_view = Some(cur),
            Some(prev) if view_differs(prev, cur) => {
                if now - self.last_change_t > GESTURE_GAP_S {
                    self.zoom_undo.push(prev);
                    if self.zoom_undo.len() > MAX_ZOOM_HISTORY {
                        self.zoom_undo.remove(0);
                    }
                    self.zoom_redo.clear();
                }
                self.last_change_t = now;
                self.last_view = Some(cur);
            }
            _ => {}
        }
    }

    fn apply_view(&mut self, p: &mut Phosphor, v: ViewState, now: f64) {
        p.hview = (v.0, v.1);
        p.zoom_on = v.2;
        self.last_view = Some(v);
        self.last_change_t = now;
    }

    pub fn undo_zoom(&mut self, p: &mut Phosphor, now: f64) {
        if let Some(prev) = self.zoom_undo.pop() {
            self.zoom_redo.push((p.hview.0, p.hview.1, p.zoom_on));
            self.apply_view(p, prev, now);
        }
    }

    pub fn redo_zoom(&mut self, p: &mut Phosphor, now: f64) {
        if let Some(next) = self.zoom_redo.pop() {
            self.zoom_undo.push((p.hview.0, p.hview.1, p.zoom_on));
            self.apply_view(p, next, now);
        }
    }

    /// Zoom `1:1`: registro completo, sin ampliación horizontal (queda en el historial).
    pub fn zoom_one_to_one(&mut self, p: &mut Phosphor, now: f64) {
        let cur: ViewState = (p.hview.0, p.hview.1, p.zoom_on);
        if !cur.2 && !view_differs(cur, (0.5, 1.0, false)) {
            return;
        }
        self.zoom_undo.push(cur);
        if self.zoom_undo.len() > MAX_ZOOM_HISTORY {
            self.zoom_undo.remove(0);
        }
        self.zoom_redo.clear();
        crate::view::set_zoom(p, false);
        self.last_view = Some((p.hview.0, p.hview.1, p.zoom_on));
        self.last_change_t = now;
    }
}

/// Caja de texto multilínea con fondo; devuelve el rectángulo dibujado.
fn draw_text_box(
    painter: &egui::Painter,
    top_left: egui::Pos2,
    lines: &[(String, egui::Color32)],
    border: egui::Color32,
) -> egui::Rect {
    let font = egui::FontId::monospace(10.5);
    let galleys: Vec<_> = lines
        .iter()
        .map(|(t, c)| painter.layout_no_wrap(t.clone(), font.clone(), *c))
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 12.0;
    let h: f32 = galleys.iter().map(|g| g.size().y).sum::<f32>() + 8.0;
    let rect = egui::Rect::from_min_size(top_left, egui::vec2(w, h));
    painter.rect_filled(rect, 3.0, egui::Color32::from_rgba_unmultiplied(10, 14, 22, 235));
    painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let mut y = rect.top() + 4.0;
    for g in galleys {
        let gh = g.size().y;
        painter.galley(egui::pos2(rect.left() + 6.0, y), g, egui::Color32::WHITE);
        y += gh;
    }
    rect
}

/// Dibuja sobre la cuadrícula: alertas Over-Range, barra de historial de zoom, anotaciones
/// ancladas y Pointer Tool Tip.
#[allow(clippy::too_many_arguments)]
pub fn show_plot_overlays(
    ctx: &egui::Context,
    l: &Layout,
    link: &mut Link,
    phosphor: &mut Phosphor,
    meas: &MeasureState,
    deep: &crate::deep::DeepView,
    auto: &mut AutomotiveUiState,
    cur: &CursorState,
    now: f64,
) {
    let plot_rect = l.points(Roi::Plot.rect(l));
    let st = &mut auto.plot_tools;
    st.track_zoom(phosphor, now);

    let record_s = if deep.on {
        deep.span
    } else {
        crate::view::record_len(link) as f64 / meas.sample_rate.max(1.0)
    };
    let (h_center, h_span) = phosphor.hview;
    let frac_to_x = |frac: f32| -> f32 {
        let vf = ((frac as f64 - (h_center - h_span / 2.0)) / h_span) as f32;
        plot_rect.left() + vf * plot_rect.width()
    };
    let x_to_frac = |x: f32| -> f32 {
        let vf = ((x - plot_rect.left()) / plot_rect.width()) as f64;
        (vf * h_span + (h_center - h_span / 2.0)) as f32
    };

    // ── 1. Alerta ! Over-Range ───────────────────────────────────────────────
    if st.over_range_alert {
        let clipped: Vec<usize> = link
            .latest
            .as_ref()
            .map(|f| f.channels.iter().filter(|c| c.clipped && c.ch < 2).map(|c| c.ch).collect())
            .unwrap_or_default();
        let mut raise: Option<usize> = None;
        for (i, ch) in clipped.iter().enumerate() {
            let pos = egui::pos2(plot_rect.left() + 8.0, plot_rect.top() + 8.0 + i as f32 * 24.0);
            let clicked = egui::Area::new(egui::Id::new(("over_range_badge", *ch)))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("! Over-Range CH{}", ch + 1))
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(190, 40, 36)),
                    )
                    .on_hover_text("La señal excede el rango del canal. Clic: aumentar V/div")
                    .clicked()
                })
                .inner;
            if clicked {
                raise = Some(*ch);
            }
        }
        if let Some(ch) = raise {
            crate::view::zoom_channel(link, ch, false);
        }
    }

    // ── 2. Historial de zoom ⟲ ⟳ 1:1 ─────────────────────────────────────────
    let tb_pos = egui::pos2(plot_rect.left() + 8.0, plot_rect.bottom() - 56.0);
    let (mut do_undo, mut do_redo, mut do_home) = (false, false, false);
    let can_undo = !st.zoom_undo.is_empty();
    let can_redo = !st.zoom_redo.is_empty();
    let can_home = phosphor.zoom_on;
    egui::Area::new(egui::Id::new("zoom_history_bar"))
        .fixed_pos(tb_pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 16, 24, 220))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 78, 104)))
                .corner_radius(3.0)
                .inner_margin(egui::Margin::same(2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        do_undo = ui
                            .add_enabled(can_undo, pro_button_widget("⟲", ButtonTone::Neutral, false, 20.0).min_size(egui::vec2(20.0, 20.0)))
                            .on_hover_text("Zoom anterior")
                            .clicked();
                        do_redo = ui
                            .add_enabled(can_redo, pro_button_widget("⟳", ButtonTone::Neutral, false, 20.0).min_size(egui::vec2(20.0, 20.0)))
                            .on_hover_text("Zoom siguiente")
                            .clicked();
                        do_home = ui
                            .add_enabled(can_home, pro_button_widget("1:1", ButtonTone::Neutral, false, 20.0).min_size(egui::vec2(28.0, 20.0)))
                            .on_hover_text("Volver al registro completo (1:1)")
                            .clicked();
                        ui.separator();
                        let split_on = phosphor.split_grid && phosphor.mode != TraceMode::Xy;
                        let split_btn = pro_button_widget("Split", ButtonTone::Primary, split_on, 20.0);
                        if ui
                            .add(split_btn)
                            .on_hover_text("Dividir grilla CH1 superior / CH2 inferior")
                            .clicked()
                        {
                            phosphor.split_grid = !phosphor.split_grid;
                            if phosphor.mode == TraceMode::Xy {
                                phosphor.mode = TraceMode::Vectors;
                            }
                        }
                        let xy_on = phosphor.mode == TraceMode::Xy;
                        let xy_btn = pro_button_widget("XY 1:1", ButtonTone::Warning, xy_on, 20.0);
                        if ui
                            .add(xy_btn)
                            .on_hover_text("Vista XY cuadrada 1:1 (CH1 en X, CH2 en Y)")
                            .clicked()
                        {
                            if xy_on {
                                phosphor.mode = TraceMode::Vectors;
                            } else {
                                phosphor.mode = TraceMode::Xy;
                                phosphor.xy_square = true;
                            }
                        }
                    });
                });
        });

    if phosphor.split_grid && phosphor.mode != TraceMode::Xy {
        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("split_grid_labels"),
        ));
        draw_text_box(
            &p,
            egui::pos2(plot_rect.left() + 8.0, plot_rect.top() + 8.0),
            &[("CH1".into(), egui::Color32::from_rgb(255, 215, 55))],
            egui::Color32::from_rgba_unmultiplied(255, 215, 55, 140),
        );
        draw_text_box(
            &p,
            egui::pos2(plot_rect.left() + 8.0, plot_rect.center().y + 8.0),
            &[("CH2".into(), egui::Color32::from_rgb(70, 205, 255))],
            egui::Color32::from_rgba_unmultiplied(70, 205, 255, 140),
        );
    } else if phosphor.mode == TraceMode::Xy && phosphor.xy_square {
        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("xy_square_label"),
        ));
        draw_text_box(
            &p,
            egui::pos2(plot_rect.center().x - plot_rect.height() * 0.5 + 6.0, plot_rect.top() + 6.0),
            &[("XY 1:1 (CH1->X, CH2->Y)".into(), egui::Color32::from_rgb(130, 210, 250))],
            egui::Color32::from_rgba_unmultiplied(80, 190, 245, 140),
        );
    }
    if do_undo {
        st.undo_zoom(phosphor, now);
    }
    if do_redo {
        st.redo_zoom(phosphor, now);
    }
    if do_home {
        st.zoom_one_to_one(phosphor, now);
    }

    // ── 3. Anotaciones ancladas (t, y) ───────────────────────────────────────
    if st.annotations_visible && !st.annotations.is_empty() {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("annotations_layer"),
        ));
        for i in 0..st.annotations.len() {
            let (fx, fy, color_idx) = {
                let a = &st.annotations[i];
                (a.frac_x, a.frac_y, a.color_idx)
            };
            let x = frac_to_x(fx);
            let y = plot_rect.center().y - fy * plot_rect.height();
            if x < plot_rect.left() || x > plot_rect.right() || y < plot_rect.top() || y > plot_rect.bottom() {
                continue;
            }
            let anchor = egui::pos2(x, y);
            let col = ANNOT_COLORS[color_idx % ANNOT_COLORS.len()];
            let flip_x = x > plot_rect.right() - 170.0;
            let flip_y = y < plot_rect.top() + 50.0;
            let label_off = egui::vec2(if flip_x { -34.0 } else { 34.0 }, if flip_y { 30.0 } else { -30.0 });
            let label_pt = anchor + label_off;
            painter.line_segment([anchor, label_pt], egui::Stroke::new(1.5, col));
            // Punta de flecha en el ancla
            let dir = (label_pt - anchor).normalized();
            let perp = egui::vec2(-dir.y, dir.x);
            painter.add(egui::Shape::convex_polygon(
                vec![anchor, anchor + dir * 10.0 + perp * 4.0, anchor + dir * 10.0 - perp * 4.0],
                col,
                egui::Stroke::NONE,
            ));
            let text = st.annotations[i].text.clone();
            let lines = vec![(text, col)];
            let galley_w = painter
                .layout_no_wrap(lines[0].0.clone(), egui::FontId::monospace(10.5), col)
                .size()
                .x
                + 12.0;
            let tl = if flip_x {
                egui::pos2(label_pt.x - galley_w, label_pt.y - if flip_y { 0.0 } else { 18.0 })
            } else {
                egui::pos2(label_pt.x, label_pt.y - if flip_y { 0.0 } else { 18.0 })
            };
            draw_text_box(&painter, tl, &lines, col);

            // Asa arrastrable sobre el ancla: mueve la anotación en (t, y)
            let handle = egui::Rect::from_center_size(anchor, egui::vec2(18.0, 18.0));
            let resp = egui::Area::new(egui::Id::new(("annot_handle", i)))
                .fixed_pos(handle.min)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let (_, r) = ui.allocate_exact_size(handle.size(), egui::Sense::drag());
                    r
                })
                .inner;
            if resp.dragged() {
                if let Some(p) = ctx.pointer_interact_pos() {
                    st.annotations[i].frac_x = x_to_frac(p.x).clamp(0.0, 1.0);
                    st.annotations[i].frac_y =
                        ((plot_rect.center().y - p.y) / plot_rect.height()).clamp(-0.5, 0.5);
                }
            }
            resp.on_hover_text("Arrastre para mover la anotación");
        }
    }

    // ── 4. Pointer Tool Tip ──────────────────────────────────────────────────
    if st.pointer_tip && !deep.on && !ctx.egui_wants_pointer_input() && !cur.dragging() {
        if let Some(pos) = ctx.pointer_hover_pos() {
            if plot_rect.contains(pos) {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("pointer_tip_layer"),
                ));
                let cross = egui::Stroke::new(0.8, egui::Color32::from_rgba_unmultiplied(200, 220, 255, 70));
                painter.line_segment(
                    [egui::pos2(pos.x, plot_rect.top()), egui::pos2(pos.x, plot_rect.bottom())],
                    cross,
                );
                painter.line_segment(
                    [egui::pos2(plot_rect.left(), pos.y), egui::pos2(plot_rect.right(), pos.y)],
                    cross,
                );
                let rec_frac = x_to_frac(pos.x);
                let t_rel = (rec_frac as f64 - link.config.position) * record_s;
                let mut lines = vec![(
                    format!("t = {}  (desde trigger)", fmt_si(t_rel, "s").trim()),
                    egui::Color32::from_gray(235),
                )];
                if let Some(frame) = link.latest.as_ref() {
                    for ch in 0..2 {
                        if !link.config.channels[ch].enabled {
                            continue;
                        }
                        if let Some(cap) = frame.channels.iter().find(|c| c.ch == ch) {
                            if let Some(v) = sample_volts_at_frac(cap, rec_frac.clamp(0.0, 1.0)) {
                                let spec = &auto.ch_ext[ch].probe_spec;
                                let col = if ch == 0 {
                                    egui::Color32::from_rgb(255, 215, 55)
                                } else {
                                    egui::Color32::from_rgb(70, 205, 255)
                                };
                                lines.push((format!("CH{}: {}", ch + 1, spec.format_scaled_value(v).trim()), col));
                            }
                        }
                    }
                }
                let est_w = 210.0;
                let est_h = 18.0 * lines.len() as f32 + 8.0;
                let tl = egui::pos2(
                    if pos.x + 16.0 + est_w > plot_rect.right() { pos.x - 16.0 - est_w } else { pos.x + 16.0 },
                    if pos.y + 16.0 + est_h > plot_rect.bottom() { pos.y - 16.0 - est_h } else { pos.y + 16.0 },
                );
                draw_text_box(&painter, tl, &lines, egui::Color32::from_rgb(88, 168, 238));
            }
        }
    }
}

/// Panel de la columna adyacente: herramientas de la gráfica y lista de anotaciones.
pub fn show_annotations_panel(ui: &mut egui::Ui, st: &mut PlotToolsState, cur: &CursorState) {
    ui.spacing_mut().item_spacing.y = 5.0;
    ui.label(egui::RichText::new("Herramientas de la gráfica").strong().size(11.5));
    ui.checkbox(&mut st.pointer_tip, "Pointer Tool Tip (valores bajo el puntero)");
    ui.checkbox(&mut st.over_range_alert, "Alerta ! Over-Range por canal");
    ui.checkbox(&mut st.annotations_visible, "Mostrar anotaciones");

    ui.separator();
    ui.label(egui::RichText::new("Anotaciones").strong().size(11.5));
    ui.horizontal(|ui| {
        if ui.button("➕ Agregar").on_hover_text("Crea una anotación (en X1 si hay reglas de tiempo)").clicked() {
            let fx = if cur.time_on { cur.pos[0] } else { 0.5 };
            let n = st.annotations.len();
            st.annotations.push(Annotation {
                frac_x: fx,
                frac_y: 0.2,
                text: format!("Nota {}", n + 1),
                color_idx: n % ANNOT_COLORS.len(),
            });
            st.annotations_visible = true;
        }
        if ui.add_enabled(!st.annotations.is_empty(), egui::Button::new("🗑 Borrar todas")).clicked() {
            st.annotations.clear();
        }
    });

    let mut remove: Option<usize> = None;
    for (i, a) in st.annotations.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut a.text).desired_width(130.0));
                if ui.small_button("🗑").on_hover_text("Eliminar").clicked() {
                    remove = Some(i);
                }
            });
            ui.horizontal(|ui| {
                for (ci, c) in ANNOT_COLORS.iter().enumerate() {
                    let sel = a.color_idx == ci;
                    let b = egui::Button::new("  ")
                        .fill(*c)
                        .stroke(egui::Stroke::new(if sel { 2.0 } else { 0.5 }, egui::Color32::WHITE));
                    if ui.add(b).clicked() {
                        a.color_idx = ci;
                    }
                }
            });
            ui.horizontal(|ui| {
                let mut xp = a.frac_x * 100.0;
                let mut yp = a.frac_y * 100.0;
                ui.label("X");
                if ui.add(egui::DragValue::new(&mut xp).range(0.0..=100.0).suffix(" %")).changed() {
                    a.frac_x = xp / 100.0;
                }
                ui.label("Y");
                if ui.add(egui::DragValue::new(&mut yp).range(-50.0..=50.0).suffix(" %")).changed() {
                    a.frac_y = yp / 100.0;
                }
            });
        });
    }
    if let Some(i) = remove {
        st.annotations.remove(i);
    }
    ui.label(
        egui::RichText::new("Arrastre el extremo de la flecha sobre la cuadrícula para anclar la nota.")
            .weak()
            .small(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_gestures_collapse_into_one_history_entry() {
        let mut st = PlotToolsState::default();
        let mut p = Phosphor::default();
        st.track_zoom(&p, 0.0);
        // Gesto continuo: tres cambios seguidos = una sola entrada
        for (i, span) in [0.8, 0.6, 0.4].iter().enumerate() {
            p.hview = (0.5, *span);
            p.zoom_on = true;
            st.track_zoom(&p, 1.0 + i as f64 * 0.1);
        }
        assert_eq!(st.zoom_undo.len(), 1);
        st.undo_zoom(&mut p, 3.0);
        assert!(!p.zoom_on);
        assert!((p.hview.1 - 1.0).abs() < 1e-9);
        st.redo_zoom(&mut p, 3.1);
        assert!((p.hview.1 - 0.4).abs() < 1e-9);
    }
}
