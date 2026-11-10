//! PicoWon Automotive Core: Módulos de Diagnóstico Automotriz Avanzado.
//! - Presets guiados estilo PicoScope 7 Automotive.
//! - Reglas de sincronismo angular de cigüeñal de 720°.
//! - Motor de Strip-Chart DataLogger con envolvente Peak-Detect.
//! - Análisis dinámico de actuadores (PWM vs Sensor / Curva de Transferencia).
//! - Monitor de tráfico y agregador de tramas de red CAN / LIN.
//! - Trazador de curvas V-I (Análisis de firmas analógicas e impedancia de pines).

pub mod actuator;
pub mod can_diff;
pub mod capture_store;
pub mod crank720;
pub mod datalogger;
pub mod dsp_filter;
pub mod presets;
pub mod scope_persistence;
pub mod traffic;
pub mod vi_tracer;

pub use actuator::{ActuatorAnalysisState, ActuatorPreset, ActuatorTransferPoint};
pub use can_diff::{CanBusHealth, CanPhysicalMetrics, analyze_can_physical_layer};
pub use capture_store::{
    DATALOGGER_CAPTURES_DIR, DataloggerSessionDialogState, SCOPE_CAPTURES_DIR,
    ScopeCaptureDialogState, ScopeCaptureMetadata, ScopeExportRange, TRACER_CAPTURES_DIR,
    ViCaptureDialogState, ViModelDatabase, ViModelEntry, current_timestamp_str,
    ensure_all_capture_dirs, SavedScopeCaptureEntry, SavedDataloggerSessionEntry,
    scan_scope_captures, scan_datalogger_sessions, delete_scope_capture_files,
    delete_datalogger_session_files,
};
pub use crank720::{CrankAngleRuler, EngineStroke};
pub use datalogger::{
    AutomotiveStripChart, DatalogMetric, DvmReading, MetricKind, MetricScale, N_DATALOG_METRICS,
    StripChartMode,
};
pub use dsp_filter::{
    AUTOMOTIVE_LPF_PRESETS, ChannelDspConfig, LpfAlgorithm, RES_ENHANCE_PRESETS,
    apply_channel_dsp, bilateral_edge_preserving_filter, gaussian_low_pass, low_pass_filtfilt,
    resolution_enhance, savitzky_golay_filter,
};
pub use presets::{AutomotivePreset, ChannelPreset, PresetCategory, TriggerPreset, get_all_presets};
pub use scope_persistence::{SavedChannelConfig, ScopePersistenceStore, StartupSafetyManager};
pub use traffic::{BusProtocol, BusTrafficRow, TrafficMonitor};
pub use vi_tracer::{
    ComponentSignature, EquivalentCircuit, PatternMatch, TracerEquipment, TracerEquipmentStore,
    ViTracerPreset, ViTracerState,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_count_and_categories() {
        let presets = get_all_presets();
        assert!(presets.len() >= 10, "Deben existir al menos 10 presets automotrices");
        
        let has_ignition = presets.iter().any(|p| p.category == PresetCategory::Ignition);
        let has_injection = presets.iter().any(|p| p.category == PresetCategory::Injection);
        let has_sensors = presets.iter().any(|p| p.category == PresetCategory::Sensors);
        let has_can = presets.iter().any(|p| p.category == PresetCategory::Networks);

        assert!(has_ignition, "Falta preset de ignición");
        assert!(has_injection, "Falta preset de inyección");
        assert!(has_sensors, "Falta preset de sensores");
        assert!(has_can, "Falta preset de CAN Bus");
    }

    #[test]
    fn test_crank_angle_ruler_strokes() {
        let ruler = CrankAngleRuler {
            enabled: true,
            t_start: 0.0,
            t_end: 0.020, // 20 ms -> 6000 RPM
            cylinder_count: 4,
            show_phase_bands: true,
        };

        assert_eq!(ruler.calculated_rpm(), 6000.0);
        assert_eq!(ruler.get_stroke_at(0.002), Some(EngineStroke::Intake));      // 72° -> Admisión
        assert_eq!(ruler.get_stroke_at(0.007), Some(EngineStroke::Compression)); // 252° -> Compresión
        assert_eq!(ruler.get_stroke_at(0.012), Some(EngineStroke::Power));       // 432° -> Combustión
        assert_eq!(ruler.get_stroke_at(0.018), Some(EngineStroke::Exhaust));     // 648° -> Escape
    }

    #[test]
    fn test_actuator_pwm_vs_sensor_processing() {
        let mut state = ActuatorAnalysisState::default();
        let sample_rate = 100_000.0;
        let n = 2000;
        let mut pwm = vec![0.0f32; n];
        let mut sensor = vec![0.0f32; n];

        // Generar onda PWM de 250 Hz (periodo = 400 muestras a 100 kHz) con Duty 40% (160 muestras en alto)
        let period = 400;
        let high_samples = 160;
        for i in 0..n {
            let phase = i % period;
            pwm[i] = if phase < high_samples { 5.0 } else { 0.0 };
            sensor[i] = 1.8 + (i as f32 / n as f32) * 0.4; // Rampa suave de 1.8V a 2.2V
        }

        state.process_waveform(&pwm, &sensor, sample_rate, 0.0);
        assert!(state.current_freq > 240.0 && state.current_freq < 260.0);
        assert!((state.current_duty - 40.0).abs() < 2.0);
        assert!(!state.points.is_empty(), "Deben haberse registrado puntos en la curva");
    }

    #[test]
    fn test_bus_traffic_monitor() {
        let mut mon = TrafficMonitor::default();
        mon.ingest_can(0.000, 0x120, false, 4, &[0x11, 0x22, 0x33, 0x44]);
        mon.ingest_can(0.020, 0x120, false, 4, &[0x11, 0x99, 0x33, 0x44]); // Byte 1 cambió
        mon.ingest_can(0.010, 0x280, false, 2, &[0xAA, 0xBB]);

        assert_eq!(mon.total_frames_captured, 3);
        let rows = mon.get_sorted_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 0x120);
        assert_eq!(rows[0].count, 2);
        assert_eq!(rows[0].changed_mask, 0b0000_0010, "El byte 1 debe estar marcado como cambiado");
    }
}
