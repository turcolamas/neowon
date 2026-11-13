//! Monitor de Tráfico de Red y Tramas de Bus (`CAN 2.0A/B`, `CAN FD`, `SAE J1939`, `LIN`, `SENT`)
//! con soporte de **Link Files (`.csv`)** estilo PicoScope 7.
//!
//! Agrupa mensajes en vivo por ID / PGN / Status, calcula frecuencia (Hz) y Δt (ms),
//! resalta bytes modificados en el payload, decodifica PGNs estándar SAE J1939 y
//! aplica diccionarios Link CSV definidos por el taller para traducir IDs a magnitudes reales.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusProtocol {
    Can,
    CanFd,
    J1939,
    Lin,
    Sent,
}

impl BusProtocol {
    pub const ALL: [BusProtocol; 5] = [
        BusProtocol::Can,
        BusProtocol::CanFd,
        BusProtocol::J1939,
        BusProtocol::Lin,
        BusProtocol::Sent,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BusProtocol::Can => "CAN 2.0A/B",
            BusProtocol::CanFd => "CAN FD",
            BusProtocol::J1939 => "SAE J1939",
            BusProtocol::Lin => "LIN 1.3/2.x",
            BusProtocol::Sent => "SENT (J2716)",
        }
    }
}

/// Campos extraídos de un identificador CAN extendido de 29 bits según SAE J1939.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct J1939Id {
    pub priority: u8,
    pub dp: u8,
    pub pf: u8,
    pub ps: u8,
    pub sa: u8,
    pub pgn: u32,
}

pub fn parse_j1939_id(id: u32) -> J1939Id {
    let priority = ((id >> 26) & 0x07) as u8;
    let dp = ((id >> 24) & 0x01) as u8;
    let pf = ((id >> 16) & 0xFF) as u8;
    let ps = ((id >> 8) & 0xFF) as u8;
    let sa = (id & 0xFF) as u8;
    let pgn = if pf < 240 {
        ((dp as u32) << 16) | ((pf as u32) << 8)
    } else {
        ((dp as u32) << 16) | ((pf as u32) << 8) | (ps as u32)
    };
    J1939Id {
        priority,
        dp,
        pf,
        ps,
        sa,
        pgn,
    }
}

/// Decodificación rápida de PGNs estándar SAE J1939 (vehículos pesados / diésel / agrícola).
pub fn decode_j1939_pgn(pgn: u32, sa: u8, data: &[u8]) -> Option<String> {
    let desc = match pgn {
        0xF004 => {
            // EEC1: Byte 2 = % Torque, Bytes 3..5 (LE) * 0.125 = RPM
            if data.len() >= 5 {
                let raw_rpm = u16::from_le_bytes([data[3], data[4]]);
                let rpm = if raw_rpm != 0xFFFF {
                    format!("{:.0} RPM", raw_rpm as f64 * 0.125)
                } else {
                    "N/A".into()
                };
                let trq = (data[2] as i32) - 125;
                format!("EEC1 Engine (SA:{sa:02X}) · {rpm} · Torque {trq}%")
            } else {
                format!("EEC1 Engine (SA:{sa:02X})")
            }
        }
        0xF003 => {
            let app = data.get(1).map_or(0.0, |&b| b as f64 * 0.4);
            format!("EEC2 Accelerator (SA:{sa:02X}) · Pedal {app:.1}%")
        }
        0xFEEE => {
            let ect = data.first().map(|&b| (b as i32) - 40);
            format!(
                "ET1 Engine Temp (SA:{sa:02X}) · ECT {}°C",
                ect.map_or("--".into(), |v| v.to_string())
            )
        }
        0xFEEF => {
            let oil_kpa = data.get(3).map_or(0, |&b| b as u32 * 4);
            format!("EFL/P1 Oil Pressure (SA:{sa:02X}) · {oil_kpa} kPa")
        }
        0xFEF1 => {
            if data.len() >= 3 {
                let spd = u16::from_le_bytes([data[1], data[2]]) as f64 / 256.0;
                format!("CCVS1 Cruise/Speed (SA:{sa:02X}) · {spd:.1} km/h")
            } else {
                format!("CCVS1 Vehicle Speed (SA:{sa:02X})")
            }
        }
        0xFEF2 => {
            if data.len() >= 2 {
                let lph = u16::from_le_bytes([data[0], data[1]]) as f64 * 0.05;
                format!("LFE1 Fuel Economy (SA:{sa:02X}) · {lph:.1} L/h")
            } else {
                format!("LFE1 Fuel Rate (SA:{sa:02X})")
            }
        }
        0xFEF7 => {
            if data.len() >= 6 {
                let bat_v = u16::from_le_bytes([data[4], data[5]]) as f64 * 0.05;
                format!("VEP1 Electrical Power (SA:{sa:02X}) · {bat_v:.2} V")
            } else {
                format!("VEP1 Battery Power (SA:{sa:02X})")
            }
        }
        0xFECA => format!("DM1 Active DTCs (SA:{sa:02X})"),
        0xFECB => format!("DM2 Previous DTCs (SA:{sa:02X})"),
        0xEC00 => format!("TP.CM Transport Connection (SA:{sa:02X})"),
        0xEB00 => format!("TP.DT Transport Data (SA:{sa:02X})"),
        _ => return None,
    };
    Some(desc)
}

