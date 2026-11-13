use bevy_egui::egui;

use crate::derived::{BICHANNEL_METRICS, METRICS, MeasureState, SLOT_NAMES};

/// Panel de opciones de Mediciones Automáticas en la columna adyacente izquierda.
/// Los valores medidos en tiempo real se presentan en la tabla flotante sobre la cuadrícula (`measure_window`).
pub fn show(ui: &mut egui::Ui, meas: &mut MeasureState) {
    ui.spacing_mut().item_spacing.y = 5.0;

    ui.checkbox(
        &mut meas.window,
        egui::RichText::new("📌 Mostrar tabla flotante en cuadrícula").strong(),
    );
    super::measure_window::opacity_slider(ui, &mut meas.window_opacity);
    ui.checkbox(
        &mut meas.guides,
        "📏 Mostrar líneas guía en pantalla (Vtop/Vbase/10-90%)",
    );

    ui.separator();
    ui.label(
        egui::RichText::new("Canales visibles en tabla flotante:")
            .size(11.0)
            .strong()
            .color(egui::Color32::from_gray(220)),
    );
    ui.horizontal(|ui| {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            ui.checkbox(&mut meas.show_slot[slot], *name);
        }
    });

    ui.horizontal(|ui| {
        if ui.button("🔄 Reiniciar Estadísticas").clicked() {
            meas.reset_stats();
        }
        if ui.button("🚗 Preset Automotriz").clicked() {
            meas.show_metric = [false; crate::derived::N_METRICS];
            // Frequency (1), +PulseWidth (4), +DutyCycle (6), Vpp (8), Vmax (9), Vmin (10), Vavg (16), Vrms (17)
            for idx in [1, 4, 6, 8, 9, 10, 16, 17] {
                meas.show_metric[idx] = true;
            }
            meas.window = true;
        }
    });

    ui.separator();
    ui.label(
        egui::RichText::new("Parámetros a mostrar en tiempo real:")
            .size(11.0)
            .strong()
            .color(egui::Color32::from_gray(220)),
    );
    ui.horizontal(|ui| {
        if ui.small_button("✔ Todos").clicked() {
            meas.show_metric = [true; crate::derived::N_METRICS];
            meas.show_bichannel = [true; crate::derived::N_BICHANNEL];
        }
        if ui.small_button("✖ Ninguno").clicked() {
            meas.show_metric = [false; crate::derived::N_METRICS];
            meas.show_bichannel = [false; crate::derived::N_BICHANNEL];
        }
    });

    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            for (i, (name, _, _)) in METRICS.iter().enumerate() {
                ui.checkbox(&mut meas.show_metric[i], *name);
            }
            ui.separator();
            ui.label(
                egui::RichText::new("Entre Canales (CH1 ↔ CH2):")
                    .size(10.5)
                    .strong()
                    .color(egui::Color32::from_rgb(120, 205, 255)),
            );
            for (i, (name, _, _)) in BICHANNEL_METRICS.iter().enumerate() {
                ui.checkbox(&mut meas.show_bichannel[i], *name);
            }
        });
}
