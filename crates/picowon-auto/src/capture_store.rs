//! Gestor Unificado de Almacenamiento, Capturas y Base de Datos de Modelos/Firmas.
//!
//! Proporciona:
//! 1. Organización en carpetas dedicadas por módulo:
//!    - `./Capturas/Osciloscopio/`
//!    - `./Capturas/Datalogger/`
//!    - `./Capturas/Tracer_VI/`
//! 2. Metadatos completos de captura (Ajustes de HW, V/div, sonda, acoplamiento, timebase, trigger, DSP, fecha/hora y notas de operario).
//! 3. Selección de rango de exportación de osciloscopio:
//!    - Frame actual en pantalla
//!    - Todos los frames del historial
//!    - Últimos N frames
//! 4. Base de datos / Biblioteca de Modelos y Firmas del Tracer VI con soporte para:
//!    - Guardar firma actual con sus ajustes de HW.
//!    - Cargar modelo desde base de datos como referencia ("Golden Sample").
//!    - Comparar automáticamente con la medición en curso usando Tolerancia Cuantitativa.
//!    - Restaurar exactamente los ajustes de HW que estuvieron presentes en la captura original.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::scope_persistence::SavedChannelConfig;
use crate::dsp_filter::ChannelDspConfig;

/// Directorios dedicados por defecto
pub const SCOPE_CAPTURES_DIR: &str = "Capturas/Osciloscopio";
pub const DATALOGGER_CAPTURES_DIR: &str = "Capturas/Datalogger";
pub const TRACER_CAPTURES_DIR: &str = "Capturas/Tracer_VI";
pub const TRACER_DB_FILE: &str = "Capturas/Tracer_VI/vi_models_database.json";

/// Asegura la existencia de las carpetas dedicadas
pub fn ensure_all_capture_dirs() {
    let _ = fs::create_dir_all(SCOPE_CAPTURES_DIR);
    let _ = fs::create_dir_all(DATALOGGER_CAPTURES_DIR);
    let _ = fs::create_dir_all(TRACER_CAPTURES_DIR);
}

/// Genera un timestamp formateado `YYYYMMDD_HHMMSS`
pub fn current_timestamp_str() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    // Cálculo aproximado UTC / hora civil sin dependencias externas pesadas
    let total_secs = now;
    let s = total_secs % 60;
    let m = (total_secs / 60) % 60;
    let h = (total_secs / 3600) % 24;
    let days = total_secs / 86400;

    // Conversión básica de días a año/mes/día desde 1970-01-01
    let mut year = 1970;
    let mut rem_days = days as i64;
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if rem_days >= days_in_year {
            rem_days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = [
        31, if is_leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1;
    for &d in &month_days {
        if rem_days >= d {
            rem_days -= d;
            month += 1;
        } else {
            break;
        }
    }
    let day = rem_days + 1;
    format!("{year:04}{month:02}{day:02}_{h:02}{m:02}{s:02}")
}

/// Rango de tramas de osciloscopio a exportar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeExportRange {
    CurrentFrame,
    AllFrames,
    LastN(usize),
}

impl ScopeExportRange {
    pub fn label(self) -> String {
        match self {
            ScopeExportRange::CurrentFrame => "Trama actual en pantalla".to_string(),
            ScopeExportRange::AllFrames => "Todas las tramas en memoria".to_string(),
            ScopeExportRange::LastN(n) => format!("Últimas {n} tramas"),
        }
    }
}

/// Metadatos completos de una captura de osciloscopio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeCaptureMetadata {
    pub title: String,
    pub timestamp: String,
    pub operator_name: String,
    pub operator_notes: String,
    pub export_range: String,
    pub channels: [SavedChannelConfig; 2],
    pub timebase_sec_div: f64,
    pub timebase_str: String,
    pub trigger_source: usize,
    pub trigger_sweep: String,
    pub trigger_slope: String,
    pub trigger_level_v: f64,
    pub dsp_configs: [Option<ChannelDspConfig>; 2],
    pub sample_rate_hz: f64,
    pub total_samples: usize,
    pub frame_count: usize,
}

