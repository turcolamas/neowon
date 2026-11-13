//! Gizmo overlays over the plot: graticule, trigger level, pass/fail mask,
//! measurement guides, drag markers, zoom band and clip warnings.

use bevy::prelude::*;

use crate::gpu::{Phosphor, TraceMode};
// Screen geometry follows the reference scope: 10 horizontal x 8 vertical
// divisions (docs/ui-ux-research.md §6), computed at runtime from the window
// size (`ui::layout::Layout`).
use crate::ui::layout::{H_DIVS, Layout, V_DIVS};
use crate::{Link, cursors, derived, ui, view};

pub(crate) fn draw_graticule(
    layout: Res<Layout>,
    phosphor: Res<Phosphor>,
    mut gizmos: Gizmos,
) {
    let w = layout.plot.width();
    let h = layout.plot.height();
    let o = layout.plot_center;

    let x_left = o.x - w / 2.0;
    let x_right = o.x + w / 2.0;
    let y_bottom = o.y - h / 2.0;
    let y_top = o.y + h / 2.0;

    // Colores SOFT (sutiles, elegantes, no deslumbrantes sobre fondo negro puro)
    let border_color = Color::srgba(0.38, 0.44, 0.52, 0.40);
    let axis_color = Color::srgba(0.48, 0.56, 0.66, 0.50);
    let dot_color = Color::srgba(0.38, 0.44, 0.52, 0.28);
    let tick_color = Color::srgba(0.44, 0.52, 0.62, 0.38);

    // 1. Marco perimetral exterior
    gizmos.line_2d(Vec2::new(x_left, y_bottom), Vec2::new(x_right, y_bottom), border_color);
    gizmos.line_2d(Vec2::new(x_left, y_top), Vec2::new(x_right, y_top), border_color);
    gizmos.line_2d(Vec2::new(x_left, y_bottom), Vec2::new(x_left, y_top), border_color);
    gizmos.line_2d(Vec2::new(x_right, y_bottom), Vec2::new(x_right, y_top), border_color);

    // 2. Ejes centrales continuos (X = 0, Y = 0)
    gizmos.line_2d(Vec2::new(o.x, y_bottom), Vec2::new(o.x, y_top), axis_color);
    gizmos.line_2d(Vec2::new(x_left, o.y), Vec2::new(x_right, o.y), axis_color);

    // 3. Lineas de cuadricula internas con puntos soft (paso 0.2 div)
    let sub_steps_v = V_DIVS * 5;
    let dy = layout.div.y / 5.0;
    for i in 1..H_DIVS {
        if i == H_DIVS / 2 {
            continue; // Eje central vertical ya continuo
        }
        let x = x_left + i as f32 * layout.div.x;
        for s in 0..=sub_steps_v {
            let y = y_bottom + s as f32 * dy;
            gizmos.line_2d(Vec2::new(x, y - 0.5), Vec2::new(x, y + 0.5), dot_color);
        }
    }

    let sub_steps_h = H_DIVS * 5;
    let dx = layout.div.x / 5.0;
    for j in 1..V_DIVS {
        if j == V_DIVS / 2 {
            continue; // Eje central horizontal ya continuo
        }
        let y = y_bottom + j as f32 * layout.div.y;
        for s in 0..=sub_steps_h {
            let x = x_left + s as f32 * dx;
            gizmos.line_2d(Vec2::new(x - 0.5, y), Vec2::new(x + 0.5, y), dot_color);
        }
    }

    // 4. Marcas de calibracion soft sobre el eje horizontal central
    for s in 0..=sub_steps_h {
        let x = x_left + s as f32 * dx;
        let tick_len = if s % 5 == 0 { 4.0 } else { 2.0 };
        gizmos.line_2d(Vec2::new(x, o.y - tick_len), Vec2::new(x, o.y + tick_len), tick_color);
    }

    // 5. Marcas de calibracion soft sobre el eje vertical central
    for s in 0..=sub_steps_v {
        let y = y_bottom + s as f32 * dy;
        let tick_len = if s % 5 == 0 { 4.0 } else { 2.0 };
        gizmos.line_2d(Vec2::new(o.x - tick_len, y), Vec2::new(o.x + tick_len, y), tick_color);
    }

    // 6. Marcas de calibracion en los bordes exteriores (superior e inferior hacia adentro)
    for s in 0..=sub_steps_h {
        let x = x_left + s as f32 * dx;
        let tick_len = if s % 5 == 0 { 3.5 } else { 2.0 };
        gizmos.line_2d(Vec2::new(x, y_top), Vec2::new(x, y_top - tick_len), tick_color);
        gizmos.line_2d(Vec2::new(x, y_bottom), Vec2::new(x, y_bottom + tick_len), tick_color);
    }

    // 7. Marcas de calibracion en los bordes exteriores (izquierdo y derecho hacia adentro)
    for s in 0..=sub_steps_v {
        let y = y_bottom + s as f32 * dy;
        let tick_len = if s % 5 == 0 { 3.5 } else { 2.0 };
        gizmos.line_2d(Vec2::new(x_left, y), Vec2::new(x_left + tick_len, y), tick_color);
        gizmos.line_2d(Vec2::new(x_right, y), Vec2::new(x_right - tick_len, y), tick_color);
    }

    // 8. Cruces de calibracion soft (+) a mitad de cuadrante (+-5 div H, +-2.5 div V)
    for sx in [-5.0, 5.0] {
        for sy in [-2.5, 2.5] {
            let cx = o.x + sx * layout.div.x;
            let cy = o.y + sy * layout.div.y;
            let arm = 3.5;
            gizmos.line_2d(Vec2::new(cx - arm, cy), Vec2::new(cx + arm, cy), tick_color);
            gizmos.line_2d(Vec2::new(cx, cy - arm), Vec2::new(cx, cy + arm), tick_color);
        }
    }

    // 9. Indicadores Split-Grid CH1/CH2 (mitad superior CH1, mitad inferior CH2)
    if phosphor.split_grid && phosphor.mode != TraceMode::Xy {
        let sep_col = Color::srgba(0.65, 0.78, 0.92, 0.78);
        gizmos.line_2d(Vec2::new(x_left, o.y), Vec2::new(x_right, o.y), sep_col);
        let y_ch1 = o.y + h * 0.25;
        let y_ch2 = o.y - h * 0.25;
        let c1 = Color::srgba(1.0, 0.85, 0.1, 0.32);
        let c2 = Color::srgba(0.2, 0.75, 1.0, 0.32);
        let mut x = x_left;
        while x < x_right {
            let x2 = (x + 6.0).min(x_right);
            gizmos.line_2d(Vec2::new(x, y_ch1), Vec2::new(x2, y_ch1), c1);
            gizmos.line_2d(Vec2::new(x, y_ch2), Vec2::new(x2, y_ch2), c2);
            x += 12.0;
        }
    }

    // 10. Marco 1:1 cuadrado centrado para Vista XY 1:1 (10x10 div)
    if phosphor.mode == TraceMode::Xy && phosphor.xy_square {
        let half = h * 0.5;
        let sq_col = Color::srgba(0.32, 0.76, 0.96, 0.65);
        gizmos.line_2d(Vec2::new(o.x - half, o.y - half), Vec2::new(o.x + half, o.y - half), sq_col);
        gizmos.line_2d(Vec2::new(o.x - half, o.y + half), Vec2::new(o.x + half, o.y + half), sq_col);
        gizmos.line_2d(Vec2::new(o.x - half, o.y - half), Vec2::new(o.x - half, o.y + half), sq_col);
        gizmos.line_2d(Vec2::new(o.x + half, o.y - half), Vec2::new(o.x + half, o.y + half), sq_col);
    }
}

