//! Decodificador SAE J2716 SENT (Single Edge Nibble Transmission).
//!
//! Protocolo unidireccional por ancho de pulso entre flancos descendentes:
//! - Pulso de Sincronización / Calibración: 56 ticks nominales (±25% tolerancia).
//! - Nibble de Estado y Comunicación: 12..=27 ticks -> valor 0..=15.
//! - Nibles de Datos (típicamente 6 nibles = Canal Rápido 1 12-bit + Canal 2 12-bit).
//! - Nibble de CRC4 (polinomio SAE J2716 x⁴ + x³ + x² + 1, semilla 5).
//! - Pulso de Pausa opcional (> 27 ticks) antes del siguiente Sync.

use super::{DecodeError, Digital, Event, EventKind, check_resolution};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Duración nominal del tick en microsegundos (por defecto 3.0 µs).
    pub tick_us: f64,
    /// Cantidad de nibles de datos entre Estado y CRC (por defecto 6 = CH1 12-bit + CH2 12-bit).
    pub data_nibbles: usize,
    /// Verificar CRC de 4 bits SAE J2716.
    pub check_crc: bool,
    /// CRC con nible adicional cero (SAE J2716 2010+) vs legado (2008).
    pub crc_with_trailing_zero: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tick_us: 3.0,
            data_nibbles: 6,
            check_crc: true,
            crc_with_trailing_zero: true,
        }
    }
}

/// Tabla SAE J2716 CRC4 (polinomio 0x1D: x⁴ + x³ + x² + 1).
const SENT_CRC_TABLE: [u8; 16] = [0, 13, 7, 10, 14, 3, 9, 4, 1, 12, 6, 11, 15, 2, 8, 5];

pub fn sent_crc4(nibbles: &[u8], with_trailing_zero: bool) -> u8 {
    let mut crc = 5u8;
    for &n in nibbles {
        crc = SENT_CRC_TABLE[(crc ^ (n & 0x0F)) as usize];
    }
    if with_trailing_zero {
        crc = SENT_CRC_TABLE[crc as usize];
    }
    crc
}