/// Gestor de Estado para Diálogo de Captura del Osciloscopio
#[derive(Debug, Clone)]
pub struct ScopeCaptureDialogState {
    pub dialog_open: bool,
    pub folder: String,
    pub filename_prefix: String,
    pub last_prefix: String,
    pub operator_name: String,
    pub operator_notes: String,
    pub export_range: ScopeExportRange,
    pub custom_n_frames: usize,
    pub export_png: bool,
    pub export_csv: bool,
    pub export_nwc: bool,
    pub last_saved_info: Option<(String, f64)>, // (ruta, tiempo)
}

impl Default for ScopeCaptureDialogState {
    fn default() -> Self {
        Self {
            dialog_open: false,
            folder: SCOPE_CAPTURES_DIR.to_string(),
            filename_prefix: "Captura_Scope".to_string(),
            last_prefix: "Captura_Scope".to_string(),
            operator_name: "Técnico".to_string(),
            operator_notes: String::new(),
            export_range: ScopeExportRange::CurrentFrame,
            custom_n_frames: 10,
            export_png: true,
            export_csv: true,
            export_nwc: true,
            last_saved_info: None,
        }
    }
}

impl ScopeCaptureDialogState {
    pub fn generate_base_path(&mut self) -> (PathBuf, String) {
        let ts = current_timestamp_str();
        let clean_prefix: String = self.filename_prefix
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let stem = format!("{clean_prefix}_{ts}");
        let dir = Path::new(&self.folder);
        let _ = fs::create_dir_all(dir);
        self.last_prefix = self.filename_prefix.clone();
        (dir.join(&stem), stem)
    }
}

/// Gestor de Estado para Diálogo de Grabación del Datalogger
#[derive(Debug, Clone)]
pub struct DataloggerSessionDialogState {
    pub is_starting_dialog_open: bool,
    pub is_stopping_dialog_open: bool,
    pub folder: String,
    pub filename_prefix: String,
    pub last_prefix: String,
    pub operator_name: String,
    pub operator_notes: String,
    pub session_start_time: f64,
    pub recorded_duration_secs: f64,
    pub recorded_samples: usize,
    pub active_file_base: Option<PathBuf>,
    pub last_saved_info: Option<(String, f64)>,
}

impl Default for DataloggerSessionDialogState {
    fn default() -> Self {
        Self {
            is_starting_dialog_open: false,
            is_stopping_dialog_open: false,
            folder: DATALOGGER_CAPTURES_DIR.to_string(),
            filename_prefix: "Log_Datalogger".to_string(),
            last_prefix: "Log_Datalogger".to_string(),
            operator_name: "Técnico".to_string(),
            operator_notes: String::new(),
            session_start_time: 0.0,
            recorded_duration_secs: 0.0,
            recorded_samples: 0,
            active_file_base: None,
            last_saved_info: None,
        }
    }
}

impl DataloggerSessionDialogState {
    pub fn prepare_start(&mut self) -> (PathBuf, String) {
        let ts = current_timestamp_str();
        let clean_prefix: String = self.filename_prefix
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let stem = format!("{clean_prefix}_{ts}");
        let dir = Path::new(&self.folder);
        let _ = fs::create_dir_all(dir);
        let base_path = dir.join(&stem);
        self.active_file_base = Some(base_path.clone());
        self.last_prefix = self.filename_prefix.clone();
        (base_path, stem)
    }
}

/// Entrada / Ficha de un Modelo de Curva V-I en la Base de Datos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViModelEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub operator_name: String,
    pub created_at: String,
    pub points: Vec<(f32, f32)>,       // (V en Voltios, I en mA)
    pub tolerance_percent: f32,        // Tolerancia sugerida (ej. 5.0%)
    
    // Parámetros y Ajustes de Hardware presentes en el momento de la captura:
    pub r_sense_ohms: f64,
    pub invert_vertical: bool,
    pub ch1_vdiv: f64,
    pub ch2_vdiv: f64,
    pub ch1_probe: f64,
    pub ch2_probe: f64,
    pub timebase: f64,
    pub burst_count: usize,
    pub grid_divs_h: usize,
    pub grid_divs_v: usize,
    pub equivalent_summary: Option<String>,
}

/// Base de Datos Persistente de Modelos y Firmas del Tracer VI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ViModelDatabase {
    pub models: Vec<ViModelEntry>,
}


