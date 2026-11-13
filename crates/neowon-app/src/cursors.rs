//! Measurement cursors: a time pair and an amplitude pair, draggable with
//! the mouse on the plot area.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

use crate::gpu::Phosphor;
use crate::ui::layout::Layout;

/// Record fraction -> visible fraction of the horizontal zoom window.
fn view_frac(pos: f32, hview: (f64, f64)) -> f32 {
    let (center, span) = hview;
    ((pos as f64 - (center - span / 2.0)) / span) as f32
}

/// Visible fraction -> record fraction (inverse of `view_frac`).
fn record_frac(xf: f32, hview: (f64, f64)) -> f32 {
    let (center, span) = hview;
    (xf as f64 * span + (center - span / 2.0)) as f32
}

/// Modo de regla vertical/amplitud por canal (Estilo PicoScope 7 — Cap. 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelRulerMode {
    #[default]
    Off,
    /// Reglas horizontales Y1, Y2 arrastrables manualmente en la cuadrícula.
    Manual,
    /// Sigue automáticamente el valor instantáneo de la forma de onda en los cursores X1 y X2.
    Track,
}

/// Límites predefinidos para las Reglas de Fase (Estilo PicoScope 7 — Cap. 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhasePreset {
    Deg360,
    #[default]
    Deg720,
    Percent100,
    Custom,
}

/// Cursor indices: 0/1 = time (x, fraction 0..1 of the record),
/// 2/3 = CH1 amplitude (y, fraction -0.5..0.5 of full scale = +-5 divisions),
/// 4/5 = phase reference rulers (0° and 360°/720°),
/// 6/7 = CH2 amplitude (y, fraction -0.5..0.5 of full scale).
#[derive(Resource)]
pub struct CursorState {
    pub time_on: bool,
    pub amp_on: bool,
    /// Modo de regla por canal (CH1, CH2): Off, Manual o Track.
    pub ch_mode: [ChannelRulerMode; 2],
    /// Bloquea el intervalo Δt entre X1 y X2 al arrastrar cualquiera de los dos (Cap. 5.5).
    pub lock_dt: bool,
    /// Bloquea el intervalo ΔY de las reglas manuales por canal (CH1, CH2).
    pub lock_dy: [bool; 2],
    /// Activa las Reglas de Fase / Giro de Motor (0° a 360° o 720°).
    pub phase_on: bool,
    /// Posiciones normalizadas (0..1 del registro) para el límite inferior y superior de fase.
    pub phase_ref: [f32; 2],
    /// Grados del ciclo completo entre las reglas de fase (360.0° o 720.0°).
    pub phase_cycle_deg: f32,
    /// Preset activo de límites de fase (360°, 720°, 100%, Custom).
    pub phase_preset: PhasePreset,
    /// Límite inferior de fase (por defecto 0.0).
    pub phase_lower: f32,
    /// Límite superior de fase (por defecto 720.0).
    pub phase_upper: f32,
    /// Símbolo de unidad para las reglas de fase ("°", "%", etc.).
    pub phase_units: String,
    /// Número de particiones iguales entre las reglas de fase (por defecto 4).
    pub phase_partitions: u32,
    /// Envolver valores de ángulo fuera de [phase_lower, phase_upper].
    pub phase_wrap: bool,
    /// Minimiza la tabla flotante de lecturas de reglas sobre la cuadrícula.
    pub rulers_table_minimized: bool,
    /// On-graph handles (trigger level/position, channel offsets).
    pub markers: bool,
    pub pos: [f32; 4],
    /// Posiciones Y1, Y2 para las reglas manuales de CH2.
    pub ch2_pos: [f32; 2],
    /// Sticky SI bands for the dialog readouts (Δt, 1/Δt, ΔV).
    pub bands: [crate::derived::Band; 3],
    drag: Option<usize>,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            time_on: false,
            amp_on: false,
            ch_mode: [ChannelRulerMode::Off; 2],
            lock_dt: false,
            lock_dy: [false; 2],
            phase_on: false,
            phase_ref: [0.15, 0.85],
            phase_cycle_deg: 720.0,
            phase_preset: PhasePreset::Deg720,
            phase_lower: 0.0,
            phase_upper: 720.0,
            phase_units: "°".to_string(),
            phase_partitions: 4,
            phase_wrap: false,
            rulers_table_minimized: false,
            markers: true,
            pos: [0.25, 0.75, -0.12, 0.12],
            ch2_pos: [-0.18, 0.18],
            bands: Default::default(),
            drag: None,
        }
    }
}

impl CursorState {
    /// True while a cursor drag owns the pointer (touch control stands down).
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn any_ruler_active(&self) -> bool {
        self.time_on
            || self.amp_on
            || self.ch_mode[0] != ChannelRulerMode::Off
            || self.ch_mode[1] != ChannelRulerMode::Off
            || self.phase_on
    }

    pub fn sync_amp_flag(&mut self) {
        self.amp_on = self.ch_mode[0] == ChannelRulerMode::Manual
            || self.ch_mode[1] == ChannelRulerMode::Manual;
    }