pub(crate) fn draw_trigger(
    link: Res<Link>,
    layout: Res<Layout>,
    auto: Option<Res<ui::automotive_dock::AutomotiveUiState>>,
    mut gizmos: Gizmos,
) {
    let w = layout.plot.width();
    let o = layout.plot_center;
    let src = link
        .config
        .trigger
        .source
        .min(link.config.channels.len() - 1);
    let ch = &link.config.channels[src];
    let range = (ch.volts_div * 10.0 * ch.probe).max(1e-9);
    // Fraction of full (10 div) range; the display window is +-4 div.
    let frac = (link.config.trigger.level / range + ch.offset).clamp(-0.49, 0.49);
    let y = layout.frac_to_world_y(frac as f32);
    gizmos.line_2d(
        Vec2::new(o.x - w / 2.0, y),
        Vec2::new(o.x + w / 2.0, y),
        Color::srgba(1.0, 0.5, 0.2, 0.55),
    );

    // Banda de armado por histéresis programable de la FPGA (Cap. 4.11)
    let hyst_frac = (link.config.trigger.hysteresis_pct as f64 / 100.0).clamp(0.005, 0.20);
    let arm_frac = match link.config.trigger.kind {
        neowon_core::TriggerKind::Edge {
            slope: neowon_core::Slope::Falling,
        } => (frac + hyst_frac).clamp(-0.49, 0.49),
        _ => (frac - hyst_frac).clamp(-0.49, 0.49),
    };
    let y_arm = layout.frac_to_world_y(arm_frac as f32);
    let arm_col = Color::srgba(1.0, 0.65, 0.25, 0.28);
    let mut x = o.x - w / 2.0;
    while x < o.x + w / 2.0 {
        let x2 = (x + 5.0).min(o.x + w / 2.0);
        gizmos.line_2d(Vec2::new(x, y_arm), Vec2::new(x2, y_arm), arm_col);
        x += 10.0;
    }

    // Umbrales duales cuando el Calificador de Ventana o Pulso Runt está activo (Cap. 4.3 / 4.6)
    if let Some(a) = auto.as_ref() {
        let dual_bounds = match a.trigger_qualifier.mode {
            picowon_auto::AdvancedTriggerMode::Window => Some((
                a.trigger_qualifier.window_lower_v,
                a.trigger_qualifier.window_upper_v,
                Color::srgba(0.35, 0.85, 1.0, 0.45),
            )),
            picowon_auto::AdvancedTriggerMode::RuntPulse => Some((
                a.trigger_qualifier.runt_low_v,
                a.trigger_qualifier.runt_high_v,
                Color::srgba(1.0, 0.35, 0.75, 0.45),
            )),
            _ => None,
        };
        if let Some((lo_v, hi_v, col)) = dual_bounds {
            for bound_v in [lo_v, hi_v] {
                let b_frac = (bound_v / range + ch.offset).clamp(-0.49, 0.49);
                let by = layout.frac_to_world_y(b_frac as f32);
                gizmos.line_2d(Vec2::new(o.x - w / 2.0, by), Vec2::new(o.x + w / 2.0, by), col);
            }
        }
    }
}

