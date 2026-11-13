use bevy_egui::egui;
use neowon_core::Coupling;

use crate::Link;
use super::automotive_dock::AutomotiveUiState;
use super::knob::{stepper_ladder, stepper_value};
use super::widgets::{FALLBACK_VDIV, PROBES, ladder_combo};
use crate::derived::fmt_si;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelTab {
    Vertical,
    Probes,
    Dsp,
    Display,
}

pub fn show(ui: &mut egui::Ui, link: &mut Link, ch: usize, auto: &mut AutomotiveUiState) {
    let vdiv_ladder: Vec<f64> = link
        .scope_caps()
        .map(|c| c.volts_div.clone())
        .unwrap_or_else(|| FALLBACK_VDIV.to_vec());

    let color = super::widgets::channel_color(ch);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.colored_label(color, "●");
            let alias = auto.ch_ext[ch].alias.trim();
            if alias.is_empty() {
                ui.heading(format!("Canal {}", ch + 1));
            } else {
                ui.heading(format!("Canal {} · {}", ch + 1, alias));
            }
        });

        // Barra de pestañas (Estilo PicoScope 7: Vertical, Sondas, DSP, Pantalla)
        let tab_id = ui.make_persistent_id(format!("ch_tab_{ch}"));
        let mut cur_tab = ui.data_mut(|d| d.get_temp::<usize>(tab_id).unwrap_or(0));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (idx, (icon, label)) in [
                ("↕", "Vertical"),
                ("🔧", "Sondas"),
                ("⚡", "DSP"),
                ("🎨", "Pantalla"),
            ]
            .iter()
            .enumerate()
            {
                let sel = cur_tab == idx;
                let btn = egui::Button::new(
                    egui::RichText::new(format!("{icon} {label}"))
                        .size(10.0)
                        .strong()
                        .color(if sel {
                            egui::Color32::BLACK
                        } else {
                            egui::Color32::from_rgb(210, 220, 235)
                        }),
                )
                .fill(if sel {
                    color
                } else {
                    egui::Color32::from_rgb(26, 30, 40)
                });
                if ui.add_sized(egui::vec2(58.0, 22.0), btn).clicked() {
                    cur_tab = idx;
                    ui.data_mut(|d| d.insert_temp(tab_id, idx));
                }
            }
        });

        ui.separator();

        match cur_tab {
            // ==================== PESTAÑA 1: VERTICAL / BÁSICO ====================
            0 => {
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

                ui.horizontal(|ui| {
                    if ui.button("🎯 Centrar a Cero").clicked() {
                        c.offset = 0.0;
                        dirty = true;
                    }
                });

                if dirty {
                    link.config.channels[ch] = c;
                    link.dirty = true;
                }
            }

            // ==================== PESTAÑA 2: SONDAS Y TRANSDUCTORES ====================
            1 => {
                let c = link.config.channels[ch];
                let mut probe = c.probe;
                if ladder_combo(
                    ui,
                    &format!("probe{ch}"),
                    "Factor Atenuación HW",
                    &mut probe,
                    &PROBES,
                    |v| format!("×{v}"),
                ) {
                    crate::view::set_channel_probe(link, ch, probe);
                }

                ui.separator();
                ui.label(egui::RichText::new("Sonda o Transductor Activo:").strong());
                let probe_spec = &mut auto.ch_ext[ch].probe_spec;

                ui.horizontal(|ui| {
                    ui.label("Modelo:");
                    ui.label(egui::RichText::new(probe_spec.name.as_str()).strong().color(egui::Color32::from_rgb(108, 185, 245)));
                });
                ui.horizontal(|ui| {
                    ui.label("Unidad física:");
                    ui.label(egui::RichText::new(probe_spec.unit.symbol()).monospace().strong());
                });

                ui.add_space(4.0);
                if ui
                    .button(egui::RichText::new("🔧 Abrir Catálogo Completo de Sondas").strong())
                    .clicked()
                {
                    auto.probe_dialog_ch = Some(ch);
                }
            }

            // ==================== PESTAÑA 3: FILTRO Y DSP ====================
            2 => {
                let dsp = &mut auto.dsp[ch];

                ui.checkbox(
                    &mut dsp.lpf_enabled,
                    egui::RichText::new("Filtro Pasa-Bajos Digital (LPF)").strong(),
                );

                if dsp.lpf_enabled {
                    let fc_str = fmt_si(dsp.lpf_cutoff_hz, "Hz");
                    ui.colored_label(egui::Color32::from_rgb(100, 220, 140), format!("fc = {fc_str}"));

                    ui.label(egui::RichText::new("Algoritmo de Filtrado:").size(10.0).strong());
                    for algo in [
                        picowon_auto::LpfAlgorithm::Gaussian,
                        picowon_auto::LpfAlgorithm::SavitzkyGolay,
                        picowon_auto::LpfAlgorithm::EdgePreserving,
                        picowon_auto::LpfAlgorithm::Butterworth,
                    ] {
                        if ui.selectable_label(dsp.filter_algorithm == algo, algo.name()).clicked() {
                            dsp.filter_algorithm = algo;
                        }
                    }

                    // Presets rápidos
                    ui.horizontal_wrapped(|ui| {
                        for (preset_f, label_short) in [
                            (1_000.0, "1 kHz"),
                            (5_000.0, "5 kHz"),
                            (10_000.0, "10 kHz"),
                            (20_000.0, "20 kHz"),
                            (50_000.0, "50 kHz"),
                        ] {
                            let is_sel = (dsp.lpf_cutoff_hz - preset_f).abs() < 1.0;
                            if ui.selectable_label(is_sel, label_short).clicked() {
                                dsp.lpf_cutoff_hz = preset_f;
                            }
                        }
                    });
                }

                ui.separator();
                ui.label(egui::RichText::new("Mejora de Resolución (ENOB Boost):").strong());
                ui.horizontal_wrapped(|ui| {
                    for (bits, label) in [
                        (0.0, "Off (8b)"),
                        (1.0, "+1.0b (9b)"),
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

                ui.separator();
                ui.checkbox(&mut dsp.invert, "Invertir Traza (Invert)");

                ui.horizontal(|ui| {
                    if ui.button("🎯 Calibrar Cero / Tara").clicked() {
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
                    if dsp.tare_offset_v.abs() > 1e-5 {
                        let t_str = fmt_si(dsp.tare_offset_v, "V");
                        ui.colored_label(egui::Color32::from_rgb(220, 180, 100), format!("Tara: {t_str}"));
                        if ui.small_button("✕").clicked() {
                            dsp.tare_offset_v = 0.0;
                        }
                    }
                });
            }

            // ==================== PESTAÑA 4: PANTALLA Y EJES ====================
            _ => {
                ui.label(egui::RichText::new("Nombre / Etiqueta del Canal:").strong());
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut auto.ch_ext[ch].alias);
                    if ui.button("Limpiar").clicked() {
                        auto.ch_ext[ch].alias.clear();
                    }
                });
                ui.label(egui::RichText::new("Ej: 'Inyector Cil 1', 'CKP Inductivo', 'CAN-H'").weak().size(10.0));

                ui.separator();
                ui.checkbox(&mut auto.ch_ext[ch].auto_range, "Escala Vertical Automática (Auto-Range)");
                ui.checkbox(&mut auto.ch_ext[ch].freq_counter, "Frecuencímetro Hardware y Tacómetro (RPM)");
            }
        }
    });
}
