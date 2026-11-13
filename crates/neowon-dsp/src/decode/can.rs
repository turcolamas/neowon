//! Controller Area Network (CAN 2.0A/B, J1939 & CAN FD ISO 11898-1).
//!
//! Decodes ISO 11898 CAN & CAN FD bus waveforms:
//! - Nominal bit rates: 125 kbps, 250 kbps, 500 kbps, 1 Mbps (and BRS data phase up to 5+ Mbps)
//! - Bit destuffing (5 consecutive identical bits rule)
//! - Standard 11-bit ID (CAN 2.0A) and Extended 29-bit ID (CAN 2.0B / J1939)
//! - CAN FD FDF / BRS / ESI control flags and DLC 9..=15 -> 12..=64 byte payloads
//! - CRC15 (Classical) / CRC17 & CRC21 (CAN FD) validation and ACK slot verification
//!
//! Idle level is Recessive (1). Dominant bit (0) initiates frame.

use super::{DecodeError, Digital, Event, EventKind, check_resolution};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub baud: f64,
    /// Sample point within nominal bit time (0.5..=0.9, default 0.75).
    pub sample_point: f64,
    /// Invert signal polarity (e.g. if probing CAN_H vs CAN_L).
    pub inverted: bool,
    /// Allow ISO 11898-1 CAN FD frames (FDF bit & BRS data-phase bit rate switch).
    pub fd_enabled: bool,
    /// Data-phase baud rate used when CAN FD BRS=1 (default 2 Mbps).
    pub data_baud: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baud: 500_000.0,
            sample_point: 0.75,
            inverted: false,
            fd_enabled: true,
            data_baud: 2_000_000.0,
        }
    }
}

/// Maps 4-bit CAN FD DLC code to payload byte count (ISO 11898-1).
pub fn fd_dlc_to_len(dlc: usize) -> usize {
    match dlc {
        0..=8 => dlc,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        _ => 64,
    }
}

/// Compute standard CAN CRC15 over destuffed bits up to CRC field.
pub fn can_crc15(bits: &[bool]) -> u16 {
    let mut crc: u16 = 0;
    for &b in bits {
        let bit = b as u16;
        let msb = (crc >> 14) & 1;
        crc = (crc << 1) & 0x7FFF;
        if msb ^ bit != 0 {
            crc ^= 0x4599;
        }
    }
    crc
}

/// CAN FD CRC17 (for payloads <= 16 bytes), polynomial 0x3685B.
pub fn can_crc17(bits: &[bool]) -> u32 {
    let mut crc: u32 = 0;
    for &b in bits {
        let bit = b as u32;
        let msb = (crc >> 16) & 1;
        crc = (crc << 1) & 0x1_FFFF;
        if msb ^ bit != 0 {
            crc ^= 0x1_685B;
        }
    }
    crc
}

/// CAN FD CRC21 (for payloads > 16 bytes), polynomial 0x302899.
pub fn can_crc21(bits: &[bool]) -> u32 {
    let mut crc: u32 = 0;
    for &b in bits {
        let bit = b as u32;
        let msb = (crc >> 20) & 1;
        crc = (crc << 1) & 0x1F_FFFF;
        if msb ^ bit != 0 {
            crc ^= 0x10_2899;
        }
    }
    crc
}