/// The pass/fail envelope as two dim-green polylines (lo and hi bounds).
pub(crate) fn draw_pf_mask(pf: Res<derived::PfState>, layout: Res<Layout>, mut gizmos: Gizmos) {
    let Some(mask) = &pf.mask else { return };
    if !pf.enabled || mask.lo.is_empty() {
        return;
    }
    let w = layout.plot.width();
    let o = layout.plot_center;
    let n = mask.lo.len();
    // Decimate so the gizmo stays cheap on a 5000-sample record.
    let step = (n / 500).max(1);
    let x_at = |i: usize| o.x - w / 2.0 + i as f32 / (n - 1).max(1) as f32 * w;
    // The display window is +-100 counts (+-4 div); pin beyond that.
    let y_at = |raw: f32| layout.frac_to_world_y(raw.clamp(-125.0, 125.0) / 250.0);
    let color = if pf.last_violations.is_empty() {
        Color::srgba(0.2, 0.72, 0.35, 0.65)
    } else {
        Color::srgba(0.95, 0.3, 0.25, 0.75)
    };
    for bounds in [&mask.lo, &mask.hi] {
        let points: Vec<Vec2> = (0..n)
            .step_by(step)
            .map(|i| Vec2::new(x_at(i), y_at(bounds[i])))
            .collect();
        gizmos.linestrip_2d(points, color);
    }
    // Marcar muestras fuera de tolerancia en rojo brillante directamente sobre la envolvente
    if !pf.last_violations.is_empty() {
        let viol_col = Color::srgba(1.0, 0.18, 0.18, 0.95);
        let max_marks = 120;
        let v_step = (pf.last_violations.len() / max_marks).max(1);
        for &idx in pf.last_violations.iter().step_by(v_step) {
            if idx < n {
                let x = x_at(idx);
                let y0 = y_at(mask.lo[idx]);
                let y1 = y_at(mask.hi[idx]);
                gizmos.line_2d(Vec2::new(x, y0), Vec2::new(x, y1), viol_col);
            }
        }
    }
}