impl ViModelDatabase {
    pub fn load_or_default() -> Self {
        ensure_all_capture_dirs();
        let path = Path::new(TRACER_DB_FILE);
        if let Ok(mut file) = File::open(path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok()
                && let Ok(db) = serde_json::from_str::<ViModelDatabase>(&content) {
                    return db;
                }
        }
        // Fallback o inicialización con firmas de referencia automotrices de fábrica
        let mut db = ViModelDatabase::default();
        db.seed_factory_models_if_empty();
        let _ = db.save();
        db
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        ensure_all_capture_dirs();
        let path = Path::new(TRACER_DB_FILE);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(e))?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn add_or_update(&mut self, entry: ViModelEntry) {
        if let Some(pos) = self.models.iter().position(|m| m.id == entry.id) {
            self.models[pos] = entry;
        } else {
            self.models.push(entry);
        }
        let _ = self.save();
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let initial_len = self.models.len();
        self.models.retain(|m| m.id != id);
        if self.models.len() != initial_len {
            let _ = self.save();
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: &str) -> Option<&ViModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    fn seed_factory_models_if_empty(&mut self) {
        if !self.models.is_empty() {
            return;
        }

        // Modelo de fábrica: Diodo Rectificador 1N4007
        let mut diode_pts = Vec::new();
        for i in 0..128 {
            let phase = (i as f32 / 128.0) * std::f32::consts::TAU;
            let v_test = 5.0 * phase.sin();
            let current = if v_test > 0.65 {
                (v_test - 0.65) / 0.1 // mA aprox
            } else {
                0.0
            };
            diode_pts.push((v_test, current));
        }

        self.models.push(ViModelEntry {
            id: "diode_1n4007_ref".to_string(),
            name: "Diodo Rectificador 1N4007 (Sano)".to_string(),
            category: "Semiconductores".to_string(),
            description: "Codo directo en 0.65V, corriente nula en inversa. Curva de referencia estándar.".to_string(),
            operator_name: "Fábrica PicoWon".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
            points: diode_pts,
            tolerance_percent: 5.0,
            r_sense_ohms: 1000.0,
            invert_vertical: true,
            ch1_vdiv: 1.0,
            ch2_vdiv: 1.0,
            ch1_probe: 10.0,
            ch2_probe: 10.0,
            timebase: 0.001,
            burst_count: 16,
            grid_divs_h: 8,
            grid_divs_v: 8,
            equivalent_summary: Some("Diodo P-N (Codo 0.65V)".to_string()),
        });

        // Modelo de fábrica: Driver Inyector Gasolina Multipunto (ECU)
        let mut inj_pts = Vec::new();
        for i in 0..128 {
            let phase = (i as f32 / 128.0) * std::f32::consts::TAU;
            let v_test = 6.0 * phase.sin();
            let current = if v_test > 0.6 {
                (v_test - 0.6) / 0.15
            } else if v_test < -20.0 {
                (v_test + 20.0) / 0.05
            } else {
                v_test * 0.05
            };
            inj_pts.push((v_test, current));
        }

        self.models.push(ViModelEntry {
            id: "ecu_inj_driver_ref".to_string(),
            name: "Salida Driver Inyector Bosch EDC17 (Sano)".to_string(),
            category: "Inyectores".to_string(),
            description: "MOSFET Low-Side con Zener/Clamp de absorción inductiva integrado.".to_string(),
            operator_name: "Fábrica PicoWon".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
            points: inj_pts,
            tolerance_percent: 6.0,
            r_sense_ohms: 1000.0,
            invert_vertical: true,
            ch1_vdiv: 2.0,
            ch2_vdiv: 1.0,
            ch1_probe: 10.0,
            ch2_probe: 10.0,
            timebase: 0.001,
            burst_count: 16,
            grid_divs_h: 8,
            grid_divs_v: 8,
            equivalent_summary: Some("MOSFET N-Ch + Zener Clamp".to_string()),
        });
    }
}

/// Gestor de Estado para Diálogos del Tracer VI
#[derive(Debug, Clone)]
pub struct ViCaptureDialogState {
    pub is_save_dialog_open: bool,
    pub is_load_db_dialog_open: bool,
    pub folder: String,
    pub filename_prefix: String,
    pub last_prefix: String,
    pub category: String,
    pub operator_name: String,
    pub operator_notes: String,
    pub tolerance_percent: f32,
    pub restore_hardware_on_load: bool,
    pub selected_db_id: Option<String>,
    pub db: ViModelDatabase,
    pub last_saved_info: Option<(String, f64)>,
}

impl Default for ViCaptureDialogState {
    fn default() -> Self {
        Self {
            is_save_dialog_open: false,
            is_load_db_dialog_open: false,
            folder: TRACER_CAPTURES_DIR.to_string(),
            filename_prefix: "Firma_Pin1".to_string(),
            last_prefix: "Firma_Pin1".to_string(),
            category: "Inyectores".to_string(),
            operator_name: "Técnico".to_string(),
            operator_notes: String::new(),
            tolerance_percent: 5.0,
            restore_hardware_on_load: true,
            selected_db_id: None,
            db: ViModelDatabase::load_or_default(),
            last_saved_info: None,
        }
    }
}

impl ViCaptureDialogState {
    pub fn prepare_save_path(&mut self) -> (PathBuf, String) {
        let ts = current_timestamp_str();
        let clean_prefix: String = self.filename_prefix
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let stem = format!("{clean_prefix}_{ts}");
        let dir = Path::new(&self.folder);
        let _ = fs::create_dir_all(dir);
        self.last_prefix = self.filename_prefix.clone();
        (dir.join(&stem), stem)
    }