/// Entrada de un Link File (.csv) estilo PicoScope para mapear ID/PGN a señales físicas.
#[derive(Debug, Clone)]
pub struct LinkEntry {
    pub id_or_pgn: u32,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub byte_index: Option<usize>,
    pub byte_len: u8,
    pub scale: f64,
    pub offset: f64,
}

impl LinkEntry {
    pub fn format_value(&self, data: &[u8]) -> String {
        if let Some(idx) = self.byte_index {
            if idx < data.len() {
                let raw: u32 = if self.byte_len >= 2 && idx + 1 < data.len() {
                    u16::from_be_bytes([data[idx], data[idx + 1]]) as u32
                } else {
                    data[idx] as u32
                };
                let eng = (raw as f64) * self.scale + self.offset;
                if self.unit.is_empty() {
                    return format!("{}: {:.2}", self.name, eng);
                }
                return format!("{}: {:.2} {}", self.name, eng, self.unit);
            }
        }
        if self.description.is_empty() {
            self.name.clone()
        } else if self.name.is_empty() {
            self.description.clone()
        } else {
            format!("{} ({})", self.name, self.description)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LinkDictionary {
    pub filename: String,
    pub entries: HashMap<u32, Vec<LinkEntry>>,
    pub entry_count: usize,
    pub error: Option<String>,
}

fn parse_hex_or_dec(s: &str) -> Option<u32> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else if t.chars().all(|c| c.is_ascii_hexdigit()) && t.chars().any(|c| c.is_ascii_alphabetic()) {
        u32::from_str_radix(t, 16).ok()
    } else {
        t.parse::<u32>().ok()
    }
}

impl LinkDictionary {
    pub fn clear(&mut self) {
        self.filename.clear();
        self.entries.clear();
        self.entry_count = 0;
        self.error = None;
    }

    /// Parsea un Link File `.csv` con columnas flexibles:
    /// `ID,Name[,Description,Unit,Byte,Length,Scale,Offset]`
    pub fn load_csv_str(&mut self, filename: &str, csv: &str) {
        self.clear();
        self.filename = filename.to_string();

        for (line_no, raw_line) in csv.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let sep = if line.contains(';') && !line.contains(',') { ';' } else { ',' };
            let cols: Vec<&str> = line.split(sep).map(str::trim).collect();
            if cols.len() < 2 {
                continue;
            }
            // Saltar cabecera
            if line_no == 0 && (cols[0].eq_ignore_ascii_case("id") || cols[0].eq_ignore_ascii_case("identifier") || cols[0].eq_ignore_ascii_case("pgn")) {
                continue;
            }
            let Some(id_val) = parse_hex_or_dec(cols[0]) else {
                continue;
            };
            let entry = LinkEntry {
                id_or_pgn: id_val,
                name: cols.get(1).copied().unwrap_or("").to_string(),
                description: cols.get(2).copied().unwrap_or("").to_string(),
                unit: cols.get(3).copied().unwrap_or("").to_string(),
                byte_index: cols.get(4).and_then(|s| s.parse::<usize>().ok()),
                byte_len: cols.get(5).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1).clamp(1, 2),
                scale: cols.get(6).and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0),
                offset: cols.get(7).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
            };
            self.entries.entry(id_val).or_default().push(entry);
            self.entry_count += 1;
        }

