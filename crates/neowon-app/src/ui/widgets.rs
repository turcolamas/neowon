use bevy_egui::egui;
use neowon_core::PulseCondition;

/// The VDS1022 prescaler ladder — 2.5 S/s to 100 MS/s, which on a 5000-point
/// record is 200 s/div down to 50 us/div.
pub use neowon_core::ladders::SCOPE_SAMPLE_RATES as FALLBACK_RATES;
/// Ladders used before the backend's capabilities arrive: the scope's own,
/// because a fallback that disagreed with what the instrument then
/// advertises would move the knob under the operator.
pub use neowon_core::ladders::SCOPE_VOLTS_DIV as FALLBACK_VDIV;
pub const PROBES: [f64; 7] = [1.0, 10.0, 20.0, 50.0, 100.0, 500.0, 1000.0];

/// Channel hues — shared with the GPU trace colors in `gpu.rs`.
pub const CH1_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 217, 26);
pub const CH2_COLOR: egui::Color32 = egui::Color32::from_rgb(51, 191, 255);
pub const MATH_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 89, 217);

/// Run-state badge colors (manual 8.5: Run = yellow, Stop = red).
pub const RUN_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 180, 30);
pub const STOP_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 60, 50);
pub const WAIT_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 140, 40);

pub fn channel_color(ch: usize) -> egui::Color32 {
    match ch {
        0 => CH1_COLOR,
        1 => CH2_COLOR,
        _ => MATH_COLOR,
    }
}

/// Discrete ladder selector (V/div, s/div, probe): scopes step ladders,
/// they don't slide.
pub fn ladder_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    current: &mut f64,
    ladder: &[f64],
    fmt_val: impl Fn(f64) -> String,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id)
            .selected_text(egui::RichText::new(fmt_val(*current)).monospace())
            .show_ui(ui, |ui| {
                for &v in ladder {
                    if ui
                        .selectable_label(
                            (*current - v).abs() < v * 1e-6,
                            egui::RichText::new(fmt_val(v)).monospace(),
                        )
                        .clicked()
                    {
                        *current = v;
                        changed = true;
                    }
                }
            });
    });
    changed
}

pub fn condition_combo(ui: &mut egui::Ui, id: &str, condition: &mut PulseCondition) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("When");
        egui::ComboBox::from_id_salt(id)
            .selected_text(condition.label())
            .show_ui(ui, |ui| {
                for c in PulseCondition::ALL {
                    if ui.selectable_label(*condition == c, c.label()).clicked() {
                        *condition = c;
                        changed = true;
                    }
                }
            });
    });
    changed
}

/// A chip drawn in the channel hue — descriptor-box building block
/// (manual 7.4: touching a descriptor box opens its dialog).
pub fn chip(
    ui: &mut egui::Ui,
    color: egui::Color32,
    text: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    // A real egui Button sized by its galley — text can never overflow the
    // box, whatever the font or content.
    let alpha = if enabled { 255 } else { 90 };
    let border = egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha);
    let fill = if selected {
        egui::Color32::from_rgb(28, 30, 38)
    } else {
        egui::Color32::from_rgb(16, 18, 24)
    };
    let text_color = if enabled { color } else { egui::Color32::GRAY };
    ui.add(
        egui::Button::new(egui::RichText::new(text).monospace().color(text_color))
            .fill(fill)
            .stroke(egui::Stroke::new(if selected { 2.5 } else { 1.5 }, border))
            .corner_radius(4.0)
            .min_size(egui::vec2(0.0, 40.0)),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepperAction {
    None,
    Prev,
    Next,
    ClickedValue,
}

/// Control selector tipo pastilla unificada [◀ Valor ▶] ultra compacto.
/// Ocupa exactamente el ancho disponible (sin desbordamiento), detecta clics en
/// flechas o texto, y soporta rueda del ratón (scroll up = Next, scroll down = Prev).
pub fn compact_stepper(
    ui: &mut egui::Ui,
    _id_salt: &str,
    text: &str,
    height: f32,
) -> StepperAction {
    let desired_size = egui::vec2(ui.available_width(), height);
    let (rect, resp) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let mut action = StepperAction::None;

    // Rueda del ratón cuando está en hover
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll > 2.0 {
            action = StepperAction::Next;
        } else if scroll < -2.0 {
            action = StepperAction::Prev;
        }
    }

    let arrow_w = 20.0_f32.min(rect.width() * 0.20);
    let left_arrow_rect = egui::Rect::from_min_size(rect.min, egui::vec2(arrow_w, rect.height()));
    let right_arrow_rect = egui::Rect::from_min_size(
        egui::pos2(rect.max.x - arrow_w, rect.min.y),
        egui::vec2(arrow_w, rect.height()),
    );
    let center_rect = egui::Rect::from_min_max(
        egui::pos2(left_arrow_rect.max.x, rect.min.y),
        egui::pos2(right_arrow_rect.min.x, rect.max.y),
    );

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let is_hover_left = pointer_pos.map_or(false, |p| left_arrow_rect.contains(p));
    let is_hover_right = pointer_pos.map_or(false, |p| right_arrow_rect.contains(p));
    let is_hover_center = pointer_pos.map_or(false, |p| center_rect.contains(p));

    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if left_arrow_rect.contains(pos) {
                action = StepperAction::Prev;
            } else if right_arrow_rect.contains(pos) {
                action = StepperAction::Next;
            } else {
                action = StepperAction::ClickedValue;
            }
        }
    }

    let painter = ui.painter_at(rect);
    let bg_color = egui::Color32::from_rgb(22, 26, 34);
    let border_color = if resp.hovered() {
        egui::Color32::from_rgb(70, 95, 130)
    } else {
        egui::Color32::from_rgb(45, 52, 65)
    };
    painter.rect_filled(rect, 3.0, bg_color);

    if is_hover_left {
        painter.rect_filled(left_arrow_rect, 3.0, egui::Color32::from_rgb(38, 48, 64));
    }
    if is_hover_right {
        painter.rect_filled(right_arrow_rect, 3.0, egui::Color32::from_rgb(38, 48, 64));
    }
    if is_hover_center {
        painter.rect_filled(center_rect, 0.0, egui::Color32::from_rgb(28, 33, 44));
    }

    painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, border_color), egui::StrokeKind::Inside);

    let sep_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 52, 65));
    painter.line_segment([left_arrow_rect.right_top(), left_arrow_rect.right_bottom()], sep_stroke);
    painter.line_segment([right_arrow_rect.left_top(), right_arrow_rect.left_bottom()], sep_stroke);

    let arrow_color = if resp.hovered() {
        egui::Color32::from_rgb(230, 235, 245)
    } else {
        egui::Color32::from_gray(160)
    };
    painter.text(
        left_arrow_rect.center(),
        egui::Align2::CENTER_CENTER,
        "◀",
        egui::FontId::monospace(10.0),
        arrow_color,
    );
    painter.text(
        right_arrow_rect.center(),
        egui::Align2::CENTER_CENTER,
        "▶",
        egui::FontId::monospace(10.0),
        arrow_color,
    );

    painter.text(
        center_rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::monospace(10.5),
        egui::Color32::WHITE,
    );

    action
}

