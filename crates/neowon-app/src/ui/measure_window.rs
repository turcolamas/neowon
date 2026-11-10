//! The measurements window: the full table, with room for it.
//!
//! Eighteen metrics across three sources does not fit in a 320-pixel rail —
//! the dock section could only ever show a handful at a time. This is the
//! same treatment the spectrum and 3D views get: a floating window that can
//! be sized and put where the reader wants it.

use bevy_egui::egui;

use crate::derived::{
    BICHANNEL_METRICS, Band, METRICS, MeasureState, SLOT_NAMES, SLOTS, Unit, fmt, fmt_opt_sticky,
};
use crate::ui::widgets::{CH1_COLOR, CH2_COLOR, MATH_COLOR};

fn slot_color(slot: usize) -> egui::Color32 {
    match slot {
        0 => CH1_COLOR,
        1 => CH2_COLOR,
        _ => MATH_COLOR,
    }
}

pub fn show(ctx: &egui::Context, meas: &mut MeasureState) {
    show_config_dialog(ctx, meas);

    if !meas.window {
        return;
    }
    let mut open = meas.window;
    egui::Window::new("Measurements")
        .open(&mut open)
        .default_width(560.0)
        .default_height(460.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (slot, name) in SLOT_NAMES.iter().enumerate() {
                    let on = meas.latest[slot].is_some();
                    ui.add_enabled_ui(on, |ui| {
                        let mut show = meas.show_slot[slot];
                        if ui
                            .checkbox(&mut show, *name)
                            .on_hover_text(if on {
                                "Show this source's column"
                            } else {
                                "Not measured: the source is off"
                            })
                            .changed()
                        {
                            meas.show_slot[slot] = show;
                        }
                    });
                }
                ui.separator();
                ui.checkbox(&mut meas.show_stats, "Statistics")
                    .on_hover_text("mean, min, max and standard deviation since the last reset");
                if ui.button("Reset").clicked() {
                    meas.reset_stats();
                }
                ui.separator();
                if ui.button(egui::RichText::new("⚙ Configurar Parámetros...").strong()).clicked() {
                    meas.config_dialog_open = true;
                }
            });
            ui.separator();

            let shown: Vec<usize> = (0..SLOTS)
                .filter(|&s| meas.show_slot[s] && meas.latest[s].is_some())
                .collect();
            if shown.is_empty() {
                ui.label(
                    egui::RichText::new("no source is being measured")
                        .weak()
                        .small(),
                );
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("meas-window")
                    .striped(true)
                    .min_col_width(52.0)
                    .show(ui, |ui| {
                        ui.label("");
                        for &slot in &shown {
                            ui.label(
                                egui::RichText::new(SLOT_NAMES[slot])
                                    .strong()
                                    .color(slot_color(slot)),
                            );
                            if meas.show_stats {
                                for h in ["mean", "min", "max", "σ"] {
                                    ui.label(egui::RichText::new(h).weak().size(10.0));
                                }
                            }
                            ui.label(egui::RichText::new("trend").weak().size(10.0));
                        }
                        ui.end_row();

                        for (i, (name, get, unit)) in METRICS.iter().enumerate() {
                            if !meas.show_metric[i] {
                                continue;
                            }
                            ui.label(*name);
                            for &slot in &shown {
                                value_cell(ui, meas, slot, i, get, *unit);
                            }
                            ui.end_row();
                        }

                        // Bichannel cross-channel rows (CH1 ↔ CH2)
                        let any_bi = meas.show_bichannel.iter().any(|&b| b);
                        if any_bi && shown.contains(&0) && shown.contains(&1) {
                            ui.label(egui::RichText::new("── BICANAL (CH1 ↔ CH2) ──").weak().size(10.0));
                            for &slot in &shown {
                                ui.label(egui::RichText::new(SLOT_NAMES[slot]).weak().size(10.0));
                                if meas.show_stats {
                                    for _ in 0..4 {
                                        ui.label("");
                                    }
                                }
                                ui.label("");
                            }
                            ui.end_row();

                            for (i, (name, get, unit)) in BICHANNEL_METRICS.iter().enumerate() {
                                if !meas.show_bichannel[i] {
                                    continue;
                                }
                                ui.label(egui::RichText::new(*name).strong().color(egui::Color32::from_rgb(100, 200, 255)));
                                bichannel_cell(ui, meas, i, get, *unit, &shown);
                                ui.end_row();
                            }
                        }
                    });
            });
        });
    meas.window = open;
}

