//! Controller Area Network (CAN 2.0A and 2.0B).
//!
//! Decodes ISO 11898 CAN bus waveforms:
//! - Nominal bit rates: 125 kbps, 250 kbps, 500 kbps, 1 Mbps
//! - Bit destuffing (5 consecutive identical bits rule)
//! - Standard 11-bit ID (CAN 2.0A) and Extended 29-bit ID (CAN 2.0B)
//! - CRC15 validation
//! - ACK slot verification
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baud: 500_000.0,
            sample_point: 0.75,
            inverted: false,
        }
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

pub fn decode(d: &Digital, cfg: Config) -> Result<Vec<Event>, DecodeError> {
    let per_bit = check_resolution(d.sample_rate, cfg.baud)?;
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
        let mut consecutive = 1usize;
        let mut prev_bit = false; // SOF is Dominant (0)

        // Raw decoded bits with their sample index ranges
        let mut raw_bits: Vec<(bool, usize, usize)> = Vec::new();
        // Destuffed payload bits for CRC and field parsing
        let mut destuffed_bits: Vec<(bool, usize, usize)> = Vec::new();

        // Push SOF bit
        let bit0_end = (sof_start as f64 + per_bit) as usize;
        raw_bits.push((false, sof_start, bit0_end.min(n)));
        destuffed_bits.push((false, sof_start, bit0_end.min(n)));
        cur_sample_pos += per_bit;

        let mut in_crc_or_later = false;
        let mut bit_idx = 1usize;
        let mut frame_valid = true;
        let mut is_extended = false;
        let mut dlc = 0usize;
        let mut payload_bytes = 0usize;
        let mut crc_start_bit = 0usize;
        let mut ack_bit_idx = 0usize;
        let mut eof_end = 0usize;

        while cur_sample_pos + per_bit < n as f64 && bit_idx < 160 {
            let sample_idx = (cur_sample_pos + sample_frac * per_bit) as usize;
            if sample_idx >= n {
                break;
            }
            let bit_val = level(sample_idx);
            let b_start = cur_sample_pos as usize;
            let b_end = (cur_sample_pos + per_bit) as usize;

            // Check for hard resync on any falling edge (Recessive -> Dominant)
            // within the bit cell
            let check_range = b_start..b_end.min(n);
            for s in check_range {
                if s > 0 && level(s - 1) && !level(s) && !bit_val {
                    // Resync clock to edge
                    cur_sample_pos = s as f64;
                    break;
                }
            }

            raw_bits.push((bit_val, b_start, b_end));

            // Bit destuffing logic applies up to the end of the CRC sequence (before CRC Delimiter)
            if !in_crc_or_later {
                if bit_val == prev_bit {
                    consecutive += 1;
                    destuffed_bits.push((bit_val, b_start, b_end));
                } else {
                    if consecutive == 5 {
                        // This was a stuff bit! Verify it's opposite polarity and skip it from destuffed stream
                        consecutive = 1;
                        prev_bit = bit_val;
                        // Move to next bit without adding to destuffed_bits
                        cur_sample_pos += per_bit;
                        bit_idx += 1;
                        continue;
                    }
                    consecutive = 1;
                    prev_bit = bit_val;
                    destuffed_bits.push((bit_val, b_start, b_end));
                }

                if consecutive > 5 {
                    // Stuff error!
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

            // Check header fields as destuffed bits accumulate
            // Standard frame header: SOF (1) + ID (11) + RTR (1) + IDE (1) = 14 bits
            if destuffed_bits.len() == 14 && !is_extended {
                let ide_bit = destuffed_bits[13].0;
                if ide_bit {
                    is_extended = true;
                }
            }

            // Once control field is reached, parse DLC
            // Standard: SOF(1) + ID(11) + RTR(1) + IDE(0) + r0(1) + DLC(4) = 18 bits
            // Extended: SOF(1) + ID(11) + SRR(1) + IDE(1) + IDext(18) + RTR(1) + r1(1) + r0(1) + DLC(4) = 39 bits
            let dlc_ready_len = if is_extended { 39 } else { 18 };
            if destuffed_bits.len() == dlc_ready_len {
                let dlc_bits = &destuffed_bits[dlc_ready_len - 4..dlc_ready_len];
                let mut d_val = 0usize;
                for b in dlc_bits {
                    d_val = (d_val << 1) | (b.0 as usize);
                }
                dlc = d_val.min(8);
                payload_bytes = dlc;
                let data_bits_total = payload_bytes * 8;
                let _crc_len = 15;
                crc_start_bit = dlc_ready_len + data_bits_total;
            }

            if crc_start_bit > 0 && destuffed_bits.len() == crc_start_bit + 15 {
                in_crc_or_later = true;
            }

            if in_crc_or_later && destuffed_bits.len() >= crc_start_bit + 15 + 3 {
                ack_bit_idx = crc_start_bit + 15 + 1; // CRC delimiter is +0, ACK is +1
                eof_end = b_end;
                break;
            }

            cur_sample_pos += per_bit;
            bit_idx += 1;
        }

        if !frame_valid || destuffed_bits.len() < 18 {
            i = sof_start + per_bit as usize;
            continue;
        }

        // Emit Decoded CAN Events:
        // 1. SOF
        events.push(Event {
            start: destuffed_bits[0].1,
            end: destuffed_bits[0].2,
            kind: EventKind::Marker("SOF"),
        });

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
        let dlc_idx = if is_extended { 35 } else { 14 };
        if destuffed_bits.len() >= dlc_idx + 4 {
            events.push(Event {
                start: destuffed_bits[dlc_idx].1,
                end: destuffed_bits[dlc_idx + 3].2,
                kind: EventKind::Word { value: dlc as u64, bits: 4 },
            });
        }

        // 4. Data bytes
        let data_start = dlc_idx + 4;
        for byte_i in 0..payload_bytes {
            let b_idx = data_start + byte_i * 8;
            if destuffed_bits.len() >= b_idx + 8 {
                let mut byte_val = 0u64;
                for bit in &destuffed_bits[b_idx..b_idx + 8] {
                    byte_val = (byte_val << 1) | (bit.0 as u64);
                }
                events.push(Event {
                    start: destuffed_bits[b_idx].1,
                    end: destuffed_bits[b_idx + 7].2,
                    kind: EventKind::Word { value: byte_val, bits: 8 },
                });
            }
        }

        // 5. CRC
        if crc_start_bit > 0 && destuffed_bits.len() >= crc_start_bit + 15 {
            let mut crc_val = 0u64;
            for bit in &destuffed_bits[crc_start_bit..crc_start_bit + 15] {
                crc_val = (crc_val << 1) | (bit.0 as u64);
            }
            let crc_bits: Vec<bool> = destuffed_bits[0..crc_start_bit].iter().map(|b| b.0).collect();
            let expected_crc = can_crc15(&crc_bits);

            events.push(Event {
                start: destuffed_bits[crc_start_bit].1,
                end: destuffed_bits[crc_start_bit + 14].2,
                kind: EventKind::Word { value: crc_val, bits: 15 },
            });

            if crc_val as u16 != expected_crc {
                events.push(Event {
                    start: destuffed_bits[crc_start_bit].1,
                    end: destuffed_bits[crc_start_bit + 14].2,
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
                kind: EventKind::Ack(!ack_bit), // 0 dominant is ACK, 1 recessive is NAK
            });
        }

        // 7. EOF
        events.push(Event {
            start: destuffed_bits.last().map_or(eof_end, |b| b.2),
            end: eof_end.max(destuffed_bits.last().map_or(eof_end, |b| b.2) + (7.0 * per_bit) as usize),
            kind: EventKind::Marker("EOF"),
        });

        // Skip to after EOF
        i = eof_end + (3.0 * per_bit) as usize;
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_crc15_basic() {
        // Simple test for CAN CRC calculation: empty stream has CRC 0
        let crc = can_crc15(&[]);
        assert_eq!(crc, 0);

        // Deterministic check of polynomial properties
        let bits = [false, true, false, false, true, true, false];
        let crc1 = can_crc15(&bits);
        assert_ne!(crc1, 0);
    }

    #[test]
    fn test_can_decode_synthetic() {
        // Construct a synthetic CAN 2.0A frame at 500 kbps, sample rate 10 MS/s (20 samples/bit)
        let sample_rate = 10_000_000.0;
        let baud = 500_000.0;
        let samples_per_bit = 20;

        let mut levels = Vec::new();
        // 50 samples of Recessive idle (true)
        levels.extend(vec![true; 50]);

        // Synthesize Standard CAN frame:
        // SOF (0)
        // ID 0x123 = 00100100011 (11 bits) -> No more than 3 consecutive identical bits
        // RTR (0)
        // IDE (0)
        // r0 (0)
        // DLC (4) = 0001 (DLC = 1) -> 4 bits
        // Data 0x55 = 01010101 (8 bits)
        // Bits so far:
        // SOF: 0
        // ID: 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1 (11 bits)
        // RTR: 0
        // IDE: 0
        // r0: 0 (Note: consecutive 0s here: RTR(0), IDE(0), r0(0)... let's watch stuff bits)
        let mut raw_stream = vec![
            false, // SOF
            false, false, true, false, false, true, false, false, false, true, true, // ID 0x123
            false, // RTR
            false, // IDE
            false, // r0
            false, false, false, true, // DLC = 1
            false, true, false, true, false, true, false, true, // Data 0x55
        ];

        // Compute CRC15 over these bits
        let crc = can_crc15(&raw_stream);
        for i in (0..15).rev() {
            raw_stream.push(((crc >> i) & 1) != 0);
        }
        // CRC Delimiter (1)
        raw_stream.push(true);
        // ACK Slot (0)
        raw_stream.push(false);
        // ACK Delimiter (1)
        raw_stream.push(true);
        // EOF (7 recessive bits)
        raw_stream.extend(vec![true; 7]);
        // IFS (3 recessive bits)
        raw_stream.extend(vec![true; 3]);

        for b in raw_stream {
            levels.extend(vec![b; samples_per_bit]);
        }
        levels.extend(vec![true; 50]);

        let digital = Digital {
            levels,
            sample_rate,
        };

        let cfg = Config {
            baud,
            sample_point: 0.75,
            inverted: false,
        };

        let res = decode(&digital, cfg);
        assert!(res.is_ok(), "CAN decoding should succeed");
        let events = res.unwrap();
        assert!(!events.is_empty(), "Should decode at least one CAN frame");
        
        // Check that SOF and ID were found
        let has_id = events.iter().any(|e| match e.kind {
            EventKind::Word { value, bits } => bits == 11 && value == 0x123,
            _ => false,
        });
        assert!(has_id, "Should correctly decode ID 0x123");
    }
}
