//! Regla Angular de Cigüeñal de 720° (Ciclo Completo de Motor de 4 Tiempos).
//! Permite fijar un punto inicial (0° - ej. PMS / Chispa Cilindro 1) y un
//! punto final (720° - siguiente ciclo del mismo cilindro) y divide
//! el intervalo temporal en las 4 fases de combustión del motor.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineStroke {
    Intake,      // 0° - 180° (Admisión)
    Compression, // 180° - 360° (Compresión)
    Power,       // 360° - 540° (Expansión / Fuerza)
    Exhaust,     // 540° - 720° (Escape)
}

impl EngineStroke {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Intake => "Admisión",
            Self::Compression => "Compresión",
            Self::Power => "Combustión / Fuerza",
            Self::Exhaust => "Escape",
        }
    }

    /// Color en formato RGBA (8-bit) para colorear la cuadrícula o zona en la GPU.
    pub fn color_rgba(&self) -> [u8; 4] {
        match self {
            Self::Intake => [50, 130, 250, 45],       // Azul suave
            Self::Compression => [60, 200, 100, 45],  // Verde suave
            Self::Power => [255, 80, 80, 55],         // Rojo / Naranja
            Self::Exhaust => [240, 200, 60, 45],      // Amarillo suave
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrankAngleRuler {
    pub enabled: bool,
    /// Marca temporal de inicio (0 grados - normalmente PMS de encendido del Cil 1)
    pub t_start: f64,
    /// Marca temporal de fin (720 grados - siguiente PMS del mismo cilindro)
    pub t_end: f64,
    /// Cantidad de cilindros del motor para calcular marcas de encendido (ej. 4, 6, 8)
    pub cylinder_count: u32,
    /// Mostrar bandas de colores en la cuadrícula
    pub show_phase_bands: bool,
}

impl Default for CrankAngleRuler {
    fn default() -> Self {
        Self {
            enabled: false,
            t_start: 0.0,
            t_end: 0.020, // 20 ms por defecto (6000 RPM / 2 = ~20ms por ciclo)
            cylinder_count: 4,
            show_phase_bands: true,
        }
    }
}

impl CrankAngleRuler {
    pub fn cycle_duration(&self) -> f64 {
        (self.t_end - self.t_start).abs().max(1e-6)
    }

    /// Calcula las RPM estimadas del motor a partir del período de los 720° (2 vueltas completas).
    /// En 4 tiempos: RPM = (2 vueltas / Duración del ciclo en segundos) * 60 s/min
    pub fn calculated_rpm(&self) -> f64 {
        let dt = self.cycle_duration();
        (2.0 / dt) * 60.0
    }

    /// Convierte una marca temporal de pantalla `t` a grados de giro de cigüeñal (0.0° a 720.0°).
    pub fn time_to_degrees(&self, t: f64) -> Option<f64> {
        let dt = self.t_end - self.t_start;
        if dt.abs() < 1e-6 {
            return None;
        }
        let deg = (t - self.t_start) / dt * 720.0;
        Some(deg)
    }

    /// Devuelve qué tiempo del motor corresponde a un instante `t`.
    pub fn get_stroke_at(&self, t: f64) -> Option<EngineStroke> {
        let deg = self.time_to_degrees(t)?;
        let normalized = deg.rem_euclid(720.0);
        if normalized < 180.0 {
            Some(EngineStroke::Intake)
        } else if normalized < 360.0 {
            Some(EngineStroke::Compression)
        } else if normalized < 540.0 {
            Some(EngineStroke::Power)
        } else {
            Some(EngineStroke::Exhaust)
        }
    }

    /// Devuelve las marcas temporales de cada fase de 180° [t_0, t_180, t_360, t_540, t_720].
    pub fn phase_timestamps(&self) -> [f64; 5] {
        let dt = self.t_end - self.t_start;
        [
            self.t_start,
            self.t_start + dt * 0.25,
            self.t_start + dt * 0.50,
            self.t_start + dt * 0.75,
            self.t_end,
        ]
    }
}