        if self.entry_count == 0 {
            self.error = Some("CSV sin filas válidas (formato: ID,Nombre,Descripción,Unidad,Byte,Bytes,Escala,Offset)".into());
        }
    }

    pub fn describe(&self, id: u32, is_extended: bool, bus: BusProtocol, data: &[u8]) -> Option<String> {
        if let Some(list) = self.entries.get(&id) {
            let parts: Vec<String> = list.iter().map(|e| e.format_value(data)).collect();
            if !parts.is_empty() {
                return Some(parts.join(" | "));
            }
        }
        if is_extended || bus == BusProtocol::J1939 {
            let j = parse_j1939_id(id);
            if let Some(list) = self.entries.get(&j.pgn) {
                let parts: Vec<String> = list.iter().map(|e| e.format_value(data)).collect();
                if !parts.is_empty() {
                    return Some(parts.join(" | "));
                }
            }
            if let Some(pgn_text) = decode_j1939_pgn(j.pgn, j.sa, data) {
                return Some(pgn_text);
            }
        }
        if bus == BusProtocol::Sent && data.len() >= 6 {
            let ch1 = ((data[0] as u16) << 8) | ((data[1] as u16) << 4) | (data[2] as u16);
            let ch2 = ((data[3] as u16) << 8) | ((data[4] as u16) << 4) | (data[5] as u16);
            return Some(format!("SENT CH1={} ({ch1:#05X}) · CH2={} ({ch2:#05X})", ch1, ch2));
        }
        None
    }

    /// Link File CSV de ejemplo automotriz (OBD-II / Engine CAN / J1939).
    pub fn sample_automotive_csv() -> &'static str {
        concat!(
            "ID,Name,Description,Unit,Byte,Bytes,Scale,Offset\n",
            "0x0C0,RPM,Régimen Motor CKP,RPM,0,2,0.25,0\n",
            "0x0D0,TPS,Posición Mariposa,%,2,1,0.392,0\n",
            "0x180,ECT,Temp. Refrigerante,°C,0,1,1.0,-40\n",
            "0x280,VBAT,Tensión Batería,V,1,1,0.1,0\n",
            "0x7E0,OBD_REQ,Petición Diagnosis ECU,,0,1,1,0\n",
            "0x7E8,OBD_RESP,Respuesta Diagnosis ECU,,0,1,1,0\n",
            "0xF004,J1939_EEC1,Motor Electrónico EEC1,,3,2,0.125,0\n"
        )
    }
}

#[derive(Debug, Clone)]
pub struct BusTrafficRow {
    pub id: u32,
    pub is_extended: bool,
    pub bus: BusProtocol,
    pub j1939_pgn: Option<u32>,
    pub signal_desc: Option<String>,
    pub count: u64,
    pub first_time: f64,
    pub last_time: f64,
    pub delta_ms: f64,
    pub freq_hz: f64,
    pub dlc: usize,
    pub data: Vec<u8>,
    pub prev_data: Vec<u8>,
    pub changed_mask: u64, // bit k = 1 si el byte k cambió (soporta hasta 64 bytes CAN FD)
    pub last_changed_time: f64,
}

#[derive(Debug, Clone)]
pub struct TrafficMonitor {
    pub active: bool,
    pub bus: BusProtocol,
    pub rows: HashMap<u32, BusTrafficRow>,
    pub filter_id: Option<u32>,
    pub filter_text: String,
    pub pause_updates: bool,
    pub total_frames_captured: u64,
    pub link_dict: LinkDictionary,
}

impl Default for TrafficMonitor {
    fn default() -> Self {
        Self {
            active: false,
            bus: BusProtocol::Can,
            rows: HashMap::new(),
            filter_id: None,
            filter_text: String::new(),
            pause_updates: false,
            total_frames_captured: 0,
            link_dict: LinkDictionary::default(),
        }
    }
}

impl TrafficMonitor {
    pub fn clear(&mut self) {
        self.rows.clear();
        self.total_frames_captured = 0;
    }

    pub fn refresh_descriptions(&mut self) {
        for r in self.rows.values_mut() {
            r.signal_desc = self.link_dict.describe(r.id, r.is_extended, r.bus, &r.data);
        }
    }

