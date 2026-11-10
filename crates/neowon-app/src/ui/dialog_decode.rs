use bevy_egui::egui;

use crate::decode::{DecodeState, Protocol};
use crate::derived::fmt_si;
use neowon_dsp::decode::EventKind;

pub fn show(ui: &mut egui::Ui, st: &mut DecodeState) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Protocolo Serie / Bus");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if st.table_open {
                        "📊 Ocultar Tabla"
                    } else {
                        "📊 Ver Tabla de Tramas"
                    })
                    .clicked()
                {
                    st.table_open = !st.table_open;
                }
            });
        });
        ui.add_space(2.0);

        ui.horizontal_wrapped(|ui| {
            for p in Protocol::ALL {
                let selected = st.protocol == p;
                let btn = egui::Button::new(
                    egui::RichText::new(p.name().to_uppercase())
                        .size(11.0)
                        .strong(),
                )
                .fill(if selected {
                    egui::Color32::from_rgb(45, 90, 160)
                } else {
                    egui::Color32::from_rgb(28, 32, 40)
                });
                if ui.add(btn).clicked() {
                    st.protocol = p;
                }
            }
        });

        if st.protocol == Protocol::Off {
            ui.label(
                egui::RichText::new("Seleccione un protocolo (CAN, LIN, UART, I²C, SPI, 1-Wire) para decodificar en tiempo real.")
                    .weak()
                    .small(),
            );
            return;
        }

        ui.separator();
        ui.label(
            egui::RichText::new(format!("Configuración: {}", st.protocol.label()))
                .strong()
                .color(egui::Color32::from_rgb(140, 200, 255)),
        );

        for (i, name) in st.protocol.lines().iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}:", name));
                for ch in 0..2 {
                    if ui
                        .selectable_label(st.channels[i] == ch, format!("CH{}", ch + 1))
                        .clicked()
                    {
                        st.channels[i] = ch;
                    }
                }
            });
        }

        // CAN Bus parameters
        if st.protocol == Protocol::Can {
            ui.horizontal(|ui| {
                ui.label("Baud:");
                for &b in &[125_000.0, 250_000.0, 500_000.0, 1_000_000.0] {
                    let label = format!("{}k", (b / 1000.0) as u32);
                    if ui.selectable_label((st.can.baud - b).abs() < 1.0, label).clicked() {
                        st.can.baud = b;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Velocidad:");
                ui.add(
                    egui::DragValue::new(&mut st.can.baud)
                        .speed(1000.0)
                        .range(10_000.0..=5_000_000.0)
                        .suffix(" bps"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Sample Point:");
                ui.add(
                    egui::Slider::new(&mut st.can.sample_point, 0.50..=0.90)
                        .fixed_decimals(2)
                        .suffix(" %"),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.can.inverted, "Invertir Polaridad (ej. CAN_L)");
            });
        }

        // LIN Bus parameters
        if st.protocol == Protocol::Lin {
            ui.horizontal(|ui| {
                ui.label("Baud:");
                for &b in &[9600.0, 19200.0, 10417.0] {
                    let label = format!("{}", b as u32);
                    if ui.selectable_label((st.lin.baud - b).abs() < 1.0, label).clicked() {
                        st.lin.baud = b;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Velocidad:");
                ui.add(
                    egui::DragValue::new(&mut st.lin.baud)
                        .speed(100.0)
                        .range(1000.0..=100_000.0)
                        .suffix(" bps"),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut st.lin.enhanced_checksum,
                    "Enhanced Checksum (LIN 2.x con PID)",
                );
            });
        }

        if st.protocol == Protocol::Uart {
            ui.horizontal(|ui| {
                ui.label("Baud");
                ui.add(
                    egui::DragValue::new(&mut st.uart.baud)
                        .speed(100.0)
                        .range(50.0..=20e6),
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.uart.inverted, "Inverted");
                ui.checkbox(&mut st.uart.lsb_first, "LSB first");
            });
        }

        if st.protocol == Protocol::Spi {
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.spi.cpol, "CPOL");
                ui.checkbox(&mut st.spi.cpha, "CPHA");
                ui.checkbox(&mut st.spi.msb_first, "MSB first");
            });
            ui.horizontal(|ui| {
                ui.label("Bits");
                let mut bits = st.spi.bits as u32;
                if ui
                    .add(egui::DragValue::new(&mut bits).range(1..=64))
                    .changed()
                {
                    st.spi.bits = bits as u8;
                }
            });
        }

        ui.add(
            egui::Slider::new(&mut st.hysteresis, 0.0..=0.45)
                .text("Hysteresis")
                .fixed_decimals(2),
        );
    });

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Resultado de Decodificación");
            if !st.packets.is_empty() {
                ui.label(
                    egui::RichText::new(format!("{} tramas", st.packets.len()))
                        .color(egui::Color32::from_rgb(100, 220, 160))
                        .small(),
                );
            }
        });
        if let Some(e) = &st.error {
            ui.label(
                egui::RichText::new(e)
                    .color(egui::Color32::from_rgb(230, 130, 90))
                    .small(),
            );
            return;
        }
        if st.events.is_empty() {
            ui.label(egui::RichText::new("Sin datos decodificados en el barrido actual").weak().small());
            return;
        }
        let errors = st.error_count();
        ui.label(
            egui::RichText::new(format!("{} eventos · {} errores", st.events.len(), errors))
                .monospace()
                .size(10.0)
                .color(if errors > 0 {
                    egui::Color32::from_rgb(230, 130, 90)
                } else {
                    egui::Color32::GRAY
                }),
        );
        let rate = st.sample_rate.max(1.0);
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("decode-grid").striped(true).show(ui, |ui| {
                    for e in &st.events {
                        let (t0, _) = e.seconds(rate);
                        ui.label(egui::RichText::new(fmt_si(t0, "s")).monospace().size(10.0));
                        let (text, color) = match &e.kind {
                            EventKind::Word { value, bits } => (
                                if *bits <= 8 {
                                    format!("0x{value:02X}  {}", printable(*value))
                                } else {
                                    format!("0x{value:X} ({bits} bits)")
                                },
                                egui::Color32::from_rgb(235, 220, 120),
                            ),
                            EventKind::Marker(m) => {
                                (m.to_string(), egui::Color32::from_rgb(120, 200, 255))
                            }
                            EventKind::Ack(true) => {
                                ("ACK".into(), egui::Color32::from_rgb(120, 220, 140))
                            }
                            EventKind::Ack(false) => {
                                ("NAK".into(), egui::Color32::from_rgb(235, 160, 90))
                            }
                            EventKind::Error(e) => {
                                (e.to_string(), egui::Color32::from_rgb(240, 110, 90))
                            }
                        };
                        ui.label(
                            egui::RichText::new(text)
                                .monospace()
                                .size(10.0)
                                .color(color),
                        );
                        ui.end_row();
                    }
                });
            });
    });
}