/// Pop-up modal dialog for toggling individual measurements (matching PicoScope style).
pub fn show_config_dialog(ctx: &egui::Context, meas: &mut MeasureState) {
    if !meas.config_dialog_open {
        return;
    }
    let mut open = meas.config_dialog_open;
    egui::Window::new("⚙ Selección de Parámetros de Medición")
        .open(&mut open)
        .resizable(false)
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.columns(2, |cols| {
                    // Columna 1: Parámetros de Tiempo / Pulsos
                    cols[0].vertical(|ui| {
                        ui.checkbox(&mut meas.show_metric[0], "Period");
                        ui.checkbox(&mut meas.show_metric[1], "Frequency");
                        ui.checkbox(&mut meas.show_metric[2], "RiseTime");
                        ui.checkbox(&mut meas.show_metric[3], "FallTime");
                        ui.checkbox(&mut meas.show_metric[4], "+PulseWidth");
                        ui.checkbox(&mut meas.show_metric[5], "-PulseWidth");
                        ui.checkbox(&mut meas.show_metric[6], "+DutyCycle");
                        ui.checkbox(&mut meas.show_metric[7], "-DutyCycle");
                        ui.checkbox(&mut meas.show_metric[8], "Vpp");
                    });

                    // Columna 2: Parámetros de Tensión / Amplitud
                    cols[1].vertical(|ui| {
                        ui.checkbox(&mut meas.show_metric[9], "Vmax");
                        ui.checkbox(&mut meas.show_metric[10], "Vmin");
                        ui.checkbox(&mut meas.show_metric[11], "Vamp");
                        ui.checkbox(&mut meas.show_metric[12], "Vtop");
                        ui.checkbox(&mut meas.show_metric[13], "Vbase");
                        ui.checkbox(&mut meas.show_metric[14], "Overshoot");
                        ui.checkbox(&mut meas.show_metric[15], "Preshoot");
                        ui.checkbox(&mut meas.show_metric[16], "Vavg");
                        ui.checkbox(&mut meas.show_metric[17], "Vrms");
                        ui.checkbox(&mut meas.show_metric[18], "Cycle Vrms");
                        ui.checkbox(&mut meas.show_metric[19], "SlewRate");
                    });
                });
            });

            // Sección inferior: Mediciones cruzadas Bicanal (CH1 ↔ CH2)
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new("Bicanal (CH1 ↔ CH2):").strong().size(11.0));
                ui.checkbox(&mut meas.show_bichannel[0], "RiseDelay 1->2");
                ui.checkbox(&mut meas.show_bichannel[1], "FallDelay 1->2");
                ui.checkbox(&mut meas.show_bichannel[2], "Phase 1->2");
            });

            ui.horizontal(|ui| {
                if ui.button("Todos").clicked() {
                    meas.show_metric.fill(true);
                    meas.show_bichannel.fill(true);
                }
                if ui.button("Ninguno").clicked() {
                    meas.show_metric.fill(false);
                    meas.show_bichannel.fill(false);
                }
                if ui.button("Por Defecto").clicked() {
                    meas.show_metric.fill(false);
                    meas.show_metric[0] = true;
                    meas.show_metric[1] = true;
                    meas.show_metric[6] = true;
                    meas.show_metric[8] = true;
                    meas.show_metric[9] = true;
                    meas.show_metric[10] = true;
                    meas.show_metric[17] = true;
                    meas.show_bichannel.fill(false);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Cerrar").strong()).clicked() {
                        meas.config_dialog_open = false;
                    }
                });
            });
        });
    meas.config_dialog_open = open;
}

#[allow(clippy::type_complexity)]
fn value_cell(
    ui: &mut egui::Ui,
    meas: &mut MeasureState,
    slot: usize,
    i: usize,
    get: &fn(&crate::derived::Measurements) -> Option<f64>,
    unit: Unit,
) {
    let m = meas.latest[slot];
    let v = m.as_ref().and_then(get);
    let text = match meas.bands.get_mut(slot) {
        Some(b) => fmt_opt_sticky(v, unit, &mut b[i]),
        None => fmt_opt_sticky(v, unit, &mut Band::default()),
    };
    ui.label(egui::RichText::new(text).monospace());

    if meas.show_stats {
        let t = meas.stats.get(slot).map(|s| s[i]);
        for pick in [0usize, 1, 2, 3] {
            let cell = t.filter(|t| t.count > 0).map(|t| match pick {
                0 => t.mean,
                1 => t.min,
                2 => t.max,
                _ => t.std_dev(),
            });
            ui.label(
                egui::RichText::new(cell.map_or_else(|| "—".into(), |x| fmt(x, unit)))
                    .monospace()
                    .size(10.0)
                    .weak(),
            );
        }
    }
    sparkline(ui, meas, slot, i);
}

