use bevy_egui::egui;
use neowon_core::{PulseCondition, Slope, Sweep, TriggerKind, VideoSync};
use picowon_auto::{AdvancedTriggerMode, LogicCombinator};

use crate::Link;
use crate::ui::automotive_dock::AutomotiveUiState;

use super::widgets::condition_combo;

pub fn show(ui: &mut egui::Ui, link: &mut Link, auto: &mut AutomotiveUiState) {
    ui.group(|ui| {
        ui.strong("⚡ Disparo Hardware FPGA (VDS1022)");
        let mut t = link.config.trigger;
        let mut dirty = false;
        ui.horizontal(|ui| {
            ui.label("Fuente");
            for src in 0..2 {
                if ui
                    .selectable_label(t.source == src, format!("CH{}", src + 1))
                    .clicked()
                {
                    t.source = src;
                    dirty = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tipo HW");
            egui::ComboBox::from_id_salt("trigkind")
                .selected_text(t.kind.label())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(matches!(t.kind, TriggerKind::Edge { .. }), "Edge")
                        .clicked()
                    {
                        let slope = match t.kind {
                            TriggerKind::Edge { slope } => slope,
                            _ => Slope::Rising,
                        };
                        t.kind = TriggerKind::Edge { slope };
                        dirty = true;
                    }
                    if ui
                        .selectable_label(matches!(t.kind, TriggerKind::Pulse { .. }), "Pulse")
                        .clicked()
                    {
                        t.kind = TriggerKind::Pulse {
                            condition: PulseCondition::PositiveGreater,
                            width: 1e-6,
                        };
                        dirty = true;
                    }
                    if ui
                        .selectable_label(matches!(t.kind, TriggerKind::Slope { .. }), "Slope")
                        .clicked()
                    {
                        t.kind = TriggerKind::Slope {
                            condition: PulseCondition::PositiveGreater,
                            width: 1e-6,
                            upper: t.level + 0.1,
                            lower: t.level - 0.1,
                        };
                        dirty = true;
                    }
                    if ui
                        .selectable_label(matches!(t.kind, TriggerKind::Video { .. }), "Video")
                        .clicked()
                    {
                        t.kind = TriggerKind::Video {
                            sync: VideoSync::Line,
                            line: 1,
                        };
                        dirty = true;
                    }
                });
        });
        match &mut t.kind {
            TriggerKind::Edge { slope } => {
                ui.horizontal(|ui| {
                    ui.label("Flanco");
                    for (label, s) in [("Subida ⬈", Slope::Rising), ("Bajada ⬊", Slope::Falling)]
                    {
                        if ui.selectable_label(*slope == s, label).clicked() {
                            *slope = s;
                            dirty = true;
                        }
                    }
                });
            }
            TriggerKind::Pulse { condition, width } => {
                dirty |= condition_combo(ui, "pulsecond", condition);
                let mut w_us = *width * 1e6;
                ui.horizontal(|ui| {
                    ui.label("Ancho");
                    if ui
                        .add(
                            egui::DragValue::new(&mut w_us)
                                .speed(0.1)
                                .range(0.01..=655_360.0)
                                .suffix(" µs"),
                        )
                        .changed()
                    {
                        *width = w_us * 1e-6;
                        dirty = true;
                    }
                });
            }
            TriggerKind::Slope {
                condition,
                width,
                upper,
                lower,
            } => {
                dirty |= condition_combo(ui, "slopecond", condition);
                let mut w_us = *width * 1e6;
                ui.horizontal(|ui| {
                    ui.label("Tiempo");
                    if ui
                        .add(
                            egui::DragValue::new(&mut w_us)
                                .speed(0.1)
                                .range(0.01..=655_360.0)
                                .suffix(" µs"),
                        )
                        .changed()
                    {
                        *width = w_us * 1e-6;
                        dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Sup.");
                    if ui
                        .add(egui::DragValue::new(upper).speed(0.01).suffix(" V"))
                        .changed()
                    {
                        dirty = true;
                    }
                    ui.label("Inf.");
                    if ui
                        .add(egui::DragValue::new(lower).speed(0.01).suffix(" V"))
                        .changed()
                    {
                        dirty = true;
                    }
                });
            }
            TriggerKind::Video { sync, line } => {
                ui.horizontal(|ui| {
                    ui.label("Sync");
                    egui::ComboBox::from_id_salt("vidsync")
                        .selected_text(sync.label())
                        .show_ui(ui, |ui| {
                            for s in VideoSync::ALL {
                                if ui.selectable_label(*sync == s, s.label()).clicked() {
                                    *sync = s;
                                    dirty = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Línea #");
                    if ui
                        .add(egui::DragValue::new(line).range(0..=65535))
                        .changed()
                    {
                        dirty = true;
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label("Barrido");
            for (label, s) in [
                ("Auto", Sweep::Auto),
                ("Normal", Sweep::Normal),
                ("Single", Sweep::Single),
            ] {
                if ui.selectable_label(t.sweep == s, label).clicked() {
                    t.sweep = s;
                    if s == Sweep::Single {
                        link.config.running = true;
                    }
                    dirty = true;
                }
            }
        });
        if matches!(t.kind, TriggerKind::Edge { .. } | TriggerKind::Pulse { .. }) {
            let mut level = t.level;
            ui.horizontal(|ui| {
                ui.label("Nivel");
                if ui
                    .add(egui::DragValue::new(&mut level).speed(0.01).suffix(" V"))
                    .changed()
                {
                    t.level = level;
                    dirty = true;
                }
            });
        }
        let mut holdoff_us = t.holdoff * 1e6;
        ui.horizontal(|ui| {
            ui.label("Holdoff");
            if ui
                .add(
                    egui::DragValue::new(&mut holdoff_us)
                        .speed(0.1)
                        .range(0.1..=10_000_000.0)
                        .suffix(" µs"),
                )
                .changed()
            {
                t.holdoff = holdoff_us * 1e-6;
                dirty = true;
            }
        });

        // Histéresis programable de Hardware FPGA (Cap. 4.11)
        ui.horizontal(|ui| {
            ui.label("Histéresis HW");
            let mut hyst = t.hysteresis_pct;
            if ui
                .add(
                    egui::Slider::new(&mut hyst, 0.5..=15.0)
                        .step_by(0.5)
                        .suffix("% FS"),
                )
                .on_hover_text("Banda de histéresis programada en los registros de umbral de la FPGA (rechazo a ruido de ignición)")
                .changed()
            {
                t.hysteresis_pct = hyst;
                auto.trigger_hysteresis_pct = hyst;
                dirty = true;
            }
        });

        if dirty {
            link.config.trigger = t;
            link.dirty = true;
        }
    });

    ui.add_space(4.0);

    // =========================================================================
    // CALIFICADOR DE DISPARO AUTOMOTRIZ AVANZADO (Cap. 4.3 – 4.8)
    // =========================================================================
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("🦷 Calificador Automotriz Avanzado (Cap. 4)")
                .strong()
                .color(egui::Color32::from_rgb(255, 200, 85)),
        );

        let q = &mut auto.trigger_qualifier;
        egui::ComboBox::from_id_salt("adv_trig_qualifier_combo")
            .selected_text(q.mode.label())
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for m in AdvancedTriggerMode::ALL {
                    if ui.selectable_label(q.mode == m, m.label()).clicked() {
                        q.mode = m;
                        auto.trigger_qualifier_rt = Default::default();
                    }
                }
            });

        if q.mode != AdvancedTriggerMode::StandardHardware {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut q.hold_until_qualified, "Retener sólo válidas")
                    .on_hover_text("Descarta tramas que no contengan el evento calificado");
                ui.checkbox(&mut q.realign_to_trigger_pos, "Alinear evento")
                    .on_hover_text("Centra automáticamente el evento detectado en la marca horizontal de disparo");
            });

            ui.separator();

            match q.mode {
                AdvancedTriggerMode::StandardHardware => {}
                AdvancedTriggerMode::IntervalMissingTooth => {
                    ui.label(
                        egui::RichText::new("Sincronismo sobre hueco de rueda fónica CKP:")
                            .small()
                            .color(egui::Color32::from_rgb(175, 220, 255)),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Rueda:");
                        for (teeth, name) in [(60u16, "60-2"), (36u16, "36-1"), (24u16, "24-1")] {
                            if ui
                                .selectable_label(q.ckp_wheel_teeth == teeth, name)
                                .clicked()
                            {
                                q.ckp_wheel_teeth = teeth;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Factor hueco:");
                        ui.add(
                            egui::Slider::new(&mut q.ckp_gap_ratio, 1.35..=3.20)
                                .step_by(0.05)
                                .suffix("× T"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Cruce diente:");
                        if ui.selectable_label(q.ckp_rising_edge, "Subida ⬈").clicked() {
                            q.ckp_rising_edge = true;
                        }
                        if ui.selectable_label(!q.ckp_rising_edge, "Bajada ⬊").clicked() {
                            q.ckp_rising_edge = false;
                        }
                    });
                }
                AdvancedTriggerMode::Window => {
                    ui.horizontal(|ui| {
                        ui.label("Condición:");
                        if ui
                            .selectable_label(!q.window_enter, "Salir de Banda")
                            .clicked()
                        {
                            q.window_enter = false;
                        }
                        if ui
                            .selectable_label(q.window_enter, "Entrar a Banda")
                            .clicked()
                        {
                            q.window_enter = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Lím. Sup:");
                        ui.add(
                            egui::DragValue::new(&mut q.window_upper_v)
                                .speed(0.05)
                                .suffix(" V"),
                        );
                        ui.label("Lím. Inf:");
                        ui.add(
                            egui::DragValue::new(&mut q.window_lower_v)
                                .speed(0.05)
                                .suffix(" V"),
                        );
                    });
                }
                AdvancedTriggerMode::Dropout => {
                    let mut timeout_ms = q.dropout_timeout_s * 1e3;
                    ui.horizontal(|ui| {
                        ui.label("Silencio mín.:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut timeout_ms)
                                    .speed(0.1)
                                    .range(0.05..=500.0)
                                    .suffix(" ms"),
                            )
                            .changed()
                        {
                            q.dropout_timeout_s = timeout_ms * 1e-3;
                        }
                    });
                }
                AdvancedTriggerMode::RuntPulse => {
                    ui.horizontal(|ui| {
                        ui.label("Polaridad:");
                        if ui.selectable_label(q.runt_positive, "Positivo (+)").clicked() {
                            q.runt_positive = true;
                        }
                        if ui.selectable_label(!q.runt_positive, "Negativo (-)").clicked() {
                            q.runt_positive = false;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Umbral 1:");
                        ui.add(
                            egui::DragValue::new(&mut q.runt_low_v)
                                .speed(0.05)
                                .suffix(" V"),
                        );
                        ui.label("Umbral 2:");
                        ui.add(
                            egui::DragValue::new(&mut q.runt_high_v)
                                .speed(0.05)
                                .suffix(" V"),
                        );
                    });
                }
                AdvancedTriggerMode::LogicPattern => {
                    ui.horizontal(|ui| {
                        ui.label("CH1:");
                        if ui
                            .selectable_label(q.logic_ch1_above, "Alto ≥")
                            .clicked()
                        {
                            q.logic_ch1_above = true;
                        }
                        if ui
                            .selectable_label(!q.logic_ch1_above, "Bajo ≤")
                            .clicked()
                        {
                            q.logic_ch1_above = false;
                        }
                        ui.add(
                            egui::DragValue::new(&mut q.logic_ch1_level_v)
                                .speed(0.05)
                                .suffix(" V"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("CH2:");
                        if ui
                            .selectable_label(q.logic_ch2_above, "Alto ≥")
                            .clicked()
                        {
                            q.logic_ch2_above = true;
                        }
                        if ui
                            .selectable_label(!q.logic_ch2_above, "Bajo ≤")
                            .clicked()
                        {
                            q.logic_ch2_above = false;
                        }
                        ui.add(
                            egui::DragValue::new(&mut q.logic_ch2_level_v)
                                .speed(0.05)
                                .suffix(" V"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Operador:");
                        for op in LogicCombinator::ALL {
                            if ui
                                .selectable_label(q.logic_op == op, format!("{op:?}"))
                                .clicked()
                            {
                                q.logic_op = op;
                            }
                        }
                    });
                }
            }

            // Estado en vivo del calificador
            let rt = &auto.trigger_qualifier_rt;
            if !rt.last_result.diagnostic_summary.is_empty() {
                ui.add_space(2.0);
                let col = if rt.last_result.matched {
                    egui::Color32::from_rgb(110, 240, 160)
                } else {
                    egui::Color32::from_rgb(255, 200, 95)
                };
                ui.colored_label(
                    col,
                    egui::RichText::new(&rt.last_result.diagnostic_summary).size(10.0),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Válidas: {} · Filtradas: {}",
                        rt.matched_frames, rt.rejected_frames
                    ))
                    .weak()
                    .size(9.5),
                );
            }
        }
    });
}
