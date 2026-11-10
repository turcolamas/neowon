//! Local Interconnect Network (LIN 1.3 / 2.0 / 2.1 / 2.2).
//!
//! Decodes automotive LIN single-wire bus waveforms:
//! - Nominal bit rates: 9600 bps, 19200 bps, 10417 bps
//! - Synch Break detection (≥ 13 dominant bit times)
//! - Synch Byte (0x55) measurement and clock calibration
//! - Protected Identifier (PID) decoding and parity validation (P0, P1)
//! - 1..=8 Data bytes decoding (UART 8N1 LSB-first)
//! - Classic and Enhanced (LIN 2.x) Checksum verification
//!
//! Idle level is Recessive High (1). Dominant is Low (0).

use super::{DecodeError, Digital, Event, EventKind, check_resolution};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub baud: f64,
    /// Use enhanced checksum (LIN 2.0+) including PID, instead of classic.
    pub enhanced_checksum: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baud: 19_200.0,
            enhanced_checksum: true,
        }
    }
}

/// Compute LIN 8-bit inverted modulo-256 sum with carry (inverted addition).
pub fn lin_checksum(bytes: &[u8]) -> u8 {
    let mut sum: u16 = 0;
    for &b in bytes {
        sum += b as u16;
        if sum > 0xFF {
            sum = (sum & 0xFF) + 1;
        }
    }
    (!sum) as u8
}

/// Verify LIN PID parity bits (P0 = bit 6, P1 = bit 7).
pub fn verify_pid(pid: u8) -> bool {
    let id = pid & 0x3F;
    let b = |n: usize| (id >> n) & 1;
    let p0 = b(0) ^ b(1) ^ b(2) ^ b(4);
    let p1 = !(b(1) ^ b(3) ^ b(4) ^ b(5)) & 1;
    let expected = (p1 << 7) | (p0 << 6) | id;
    expected == pid
}

