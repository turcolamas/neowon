//! Persistencia y Secuencia de Arranque Seguro para el Hardware del Osciloscopio
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedChannelConfig {
    pub enabled: bool,
    pub volts_div: f64,
    pub probe: f64,
    pub coupling_dc: bool,
    pub offset: f64,
}

impl SavedChannelConfig {
    pub fn new(enabled: bool, volts_div: f64, probe: f64, coupling_dc: bool, offset: f64) -> Self {
        Self {
            enabled,
            volts_div,
            probe,
            coupling_dc,
            offset,
        }
    }
}

impl Default for SavedChannelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volts_div: 2.0,
            probe: 10.0,
            coupling_dc: true,
            offset: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopePersistenceStore {
    pub ch1: SavedChannelConfig,
    pub ch2: SavedChannelConfig,
}

impl Default for ScopePersistenceStore {
    fn default() -> Self {
        Self {
            ch1: SavedChannelConfig {
                enabled: true,
                volts_div: 2.0,
                probe: 10.0,
                coupling_dc: true,
                offset: 0.0,
            },
            ch2: SavedChannelConfig {
                enabled: false,
                volts_div: 2.0,
                probe: 10.0,
                coupling_dc: true,
                offset: 0.0,
            },
        }
    }
}

impl ScopePersistenceStore {
    pub const STORE_PATH: &'static str = "scope_channels_config.json";

    pub fn load_or_default() -> Self {
        if let Ok(data) = std::fs::read_to_string(Self::STORE_PATH)
            && let Ok(store) = serde_json::from_str::<Self>(&data) {
                return store;
            }
        let def = Self::default();
        let _ = def.save();
        def
    }

    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::STORE_PATH, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Gestor de la secuencia de arranque seguro por hardware
#[derive(Debug, Clone)]
pub struct StartupSafetyManager {
    pub active: bool,
    pub boot_time: std::time::Instant,
    pub store: ScopePersistenceStore,
    pub stage: usize,
}

impl StartupSafetyManager {
    pub fn new() -> Self {
        let store = ScopePersistenceStore::load_or_default();
        Self {
            active: true,
            boot_time: std::time::Instant::now(),
            store,
            stage: 0,
        }
    }
}

impl Default for StartupSafetyManager {
    fn default() -> Self {
        Self::new()
    }
}
