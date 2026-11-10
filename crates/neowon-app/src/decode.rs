//! Protocol decoding in the app: pick a protocol, point it at channels, and
//! get an annotated event list back.
//!
//! Supports UART, I²C, SPI, 1-Wire, CAN Bus (2.0A/B), and LIN Bus (1.3/2.x).
//! Keeps packets for tabular inspection, CSV export, and on-plot bubble annotations.

use bevy::prelude::*;
use neowon_dsp::decode::{self, Digital, Event, EventKind, Threshold, digitize};

use crate::Link;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Protocol {
    #[default]
    Off,
    Uart,
    I2c,
    Spi,
    OneWire,
    Can,
    Lin,
}

impl Protocol {
    pub const ALL: [Protocol; 7] = [
        Protocol::Off,
        Protocol::Can,
        Protocol::Lin,
        Protocol::Uart,
        Protocol::I2c,
        Protocol::Spi,
        Protocol::OneWire,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Protocol::Off => "off",
            Protocol::Can => "can",
            Protocol::Lin => "lin",
            Protocol::Uart => "uart",
            Protocol::I2c => "i2c",
            Protocol::Spi => "spi",
            Protocol::OneWire => "1-wire",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Protocol::Off => "Desactivado",
            Protocol::Can => "CAN Bus (2.0A/B)",
            Protocol::Lin => "LIN Bus (1.3/2.x)",
            Protocol::Uart => "UART / Serie",
            Protocol::I2c => "I²C",
            Protocol::Spi => "SPI",
            Protocol::OneWire => "1-Wire",
        }
    }

    pub fn parse(s: &str) -> Option<Protocol> {
        Protocol::ALL
            .into_iter()
            .find(|p| p.name() == s || (s == "onewire" && *p == Protocol::OneWire))
    }

    /// Which channels this protocol needs, in order, for the UI to label.
    pub fn lines(self) -> &'static [&'static str] {
        match self {
            Protocol::Off => &[],
            Protocol::Can => &["CAN_H / Rx", "CAN_L (opt)"],
            Protocol::Lin => &["LIN"],
            Protocol::Uart | Protocol::OneWire => &["data"],
            Protocol::I2c => &["SCL", "SDA"],
            Protocol::Spi => &["SCK", "data", "CS"],
        }
    }
}

/// A structured protocol packet ready for tabular display, filtering, and export.
#[derive(Debug, Clone)]
pub struct DecodedPacket {
    pub index: usize,
    pub start_sample: usize,
    pub end_sample: usize,
    pub start_time: f64,
    pub end_time: f64,
    pub protocol: Protocol,
    pub id_or_pid: String,
    pub dlc: usize,
    pub data: Vec<u8>,
    pub data_hex: String,
    pub crc_or_chk: String,
    pub ack: Option<bool>,
    pub valid: bool,
    pub error: Option<String>,
}

#[derive(Resource)]
pub struct DecodeState {
    pub protocol: Protocol,
    /// Channel index per line, indexed as `Protocol::lines`.
    pub channels: [usize; 3],
    pub uart: decode::uart::Config,
    pub spi: decode::spi::Config,
    pub can: decode::can::Config,
    pub lin: decode::lin::Config,
    /// Hysteresis as a fraction of the signal's peak-to-peak swing.
    pub hysteresis: f64,
    /// Results of the last run, and what went wrong if it did not run.
    pub events: Vec<Event>,
    pub packets: Vec<DecodedPacket>,
    pub error: Option<String>,
    /// Sample rate the results were produced at, for time readouts.
    pub sample_rate: f64,
    /// Table window open state (PicoScope 7 style)
    pub table_open: bool,
    /// Search filter for the decoding table
    pub filter_text: String,
    /// Highlighted packet index
    pub selected_packet: Option<usize>,
    last_seq: u64,
}

impl Default for DecodeState {
    fn default() -> Self {
        Self {
            protocol: Protocol::Off,
            channels: [0, 1, 1],
            uart: decode::uart::Config::default(),
            spi: decode::spi::Config::default(),
            can: decode::can::Config::default(),
            lin: decode::lin::Config::default(),
            hysteresis: 0.2,
            events: Vec::new(),
            packets: Vec::new(),
            error: None,
            sample_rate: 0.0,
            table_open: false,
            filter_text: String::new(),
            selected_packet: None,
            last_seq: 0,
        }
    }
}

impl DecodeState {
    pub fn error_count(&self) -> usize {
        self.events.iter().filter(|e| e.kind.is_error()).count()
    }