    /// Ingesta genérica para cualquier protocolo de bus (`CAN`, `CAN FD`, `J1939`, `LIN`, `SENT`).
    pub fn ingest_packet(
        &mut self,
        bus: BusProtocol,
        timestamp: f64,
        id: u32,
        is_extended: bool,
        dlc: usize,
        data: &[u8],
    ) {
        if self.pause_updates {
            return;
        }

        self.total_frames_captured += 1;
        let pgn = (is_extended || bus == BusProtocol::J1939).then(|| parse_j1939_id(id).pgn);
        let desc = self.link_dict.describe(id, is_extended, bus, data);

        let row = self.rows.entry(id).or_insert_with(|| BusTrafficRow {
            id,
            is_extended,
            bus,
            j1939_pgn: pgn,
            signal_desc: desc.clone(),
            count: 0,
            first_time: timestamp,
            last_time: timestamp,
            delta_ms: 0.0,
            freq_hz: 0.0,
            dlc,
            data: data.to_vec(),
            prev_data: data.to_vec(),
            changed_mask: 0,
            last_changed_time: timestamp,
        });

        row.is_extended |= is_extended;
        row.bus = bus;
        row.j1939_pgn = pgn.or(row.j1939_pgn);
        row.signal_desc = desc;
        row.count += 1;
        if row.count > 1 {
            let dt = timestamp - row.last_time;
            if dt > 0.0 {
                row.delta_ms = dt * 1000.0;
                let inst_freq = 1.0 / dt;
                row.freq_hz = row.freq_hz * 0.7 + inst_freq * 0.3;
            }
        }
        row.last_time = timestamp;
        row.dlc = dlc;

        let mut mask = 0u64;
        for (i, (&new_b, &old_b)) in data.iter().zip(row.data.iter()).enumerate() {
            if i < 64 && new_b != old_b {
                mask |= 1u64 << i;
            }
        }

        if mask != 0 {
            row.prev_data = row.data.clone();
            row.changed_mask = mask;
            row.last_changed_time = timestamp;
        } else if timestamp - row.last_changed_time > 1.5 {
            row.changed_mask = 0;
        }

        row.data = data.to_vec();
    }

    /// Ingesta un paquete CAN / CAN FD / J1939 decodificado
    pub fn ingest_can(
        &mut self,
        timestamp: f64,
        id: u32,
        is_extended: bool,
        dlc: usize,
        data: &[u8],
    ) {
        let bus = if dlc > 8 {
            BusProtocol::CanFd
        } else if is_extended && self.bus == BusProtocol::J1939 {
            BusProtocol::J1939
        } else {
            BusProtocol::Can
        };
        self.ingest_packet(bus, timestamp, id, is_extended, dlc, data);
    }

    /// Ingesta un paquete LIN decodificado
    pub fn ingest_lin(&mut self, timestamp: f64, pid: u8, dlc: usize, data: &[u8]) {
        self.ingest_packet(BusProtocol::Lin, timestamp, pid as u32, false, dlc, data);
    }

    /// Devuelve las filas ordenadas por ID filtradas por ID o texto de Link File
    pub fn get_sorted_rows(&self) -> Vec<&BusTrafficRow> {
        let q = self.filter_text.trim().to_lowercase();
        let mut list: Vec<&BusTrafficRow> = self
            .rows
            .values()
            .filter(|r| {
                if let Some(fid) = self.filter_id {
                    if r.id != fid {
                        return false;
                    }
                }
                if !q.is_empty() {
                    let id_hex = format!("0x{:x}", r.id);
                    let desc = r.signal_desc.as_deref().unwrap_or("").to_lowercase();
                    if !id_hex.contains(&q) && !desc.contains(&q) {
                        return false;
                    }
                }
                true
            })
            .collect();
        list.sort_by_key(|r| r.id);
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_csv_and_j1939_decode() {
        let mut dict = LinkDictionary::default();
        dict.load_csv_str("auto.csv", LinkDictionary::sample_automotive_csv());
        assert!(dict.error.is_none());
        assert_eq!(dict.entry_count, 7);

        let desc = dict.describe(0x180, false, BusProtocol::Can, &[130]);
        assert!(desc.as_deref().unwrap().contains("90.00 °C"), "{desc:?}");

        // 29-bit J1939 PGN 0xF004 (EEC1) from SA 0x00: bytes 3..5 = 8000 -> 1000 RPM
        let id = 0x0CF0_0400u32;
        let j = parse_j1939_id(id);
        assert_eq!(j.pgn, 0xF004);
        assert_eq!(j.sa, 0x00);
        let eec1 = decode_j1939_pgn(j.pgn, j.sa, &[0, 0, 125, 0x40, 0x1F, 0, 0, 0]).unwrap();
        assert!(eec1.contains("1000 RPM"), "{eec1}");
    }
}