/// Measurement guides: dashed levels at Vtop/Vbase/Vavg and the 10%/90%
/// rise-time thresholds of the stats trace, drawn while the Measure dialog
/// is open (toggleable there).
pub(crate) fn draw_guides(
    meas: Res<derived::MeasureState>,
    math: Res<derived::MathState>,
    menus: Res<ui::MenuState>,
    link: Res<Link>,
    layout: Res<Layout>,
    mut gizmos: Gizmos,
) {
    if !meas.guides || !menus.is_open(ui::Menu::Measure) {
        return;
    }
    let slot = meas.stats_slot;
    let Some(m) = &meas.latest[slot] else { return };
    let scale = match slot {
        2 => math.trace.as_ref().map(|t| (t.cal.scale_i, t.cal.offset_i)),
        s => link
            .latest
            .as_ref()
            .and_then(|f| f.channels.iter().find(|c| c.ch == s))
            .map(|c| (c.cal.scale_i, c.cal.offset_i)),
    };
    let Some((lsb, zero)) = scale else { return };
    let base = match slot {
        0 => Color::srgb(1.0, 0.85, 0.1),
        1 => Color::srgb(0.2, 0.75, 1.0),
        _ => Color::srgb(1.0, 0.35, 0.85),
    };
    let w = layout.plot.width();
    let o = layout.plot_center;
    let lines = [
        (m.vtop, 0.55),
        (m.vbase, 0.55),
        (m.vavg, 0.4),
        (m.vbase + 0.1 * (m.vtop - m.vbase), 0.25),
        (m.vbase + 0.9 * (m.vtop - m.vbase), 0.25),
    ];
    for (v, alpha) in lines {
        let frac = (((v - zero) / lsb) / 250.0) as f32;
        if frac.abs() > 0.49 {
            continue; // outside the visible +-4-division window
        }
        let y = layout.frac_to_world_y(frac);
        let color = base.with_alpha(alpha);
        // Dashed: 6 px on / 6 px off.
        let mut x = o.x - w / 2.0;
        while x < o.x + w / 2.0 {
            let x2 = (x + 6.0).min(o.x + w / 2.0);
            gizmos.line_2d(Vec2::new(x, y), Vec2::new(x2, y), color);
            x += 12.0;
        }
    }
}