    /// Guarda la firma actual tanto en la base de datos como en archivos en disco
    pub fn save_current_signature(
        &mut self,
        name: &str,
        category: &str,
        description: &str,
        operator: &str,
        tolerance_pct: f32,
        points: Vec<(f32, f32)>,
        r_sense: f64,
        invert_v: bool,
        ch1_vdiv: f64,
        ch2_vdiv: f64,
        ch1_probe: f64,
        ch2_probe: f64,
        timebase: f64,
        burst: usize,
        grid_h: usize,
        grid_v: usize,
        equiv: Option<String>,
    ) -> Result<PathBuf, String> {
        let (base_path, stem) = self.prepare_save_path();
        let id = format!("{stem}_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis());

        let entry = ViModelEntry {
            id,
            name: name.to_string(),
            category: category.to_string(),
            description: description.to_string(),
            operator_name: operator.to_string(),
            created_at: current_timestamp_str(),
            points: points.clone(),
            tolerance_percent: tolerance_pct,
            r_sense_ohms: r_sense,
            invert_vertical: invert_v,
            ch1_vdiv,
            ch2_vdiv,
            ch1_probe,
            ch2_probe,
            timebase,
            burst_count: burst,
            grid_divs_h: grid_h,
            grid_divs_v: grid_v,
            equivalent_summary: equiv,
        };

        // 1. Guardar en Base de Datos centralizada
        self.db.add_or_update(entry.clone());

        // 2. Exportar ficha individual JSON en la carpeta dedicada
        let json_path = base_path.with_extension("json");
        if let Ok(json_str) = serde_json::to_string_pretty(&entry) {
            let _ = fs::write(&json_path, json_str);
        }

        // 3. Exportar datos CSV de la curva (V, I)
        let csv_path = base_path.with_extension("csv");
        let mut csv_content = String::from("Voltaje_V,Corriente_mA\n");
        for (v, i) in &points {
            csv_content.push_str(&format!("{v:.4},{i:.4}\n"));
        }
        let _ = fs::write(&csv_path, csv_content);

        Ok(base_path)
    }
}

impl ScopeCaptureMetadata {
    /// Carga la ficha de metadatos desde un archivo JSON
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
        serde_json::from_str::<ScopeCaptureMetadata>(&content).map_err(|e| e.to_string())
    }
}

/// Elemento representativo de una captura de osciloscopio almacenada en disco
#[derive(Debug, Clone)]
pub struct SavedScopeCaptureEntry {
    pub stem: String,
    pub folder: PathBuf,
    pub json_path: Option<PathBuf>,
    pub nwc_path: Option<PathBuf>,
    pub csv_path: Option<PathBuf>,
    pub png_path: Option<PathBuf>,
    pub metadata: Option<ScopeCaptureMetadata>,
    pub display_title: String,
    pub timestamp_str: String,
    pub operator_name: String,
    pub operator_notes: String,
    pub technical_summary: String,
}

/// Escanea la carpeta de osciloscopio y recopila todas las capturas existentes
pub fn scan_scope_captures(folder_path: &str) -> Vec<SavedScopeCaptureEntry> {
    let dir = Path::new(folder_path);
    if !dir.exists() {
        let _ = fs::create_dir_all(dir);
        return Vec::new();
    }

    let entries = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return Vec::new(),
    };