#[allow(clippy::type_complexity)]
fn bichannel_cell(
    ui: &mut egui::Ui,
    meas: &mut MeasureState,
    i: usize,
    get: &fn(&neowon_dsp::BichannelMeasurements) -> Option<f64>,
    unit: Unit,
    shown: &[usize],
) {
    let v = meas.bichannel.as_ref().and_then(get);
    let text = fmt_opt_sticky(v, unit, &mut meas.bichannel_bands[i]);
    ui.label(egui::RichText::new(text).monospace());

    if meas.show_stats {
        let t = &meas.bichannel_stats[i];
        for pick in [0usize, 1, 2, 3] {
            let cell = if t.count > 0 {
                match pick {
                    0 => Some(t.mean),
                    1 => Some(t.min),
                    2 => Some(t.max),
                    _ => Some(t.std_dev()),
                }
            } else {
                None
            };
            ui.label(
                egui::RichText::new(cell.map_or_else(|| "—".into(), |x| fmt(x, unit)))
                    .monospace()
                    .size(10.0)
                    .weak(),
            );
        }
    }

    // Sparkline bicanal
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(72.0, 14.0), egui::Sense::hover());
    let ring = &meas.bichannel_history[i];
    if ring.len() >= 2 {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for &val in ring {
            if val.is_finite() {
                lo = lo.min(val);
                hi = hi.max(val);
            }
        }
        if lo.is_finite() && hi.is_finite() {
            let span = (hi - lo).max(f64::EPSILON.max((hi.abs() + lo.abs()) * 1e-9));
            let pts: Vec<egui::Pos2> = ring
                .iter()
                .enumerate()
                .map(|(k, &val)| {
                    let x = rect.left() + rect.width() * k as f32 / (ring.len() - 1) as f32;
                    let f = if hi > lo { (val - lo) / span } else { 0.5 };
                    egui::pos2(x, rect.bottom() - rect.height() * f as f32)
                })
                .collect();
            ui.painter().add(egui::Shape::line(
                pts,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 255)),
            ));
            resp.on_hover_text(format!(
                "last {} acquisitions\nrange {:.6} … {:.6}",
                ring.len(),
                lo,
                hi
            ));
        }
    }

    // Pad if more than 1 slot is shown
    for _ in 1..shown.len() {
        ui.label(egui::RichText::new("—").weak().monospace());
        if meas.show_stats {
            for _ in 0..4 {
                ui.label(egui::RichText::new("—").weak().monospace());
            }
        }
        ui.allocate_exact_size(egui::vec2(72.0, 14.0), egui::Sense::hover());
    }
}

/// A small trend line for one metric: enough to see drift or jitter that a
/// single number and a standard deviation both hide.
fn sparkline(ui: &mut egui::Ui, meas: &MeasureState, slot: usize, i: usize) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(72.0, 14.0), egui::Sense::hover());
    let Some(ring) = meas.history.get(slot).map(|h| &h[i]) else {
        return;
    };
    if ring.len() < 2 {
        return;
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in ring {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return;
    }
    // A flat trace is the common and desirable case; centre it rather than
    // amplifying float noise into a fake wiggle.
    let span = (hi - lo).max(f64::EPSILON.max((hi.abs() + lo.abs()) * 1e-9));
    let pts: Vec<egui::Pos2> = ring
        .iter()
        .enumerate()
        .map(|(k, &v)| {
            let x = rect.left() + rect.width() * k as f32 / (ring.len() - 1) as f32;
            let f = if hi > lo { (v - lo) / span } else { 0.5 };
            egui::pos2(x, rect.bottom() - rect.height() * f as f32)
        })
        .collect();
    ui.painter().add(egui::Shape::line(
        pts,
        egui::Stroke::new(1.0, slot_color(slot).gamma_multiply(0.8)),
    ));
    resp.on_hover_text(format!(
        "last {} acquisitions\nrange {:.6} … {:.6}",
        ring.len(),
        lo,
        hi
    ));
}
