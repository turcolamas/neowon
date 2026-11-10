//! Monitor de Tráfico de Red y Tramas de Bus (CAN 2.0A/B & LIN 1.3/2.x).
//!
//! A diferencia del decodificador sobre la waveform física que analiza tiempos de bit y forma de onda,
//! el Monitor de Tráfico agrega y analiza los paquetes a nivel de capa de enlace/protocolo:
//! - Agrupación continua por ID / PID con conteo de tramas recibidas y tasa de repetición (Hz).
//! - Delta de tiempo (Δt) entre mensajes sucesivos del mismo ID.
//! - Resaltado dinámico en tiempo real de los bytes de datos (payload) que cambian de valor.
//! - Exportación y filtrado rápido por ID / Máscara.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusProtocol {
    Can,
    Lin,
}

#[derive(Debug, Clone)]
pub struct BusTrafficRow {
    pub id: u32,
    pub is_extended: bool,
    pub count: u64,
    pub first_time: f64,
    pub last_time: f64,
    pub delta_ms: f64,
    pub freq_hz: f64,
    pub dlc: usize,
    pub data: Vec<u8>,
    pub prev_data: Vec<u8>,
    pub changed_mask: u8, // bit k = 1 si el byte k cambió respecto a la última recepción
    pub last_changed_time: f64,
}

#[derive(Debug, Clone)]
pub struct TrafficMonitor {
    pub active: bool,
    pub bus: BusProtocol,
    pub rows: HashMap<u32, BusTrafficRow>,
    pub filter_id: Option<u32>,
    pub pause_updates: bool,
    pub total_frames_captured: u64,
}

impl Default for TrafficMonitor {
    fn default() -> Self {
        Self {
            active: false,
            bus: BusProtocol::Can,
            rows: HashMap::new(),
            filter_id: None,
            pause_updates: false,
            total_frames_captured: 0,
        }
    }
}

impl TrafficMonitor {
    pub fn clear(&mut self) {
        self.rows.clear();
        self.total_frames_captured = 0;
    }

    /// Ingesta un paquete CAN decodificado
    pub fn ingest_can(
        &mut self,
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

        let row = self.rows.entry(id).or_insert_with(|| BusTrafficRow {
            id,
            is_extended,
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

        row.count += 1;
        if row.count > 1 {
            let dt = timestamp - row.last_time;
            if dt > 0.0 {
                row.delta_ms = dt * 1000.0;
                let inst_freq = 1.0 / dt;
                // Filtro IIR simple para frecuencia estable
                row.freq_hz = row.freq_hz * 0.7 + inst_freq * 0.3;
            }
        }
        row.last_time = timestamp;
        row.dlc = dlc;

        // Comprobar bytes cambiados
        let mut mask = 0u8;
        for (i, (&new_b, &old_b)) in data.iter().zip(row.data.iter()).enumerate() {
            if i < 8 && new_b != old_b {
                mask |= 1 << i;
            }
        }

        if mask != 0 {
            row.prev_data = row.data.clone();
            row.changed_mask = mask;
            row.last_changed_time = timestamp;
        } else if timestamp - row.last_changed_time > 1.5 {
            // Desvanecer el resaltado tras 1.5 segundos sin cambios
            row.changed_mask = 0;
        }

        row.data = data.to_vec();
    }

    /// Ingesta un paquete LIN decodificado
    pub fn ingest_lin(&mut self, timestamp: f64, pid: u8, dlc: usize, data: &[u8]) {
        self.ingest_can(timestamp, pid as u32, false, dlc, data);
    }

    /// Devuelve las filas ordenadas por ID para la tabla de visualización
    pub fn get_sorted_rows(&self) -> Vec<&BusTrafficRow> {
        let mut list: Vec<&BusTrafficRow> = self
            .rows
            .values()
            .filter(|r| {
                if let Some(fid) = self.filter_id {
                    r.id == fid
                } else {
                    true
                }
            })
            .collect();
        list.sort_by_key(|r| r.id);
        list
    }
}