    pub fn x_world(&self, l: &Layout, hview: (f64, f64), i: usize) -> f32 {
        l.plot_center.x + (view_frac(self.pos[i], hview) - 0.5) * l.plot.width()
    }

    pub fn phase_x_world(&self, l: &Layout, hview: (f64, f64), p_idx: usize) -> f32 {
        l.plot_center.x + (view_frac(self.phase_ref[p_idx], hview) - 0.5) * l.plot.width()
    }

    pub fn y_world(&self, l: &Layout, i: usize) -> f32 {
        l.frac_to_world_y(self.pos[i])
    }

    pub fn ch2_y_world(&self, l: &Layout, idx: usize) -> f32 {
        l.frac_to_world_y(self.ch2_pos[idx])
    }

    /// Convierte una fracción de registro (0..1) a ángulo/fase según `phase_ref`, `phase_lower`, `phase_upper` y `phase_wrap`.
    pub fn pos_to_phase_deg(&self, pos_frac: f32) -> f32 {
        let span = self.phase_ref[1] - self.phase_ref[0];
        if span.abs() < 1e-6 {
            return self.phase_lower;
        }
        let deg_span = self.phase_upper - self.phase_lower;
        let raw = self.phase_lower + ((pos_frac - self.phase_ref[0]) / span) * deg_span;
        if self.phase_wrap && deg_span.abs() > 1e-3 {
            self.phase_lower + (raw - self.phase_lower).rem_euclid(deg_span.abs())
        } else {
            raw
        }
    }
}

pub fn cursor_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    egui_wants: Res<EguiWantsInput>,
    layout: Res<Layout>,
    phosphor: Res<Phosphor>,
    mut cur: ResMut<CursorState>,
) {
    if cur.drag.is_none() && egui_wants.wants_any_pointer_input() {
        return;
    }
    let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), camera.single()) else {
        // A broken query here kills all on-graph dragging — never silent.
        warn_once!("cursor_input: window/camera query failed; drags disabled");
        return;
    };
    let Some(screen) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, screen) else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        let mut best: Option<(usize, f32)> = None;
        let mut consider = |i: usize, d: f32| {
            if d < 12.0 && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        };
        if cur.time_on {
            consider(0, (world.x - cur.x_world(&layout, phosphor.hview, 0)).abs());
            consider(1, (world.x - cur.x_world(&layout, phosphor.hview, 1)).abs());
        }
        if cur.amp_on || cur.ch_mode[0] == ChannelRulerMode::Manual {
            consider(2, (world.y - cur.y_world(&layout, 2)).abs());
            consider(3, (world.y - cur.y_world(&layout, 3)).abs());
        }
        if cur.phase_on {
            consider(4, (world.x - cur.phase_x_world(&layout, phosphor.hview, 0)).abs());
            consider(5, (world.x - cur.phase_x_world(&layout, phosphor.hview, 1)).abs());
        }
        if cur.ch_mode[1] == ChannelRulerMode::Manual {
            consider(6, (world.y - cur.ch2_y_world(&layout, 0)).abs());
            consider(7, (world.y - cur.ch2_y_world(&layout, 1)).abs());
        }
        cur.drag = best.map(|(i, _)| i);
    }
    if mouse.pressed(MouseButton::Left) {
        if let Some(i) = cur.drag {
            if i < 2 {
                let xf = (world.x - layout.plot_center.x) / layout.plot.width() + 0.5;
                let new_pos = record_frac(xf, phosphor.hview).clamp(0.0, 1.0);
                if cur.lock_dt {
                    let other = 1 - i;
                    let delta = new_pos - cur.pos[i];
                    let next_other = (cur.pos[other] + delta).clamp(0.0, 1.0);
                    let actual_delta = next_other - cur.pos[other];
                    cur.pos[i] = (cur.pos[i] + actual_delta).clamp(0.0, 1.0);
                    cur.pos[other] = next_other;
                } else {
                    cur.pos[i] = new_pos;
                }
            } else if i < 4 {
                let new_y =
                    ((world.y - layout.plot_center.y) / (10.0 * layout.div.y)).clamp(-0.48, 0.48);
                if cur.lock_dy[0] {
                    let other = if i == 2 { 3 } else { 2 };
                    let delta = new_y - cur.pos[i];
                    let next_other = (cur.pos[other] + delta).clamp(-0.48, 0.48);
                    let actual_delta = next_other - cur.pos[other];
                    cur.pos[i] = (cur.pos[i] + actual_delta).clamp(-0.48, 0.48);
                    cur.pos[other] = next_other;
                } else {
                    cur.pos[i] = new_y;
                }
            } else if i < 6 {
                let p_idx = i - 4;
                let xf = (world.x - layout.plot_center.x) / layout.plot.width() + 0.5;
                cur.phase_ref[p_idx] = record_frac(xf, phosphor.hview).clamp(0.0, 1.0);
            } else if i < 8 {
                let c_idx = i - 6;
                let new_y =
                    ((world.y - layout.plot_center.y) / (10.0 * layout.div.y)).clamp(-0.48, 0.48);
                if cur.lock_dy[1] {
                    let other = 1 - c_idx;
                    let delta = new_y - cur.ch2_pos[c_idx];
                    let next_other = (cur.ch2_pos[other] + delta).clamp(-0.48, 0.48);
                    let actual_delta = next_other - cur.ch2_pos[other];
                    cur.ch2_pos[c_idx] = (cur.ch2_pos[c_idx] + actual_delta).clamp(-0.48, 0.48);
                    cur.ch2_pos[other] = next_other;
                } else {
                    cur.ch2_pos[c_idx] = new_y;
                }
            }
        }
    } else {
        cur.drag = None;
    }
}