pub fn decode(d: &Digital, cfg: Config) -> Result<Vec<Event>, DecodeError> {
    if cfg.tick_us <= 0.0 || cfg.data_nibbles == 0 || cfg.data_nibbles > 8 {
        return Err(DecodeError::BadParameter("SENT tick_us / data_nibbles"));
    }
    let tick_s = cfg.tick_us * 1e-6;
    // En SENT el pulso mínimo dura 12 ticks (y parte baja >= 4 ticks); garantizamos resolución razonable por tick.
    if d.sample_rate <= 0.0 {
        return Err(DecodeError::BadParameter("sample_rate"));
    }
    // Exigimos al menos MIN_SAMPLES_BIT por pulso mínimo de 12 ticks (o 1 muestra/tick mínimo).
    let _ = check_resolution(d.sample_rate, 1.0 / (12.0 * tick_s))?;
    let nom_tick_samples = d.sample_rate * tick_s;

    // Extraer todos los flancos descendentes (High -> Low)
    let mut falling = Vec::new();
    let n = d.levels.len();
    for i in 1..n {
        if d.level_at(i - 1) && !d.level_at(i) {
            falling.push(i);
        }
    }
    if falling.len() < 3 {
        return Err(DecodeError::NoActivity);
    }

    let mut events = Vec::new();
    let expected_sync = 56.0 * nom_tick_samples;
    let total_nibbles = 1 + cfg.data_nibbles + 1; // Status + Data + CRC

    let mut idx = 0usize;
    while idx + 1 < falling.len() {
        let sync_start = falling[idx];
        let sync_end = falling[idx + 1];
        let sync_len = (sync_end - sync_start) as f64;

        // Validar ventana de Sync Pulse (56 ticks ± 25%)
        if sync_len < expected_sync * 0.75 || sync_len > expected_sync * 1.25 {
            idx += 1;
            continue;
        }

        let tick_samples = sync_len / 56.0;
        events.push(Event {
            start: sync_start,
            end: sync_end,
            kind: EventKind::Marker("SYNC"),
        });

        let mut cursor = idx + 1;
        let mut decoded_nibbles: Vec<(u8, usize, usize)> = Vec::with_capacity(total_nibbles);
        let mut frame_broken = false;

        for _ in 0..total_nibbles {
            if cursor + 1 >= falling.len() {
                frame_broken = true;
                break;
            }
            let p0 = falling[cursor];
            let p1 = falling[cursor + 1];
            let ticks = (p1 - p0) as f64 / tick_samples;
            let ticks_rounded = ticks.round() as i32;

            // Si aparece otro pulso SYNC (56 ticks) prematuro, cortar trama e iniciar en él
            if (50.0..=62.0).contains(&ticks) {
                events.push(Event {
                    start: p0,
                    end: p1,
                    kind: EventKind::Error("SENT trama incompleta"),
                });
                frame_broken = true;
                break;
            }

            if !(12..=27).contains(&ticks_rounded) {
                events.push(Event {
                    start: p0,
                    end: p1,
                    kind: EventKind::Error("SENT nible fuera de rango (12..27 ticks)"),
                });
                frame_broken = true;
                cursor += 1;
                break;
            }

            let val = (ticks_rounded - 12) as u8;
            decoded_nibbles.push((val, p0, p1));
            cursor += 1;
        }

        if decoded_nibbles.len() == total_nibbles {
            // 1. Nibble de estado (4 bits)
            let (status, s0, s1) = decoded_nibbles[0];
            events.push(Event {
                start: s0,
                end: s1,
                kind: EventKind::Word {
                    value: status as u64,
                    bits: 4,
                },
            });

            // 2. Nibles de datos (4 bits cada uno)
            let mut data_only = Vec::with_capacity(cfg.data_nibbles);
            for &(val, a, b) in &decoded_nibbles[1..=cfg.data_nibbles] {
                data_only.push(val);
                events.push(Event {
                    start: a,
                    end: b,
                    kind: EventKind::Word {
                        value: val as u64,
                        bits: 4,
                    },
                });
            }

            // 3. Nibble CRC4 (4 bits)
            let (crc_val, c0, c1) = decoded_nibbles[total_nibbles - 1];
            events.push(Event {
                start: c0,
                end: c1,
                kind: EventKind::Word {
                    value: crc_val as u64,
                    bits: 4,
                },
            });

            if cfg.check_crc {
                let expected_crc = sent_crc4(&data_only, cfg.crc_with_trailing_zero);
                if crc_val != expected_crc {
                    events.push(Event {
                        start: c0,
                        end: c1,
                        kind: EventKind::Error("SENT CRC mismatch"),
                    });
                }
            }

            events.push(Event {
                start: decoded_nibbles.last().unwrap().2,
                end: decoded_nibbles.last().unwrap().2,
                kind: EventKind::Marker("EOF"),
            });
        }

        idx = if frame_broken { cursor.max(idx + 1) } else { cursor };
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_pulse(levels: &mut Vec<bool>, ticks: usize, samples_per_tick: usize) {
        let low = 4 * samples_per_tick;
        let high = (ticks.saturating_sub(4)) * samples_per_tick;
        levels.extend(std::iter::repeat_n(false, low));
        levels.extend(std::iter::repeat_n(true, high));
    }

    #[test]
    fn test_sent_decode_synthetic() {
        let tick_us = 3.0;
        let samples_per_tick = 6; // 2 MS/s => 1 tick = 6 muestras, 12 ticks = 72 muestras >= 12
        let sample_rate = samples_per_tick as f64 / (tick_us * 1e-6);

        let status = 0x3u8;
        let data = [0xAu8, 0x5, 0xC, 0x1, 0xF, 0x2];
        let crc = sent_crc4(&data, true);

        let mut levels = vec![true; 30];
        push_pulse(&mut levels, 56, samples_per_tick); // SYNC
        push_pulse(&mut levels, 12 + status as usize, samples_per_tick);
        for &n in &data {
            push_pulse(&mut levels, 12 + n as usize, samples_per_tick);
        }
        push_pulse(&mut levels, 12 + crc as usize, samples_per_tick);
        levels.extend(vec![false; 24]); // flanco descendente final del CRC
        levels.extend(vec![true; 30]);

        let dig = Digital {
            levels,
            sample_rate,
        };
        let events = decode(&dig, Config::default()).expect("SENT decode ok");
        assert!(events.iter().any(|e| e.kind == EventKind::Marker("SYNC")));
        assert!(!events.iter().any(|e| e.kind.is_error()), "Unexpected errors: {events:?}");

        let words: Vec<u64> = events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::Word { value, bits: 4 } => Some(value),
                _ => None,
            })
            .collect();
        assert_eq!(
            words,
            vec![
                status as u64,
                data[0] as u64,
                data[1] as u64,
                data[2] as u64,
                data[3] as u64,
                data[4] as u64,
                data[5] as u64,
                crc as u64,
            ]
        );
    }
}
