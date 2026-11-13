use bevy_egui::egui;
use neowon_backend::{Command, MultiMode};
use neowon_dsp::Window;

use crate::Link;
use crate::derived::{FftState, MathState, PfState, SLOT_NAMES, build_pf_mask};
use crate::script::{Action, Script};

pub fn show(
    ui: &mut egui::Ui,
    link: &mut Link,
    math: &MathState,
    pf: &mut PfState,
    fft: &mut FftState,
    script: &mut Script,
) {
    ui.group(|ui| {
        ui.strong("Session");
        ui.horizontal(|ui| {
            let path = crate::record::export_dir().join("setup.nws");
            if ui.button("Save setup").clicked() {
                script.inject(Action::SessionSave(path.display().to_string()));
            }
            let exists = path.exists();
            if ui
                .add_enabled(exists, egui::Button::new("Load setup"))
                .clicked()
            {
                script.inject(Action::SessionLoad(path.display().to_string()));
            }
        });
        ui.label(
            egui::RichText::new("a session file is a neowon script (setup.nws)")
                .weak()
                .small(),
        );
    });
    ui.group(|ui| {
        ui.strong("MULTI port");
        ui.horizontal(|ui| {
            for (m, label) in [
                (MultiMode::TriggerOut, "Trigger out"),
                (MultiMode::PassFailOut, "Pass-fail out"),
                (MultiMode::TriggerIn, "Trigger in"),
            ] {
                if ui.selectable_label(link.multi == m, label).clicked() {
                    link.multi = m;
                    let _ = link.sup.commands.send(Command::Multi(m));
                }
            }
        });
    });

    ui.group(|ui| {
        ui.strong("🎭 Máscara Pasa/Falla y Acciones por Evento (Cap. 7.3 / 7.5)");
        ui.horizontal(|ui| {
            ui.checkbox(&mut pf.enabled, "Activar Máscara");
            ui.label("Fuente:");
            for (slot, name) in SLOT_NAMES.iter().enumerate() {
                if ui.selectable_label(pf.source_slot == slot, *name).clicked() {
                    pf.source_slot = slot;
                    pf.mask = None; // reference came from another trace
                    pf.reference = None;
                    pf.last_violations.clear();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tol. H:");
            let h_changed = ui
                .add(
                    egui::DragValue::new(&mut pf.h_div)
                        .speed(0.05)
                        .range(0.0..=20.0)
                        .suffix(" div"),
                )
                .changed();
            ui.label("Tol. V:");
            let v_changed = ui
                .add(
                    egui::DragValue::new(&mut pf.v_div)
                        .speed(0.05)
                        .range(0.0..=10.0)
                        .suffix(" div"),
                )
                .changed();
            if (h_changed || v_changed) && pf.reference.is_some() {
                pf.rebuild_mask_from_reference();
            }
        });
        ui.horizontal(|ui| {
            let can_capture = if pf.source_slot < 2 {
                link.latest
                    .as_ref()
                    .and_then(|f| f.channels.iter().find(|c| c.ch == pf.source_slot))
                    .is_some()
            } else {
                math.trace.is_some()
            };
            if ui
                .add_enabled(can_capture, egui::Button::new("📸 Crear Máscara de Referencia"))
                .clicked()
            {
                let raw: Option<Vec<f32>> = if pf.source_slot < 2 {
                    link.latest
                        .as_ref()
                        .and_then(|f| f.channels.iter().find(|c| c.ch == pf.source_slot))
                        .map(|c| c.data.clone())
                } else {
                    math.trace.as_ref().map(|c| c.data.clone())
                };
                if let Some(raw) = raw {
                    pf.mask = Some(build_pf_mask(&raw, pf.h_div, pf.v_div));
                    pf.reference = Some(raw);
                    pf.reset_counters();
                }
            }
            if ui.button("🔄 Reiniciar").clicked() {
                pf.reset_counters();
            }
        });

        ui.horizontal(|ui| {
            if let Some(csv) = pf.mask_to_csv() {
                if ui.small_button("📤 Exportar Máscara CSV").clicked() {
                    let dir = crate::record::export_dir().join("Masks");
                    let _ = std::fs::create_dir_all(&dir);
                    let path = dir.join("mask_tolerance.csv");
                    if std::fs::write(&path, &csv).is_ok() {
                        pf.last_autosave_msg = Some(format!("Máscara exportada: {}", path.display()));
                    }
                }
            }
            if ui.small_button("📥 Importar Máscara CSV").clicked() {
                let path = crate::record::export_dir().join("Masks").join("mask_tolerance.csv");
                match std::fs::read_to_string(&path) {
                    Ok(csv) => match pf.load_mask_csv(&csv) {
                        Ok(n) => {
                            pf.enabled = true;
                            pf.last_autosave_msg = Some(format!("Máscara cargada ({n} muestras)"));
                        }
                        Err(e) => {
                            pf.last_autosave_msg = Some(format!("Error CSV: {e}"));
                        }
                    },
                    Err(_) => {
                        pf.last_autosave_msg = Some(format!("No existe {}", path.display()));
                    }
                }
            }
        });

        ui.separator();
        ui.label(
            egui::RichText::new("⚡ Acciones Automáticas (Cap. 7.3 Actions):")
                .size(10.5)
                .strong()
                .color(egui::Color32::from_rgb(255, 200, 95)),
        );
        ui.checkbox(
            &mut pf.action_on_trigger,
            "⚡ Disparar por cada captura nueva (aún sin máscara)",
        );
        ui.checkbox(&mut pf.stop_on_fail, "⏹ Detener adquisición al disparar (Stop)");
        ui.checkbox(&mut pf.beep_on_fail, "🔊 Alerta sonora Windows al disparar (Beep)");
        ui.checkbox(
            &mut pf.autosave_on_fail,
            "💾 Auto-guardar captura en disco (.nwc)",
        );
        ui.checkbox(&mut pf.output_multi, "🔌 Emitir señal Pasa/Falla en puerto MULTI");

        let total = pf.pass + pf.fail;
        let fail_pct = if total > 0 {
            (pf.fail as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("✔ PASA: {}", pf.pass))
                    .size(11.0)
                    .strong()
                    .color(egui::Color32::from_rgb(100, 230, 140))
                    .monospace(),
            );
            ui.label(
                egui::RichText::new(format!("✖ FALLA: {} ({fail_pct:.1}%)", pf.fail))
                    .size(11.0)
                    .strong()
                    .color(if pf.fail > 0 {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::from_gray(160)
                    })
                    .monospace(),
            );
            ui.label(
                egui::RichText::new(format!("(Total: {total})"))
                    .size(10.5)
                    .weak()
                    .monospace(),
            );
        });
        if !pf.last_violations.is_empty() {
            ui.label(
                egui::RichText::new(format!(
                    "⚠ Última trama: {} muestras fuera de máscara (marcadas en rojo)",
                    pf.last_violations.len()
                ))
                .size(10.0)
                .color(egui::Color32::from_rgb(255, 130, 120)),
            );
        }
        if let Some(msg) = &pf.last_autosave_msg {
            ui.label(
                egui::RichText::new(format!("💾 {msg}"))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 215, 255)),
            );
        }
        if pf.mask.is_none() {
            ui.label(
                egui::RichText::new("Sin máscara capturada — pulse 'Crear Máscara de Referencia'")
                    .weak()
                    .small(),
            );
        }
    });

    ui.group(|ui| {
        ui.strong("Spectrum (FFT)");
        ui.checkbox(&mut fft.enabled, "Enabled");
        ui.horizontal(|ui| {
            ui.label("Source");
            for (slot, name) in SLOT_NAMES.iter().enumerate() {
                if ui.selectable_label(fft.source == slot, *name).clicked() {
                    fft.source = slot;
                }
            }
            egui::ComboBox::from_id_salt("fftwnd")
                .selected_text(fft.window.label())
                .show_ui(ui, |ui| {
                    for w in Window::ALL {
                        if ui.selectable_label(fft.window == w, w.label()).clicked() {
                            fft.window = w;
                        }
                    }
                });
        });
    });
}