    pub fn rebuild_packets(&mut self) {
        self.packets.clear();
        let rate = self.sample_rate.max(1.0);

        match self.protocol {
            Protocol::Can => {
                let mut cur_packet: Option<DecodedPacket> = None;
                let mut pkt_idx = 1;

                for e in &self.events {
                    match &e.kind {
                        EventKind::Marker("SOF") => {
                            if let Some(p) = cur_packet.take() {
                                self.packets.push(p);
                            }
                            cur_packet = Some(DecodedPacket {
                                index: pkt_idx,
                                start_sample: e.start,
                                end_sample: e.end,
                                start_time: e.start as f64 / rate,
                                end_time: e.end as f64 / rate,
                                protocol: Protocol::Can,
                                id_or_pid: String::new(),
                                dlc: 0,
                                data: Vec::new(),
                                data_hex: String::new(),
                                crc_or_chk: String::new(),
                                ack: None,
                                valid: true,
                                error: None,
                            });
                            pkt_idx += 1;
                        }
                        EventKind::Word { value, bits } => {
                            if let Some(ref mut p) = cur_packet {
                                p.end_sample = e.end;
                                p.end_time = e.end as f64 / rate;
                                if *bits == 11 || *bits == 29 {
                                    p.id_or_pid = format!("0x{:03X}", value);
                                } else if *bits == 4 && p.dlc == 0 {
                                    p.dlc = *value as usize;
                                } else if *bits == 8 && p.data.len() < p.dlc {
                                    p.data.push(*value as u8);
                                    if !p.data_hex.is_empty() {
                                        p.data_hex.push(' ');
                                    }
                                    p.data_hex.push_str(&format!("{:02X}", value));
                                } else if *bits == 15 {
                                    p.crc_or_chk = format!("0x{:04X}", value);
                                }
                            }
                        }
                        EventKind::Ack(ack) => {
                            if let Some(ref mut p) = cur_packet {
                                p.end_sample = e.end;
                                p.end_time = e.end as f64 / rate;
                                p.ack = Some(*ack);
                            }
                        }
                        EventKind::Error(err) => {
                            if let Some(ref mut p) = cur_packet {
                                p.valid = false;
                                p.error = Some(err.to_string());
                            }
                        }
                        EventKind::Marker("EOF") => {
                            if let Some(mut p) = cur_packet.take() {
                                p.end_sample = e.end;
                                p.end_time = e.end as f64 / rate;
                                self.packets.push(p);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(p) = cur_packet {
                    self.packets.push(p);
                }
            }
            Protocol::Lin => {
                let mut cur_packet: Option<DecodedPacket> = None;
                let mut pkt_idx = 1;

                for e in &self.events {
                    match &e.kind {
                        EventKind::Marker("BREAK") => {
                            if let Some(mut p) = cur_packet.take() {
                                if p.data.len() > 1 {
                                    let chk = p.data.pop().unwrap();
                                    p.crc_or_chk = format!("0x{:02X}", chk);
                                    p.dlc = p.data.len();
                                    p.data_hex = p.data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                                }
                                self.packets.push(p);
                            }
                            cur_packet = Some(DecodedPacket {
                                index: pkt_idx,
                                start_sample: e.start,
                                end_sample: e.end,
                                start_time: e.start as f64 / rate,
                                end_time: e.end as f64 / rate,
                                protocol: Protocol::Lin,
                                id_or_pid: String::new(),
                                dlc: 0,
                                data: Vec::new(),
                                data_hex: String::new(),
                                crc_or_chk: String::new(),
                                ack: None,
                                valid: true,
                                error: None,
                            });
                            pkt_idx += 1;
                        }
                        EventKind::Word { value, bits: 8 } => {
                            if let Some(ref mut p) = cur_packet {
                                p.end_sample = e.end;
                                p.end_time = e.end as f64 / rate;
                                if p.id_or_pid.is_empty() {
                                    let pid = *value as u8;
                                    let id = pid & 0x3F;
                                    p.id_or_pid = format!("0x{:02X} (ID:0x{:02X})", pid, id);
                                } else {
                                    p.data.push(*value as u8);
                                }
                            }
                        }
                        EventKind::Error(err) => {
                            if let Some(ref mut p) = cur_packet {
                                p.valid = false;
                                p.error = Some(err.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(mut p) = cur_packet {
                    if p.data.len() > 1 {
                        let chk = p.data.pop().unwrap();
                        p.crc_or_chk = format!("0x{:02X}", chk);
                        p.dlc = p.data.len();
                        p.data_hex = p.data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                    }
                    self.packets.push(p);
                }
            }
            Protocol::Uart | Protocol::I2c | Protocol::Spi | Protocol::OneWire => {
                let mut pkt_idx = 1;
                for (i, e) in self.events.iter().enumerate() {
                    if let EventKind::Word { value, bits } = &e.kind {
                        self.packets.push(DecodedPacket {
                            index: pkt_idx,
                            start_sample: e.start,
                            end_sample: e.end,
                            start_time: e.start as f64 / rate,
                            end_time: e.end as f64 / rate,
                            protocol: self.protocol,
                            id_or_pid: format!("#{}", i + 1),
                            dlc: (*bits as usize + 7) / 8,
                            data: vec![*value as u8],
                            data_hex: format!("{:02X}", value),
                            crc_or_chk: "-".into(),
                            ack: None,
                            valid: true,
                            error: None,
                        });
                        pkt_idx += 1;
                    }
                }
            }
            Protocol::Off => {}
        }
    }
}

fn line(link: &Link, ch: usize, hysteresis: f64) -> Option<Digital> {
    let frame = link.latest.as_ref()?;
    let cap = frame.channels.iter().find(|c| c.ch == ch)?;
    digitize(
        &cap.data,
        frame.sample_rate,
        Threshold::Relative { hysteresis },
    )
}

pub fn run(link: Res<Link>, mut st: ResMut<DecodeState>) {
    if st.protocol == Protocol::Off {
        if !st.events.is_empty() {
            st.events.clear();
            st.packets.clear();
            st.error = None;
        }
        return;
    }
    let Some(frame) = &link.latest else { return };
    if frame.seq == st.last_seq {
        return;
    }
    st.last_seq = frame.seq;
    st.sample_rate = frame.sample_rate;

    let h = st.hysteresis;
    let ch = st.channels;
    let result = match st.protocol {
        Protocol::Off => unreachable!(),
        Protocol::Can => match line(&link, ch[0], h) {
            Some(d) => decode::can::decode(&d, st.can).map_err(|e| e.to_string()),
            None => Err("no usable signal on CAN line".into()),
        },
        Protocol::Lin => match line(&link, ch[0], h) {
            Some(d) => decode::lin::decode(&d, st.lin).map_err(|e| e.to_string()),
            None => Err("no usable signal on LIN line".into()),
        },
        Protocol::Uart => match line(&link, ch[0], h) {
            Some(d) => decode::uart::decode(&d, st.uart).map_err(|e| e.to_string()),
            None => Err("no usable signal on the data line".into()),
        },
        Protocol::OneWire => match line(&link, ch[0], h) {
            Some(d) => decode::onewire::decode(&d).map_err(|e| e.to_string()),
            None => Err("no usable signal on the data line".into()),
        },
        Protocol::I2c => match (line(&link, ch[0], h), line(&link, ch[1], h)) {
            (Some(scl), Some(sda)) => decode::i2c::decode(&scl, &sda).map_err(|e| e.to_string()),
            _ => Err("I2C needs a usable signal on both SCL and SDA".into()),
        },
        Protocol::Spi => match (line(&link, ch[0], h), line(&link, ch[1], h)) {
            (Some(sck), Some(data)) => {
                let cs = line(&link, ch[2], h);
                decode::spi::decode(&sck, &data, cs.as_ref(), st.spi).map_err(|e| e.to_string())
            }
            _ => Err("SPI needs a usable signal on both SCK and the data line".into()),
        },
    };
    match result {
        Ok(events) => {
            st.events = events;
            st.error = None;
            st.rebuild_packets();
        }
        Err(e) => {
            st.events.clear();
            st.packets.clear();
            st.error = Some(e);
        }
    }
}

/// Annotations over the trace: colorful packet bubbles drawn along the bottom
/// of the graticule plot (matching PicoScope 7 serial decoding style).
pub fn draw(
    st: Res<DecodeState>,
    link: Res<Link>,
    layout: Res<crate::ui::layout::Layout>,
    deep: Res<crate::deep::DeepView>,
    mut gizmos: Gizmos,
) {
    if st.protocol == Protocol::Off || st.events.is_empty() || deep.on {
        return;
    }
    let Some(frame) = &link.latest else { return };
    let n = frame.channels.first().map_or(0, |c| c.data.len()).max(1);
    let w = layout.plot.width();
    let h = layout.plot.height();
    let o = layout.plot_center;
    let left = o.x - w / 2.0;
    let y = o.y - h / 2.0 + 16.0;

    for e in &st.events {
        let x0 = left + (e.start as f32 / n as f32) * w;
        let x1 = left + (e.end.max(e.start + 1) as f32 / n as f32) * w;
        let color = match &e.kind {
            EventKind::Error(_) => Color::srgb(0.95, 0.25, 0.2),
            EventKind::Marker(m) if *m == "SOF" || *m == "BREAK" => Color::srgb(0.95, 0.65, 0.1),
            EventKind::Marker(_) => Color::srgb(0.4, 0.8, 1.0),
            EventKind::Ack(true) => Color::srgb(0.2, 0.85, 0.4),
            EventKind::Ack(false) => Color::srgb(0.95, 0.5, 0.2),
            EventKind::Word { bits, .. } if *bits == 11 || *bits == 29 => Color::srgb(0.95, 0.55, 0.15),
            EventKind::Word { bits, .. } if *bits == 4 => Color::srgb(0.2, 0.8, 0.5),
            EventKind::Word { bits, .. } if *bits == 15 => Color::srgb(0.7, 0.4, 0.9),
            EventKind::Word { .. } => Color::srgb(0.25, 0.65, 0.95),
        };
        gizmos.line_2d(Vec2::new(x0, y), Vec2::new(x1, y), color);
        gizmos.line_2d(Vec2::new(x0, y - 5.0), Vec2::new(x0, y + 5.0), color);
    }
}
