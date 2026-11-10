//! Touchscreen-style pointer control on the plot, mapped to the mouse the
//! way modern scopes map gestures: drag the trigger-level line to move the
//! level, drag the waveform to move the selected channel's offset
//! (vertical) and the horizontal zoom window (horizontal), scroll to step
//! volts/div, shift+scroll to zoom the horizontal window at the pointer,
//! and a 2-D wheel's x axis to pan the window. Double-click resets the
//! window. Zoom/pan share the `view` ops with the dock toolbar, keys, and
//! scripts.
//!
//! Priority: measurement-cursor drags (cursors.rs, runs earlier) win; this
//! system stands down while one is active.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;
use neowon_backend::ScopeConfig;

use crate::Link;
use crate::cursors::CursorState;
use crate::gpu::Phosphor;
use crate::ui::layout::Layout;
use crate::view;

/// End of the newest recorded acquisition on the session clock — where
/// "follow live" points.
fn newest_time(rec: &crate::record::Recorder) -> f64 {
    rec.frames
        .last()
        .map(|f| f.t_start() + f.duration())
        .unwrap_or(0.0)
}

/// Second click within this window counts as a double-click.
const DOUBLE_CLICK_S: f64 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drag {
    TriggerLevel,
    TriggerPosition,
    OffsetMarker(usize),
    Waveform,
}

#[derive(Resource, Default)]
pub struct TouchState {
    drag: Option<Drag>,
    last_world: Vec2,
    /// Time of the last press inside the plot (double-click detection).
    last_click: f64,
    right_drag: bool,
    right_last_world: Vec2,
}

fn world_pos(
    windows: &Query<&Window>,
    camera: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let (Ok(window), Ok((camera, cam_tf))) = (windows.single(), camera.single()) else {
        // A broken query here kills all plot pointer control — never silent.
        bevy::log::warn_once!("plot pointer: window/camera query failed; gestures disabled");
        return None;
    };
    let screen = window.cursor_position()?;
    camera.viewport_to_world_2d(cam_tf, screen).ok()
}

fn on_plot(l: &Layout, world: Vec2) -> bool {
    let half = Vec2::new(l.plot.width(), l.plot.height()) / 2.0;
    (world - l.plot_center).abs().cmple(half).all()
}

/// World y of the trigger-level line (same mapping as `draw_trigger`).
pub fn trigger_line_y(l: &Layout, config: &ScopeConfig) -> f32 {
    let src = config.trigger.source.min(config.channels.len() - 1);
    let ch = &config.channels[src];
    let range = ch.volts_div * 10.0 * ch.probe;
    let frac = (config.trigger.level / range + ch.offset).clamp(-0.44, 0.44);
    l.frac_to_world_y(frac as f32)
}

/// Marker/waveform hit test for a press inside the plot. Priority: channel
/// offset markers (left edge) > trigger position marker (top edge) >
/// trigger level line > waveform (pan).
pub fn hit_drag(layout: &Layout, config: &ScopeConfig, world: Vec2) -> Drag {
    let left = layout.plot_center.x - layout.plot.width() / 2.0;
    let right = layout.plot_center.x + layout.plot.width() / 2.0;
    let top = layout.plot_center.y + layout.plot.height() / 2.0;
    // Marcadores de referencia cero (triángulo en el borde izquierdo)
    if world.x - left < 24.0 {
        for ch in 0..2 {
            let c = &config.channels[ch];
            if c.enabled {
                let y = layout.frac_to_world_y(c.offset as f32);
                if (world.y - y).abs() < 14.0 {
                    return Drag::OffsetMarker(ch);
                }
            }
        }
    }
    // Marcador de posición de trigger (triángulo en el borde superior)
    if top - world.y < 24.0 {
        let x = left + config.position as f32 * layout.plot.width();
        if (world.x - x).abs() < 14.0 {
            return Drag::TriggerPosition;
        }
    }
    // Marcador de nivel de trigger (triángulo en el borde derecho)
    // Solo se activa en el triángulo derecho para no interferir en el centro de la pantalla
    if right - world.x < 24.0 {
        let ty = trigger_line_y(layout, config);
        if (world.y - ty).abs() < 14.0 {
            return Drag::TriggerLevel;
        }
    }
    Drag::Waveform
}