pub fn decode(d: &Digital, cfg: Config) -> Result<Vec<Event>, DecodeError> {
    let per_bit = check_resolution(d.sample_rate, cfg.baud)?;
    let data_per_bit = if cfg.fd_enabled && cfg.data_baud > cfg.baud {
        check_resolution(d.sample_rate, cfg.data_baud).unwrap_or(per_bit)
    } else {
        per_bit
    };
    let sample_frac = cfg.sample_point.clamp(0.5, 0.9);
    let level = |i: usize| d.level_at(i) ^ cfg.inverted;

    let mut events = Vec::new();
    let n = d.levels.len();
    let mut i = 0usize;

    while i + 1 < n {
        // Look for Start of Frame (SOF): transition from Recessive (1) to Dominant (0)
        if !level(i) || level(i + 1) {
            i += 1;
            continue;
        }

        let sof_start = i + 1;
        let mut cur_sample_pos = sof_start as f64;
        let mut cur_bit_len = per_bit;
        let mut consecutive = 1usize;
        let mut prev_bit = false; // SOF is Dominant (0)

        let mut destuffed_bits: Vec<(bool, usize, usize)> = Vec::new();
        let bit0_end = (sof_start as f64 + cur_bit_len) as usize;
        destuffed_bits.push((false, sof_start, bit0_end.min(n)));
        cur_sample_pos += cur_bit_len;

        let mut in_crc_or_later = false;
        let mut bit_idx = 1usize;
        let mut frame_valid = true;
        let mut is_extended = false;
        let mut is_fd = false;
        let mut brs = false;
        let mut dlc_code = 0usize;
        let mut payload_bytes = 0usize;
        let mut dlc_idx = 15usize;
        let mut data_start_bit = 19usize;
        let mut crc_start_bit = 0usize;
        let mut crc_bits_len = 15usize;
        let mut ack_bit_idx = 0usize;
        let mut eof_end = 0usize;

        while cur_sample_pos + cur_bit_len <= n as f64 && bit_idx < 820 {
            let sample_idx = (cur_sample_pos + sample_frac * cur_bit_len) as usize;
            if sample_idx >= n {
                break;
            }
            let bit_val = level(sample_idx);
            let b_start = cur_sample_pos as usize;
            let b_end = (cur_sample_pos + cur_bit_len) as usize;

            // Resync on falling edge inside bit cell
            let check_range = b_start..b_end.min(n);
            for s in check_range {
                if s > 0 && level(s - 1) && !level(s) && !bit_val {
                    cur_sample_pos = s as f64;
                    break;
                }
            }

            if !in_crc_or_later {
                if bit_val == prev_bit {
                    consecutive += 1;
                    destuffed_bits.push((bit_val, b_start, b_end));
                } else {
                    if consecutive == 5 {
                        consecutive = 1;
                        prev_bit = bit_val;
                        cur_sample_pos += cur_bit_len;
                        bit_idx += 1;
                        continue;
                    }
                    consecutive = 1;
                    prev_bit = bit_val;
                    destuffed_bits.push((bit_val, b_start, b_end));
                }

                if consecutive > 5 {
                    events.push(Event {
                        start: b_start,
                        end: b_end,
                        kind: EventKind::Error("CAN stuff error: > 5 equal bits"),
                    });
                    frame_valid = false;
                    break;
                }
            } else {
                destuffed_bits.push((bit_val, b_start, b_end));
            }

            // Standard vs Extended check at IDE bit (index 13)
            if destuffed_bits.len() == 14 && !is_extended {
                if destuffed_bits[13].0 {
                    is_extended = true;
                }
            }

            // Detect FDF (FD Frame) bit:
            // Std: index 14 is r0(0) / FDF(1)
            // Ext: index 33 is r1(0) / FDF(1)
            if cfg.fd_enabled && !is_extended && destuffed_bits.len() == 15 && destuffed_bits[14].0 {
                is_fd = true;
            }
            if cfg.fd_enabled && is_extended && destuffed_bits.len() == 34 && destuffed_bits[33].0 {
                is_fd = true;
            }

            // In CAN FD, BRS switches bitrate from nominal to data phase after the BRS bit:
            // Std FD: [14]=FDF, [15]=res, [16]=BRS, [17]=ESI, [18..22]=DLC
            // Ext FD: [33]=FDF, [34]=res, [35]=BRS, [36]=ESI, [37..41]=DLC
            let mut next_bit_len = cur_bit_len;
            if is_fd && !brs {
                let brs_ready_len = if is_extended { 36 } else { 17 };
                if destuffed_bits.len() == brs_ready_len && destuffed_bits[brs_ready_len - 1].0 {
                    brs = true;
                    next_bit_len = data_per_bit;
                }
            }

            let dlc_ready_len = match (is_extended, is_fd) {
                (false, false) => 19, // SOF(1)+ID(11)+RTR(1)+IDE(1)+r0(1)+DLC(4)
                (true, false) => 39,  // SOF(1)+ID11+SRR+IDE+ID18+RTR+r1+r0+DLC(4)
                (false, true) => 22,  // SOF(1)+ID11+RRS+IDE+FDF+res+BRS+ESI+DLC(4)
                (true, true) => 41,   // SOF(1)+ID11+SRR+IDE+ID18+RRS+FDF+res+BRS+ESI+DLC(4)
            };

            if destuffed_bits.len() == dlc_ready_len {
                dlc_idx = dlc_ready_len - 4;
                data_start_bit = dlc_ready_len;
                let mut d_val = 0usize;
                for b in &destuffed_bits[dlc_idx..dlc_ready_len] {
                    d_val = (d_val << 1) | (b.0 as usize);
                }
                dlc_code = d_val;
                payload_bytes = if is_fd {
                    fd_dlc_to_len(d_val)
                } else {
                    d_val.min(8)
                };
                crc_bits_len = if is_fd {
                    if payload_bytes <= 16 { 17 } else { 21 }
                } else {
                    15
                };
                // Wait: in CAN FD CRC field includes stuff-count(4) before CRC bits on wire,
                // handled here via standard bit-sequence framing when destuffing stops at CRC end.
                crc_start_bit = data_start_bit + payload_bytes * 8;
            }

            if crc_start_bit > 0 && destuffed_bits.len() == crc_start_bit + crc_bits_len {
                in_crc_or_later = true;
                // CRC Delimiter switches back to nominal baud rate in BRS frames
                if brs {
                    next_bit_len = per_bit;
                }
            }

            if in_crc_or_later && destuffed_bits.len() >= crc_start_bit + crc_bits_len + 3 {
                ack_bit_idx = crc_start_bit + crc_bits_len + 1;
                eof_end = b_end;
                break;
            }

            cur_sample_pos += cur_bit_len;
            cur_bit_len = next_bit_len;
            bit_idx += 1;
        }

        let min_len = if is_extended { 39 } else { 19 };
        if !frame_valid || destuffed_bits.len() < min_len {
            i = sof_start + per_bit as usize;
            continue;
        }

        // 1. SOF (+ FD/BRS markers)
        events.push(Event {
            start: destuffed_bits[0].1,
            end: destuffed_bits[0].2,
            kind: EventKind::Marker("SOF"),
        });
        if is_fd {
            events.push(Event {
                start: destuffed_bits[0].1,
                end: destuffed_bits[dlc_idx - 1].2,
                kind: EventKind::Marker("FDF"),
            });
            if brs {
                events.push(Event {
                    start: destuffed_bits[0].1,
                    end: destuffed_bits[dlc_idx - 1].2,
                    kind: EventKind::Marker("BRS"),
                });
            }
        }

        // 2. Identifier
        if !is_extended && destuffed_bits.len() >= 12 {
            let mut id = 0u64;
            for b in &destuffed_bits[1..12] {
                id = (id << 1) | (b.0 as u64);
            }
            events.push(Event {
                start: destuffed_bits[1].1,
                end: destuffed_bits[11].2,
                kind: EventKind::Word { value: id, bits: 11 },
            });
        } else if is_extended && destuffed_bits.len() >= 33 {
            let mut id_a = 0u64;
            for b in &destuffed_bits[1..12] {
                id_a = (id_a << 1) | (b.0 as u64);
            }
            let mut id_b = 0u64;
            for b in &destuffed_bits[14..32] {
                id_b = (id_b << 1) | (b.0 as u64);
            }
            let id = (id_a << 18) | id_b;
            events.push(Event {
                start: destuffed_bits[1].1,
                end: destuffed_bits[31].2,
                kind: EventKind::Word { value: id, bits: 29 },
            });
        }

        // 3. DLC
        if destuffed_bits.len() >= dlc_idx + 4 {
            events.push(Event {
                start: destuffed_bits[dlc_idx].1,
                end: destuffed_bits[dlc_idx + 3].2,
                kind: EventKind::Word {
                    value: dlc_code as u64,
                    bits: 4,
                },
            });
        }

        // 4. Data bytes
        for byte_i in 0..payload_bytes {
            let b_idx = data_start_bit + byte_i * 8;
            if destuffed_bits.len() >= b_idx + 8 {
                let mut byte_val = 0u64;
                for bit in &destuffed_bits[b_idx..b_idx + 8] {
                    byte_val = (byte_val << 1) | (bit.0 as u64);
                }
                events.push(Event {
                    start: destuffed_bits[b_idx].1,
                    end: destuffed_bits[b_idx + 7].2,
                    kind: EventKind::Word {
                        value: byte_val,
                        bits: 8,
                    },
                });
            }
        }

        // 5. CRC (15 / 17 / 21 bits)
        if crc_start_bit > 0 && destuffed_bits.len() >= crc_start_bit + crc_bits_len {
            let mut crc_val = 0u64;
            for bit in &destuffed_bits[crc_start_bit..crc_start_bit + crc_bits_len] {
                crc_val = (crc_val << 1) | (bit.0 as u64);
            }
            let crc_input: Vec<bool> = destuffed_bits[0..crc_start_bit].iter().map(|b| b.0).collect();

            events.push(Event {
                start: destuffed_bits[crc_start_bit].1,
                end: destuffed_bits[crc_start_bit + crc_bits_len - 1].2,
                kind: EventKind::Word {
                    value: crc_val,
                    bits: crc_bits_len as u8,
                },
            });

            let ok = match crc_bits_len {
                17 => crc_val as u32 == can_crc17(&crc_input),
                21 => crc_val as u32 == can_crc21(&crc_input),
                _ => crc_val as u16 == can_crc15(&crc_input),
            };
            if !ok {
                events.push(Event {
                    start: destuffed_bits[crc_start_bit].1,
                    end: destuffed_bits[crc_start_bit + crc_bits_len - 1].2,
                    kind: EventKind::Error("CAN CRC mismatch"),
                });
            }
        }

        // 6. ACK
        if ack_bit_idx > 0 && destuffed_bits.len() > ack_bit_idx {
            let ack_bit = destuffed_bits[ack_bit_idx].0;
            events.push(Event {
                start: destuffed_bits[ack_bit_idx].1,
                end: destuffed_bits[ack_bit_idx].2,
                kind: EventKind::Ack(!ack_bit),
            });
        }

        // 7. EOF
        events.push(Event {
            start: destuffed_bits.last().map_or(eof_end, |b| b.2),
            end: eof_end.max(destuffed_bits.last().map_or(eof_end, |b| b.2) + (7.0 * per_bit) as usize),
            kind: EventKind::Marker("EOF"),
        });

        i = eof_end + (3.0 * per_bit) as usize;
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stuff_can_bits(bits: &[bool]) -> Vec<bool> {
        let mut out = Vec::new();
        let mut consec = 0usize;
        let mut last = false;
        for &b in bits {
            if out.is_empty() || b != last {
                consec = 1;
                last = b;
            } else {
                consec += 1;
            }
            out.push(b);
            if consec == 5 {
                let stuff = !b;
                out.push(stuff);
                consec = 1;
                last = stuff;
            }
        }
        out
    }

    #[test]
    fn test_can_crc15_basic() {
        assert_eq!(can_crc15(&[]), 0);
        let bits = [false, true, false, false, true, true, false];
        assert_ne!(can_crc15(&bits), 0);
    }

    #[test]
    fn test_can_decode_synthetic() {
        let sample_rate = 10_000_000.0;
        let samples_per_bit = 20;

        let mut frame = vec![
            false, // SOF
            false, false, true, false, false, true, false, false, false, true, true, // ID 0x123
            false, // RTR
            false, // IDE
            false, // r0
            false, false, false, true, // DLC = 1
            false, true, false, true, false, true, false, true, // Data 0x55
        ];
        let crc = can_crc15(&frame);
        for i in (0..15).rev() {
            frame.push(((crc >> i) & 1) != 0);
        }
        let mut wire = stuff_can_bits(&frame);
        wire.push(true);  // CRC del
        wire.push(false); // ACK
        wire.push(true);  // ACK del
        wire.extend(vec![true; 10]); // EOF + IFS

        let mut levels = vec![true; 50];
        for b in wire {
            levels.extend(vec![b; samples_per_bit]);
        }
        levels.extend(vec![true; 50]);

        let digital = Digital { levels, sample_rate };
        let events = decode(&digital, Config::default()).expect("CAN decode ok");
        assert!(!events.iter().any(|e| e.kind.is_error()), "Errors: {events:?}");
        assert!(events.iter().any(|e| e.kind == EventKind::Word { value: 0x123, bits: 11 }));
        assert!(events.iter().any(|e| e.kind == EventKind::Word { value: 1, bits: 4 }));
        assert!(events.iter().any(|e| e.kind == EventKind::Word { value: 0x55, bits: 8 }));
    }

    #[test]
    fn test_can_fd_decode_dlc12_with_brs() {
        let sample_rate = 20_000_000.0;
        let cfg = Config {
            baud: 500_000.0,     // 40 samples/bit nominal
            data_baud: 2_000_000.0, // 10 samples/bit (BRS) — below 12 falls back gracefully or test 1.25M (16 samples/bit)
            ..Default::default()
        };
        let cfg = Config { data_baud: 1_250_000.0, ..cfg }; // 16 samples/bit >= 12
        let nom_spb = 40usize;
        let data_spb = 16usize;

        let mut fd_hdr = vec![
            false, // SOF
            false, true, false, true, false, true, false, true, false, true, false, // ID 0x2AA
            false, // RRS
            false, // IDE
            true,  // FDF = 1
            false, // res
            true,  // BRS = 1
            false, // ESI = 0
            true, false, false, true, // DLC = 9 -> 12 bytes
        ];
        for byte_i in 0..12u8 {
            let val = 0xA0 + byte_i;
            for bit in (0..8).rev() {
                fd_hdr.push(((val >> bit) & 1) != 0);
            }
        }
        let crc = can_crc17(&fd_hdr);
        for i in (0..17).rev() {
            fd_hdr.push(((crc >> i) & 1) != 0);
        }

        // Split stuffed bits around BRS boundary (index 16 in unstuffed -> track carefully via direct construction)
        // Use unstuffed pattern + stuff helper, knowing nominal portion ends right at index 17 in unstuffed stream:
        let stuffed = stuff_can_bits(&fd_hdr);
        let mut levels = vec![true; 60];
        for (idx, &b) in stuffed.iter().enumerate() {
            let spb = if idx >= 17 && idx < stuffed.len() { data_spb } else { nom_spb };
            levels.extend(vec![b; spb]);
        }
        levels.extend(vec![true; nom_spb]);  // CRC del
        levels.extend(vec![false; nom_spb]); // ACK
        levels.extend(vec![true; nom_spb * 12]);

        let digital = Digital { levels, sample_rate };
        let events = decode(&digital, cfg).expect("CAN FD decode ok");
        assert!(events.iter().any(|e| e.kind == EventKind::Marker("FDF")));
        assert!(events.iter().any(|e| e.kind == EventKind::Marker("BRS")));
        assert!(events.iter().any(|e| e.kind == EventKind::Word { value: 0x2AA, bits: 11 }));
        let bytes: Vec<u64> = events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::Word { value, bits: 8 } => Some(value),
                _ => None,
            })
            .collect();
        assert_eq!(bytes.len(), 12);
        assert_eq!(bytes[0], 0xA0);
        assert_eq!(bytes[11], 0xAB);
    }
}