/// Floating Protocol Decoding Table Window matching PicoScope 7 style.
pub fn show_decode_table(ctx: &egui::Context, st: &mut DecodeState) {
    if !st.table_open {
        return;
    }

    let title = format!(
        "PicoScope 7 T&M — Decodificación de Protocolo Serie ({})",
        st.protocol.name().to_uppercase()
    );

    egui::Window::new(title)
        .open(&mut st.table_open)
        .default_width(760.0)
        .default_height(320.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Top Toolbar: Search filter, Export CSV, Clear
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍 Buscar:").strong().size(11.0));
                ui.add(
                    egui::TextEdit::singleline(&mut st.filter_text)
                        .hint_text("Filtrar por ID, datos hex, texto...")
                        .desired_width(220.0),
                );
                if !st.filter_text.is_empty() && ui.button("✕").clicked() {
                    st.filter_text.clear();
                }

                ui.separator();

                if ui.button("📋 Copiar CSV").clicked() {
                    let mut csv = String::from("Index,StartTime_s,EndTime_s,Protocol,ID_PID,DLC,DataHex,CRC_Chk,Valid,Error\n");
                    for p in &st.packets {
                        csv.push_str(&format!(
                            "{},{:.6},{:.6},{},{},{},{},{},{},{}\n",
                            p.index,
                            p.start_time,
                            p.end_time,
                            p.protocol.name(),
                            p.id_or_pid,
                            p.dlc,
                            p.data_hex,
                            p.crc_or_chk,
                            p.valid,
                            p.error.as_deref().unwrap_or("")
                        ));
                    }
                    ui.ctx().copy_text(csv);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let valid_cnt = st.packets.iter().filter(|p| p.valid).count();
                    let total = st.packets.len();
                    ui.label(
                        egui::RichText::new(format!("{valid_cnt} / {total} válidas"))
                            .size(11.0)
                            .color(if total > 0 && valid_cnt == total {
                                egui::Color32::from_rgb(100, 220, 160)
                            } else {
                                egui::Color32::from_rgb(240, 180, 80)
                            }),
                    );
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);

            // Table Header & Rows
            egui::ScrollArea::both()
                .max_height(ui.available_height() - 10.0)
                .show(ui, |ui| {
                    egui::Grid::new("serial-decoding-table")
                        .striped(true)
                        .min_col_width(50.0)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            // Column Headers (PicoScope 7 standard)
                            ui.label(egui::RichText::new("#").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("Inicio").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("Fin").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("ID / PID").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("DLC").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("Datos (Hex)").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("CRC / Chk").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("ACK").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.label(egui::RichText::new("Estado").strong().size(11.0).color(egui::Color32::LIGHT_BLUE));
                            ui.end_row();

                            let filter = st.filter_text.to_lowercase();

                            for p in &st.packets {
                                if !filter.is_empty() {
                                    let matches = p.id_or_pid.to_lowercase().contains(&filter)
                                        || p.data_hex.to_lowercase().contains(&filter)
                                        || p.crc_or_chk.to_lowercase().contains(&filter);
                                    if !matches {
                                        continue;
                                    }
                                }

                                ui.label(egui::RichText::new(format!("{}", p.index)).monospace().size(10.5));
                                ui.label(egui::RichText::new(fmt_si(p.start_time, "s")).monospace().size(10.5));
                                ui.label(egui::RichText::new(fmt_si(p.end_time, "s")).monospace().size(10.5));

                                // ID (Orange)
                                ui.label(
                                    egui::RichText::new(&p.id_or_pid)
                                        .monospace()
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(255, 170, 60))
                                        .strong(),
                                );

                                // DLC (Green)
                                ui.label(
                                    egui::RichText::new(format!("{}", p.dlc))
                                        .monospace()
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(100, 220, 140)),
                                );

                                // Data Hex (Blue/Cyan)
                                ui.label(
                                    egui::RichText::new(&p.data_hex)
                                        .monospace()
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(110, 200, 255)),
                                );

                                // CRC / Checksum (Purple)
                                ui.label(
                                    egui::RichText::new(&p.crc_or_chk)
                                        .monospace()
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(210, 150, 255)),
                                );

                                // ACK slot
                                let (ack_text, ack_color) = match p.ack {
                                    Some(true) => ("ACK", egui::Color32::from_rgb(100, 230, 140)),
                                    Some(false) => ("NAK", egui::Color32::from_rgb(250, 140, 70)),
                                    None => ("-", egui::Color32::GRAY),
                                };
                                ui.label(egui::RichText::new(ack_text).monospace().size(10.5).color(ack_color));

                                // Status (Valid / Error)
                                if p.valid {
                                    ui.label(
                                        egui::RichText::new("✓ Válida")
                                            .size(10.5)
                                            .color(egui::Color32::from_rgb(100, 230, 140)),
                                    );
                                } else {
                                    let err_msg = p.error.as_deref().unwrap_or("Error");
                                    ui.label(
                                        egui::RichText::new(format!("⚠ {err_msg}"))
                                            .size(10.5)
                                            .color(egui::Color32::from_rgb(255, 90, 80)),
                                    );
                                }
                                ui.end_row();
                            }
                        });
                });
        });
}

fn printable(v: u64) -> String {
    let c = v as u8;
    if c.is_ascii_graphic() || c == b' ' {
        format!("'{}'", c as char)
    } else {
        String::new()
    }
}