pub fn draw_cursors(
    cur: Res<CursorState>,
    layout: Res<Layout>,
    phosphor: Res<Phosphor>,
    mut gizmos: Gizmos,
) {
    let w = layout.plot.width();
    let h = layout.plot.height();
    let o = layout.plot_center;
    let time_col = Color::srgba(0.88, 0.94, 1.0, 0.85);
    if cur.time_on {
        for i in 0..2 {
            let xf = view_frac(cur.pos[i], phosphor.hview);
            if !(0.0..=1.0).contains(&xf) {
                continue;
            }
            let x = cur.x_world(&layout, phosphor.hview, i);
            let mut y = o.y - h / 2.0;
            while y < o.y + h / 2.0 {
                let y2 = (y + 7.0).min(o.y + h / 2.0);
                gizmos.line_2d(Vec2::new(x, y), Vec2::new(x, y2), time_col);
                y += 12.0;
            }
        }
    }
    if cur.amp_on || cur.ch_mode[0] == ChannelRulerMode::Manual {
        let ch1_col = Color::srgba(1.0, 0.85, 0.20, 0.82);
        for i in 2..4 {
            let y = cur.y_world(&layout, i);
            let mut x = o.x - w / 2.0;
            while x < o.x + w / 2.0 {
                let x2 = (x + 7.0).min(o.x + w / 2.0);
                gizmos.line_2d(Vec2::new(x, y), Vec2::new(x2, y), ch1_col);
                x += 12.0;
            }
        }
    }
    if cur.ch_mode[1] == ChannelRulerMode::Manual {
        let ch2_col = Color::srgba(0.25, 0.80, 1.0, 0.82);
        for idx in 0..2 {
            let y = cur.ch2_y_world(&layout, idx);
            let mut x = o.x - w / 2.0;
            while x < o.x + w / 2.0 {
                let x2 = (x + 7.0).min(o.x + w / 2.0);
                gizmos.line_2d(Vec2::new(x, y), Vec2::new(x2, y), ch2_col);
                x += 12.0;
            }
        }
    }
    if cur.phase_on {
        let phase_col = Color::srgba(1.0, 0.68, 0.18, 0.85);
        for p_idx in 0..2 {
            let xf = view_frac(cur.phase_ref[p_idx], phosphor.hview);
            if !(0.0..=1.0).contains(&xf) {
                continue;
            }
            let x = cur.phase_x_world(&layout, phosphor.hview, p_idx);
            let mut y = o.y - h / 2.0;
            while y < o.y + h / 2.0 {
                let y2 = (y + 8.0).min(o.y + h / 2.0);
                gizmos.line_2d(Vec2::new(x, y), Vec2::new(x, y2), phase_col);
                y += 14.0;
            }
        }
        // Líneas de partición angular (Partitions — ej. 4 particiones = 180°, 360°, 540°)
        let parts = cur.phase_partitions.clamp(1, 16);
        if parts > 1 {
            let part_col = Color::srgba(1.0, 0.68, 0.18, 0.35);
            for k in 1..parts {
                let frac = cur.phase_ref[0]
                    + (cur.phase_ref[1] - cur.phase_ref[0]) * (k as f32 / parts as f32);
                let xf = view_frac(frac, phosphor.hview);
                if !(0.0..=1.0).contains(&xf) {
                    continue;
                }
                let x = o.x + (xf - 0.5) * w;
                let mut y = o.y - h / 2.0;
                while y < o.y + h / 2.0 {
                    let y2 = (y + 4.0).min(o.y + h / 2.0);
                    gizmos.line_2d(Vec2::new(x, y), Vec2::new(x, y2), part_col);
                    y += 12.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CursorState;

    #[test]
    fn phase_rulers_convert_positions_to_360_and_720_deg() {
        let mut cur = CursorState::default();
        cur.phase_ref = [0.20, 0.80];
        cur.phase_lower = 0.0;
        cur.phase_upper = 720.0;
        assert!((cur.pos_to_phase_deg(0.20) - 0.0).abs() < 1e-3);
        assert!((cur.pos_to_phase_deg(0.50) - 360.0).abs() < 1e-3);
        assert!((cur.pos_to_phase_deg(0.80) - 720.0).abs() < 1e-3);

        cur.phase_upper = 360.0;
        assert!((cur.pos_to_phase_deg(0.50) - 180.0).abs() < 1e-3);
    }
}