    let mut map: std::collections::BTreeMap<String, (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>, Option<PathBuf>)> =
        std::collections::BTreeMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let entry_slot = map.entry(stem.to_string()).or_insert((None, None, None, None));
            match ext.as_str() {
                "json" => entry_slot.0 = Some(path),
                "nwc" => entry_slot.1 = Some(path),
                "csv" => entry_slot.2 = Some(path),
                "png" => entry_slot.3 = Some(path),
                _ => {}
            }
        }
    }

    let mut result = Vec::new();
    for (stem, (json_opt, nwc_opt, csv_opt, png_opt)) in map {
        let mut meta = None;
        let mut title = stem.clone();
        let mut timestamp = String::new();
        let mut op_name = "Técnico".to_string();
        let mut op_notes = String::new();
        let mut tech_sum = String::new();

        if let Some(ref jpath) = json_opt
            && let Ok(m) = ScopeCaptureMetadata::load_from_file(jpath) {
                title = if !m.title.is_empty() { m.title.clone() } else { stem.clone() };
                timestamp = m.timestamp.clone();
                op_name = m.operator_name.clone();
                op_notes = m.operator_notes.clone();

                let ch0_v = m.channels[0].volts_div * m.channels[0].probe;
                let ch1_v = m.channels[1].volts_div * m.channels[1].probe;
                let ch0_str = if m.channels[0].enabled { format!("{ch0_v:.1}V/div") } else { "Off".to_string() };
                let ch1_str = if m.channels[1].enabled { format!("{ch1_v:.1}V/div") } else { "Off".to_string() };
                tech_sum = format!(
                    "CH1: {ch0_str} · CH2: {ch1_str} · Base: {} · Disparo: C{} ({:.2}V) · {} tramas",
                    m.timebase_str,
                    m.trigger_source + 1,
                    m.trigger_level_v,
                    m.frame_count
                );
                meta = Some(m);
            }

        if tech_sum.is_empty() {
            tech_sum = format!("Archivos: {}", [
                json_opt.is_some().then_some("JSON"),
                nwc_opt.is_some().then_some("NWC"),
                csv_opt.is_some().then_some("CSV"),
                png_opt.is_some().then_some("PNG"),
            ].into_iter().flatten().collect::<Vec<_>>().join(", "));
        }

        result.push(SavedScopeCaptureEntry {
            stem,
            folder: dir.to_path_buf(),
            json_path: json_opt,
            nwc_path: nwc_opt,
            csv_path: csv_opt,
            png_path: png_opt,
            metadata: meta,
            display_title: title,
            timestamp_str: timestamp,
            operator_name: op_name,
            operator_notes: op_notes,
            technical_summary: tech_sum,
        });
    }

    // Ordenar de más reciente a más antiguo por stem/timestamp
    result.sort_by(|a, b| b.stem.cmp(&a.stem));
    result
}

/// Elemento representativo de una sesión de datalogger almacenada en disco
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedDataloggerSessionEntry {
    pub stem: String,
    pub folder: PathBuf,
    pub csv_path: PathBuf,
    pub json_path: Option<PathBuf>,
    pub title: String,
    pub operator_name: String,
    pub operator_notes: String,
    pub duration_secs: f64,
    pub sample_count: usize,
    pub timestamp_str: String,
}