#[allow(clippy::too_many_arguments)]
pub fn plot_pointer(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    egui_wants: Res<EguiWantsInput>,
    layout: Res<Layout>,
    cursors: Res<CursorState>,
    mut touch: ResMut<TouchState>,
    mut link: ResMut<Link>,
    mut phosphor: ResMut<Phosphor>,
    mut deep: ResMut<crate::deep::DeepView>,
    rec: Res<crate::record::Recorder>,
    mut hist: ResMut<crate::record::History>,
) {
    // Measurement cursors (earlier in the chain) own the pointer while
    // dragging; egui owns it over panels.
    if cursors.dragging() {
        touch.drag = None;
        touch.right_drag = false;
        return;
    }
    if touch.drag.is_none() && !touch.right_drag && egui_wants.wants_any_pointer_input() {
        wheel.clear();
        return;
    }
    let Some(world) = world_pos(&windows, &camera) else {
        touch.drag = None;
        touch.right_drag = false;
        return;
    };

    // --- Clic Derecho Sostenido: Desplazamiento Vertical de Waveform + Cero + Trigger ---
    if mouse.just_pressed(MouseButton::Right) && on_plot(&layout, world) {
        touch.right_drag = true;
        touch.right_last_world = world;
    }
    if !mouse.pressed(MouseButton::Right) {
        touch.right_drag = false;
    } else if touch.right_drag {
        let delta = world - touch.right_last_world;
        touch.right_last_world = world;
        if delta.y != 0.0 {
            let dfrac = (delta.y / (10.0 * layout.div.y)) as f64;
            let sel = link.selected.min(1);
            let off = (link.config.channels[sel].offset + dfrac).clamp(-0.5, 0.5);
            link.config.channels[sel].offset = off;
            link.dirty = true;
        }
    }

    // Scroll: volts/div ladder on the selected channel; shift = horizontal
    // zoom of the record window at the pointer; a 2-D wheel's x axis pans
    // the window.
    if on_plot(&layout, world) {
        let mut steps = [0.0f32; 2];
        for ev in wheel.read() {
            let d = match ev.unit {
                MouseScrollUnit::Line => [ev.y, ev.x],
                MouseScrollUnit::Pixel => [ev.y / 40.0, ev.x / 40.0],
            };
            steps[0] += d[0];
            steps[1] += d[1];
        }
        if steps[0].abs() >= 0.5 {
            let inward = steps[0] > 0.0;
            if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
                // Ctrl + Rueda: Zoom vertical (volts/div) del canal seleccionado
                let sel = link.selected.min(1);
                view::zoom_channel(&mut link, sel, inward);
            } else {
                // Rueda normal: Zoom horizontal DENTRO de la captura (tope estricto al 100%)
                let anchor = ((world.x - (layout.plot_center.x - layout.plot.width() / 2.0))
                    / layout.plot.width())
                .clamp(0.0, 1.0);
                let (c, s) = phosphor.hview;
                let anchor = (c - s / 2.0) + anchor as f64 * s;

                if !inward && s >= 0.99 {
                    // Ya está al 100%: fijar vista completa limpia sin desbordar ni activar timeline
                    view::hview_home(&mut phosphor);
                } else {
                    view::hview_zoom(&mut phosphor, anchor, inward);
                    if phosphor.hview.1 >= 0.999 {
                        view::hview_home(&mut phosphor);
                    }
                }
            }
        }
        if steps[1].abs() >= 0.5 {
            // Content follows the finger: swipe right slides the waveform
            // right (later data leaves the screen).
            let newest = newest_time(&rec);
            view::hposition_timeline(
                &mut link,
                &mut phosphor,
                &mut deep,
                newest,
                steps[1] as f64 * 0.02,
            );
        }
    } else {
        wheel.clear();
    }

    if mouse.just_pressed(MouseButton::Left) && on_plot(&layout, world) {
        let now = time.elapsed_secs_f64();
        let drag = hit_drag(&layout, &link.config, world);
        if drag == Drag::Waveform && now - touch.last_click < DOUBLE_CLICK_S {
            view::hview_home(&mut phosphor);
            touch.drag = None;
            touch.last_click = 0.0;
            return;
        }
        touch.last_click = now;
        if let Drag::OffsetMarker(ch) = drag {
            link.selected = ch;
        }
        touch.drag = Some(drag);
        touch.last_world = world;
    }
    if !mouse.pressed(MouseButton::Left) {
        touch.drag = None;
        return;
    }
    let Some(drag) = touch.drag else { return };
    let delta = world - touch.last_world;
    if delta == Vec2::ZERO {
        return;
    }
    touch.last_world = world;

    // One full-scale fraction per 10 divisions of screen (the encoding the
    // config uses; the visible window is 8 of those 10).
    let dfrac = (delta.y / (10.0 * layout.div.y)) as f64;
    match drag {
        Drag::TriggerPosition => {
            let pos =
                (link.config.position + (delta.x / layout.plot.width()) as f64).clamp(0.0, 1.0);
            link.config.position = pos;
            link.dirty = true;
        }
        Drag::OffsetMarker(ch) => {
            // Arrastre específico del triángulo de cero (borde izquierdo)
            let off = (link.config.channels[ch].offset + dfrac).clamp(-0.5, 0.5);
            link.config.channels[ch].offset = off;
            link.dirty = true;
        }
        Drag::TriggerLevel => {
            // Arrastre específico del triángulo de trigger (borde derecho)
            let src = link.config.trigger.source.min(1);
            let ch = link.config.channels[src];
            let range = ch.volts_div * 10.0 * ch.probe;
            link.config.trigger.level =
                (link.config.trigger.level + dfrac * range).clamp(-range, range);
            link.dirty = true;
        }
        Drag::Waveform => {
            // Clic izquierdo sostenido sobre la waveform:
            // Nivel de trigger y nivel de referencia se mantienen COMPLETAMENTE FIJOS.
            if !link.config.running && !view::zoom_active(&phosphor) {
                // En modo STOP y sin zoom (vista 1:1 de captura total):
                // Arrastrar horizontalmente navega suavemente entre las tramas del historial
                let n = rec.frames.len();
                if n > 1 {
                    let cur = hist.active.unwrap_or(n.saturating_sub(1)) as f64;
                    let step = (delta.x / 18.0) as f64;
                    let new_idx = (cur + step).clamp(0.0, (n - 1) as f64).round() as usize;
                    if Some(new_idx) != hist.active {
                        hist.show(&mut link, &rec, new_idx);
                    }
                }
            } else {
                // Modo RUN o Modo Zoom:
                // Desplazar lateralmente en el mismo sentido (pan horizontal).
                let d = (delta.x / layout.plot.width()) as f64;
                let newest = newest_time(&rec);
                view::hposition_timeline(&mut link, &mut phosphor, &mut deep, newest, d);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::compute(1520.0, 820.0, 1.0)
    }

    #[test]
    fn empty_plot_area_starts_waveform_pan() {
        let l = layout();
        let cfg = crate::view::startup_config();
        assert_eq!(hit_drag(&l, &cfg, l.plot_center), Drag::Waveform);
    }

    #[test]
    fn center_plot_near_trigger_line_is_waveform_pan() {
        let l = layout();
        let cfg = crate::view::startup_config();
        let y = trigger_line_y(&l, &cfg);
        // En el centro del osciloscopio jamás debe arrastrarse el nivel de trigger
        let world = Vec2::new(l.plot_center.x, y + 5.0);
        assert_eq!(hit_drag(&l, &cfg, world), Drag::Waveform);
    }

    #[test]
    fn right_edge_near_trigger_triangle_drags_the_level() {
        let l = layout();
        let cfg = crate::view::startup_config();
        let y = trigger_line_y(&l, &cfg);
        let right = l.plot_center.x + l.plot.width() / 2.0;
        // Solo en el triángulo del borde derecho debe arrastrarse el nivel de trigger
        let world = Vec2::new(right - 5.0, y + 5.0);
        assert_eq!(hit_drag(&l, &cfg, world), Drag::TriggerLevel);
    }

    #[test]
    fn left_edge_at_channel_zero_is_its_offset_marker() {
        let l = layout();
        let mut cfg = crate::view::startup_config();
        cfg.channels[0].enabled = true;
        cfg.channels[1].enabled = true;
        cfg.channels[1].offset = 0.25;
        let left = l.plot_center.x - l.plot.width() / 2.0;
        // CH1 marker sits at offset 0 (plot center in y).
        let world = Vec2::new(left + 5.0, l.frac_to_world_y(0.0));
        assert_eq!(hit_drag(&l, &cfg, world), Drag::OffsetMarker(0));
        // CH2 marker a quarter full-scale up.
        let world = Vec2::new(left + 5.0, l.frac_to_world_y(0.25));
        assert_eq!(hit_drag(&l, &cfg, world), Drag::OffsetMarker(1));
    }

    #[test]
    fn top_edge_at_position_is_the_trigger_marker() {
        let l = layout();
        let cfg = crate::view::startup_config();
        let left = l.plot_center.x - l.plot.width() / 2.0;
        let top = l.plot_center.y + l.plot.height() / 2.0;
        let x = left + cfg.position as f32 * l.plot.width();
        let world = Vec2::new(x, top - 5.0);
        assert_eq!(hit_drag(&l, &cfg, world), Drag::TriggerPosition);
    }
}
