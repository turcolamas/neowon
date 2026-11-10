use bevy_egui::egui;
use neowon_core::Coupling;
use picowon_auto::ChannelDspConfig;

use crate::Link;
use super::knob::{stepper_ladder, stepper_value};
use super::widgets::{FALLBACK_VDIV, PROBES, StepperAction, compact_stepper, ladder_combo};
use crate::derived::fmt_si;

pub fn show(ui: &mut egui::Ui, link: &mut Link, ch: usize, dsp: &mut ChannelDspConfig) {
    let vdiv_ladder: Vec<f64> = link
        .scope_caps()
        .map(|c| c.volts_div.clone())
        .unwrap_or_else(|| FALLBACK_VDIV.to_vec());

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.heading(format!("Canal {}", ch + 1));
            let color = super::widgets::channel_color(ch);
            ui.colored_label(color, "●");
        });

        let mut c = link.config.channels[ch];
        let mut dirty = false;
        dirty |= ui.checkbox(&mut c.enabled, "Habilitado (Enabled)").changed();

        dirty |= ladder_combo(
            ui,
            &format!("vdiv{ch}"),
            "Escala V/div",
            &mut c.volts_div,
            &vdiv_ladder,
            |v| fmt_si(v, "V"),
        );

        ui.horizontal(|ui| {
            ui.label("Acoplamiento");
            for (label, v) in [
                ("DC", Coupling::Dc),
                ("AC", Coupling::Ac),
                ("GND", Coupling::Gnd),
            ] {
                if ui.selectable_label(c.coupling == v, label).clicked() {
                    c.coupling = v;
                    dirty = true;
                }
            }
        });

        let mut probe = c.probe;
        if ladder_combo(
            ui,
            &format!("probe{ch}"),
            "Sonda / Atenuador",
            &mut probe,
            &PROBES,
            |v| format!("×{v}"),
        ) {
            crate::view::set_channel_probe(link, ch, probe);
            c = link.config.channels[ch];
            dirty = true;
        }

        ui.horizontal(|ui| {
            let mut vdiv = c.volts_div;
            if stepper_ladder(ui, "V/div fino", &mut vdiv, &vdiv_ladder, |v| fmt_si(v, "V")) {
                c.volts_div = vdiv;
                dirty = true;
            }
        });

        ui.horizontal(|ui| {
            let full_scale = c.volts_div * 10.0 * c.probe;
            let mut off = c.offset;
            if stepper_value(ui, "Offset Hardware", &mut off, 0.05, (-0.5, 0.5), 0.0, |v| {
                fmt_si(v * full_scale, "V")
            }) {
                c.offset = off;
                dirty = true;
            }
        });

        if dirty {
            link.config.channels[ch] = c;
            link.dirty = true;
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // SECCIÓN ACONDICIONAMIENTO Y DSP (Estilo PicoScope 7)
        ui.horizontal(|ui| {
            ui.strong("⚡ Procesamiento Digital DSP");
            let dsp_active = dsp.lpf_enabled
                || dsp.res_enhance_bits > 0.0
                || dsp.invert
                || dsp.tare_offset_v.abs() > 1e-6;
            if dsp_active {
                ui.colored_label(egui::Color32::from_rgb(100, 220, 140), "● DSP ON");
            }
        });

        // 1. Filtro Pasa-Bajos Digital (Software Low-Pass Filter)
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut dsp.lpf_enabled,
                    egui::RichText::new("Filtro Pasa-Bajos (LPF)").strong(),
                );
                if dsp.lpf_enabled {
                    let fc_str = fmt_si(dsp.lpf_cutoff_hz, "Hz");
                    ui.colored_label(egui::Color32::from_rgb(100, 220, 140), format!("fc = {fc_str}"));
                }
            });

            if dsp.lpf_enabled {
                ui.label(
                    egui::RichText::new("Atenúa ruido de encendido, alternador y rebotes sin distorsionar")
                        .weak()
                        .size(10.0),
                );

                ui.add_space(2.0);
                ui.label(egui::RichText::new("Algoritmo de Filtrado (Selección sin distorsión):").size(10.0).strong());
                for algo in [
                    picowon_auto::LpfAlgorithm::Gaussian,
                    picowon_auto::LpfAlgorithm::SavitzkyGolay,
                    picowon_auto::LpfAlgorithm::EdgePreserving,
                    picowon_auto::LpfAlgorithm::Butterworth,
                ] {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(dsp.filter_algorithm == algo, algo.name()).clicked() {
                            dsp.filter_algorithm = algo;
                        }
                    });
                }

                ui.label(
                    egui::RichText::new(dsp.filter_algorithm.description())
                        .weak()
                        .size(9.5)
                        .color(egui::Color32::from_rgb(170, 185, 210)),
                );

                if dsp.filter_algorithm == picowon_auto::LpfAlgorithm::EdgePreserving {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Umbral borde:").size(10.0));
                        let mut pct = (dsp.edge_threshold_frac * 100.0).clamp(1.0, 25.0);
                        if ui.add(egui::Slider::new(&mut pct, 1.0..=20.0).suffix("%")).changed() {
                            dsp.edge_threshold_frac = pct / 100.0;
                        }
                    });
                }

                ui.add_space(2.0);

                // Presets rápidos de corte automotriz
                ui.horizontal_wrapped(|ui| {
                    for (preset_f, label_short) in [
                        (1_000.0, "1 kHz"),
                        (5_000.0, "5 kHz"),
                        (10_000.0, "10 kHz"),
                        (20_000.0, "20 kHz"),
                        (50_000.0, "50 kHz"),
                        (100_000.0, "100 kHz"),
                    ] {
                        let is_sel = (dsp.lpf_cutoff_hz - preset_f).abs() < 1.0;
                        if ui.selectable_label(is_sel, label_short).clicked() {
                            dsp.lpf_cutoff_hz = preset_f;
                        }
                    }
                });

                // Stepper de ajuste continuo de frecuencia de corte
                let fc_str = fmt_si(dsp.lpf_cutoff_hz, "Hz");
                let step_action =
                    compact_stepper(ui, &format!("lpf_ch{ch}"), &format!("Corte: {fc_str}"), 20.0);
                match step_action {
                    StepperAction::Prev => {
                        dsp.lpf_cutoff_hz = (dsp.lpf_cutoff_hz * 0.8).max(100.0);
                    }
                    StepperAction::Next => {
                        dsp.lpf_cutoff_hz = (dsp.lpf_cutoff_hz * 1.25).min(20_000_000.0);
                    }
                    StepperAction::ClickedValue => {
                        dsp.lpf_cutoff_hz = 10_000.0;
                    }
                    StepperAction::None => {}
                }
            }
        });

        // 2. Mejora de Resolución (Resolution Enhancement / ENOB Boost)
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Mejora de Resolución").strong());
                if dsp.res_enhance_bits > 0.0 {
                    let total_bits = 8.0 + dsp.res_enhance_bits;
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 200, 255),
                        format!("{total_bits:.1} bits"),
                    );
                } else {
                    ui.label(egui::RichText::new("8 bits nativo").weak());
                }
            });
            ui.label(
                egui::RichText::new("Promedia cuantización para revelar micro-ondulaciones")
                    .weak()
                    .size(10.0),
            );

            ui.horizontal_wrapped(|ui| {
                for (bits, label) in [
                    (0.0, "Off (8b)"),
                    (0.5, "+0.5b"),
                    (1.0, "+1.0b (9b)"),
                    (1.5, "+1.5b"),
                    (2.0, "+2.0b (10b)"),
                    (3.0, "+3.0b (11b)"),
                    (4.0, "+4.0b (12b)"),
                ] {
                    let is_sel = (dsp.res_enhance_bits - bits).abs() < 0.1;
                    if ui.selectable_label(is_sel, label).clicked() {
                        dsp.res_enhance_bits = bits;
                    }
                }
            });
        });

        // 3. Inversión y Tara de Sonda (Auto-Zero)
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut dsp.invert, "Invertir Traza (Invert)");
            });

            ui.horizontal(|ui| {
                let tare_active = dsp.tare_offset_v.abs() > 1e-5;
                if ui
                    .button("🎯 Calibrar Cero / Tara")
                    .on_hover_text("Resta la desviación DC actual (esencial para pinzas amperimétricas)")
                    .clicked()
                {
                    if let Some(frame) = &link.latest {
                        if let Some(cap) = frame.channels.iter().find(|c| c.ch == ch) {
                            if !cap.data.is_empty() {
                                let sum: f32 = cap.data.iter().sum();
                                let avg_counts = sum / cap.data.len() as f32;
                                dsp.tare_offset_v = cap.cal.volts_i(avg_counts);
                            }
                        }
                    }
                }
                if tare_active {
                    let t_str = fmt_si(dsp.tare_offset_v, "V");
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 180, 100),
                        format!("Tara: {t_str}"),
                    );
                    if ui.small_button("✕").on_hover_text("Restablecer tara a 0 V").clicked() {
                        dsp.tare_offset_v = 0.0;
                    }
                }
            });
        });
    });
}