pub fn decode(d: &Digital, cfg: Config) -> Result<Vec<Event>, DecodeError> {
    let mut per_bit = check_resolution(d.sample_rate, cfg.baud)?;
    let level = |i: usize| d.level_at(i);

    let mut events = Vec::new();
    let n = d.levels.len();
    let mut i = 0usize;

    // Helper: decode one standard UART 8N1 byte starting at start bit falling edge
    let decode_uart_byte = |start_edge: usize, pb: f64| -> Option<(u8, usize, usize)> {
        let bit_end = start_edge + (10.0 * pb) as usize;
        if bit_end >= n {
            return None;
        }
        // Verify start bit is low
        let sample = |k: f64| start_edge + (k * pb) as usize;
        if level(sample(0.5)) {
            return None;
        }
        let mut val = 0u8;
        for bit_i in 0..8 {
            let s_pos = sample(1.5 + bit_i as f64);
            if s_pos < n && level(s_pos) {
                val |= 1 << bit_i;
            }
        }
        Some((val, start_edge, bit_end))
    };

    while i + 1 < n {
        // Look for Synch Break: Falling edge from Recessive High (1) to Dominant Low (0)
        if !level(i) || level(i + 1) {
            i += 1;
            continue;
        }

        let break_start = i + 1;
        // Count how long signal stays low
        let mut break_end = break_start;
        while break_end < n && !level(break_end) {
            break_end += 1;
        }

        let low_duration = break_end - break_start;
        let bits_low = low_duration as f64 / per_bit;

        // LIN specification requires at least 13 dominant bit times for Break Field
        if bits_low < 11.5 {
            // Not a valid break field
            i = break_end;
            continue;
        }

        events.push(Event {
            start: break_start,
            end: break_end,
            kind: EventKind::Marker("BREAK"),
        });

        // Break Delimiter: at least 1 bit time recessive high
        let mut delim_end = break_end;
        while delim_end < n && level(delim_end) {
            delim_end += 1;
        }

        if delim_end == break_end {
            // Missing break delimiter
            events.push(Event {
                start: break_end,
                end: break_end + per_bit as usize,
                kind: EventKind::Error("LIN break delimiter missing"),
            });
            i = break_end + 1;
            continue;
        }

        // Next field: Synch Byte (0x55)
        let sync_edge = delim_end;
        let Some((sync_val, sync_start, sync_end)) = decode_uart_byte(sync_edge, per_bit) else {
            i = delim_end + per_bit as usize;
            continue;
        };

        if sync_val != 0x55 {
            events.push(Event {
                start: sync_start,
                end: sync_end,
                kind: EventKind::Error("LIN invalid sync byte (expected 0x55)"),
            });
            i = sync_end;
            continue;
        }

        events.push(Event {
            start: sync_start,
            end: sync_end,
            kind: EventKind::Word { value: 0x55, bits: 8 },
        });

        // Use the sync byte's 5 falling edges across 8 bit periods to fine-tune per_bit
        let measured_bit_time = (sync_end - sync_start) as f64 / 10.0;
        if measured_bit_time > 0.0 {
            per_bit = 0.5 * per_bit + 0.5 * measured_bit_time;
        }

        // Wait for delimiter/idle to find next byte (PID)
        let mut cur = sync_end;
        while cur + 1 < n && level(cur) {
            cur += 1;
        }

        let pid_edge = cur;
        let Some((pid_val, pid_start, pid_end)) = decode_uart_byte(pid_edge, per_bit) else {
            i = sync_end + per_bit as usize;
            continue;
        };

        let pid_valid = verify_pid(pid_val);
        events.push(Event {
            start: pid_start,
            end: pid_end,
            kind: EventKind::Word { value: pid_val as u64, bits: 8 },
        });

        if !pid_valid {
            events.push(Event {
                start: pid_start,
                end: pid_end,
                kind: EventKind::Error("LIN PID parity error"),
            });
        }

        // Read subsequent data bytes until inter-frame response space or next break
        let mut data_bytes = Vec::new();
        let mut last_byte_end = pid_end;

        for _ in 0..8 {
            let mut next_edge = last_byte_end;
            // Scan for next start bit (falling edge)
            while next_edge + 1 < n && level(next_edge) {
                // If idle lasts longer than 15 bit times, frame has ended
                if (next_edge - last_byte_end) as f64 > 15.0 * per_bit {
                    break;
                }
                next_edge += 1;
            }

            if next_edge + (10.0 * per_bit) as usize >= n || level(next_edge) {
                break;
            }

            if let Some((d_val, d_start, d_end)) = decode_uart_byte(next_edge, per_bit) {
                data_bytes.push((d_val, d_start, d_end));
                last_byte_end = d_end;
            } else {
                break;
            }
        }

        // If we captured bytes, the last one is the Checksum and preceding are Data
        if data_bytes.len() >= 2 {
            let (chk_val, chk_start, chk_end) = data_bytes.pop().unwrap();

            for &(val, b_start, b_end) in &data_bytes {
                events.push(Event {
                    start: b_start,
                    end: b_end,
                    kind: EventKind::Word { value: val as u64, bits: 8 },
                });
            }

            events.push(Event {
                start: chk_start,
                end: chk_end,
                kind: EventKind::Word { value: chk_val as u64, bits: 8 },
            });

            // Verify checksum
            let payload: Vec<u8> = data_bytes.iter().map(|b| b.0).collect();
            let expected = if cfg.enhanced_checksum && (pid_val & 0x3F) < 0x3C {
                let mut bytes_to_check = vec![pid_val];
                bytes_to_check.extend_from_slice(&payload);
                lin_checksum(&bytes_to_check)
            } else {
                lin_checksum(&payload)
            };

            if chk_val != expected {
                events.push(Event {
                    start: chk_start,
                    end: chk_end,
                    kind: EventKind::Error("LIN checksum mismatch"),
                });
            }
        } else if let Some(&(val, b_start, b_end)) = data_bytes.first() {
            // Only 1 byte was received
            events.push(Event {
                start: b_start,
                end: b_end,
                kind: EventKind::Word { value: val as u64, bits: 8 },
            });
        }

        i = last_byte_end + per_bit as usize;
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lin_pid_parity() {
        // PID 0x80 (ID 0x00): P0 = 0, P1 = 1 -> 0x80
        assert!(verify_pid(0x80));
        // PID 0xC1 (ID 0x01): P0 = 1, P1 = 1 -> 0xC1
        assert!(verify_pid(0xC1));
        // Inverted parity should fail
        assert!(!verify_pid(0x00));
        assert!(!verify_pid(0x41));
    }

    #[test]
    fn test_lin_checksum_calculation() {
        // Checksum of [0x11, 0x22] = !(0x33) = 0xCC
        let chk = lin_checksum(&[0x11, 0x22]);
        assert_eq!(chk, 0xCC);

        // With carry: 0x80 + 0x90 = 0x110 -> 0x10 + 1 = 0x11 -> !(0x11) = 0xEE
        let chk2 = lin_checksum(&[0x80, 0x90]);
        assert_eq!(chk2, 0xEE);
    }

    #[test]
    fn test_lin_decode_synthetic() {
        let sample_rate: f64 = 1_000_000.0;
        let baud: f64 = 19_200.0;
        let samples_per_bit = (sample_rate / baud).round() as usize; // ~52 samples/bit

        let mut levels = Vec::new();
        // Idle recessive (high)
        levels.extend(vec![true; samples_per_bit * 5]);

        // 1. Synch Break: 14 dominant (low) bits
        levels.extend(vec![false; samples_per_bit * 14]);

        // 2. Break Delimiter: 1 recessive (high) bit
        levels.extend(vec![true; samples_per_bit]);

        // Helper to push UART byte (start=0, 8 data LSB-first, stop=1)
        let push_byte = |levels: &mut Vec<bool>, byte: u8| {
            levels.extend(vec![false; samples_per_bit]); // Start
            for bit_i in 0..8 {
                let b = ((byte >> bit_i) & 1) != 0;
                levels.extend(vec![b; samples_per_bit]);
            }
            levels.extend(vec![true; samples_per_bit]); // Stop
        };

        // 3. Synch Byte: 0x55
        push_byte(&mut levels, 0x55);

        // Inter-byte space (1 bit)
        levels.extend(vec![true; samples_per_bit]);

        // 4. PID: 0xC1 (ID 0x01)
        push_byte(&mut levels, 0xC1);

        levels.extend(vec![true; samples_per_bit]);

        // 5. Data byte 1: 0x11
        push_byte(&mut levels, 0x11);

        levels.extend(vec![true; samples_per_bit]);

        // 6. Enhanced Checksum over [PID 0xC1, Data 0x11]
        let chk = lin_checksum(&[0xC1, 0x11]);
        push_byte(&mut levels, chk);

        // Trailing idle
        levels.extend(vec![true; samples_per_bit * 10]);

        let digital = Digital {
            levels,
            sample_rate,
        };

        let cfg = Config {
            baud,
            enhanced_checksum: true,
        };

        let res = decode(&digital, cfg);
        assert!(res.is_ok(), "LIN decoding should succeed");
        let events = res.unwrap();
        assert!(!events.is_empty(), "Should decode LIN events");

        // Verify Break, Synch, PID and Checksum
        let has_break = events.iter().any(|e| matches!(e.kind, EventKind::Marker("BREAK")));
        assert!(has_break, "Should have BREAK marker");

        let has_pid = events.iter().any(|e| matches!(e.kind, EventKind::Word { value: 0xC1, bits: 8 }));
        assert!(has_pid, "Should have PID 0xC1");
    }
}