/// Escanea la carpeta del datalogger y recopila todas las sesiones guardadas
pub fn scan_datalogger_sessions(folder_path: &str) -> Vec<SavedDataloggerSessionEntry> {
    let dir = Path::new(folder_path);
    if !dir.exists() {
        let _ = fs::create_dir_all(dir);
        return Vec::new();
    }

    let entries = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return Vec::new(),
    };

    let mut csv_stems: std::collections::BTreeMap<String, (PathBuf, Option<PathBuf>)> =
        std::collections::BTreeMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if ext == "csv" {
                let slot = csv_stems.entry(stem.to_string()).or_insert((path.clone(), None));
                slot.0 = path;
            } else if ext == "json" {
                let slot = csv_stems.entry(stem.to_string()).or_insert((PathBuf::new(), None));
                slot.1 = Some(path);
            }
        }
    }

    let mut result = Vec::new();
    for (stem, (csv_path, json_path)) in csv_stems {
        if !csv_path.exists() {
            continue;
        }

        let mut title = stem.clone();
        let mut op_name = "Técnico".to_string();
        let mut op_notes = String::new();
        let mut duration = 0.0;
        let mut samples = 0;
        let mut ts = String::new();

        if let Some(ref jpath) = json_path
            && let Ok(mut f) = File::open(jpath) {
                let mut content = String::new();
                if f.read_to_string(&mut content).is_ok()
                    && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(t) = val.get("titulo").and_then(|v| v.as_str()) {
                            title = t.to_string();
                        }
                        if let Some(op) = val.get("operario").and_then(|v| v.as_str()) {
                            op_name = op.to_string();
                        }
                        if let Some(n) = val.get("notas").and_then(|v| v.as_str()) {
                            op_notes = n.to_string();
                        }
                        if let Some(d) = val.get("duracion_segundos").and_then(|v| v.as_f64()) {
                            duration = d;
                        }
                        if let Some(s) = val.get("muestras_registradas").and_then(|v| v.as_u64()) {
                            samples = s as usize;
                        }
                        if let Some(time) = val.get("timestamp").and_then(|v| v.as_str()) {
                            ts = time.to_string();
                        }
                    }
            }

        result.push(SavedDataloggerSessionEntry {
            stem,
            folder: dir.to_path_buf(),
            csv_path,
            json_path,
            title,
            operator_name: op_name,
            operator_notes: op_notes,
            duration_secs: duration,
            sample_count: samples,
            timestamp_str: ts,
        });
    }

    result.sort_by(|a, b| b.stem.cmp(&a.stem));
    result
}

/// Elimina los archivos asociados a una captura de osciloscopio
pub fn delete_scope_capture_files(entry: &SavedScopeCaptureEntry) {
    if let Some(ref p) = entry.json_path { let _ = fs::remove_file(p); }
    if let Some(ref p) = entry.nwc_path { let _ = fs::remove_file(p); }
    if let Some(ref p) = entry.csv_path { let _ = fs::remove_file(p); }
    if let Some(ref p) = entry.png_path { let _ = fs::remove_file(p); }
}

/// Elimina los archivos asociados a una sesión de datalogger
pub fn delete_datalogger_session_files(entry: &SavedDataloggerSessionEntry) {
    let _ = fs::remove_file(&entry.csv_path);
    if let Some(ref p) = entry.json_path { let _ = fs::remove_file(p); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_capture_dirs() {
        ensure_all_capture_dirs();
        assert!(Path::new(SCOPE_CAPTURES_DIR).exists());
        assert!(Path::new(DATALOGGER_CAPTURES_DIR).exists());
        assert!(Path::new(TRACER_CAPTURES_DIR).exists());
    }

    #[test]
    fn test_timestamp_format() {
        let ts = current_timestamp_str();
        assert_eq!(ts.len(), 15); // YYYYMMDD_HHMMSS
        assert!(ts.contains('_'));
    }

    #[test]
    fn test_vi_model_database_crud() {
        let mut db = ViModelDatabase::default();
        let entry = ViModelEntry {
            id: "test_pin1".to_string(),
            name: "Pin 1 Inyector".to_string(),
            category: "Inyectores".to_string(),
            description: "Firma de prueba".to_string(),
            operator_name: "Carlos".to_string(),
            created_at: "2026-11-09".to_string(),
            points: vec![(0.0, 0.0), (1.0, 5.0), (2.0, 15.0)],
            tolerance_percent: 5.0,
            r_sense_ohms: 1000.0,
            invert_vertical: true,
            ch1_vdiv: 1.0,
            ch2_vdiv: 0.5,
            ch1_probe: 10.0,
            ch2_probe: 10.0,
            timebase: 0.001,
            burst_count: 16,
            grid_divs_h: 8,
            grid_divs_v: 8,
            equivalent_summary: Some("Diodo".to_string()),
        };

        db.add_or_update(entry.clone());
        assert_eq!(db.models.len(), 1);
        assert_eq!(db.get("test_pin1").unwrap().name, "Pin 1 Inyector");

        let deleted = db.remove("test_pin1");
        assert!(deleted);
        assert_eq!(db.models.len(), 0);
    }

    #[test]
    fn test_scan_scope_captures_empty() {
        let list = scan_scope_captures("Temp/test_captures_empty");
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_scan_datalogger_sessions_empty() {
        let list = scan_datalogger_sessions("Temp/test_datalogger_empty");
        assert_eq!(list.len(), 0);
    }
}