/// On-graph handles: trigger-level arrow at the right edge, trigger-position
/// arrow at the top edge, per-channel offset arrows at the left edge — all
/// draggable (ui::touch), all hidden by the Markers toggle.
pub(crate) fn draw_markers(
    link: Res<Link>,
    cur: Res<cursors::CursorState>,
    layout: Res<Layout>,
    mut gizmos: Gizmos,
) {
    if !cur.markers {
        return;
    }
    let w = layout.plot.width();
    let h = layout.plot.height();
    let o = layout.plot_center;
    let (left, right, top) = (o.x - w / 2.0, o.x + w / 2.0, o.y + h / 2.0);

    // Left-pointing arrowhead at the right edge: trigger level.
    let ty = ui::touch::trigger_line_y(&layout, &link.config);
    let tcol = Color::srgb(1.0, 0.55, 0.25);
    for d in 0..6 {
        let f = d as f32;
        gizmos.line_2d(
            Vec2::new(right - f, ty - (6.0 - f)),
            Vec2::new(right - f, ty + (6.0 - f)),
            tcol,
        );
    }

    // Down-pointing arrowhead at the top edge: trigger position.
    let tx = left + link.config.position as f32 * w;
    for d in 0..6 {
        let f = d as f32;
        gizmos.line_2d(
            Vec2::new(tx - (6.0 - f), top - f),
            Vec2::new(tx + (6.0 - f), top - f),
            tcol,
        );
    }

    // Right-pointing arrowheads at the left edge: channel zero offsets.
    for ch in 0..2 {
        let c = link.config.channels[ch];
        if !c.enabled {
            continue;
        }
        let y = layout.frac_to_world_y(c.offset as f32);
        let col = if ch == 0 {
            Color::srgb(1.0, 0.85, 0.1)
        } else {
            Color::srgb(0.2, 0.75, 1.0)
        };
        for d in 0..6 {
            let f = d as f32;
            gizmos.line_2d(
                Vec2::new(left + f, y - (6.0 - f)),
                Vec2::new(left + f, y + (6.0 - f)),
                col,
            );
        }
    }
}

/// Zoom-window band along the plot's top edge: a full-width track for the
/// record with the magnified slice highlighted. Scopes show the zoom region
/// as a box on the main sweep; the app has one grid, so the band is the
/// compact equivalent — without it a zoomed display gives no clue which part
/// of the record is on screen.
pub(crate) fn draw_zoom_band(phosphor: Res<Phosphor>, layout: Res<Layout>, mut gizmos: Gizmos) {
    if !view::zoom_active(&phosphor) {
        return;
    }
    let (w, h) = (layout.plot.width(), layout.plot.height());
    let o = layout.plot_center;
    let (left, top) = (o.x - w / 2.0, o.y + h / 2.0);
    let y = top + 5.0;
    gizmos.line_2d(
        Vec2::new(left, y),
        Vec2::new(left + w, y),
        Color::srgba(0.5, 0.5, 0.55, 0.5),
    );
    let (center, span) = phosphor.hview;
    let (x0, x1) = (
        left + (center - span / 2.0) as f32 * w,
        left + (center + span / 2.0) as f32 * w,
    );
    let col = Color::srgb(0.3, 0.9, 0.6);
    for dy in [-1.0f32, 0.0, 1.0] {
        gizmos.line_2d(Vec2::new(x0, y + dy), Vec2::new(x1, y + dy), col);
    }
    for x in [x0, x1] {
        gizmos.line_2d(Vec2::new(x, y - 4.0), Vec2::new(x, y + 4.0), col);
    }
}

/// Red clip arrows at the plot edge when a channel's samples sit on the ADC
/// rails — the honest companion to the shader's off-screen suppression.
pub(crate) fn draw_clip_warnings(link: Res<Link>, layout: Res<Layout>, mut gizmos: Gizmos) {
    let Some(frame) = &link.latest else { return };
    let w = layout.plot.width();
    let h = layout.plot.height();
    let o = layout.plot_center;
    let red = Color::srgb(1.0, 0.25, 0.2);
    for cap in &frame.channels {
        if !cap.clipped {
            continue;
        }
        let (mut top, mut bottom) = (false, false);
        for &r in &cap.data {
            top |= r >= 125.0;
            bottom |= r <= -125.0;
        }
        let x = o.x + w / 2.0 - 26.0 - cap.ch as f32 * 22.0;
        let mut arrow = |y: f32, dir: f32| {
            gizmos.line_2d(Vec2::new(x, y), Vec2::new(x, y + 12.0 * dir), red);
            gizmos.line_2d(
                Vec2::new(x, y + 12.0 * dir),
                Vec2::new(x - 4.0, y + 7.0 * dir),
                red,
            );
            gizmos.line_2d(
                Vec2::new(x, y + 12.0 * dir),
                Vec2::new(x + 4.0, y + 7.0 * dir),
                red,
            );
        };
        if top {
            arrow(o.y + h / 2.0 - 16.0, 1.0);
        }
        if bottom {
            arrow(o.y - h / 2.0 + 16.0, -1.0);
        }
    }
}
