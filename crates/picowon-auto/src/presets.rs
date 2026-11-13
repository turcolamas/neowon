//! Presets Automotrices Profesionales (Estilo PicoScope 7 Automotive).
//! Cada preset define la configuración completa de canales, base de tiempo,
//! trigger, atenuación y notas técnicas de conexión para el taller mecánico.

use neowon_core::{Coupling, Slope, Sweep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetCategory {
    StartingCharging,
    Sensors,
    Actuators,
    Ignition,
    Injection,
    PressureVacuum,
    Networks,
    ElectricHybrid,
    BrakesChassis,
    AccessoriesParasitic,
}

impl PresetCategory {
    pub const ALL: [Self; 10] = [
        Self::StartingCharging,
        Self::Sensors,
        Self::Actuators,
        Self::Ignition,
        Self::Injection,
        Self::PressureVacuum,
        Self::Networks,
        Self::ElectricHybrid,
        Self::BrakesChassis,
        Self::AccessoriesParasitic,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::StartingCharging => "🔋 1. Carga y Arranque",
            Self::Sensors => "📡 2. Sensores de Motor",
            Self::Actuators => "⚙️ 3. Actuadores y Válvulas",
            Self::Ignition => "🔥 4. Encendido e Ignición",
            Self::Injection => "⛽ 5. Inyección y Combustible",
            Self::PressureVacuum => "💨 6. Presión y Vacío en Cilindro",
            Self::Networks => "🌐 7. Redes y Comunicaciones (CAN/LIN/SENT)",
            Self::ElectricHybrid => "⚡ 8. Baterías y EV/Híbridos",
            Self::BrakesChassis => "🚗 9. Frenos, ABS y Chasis",
            Self::AccessoriesParasitic => "🔌 10. Accesorios y Consumos Parásitos",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelPreset {
    pub enabled: bool,
    pub volts_per_div: f64,
    pub attenuation: f64, // 1.0 (1x), 10.0 (10x), 20.0 (20x)
    pub coupling: Coupling,
    pub label: &'static str,
}

#[derive(Debug, Clone)]
pub struct TriggerPreset {
    pub enabled: bool,
    pub source: u8, // 1 = CH1, 2 = CH2
    pub slope: Slope,
    pub level_volts: f64,
    pub sweep: Sweep,
}

#[derive(Debug, Clone)]
pub struct AutomotivePreset {
    pub id: &'static str,
    pub category: PresetCategory,
    pub name: &'static str,
    pub description: &'static str,
    pub connection_guide: &'static str,
    pub expected_waveform: &'static str,
    pub timebase_s_per_div: f64,
    pub ch1: ChannelPreset,
    pub ch2: ChannelPreset,
    pub trigger: TriggerPreset,
}

pub fn get_all_presets() -> Vec<AutomotivePreset> {
    vec![
        // ====================================================================
        // 1. ENCENDIDO E IGNICIÓN
        // ====================================================================
        AutomotivePreset {
            id: "ign_primary_cop",
            category: PresetCategory::Ignition,
            name: "Primario de Bobina COP (Transistorizado)",
            description: "Diagnóstico del circuito primario de bobina individual (Coil On Plug). Permite evaluar el tiempo de saturación (Dwell), la calidad de masa de la ECU y el pico inductivo.",
            connection_guide: "Canal 1: Punta con ATENUADOR 20:1 o 10:1 al pin de señal/control de la bobina (conmutación a masa). Pinza cocodrilo de masa a negativo de batería o bloque motor.",
            expected_waveform: "Tensión de batería (~12-14V). Caída a masa cercana a 0V durante saturación (2 a 4 ms). Pico inductivo de flyback de 300V a 400V (30-40V en la punta atenuada) seguido de tiempo de quemado de 1.2 a 1.8 ms con oscilaciones finales.",
            timebase_s_per_div: 1.0e-3, // 1 ms/div (10 ms total en pantalla)
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 10.0,
                attenuation: 20.0,
                coupling: Coupling::Dc,
                label: "CH1 Primario COP (20x)",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Falling,
                level_volts: 2.0,
                sweep: Sweep::Normal,
            },
        },
        AutomotivePreset {
            id: "ign_coil_current",
            category: PresetCategory::Ignition,
            name: "Rampa de Corriente de Bobina de Encendido",
            description: "Mide la rampa de corriente en el primario para verificar si la bobina alcanza su saturación nominal sin limitación excesiva ni cortocircuito interno.",
            connection_guide: "Canal 1: Pinza amperimétrica (rango 60A / 100mV/A o 10mV/A) abrazando el cable de alimentación positiva de la bobina. Respetar la flecha de sentido de corriente hacia la bobina.",
            expected_waveform: "Rampa lineal ascendente desde 0A hasta 6A - 9A antes de la apertura del transistor, con corte abrupto a cero.",
            timebase_s_per_div: 1.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 0.5, // 5A/div con 100mV/A
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Corriente Bobina (Pinza)",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 0.2, // Disparo a 2A
                sweep: Sweep::Normal,
            },
        },

        // ====================================================================
        // 2. INYECCIÓN Y COMBUSTIBLE (Categoría Actuadores y Válvulas)
        // ====================================================================
        AutomotivePreset {
            id: "inj_petrol_voltage",
            category: PresetCategory::Actuators,
            name: "Inyector de Nafta / Gasolina Multipunto (Tensión)",
            description: "Comprueba el tiempo de inyección (ancho de pulso), la caída a masa y el pico inductivo al desactivarse el solenoide del inyector.",
            connection_guide: "Canal 1: Punta con atenuador 10:1 o 20:1 al cable de control negativo del inyector. Masa a tierra firme de chasis.",
            expected_waveform: "Reposo en 12-14V. Caída nítida a <0.5V durante 1.5 a 4.0 ms (tiempo de inyección). Al desconectar, pico inductivo pronunciado de 60V a 85V, seguido de estabilización.",
            timebase_s_per_div: 2.0e-3, // 2 ms/div
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 10.0,
                attenuation: 10.0,
                coupling: Coupling::Dc,
                label: "CH1 Inyector (10x)",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Falling,
                level_volts: 3.0,
                sweep: Sweep::Normal,
            },
        },
        AutomotivePreset {
            id: "inj_voltage_current_dual",
            category: PresetCategory::Actuators,
            name: "Inyector Gasolina: Tensión (CH1) + Corriente (CH2)",
            description: "Captura dual que permite observar el momento exacto en que la aguja del inyector se despega de su asiento magnéticamente (punto de inflexión de la corriente).",
            connection_guide: "CH1: Punta 10:1 al control del inyector. CH2: Pinza amperimétrica abrazando el cable de alimentación.",
            expected_waveform: "CH1 muestra el pulso de tensión. CH2 muestra la rampa de corriente (~1A) con una pequeña muesca o inflexión visible correspondiente al movimiento mecánico de la aguja.",
            timebase_s_per_div: 1.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 10.0,
                attenuation: 10.0,
                coupling: Coupling::Dc,
                label: "CH1 Tensión Inyector",
            },
            ch2: ChannelPreset {
                enabled: true,
                volts_per_div: 0.1, // 1A/div
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2 Corriente (Pinza)",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Falling,
                level_volts: 3.0,
                sweep: Sweep::Normal,
            },
        },

        // ====================================================================
        // 3. SENSORES DE MOTOR
        // ====================================================================
        AutomotivePreset {
            id: "sensor_ckp_inductive",
            category: PresetCategory::Sensors,
            name: "Sensor de Cigüeñal Inductivo (VR 60-2 dientes)",
            description: "Analiza la señal senoidal generada por la rueda fónica del cigüeñal y la amplitud de tensión en los dientes faltantes (marca de sincronismo).",
            connection_guide: "Canal 1: Punta 1:1 conectada al cable de señal positiva del sensor inductivo de cigüeñal. Masa al blindaje o masa de motor.",
            expected_waveform: "Onda senoidal pura cuya amplitud y frecuencia aumentan con las RPM (de 2Vpp en arranque hasta >20Vpp a altas RPM). Brecha evidente de amplitud ampliada en los dos dientes faltantes.",
            timebase_s_per_div: 10.0e-3, // 10 ms/div
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 5.0,
                attenuation: 1.0,
                coupling: Coupling::Ac,
                label: "CH1 CKP Inductivo",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 1.5,
                sweep: Sweep::Auto,
            },
        },
        AutomotivePreset {
            id: "sensor_ckp_hall",
            category: PresetCategory::Sensors,
            name: "Sensor de Cigüeñal Efecto Hall (Digital 5V / 12V)",
            description: "Verifica la onda cuadrada digital producida por sensores de tipo Hall, detectando pulsos deformados o faltantes.",
            connection_guide: "Canal 1: Punta 1:1 conectada a la salida de señal del sensor Hall. Masa a masa de chasis.",
            expected_waveform: "Onda cuadrada limpia con nivel bajo (<0.3V) y nivel alto (5.0V o tensión de batería 12V). Flancos verticales rápidos sin ruidos parásitos.",
            timebase_s_per_div: 10.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 2.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 CKP Hall",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 2.5,
                sweep: Sweep::Auto,
            },
        },
        AutomotivePreset {
            id: "sensor_sync_ckp_cmp",
            category: PresetCategory::Sensors,
            name: "Sincronismo de Distribución: CKP (CH1) vs CMP (CH2)",
            description: "La prueba reina de la distribución mecánica: compara la posición relativa del árbol de levas frente al cigüeñal para detectar saltos de correa o cadena estirada.",
            connection_guide: "CH1: Señal de Cigüeñal (CKP). CH2: Señal de Árbol de Levas (CMP). Masa común al motor.",
            expected_waveform: "CH1 muestra el tren de dientes del cigüeñal con el espacio de referencia. CH2 muestra el pulso de fase de levas. La posición del pulso de levas respecto a los dientes del cigüeñal debe coincidir con el manual de taller.",
            timebase_s_per_div: 20.0e-3, // 20 ms/div (200 ms total)
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 5.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Cigüeñal (CKP)",
            },
            ch2: ChannelPreset {
                enabled: true,
                volts_per_div: 2.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2 Levas (CMP)",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 2, // Sincronizar con CMP (evento menos frecuente)
                slope: Slope::Rising,
                level_volts: 2.5,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 4. ACTUADORES Y PWM
        // ====================================================================
        AutomotivePreset {
            id: "act_vvt_pwm",
            category: PresetCategory::Actuators,
            name: "Solenoide de Distribución Variable (VVT / VANOS / Ti-VCT)",
            description: "Verifica la señal PWM enviada por la ECU para modular el caudal de aceite hacia el variador de fase del árbol de levas.",
            connection_guide: "Canal 1: Punta 1:1 o 10:1 al terminal de control del solenoide VVT. Masa al chasis.",
            expected_waveform: "Tren de pulsos de frecuencia constante (típicamente 200 Hz a 300 Hz) con ciclo de trabajo modulado (Duty Cycle de 10% a 90% según demanda de aceleración).",
            timebase_s_per_div: 2.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 5.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Solenoide VVT",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Falling,
                level_volts: 2.5,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 5. REDES CAN BUS Y COMUNICACIÓN
        // ====================================================================
        AutomotivePreset {
            id: "net_can_bus_differential",
            category: PresetCategory::Networks,
            name: "Red CAN Bus: CAN-High (CH1) y CAN-Low (CH2)",
            description: "Inspecciona la integridad de las tramas de comunicación CAN (500 kbps / 250 kbps), reflejos de línea y terminaciones resistivas de 60 ohms.",
            connection_guide: "CH1: Pin 6 del conector OBD-II (CAN-High). CH2: Pin 14 del conector OBD-II (CAN-Low). Pin 4 o 5 (Masa de chasis).",
            expected_waveform: "En reposo (recesivo): ambas líneas a 2.5V. En transmisión (dominante): CAN-High sube a 3.5V (+1.0V) y CAN-Low baja a 1.5V (-1.0V) en espejo simétrico exacto.",
            timebase_s_per_div: 50.0e-6, // 50 µs/div
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 CAN-High (Pin 6)",
            },
            ch2: ChannelPreset {
                enabled: true,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2 CAN-Low (Pin 14)",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 2.8,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 6. ARRANQUE Y CARGA
        // ====================================================================
        AutomotivePreset {
            id: "charge_battery_cranking",
            category: PresetCategory::StartingCharging,
            name: "Caída de Tensión en Arranque (Prueba de Batería y Motor de Partida)",
            description: "Monitorea la caída instantánea de voltaje durante el accionamiento de la llave de arranque para diagnosticar baterías degradadas o motor de arranque en cortocircuito.",
            connection_guide: "Canal 1: Punta 1:1 conectada directamente a los bornes de plomo de la batería (CH1 al polo positivo, masa al polo negativo).",
            expected_waveform: "Tensión inicial de 12.6V. Durante el acople del bendix y primer giro, la caída no debe descender de 9.6V en un motor sano. Al arrancar, debe recuperarse a 13.8V - 14.4V.",
            timebase_s_per_div: 500.0e-3, // 500 ms/div (5 segundos en pantalla)
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 2.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Tensión Batería",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Falling,
                level_volts: 11.0,
                sweep: Sweep::Single, // Disparo único para congelar el evento
            },
        },
        AutomotivePreset {
            id: "charge_alternator_ripple",
            category: PresetCategory::StartingCharging,
            name: "Rizado del Alternador (Diodos Rectificadores)",
            description: "Utiliza acoplamiento AC para eliminar los 14V de continua y放大 amplificar las ondulaciones de los diodos para detectar diodos abiertos o en fuga.",
            connection_guide: "Canal 1: Punta 1:1 al borne B+ del alternador o polo positivo de la batería. Acoplamiento AC obligatorio. Motor en marcha con luces altas encendidas.",
            expected_waveform: "Ondulación regular y uniforme en forma de M (tri-fásica rectificada) de menos de 200 mVpp a 400 mVpp. Picos irregulares o cortes indican diodo dañado.",
            timebase_s_per_div: 2.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 0.1, // 100 mV/div
                attenuation: 1.0,
                coupling: Coupling::Ac, // Acoplamiento AC
                label: "CH1 Rizado Alternador (AC)",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 0.05,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 5. PRESIÓN Y VACÍO EN CILINDRO
        // ====================================================================
        AutomotivePreset {
            id: "pressure_in_cylinder_cranking",
            category: PresetCategory::PressureVacuum,
            name: "Presión en Cilindro con Transductor (Compresión Dinámica + Regla 720°)",
            description: "Analiza la compresión real de cámara, apertura/cierre de válvulas, cruce y contrapresión de escape mediante transductor de presión roscado en bujía.",
            connection_guide: "CH1: Transductor de presión (WPS500X / Autel / Hantek) en cilindro 1 (1V/100psi o 1V/7bar). CH2: Pinza inductiva o secundario en cilindro 1 para sincronismo de fase 720°.",
            expected_waveform: "Pico de compresión simétrico de 120-180 psi (8-12 bar). Torre de expansión simétrica, muesca de apertura de escape 40° antes de PMI, carrera de escape <1.5 psi, y pozo de vacío de admisión profundo.",
            timebase_s_per_div: 20.0e-3, // 20 ms/div
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Transductor Presión (bar/psi)",
            },
            ch2: ChannelPreset {
                enabled: true,
                volts_per_div: 2.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2 Sync Chispa Cil 1",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 2.5,
                sweep: Sweep::Normal,
            },
        },
        AutomotivePreset {
            id: "vacuum_intake_manifold",
            category: PresetCategory::PressureVacuum,
            name: "Vacío en Múltiple de Admisión (Detección de Válvulas Pisadas / Sellado)",
            description: "Conexión de transductor de vacío a toma de admisión para evaluar el sellado de válvulas de admisión y escape en cada cilindro.",
            connection_guide: "Canal 1: Transductor de presión/vacío con adaptador a manguera de vacío directo de múltiple. Motor regulando en ralentí estable.",
            expected_waveform: "Ondas pulsantes regulares de 4 a 6 picos idénticos por ciclo de 720°. Una caída repetitiva en un cilindro indica válvula que no asienta o guía con fuga.",
            timebase_s_per_div: 50.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 0.5,
                attenuation: 1.0,
                coupling: Coupling::Ac,
                label: "CH1 Pulsos Vacío Admisión",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 0.1,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 7. BATERÍAS Y EV/HÍBRIDOS
        // ====================================================================
        AutomotivePreset {
            id: "ev_resolver_motor",
            category: PresetCategory::ElectricHybrid,
            name: "Señales Resolver Motor de Tracción EV/Híbrido (Seno / Coseno)",
            description: "Comprueba las señales ortogonales de 90° del sensor resolver acoplado al rotor del motor-generador eléctrico trifásico.",
            connection_guide: "CH1: Señal Sin(+) respecto a Sin(-). CH2: Señal Cos(+) respecto a Cos(-). Precaución: desconectar previamente la alta tensión según protocolo de seguridad.",
            expected_waveform: "Dos ondas senoidales moduladas de alta frecuencia desfasadas exactamente 90° eléctricos entre sí mientras gira el rotor.",
            timebase_s_per_div: 5.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Ac,
                label: "CH1 Resolver Seno",
            },
            ch2: ChannelPreset {
                enabled: true,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Ac,
                label: "CH2 Resolver Coseno",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 0.5,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 8. FRENOS, ABS Y CHASIS
        // ====================================================================
        AutomotivePreset {
            id: "brakes_abs_wheel_speed",
            category: PresetCategory::BrakesChassis,
            name: "Sensor de Velocidad de Rueda ABS (Activo Magnetorresistivo / Pasivo)",
            description: "Verifica la amplitud, frecuencia e integridad de los pulsos generados por el sensor de rueda al girar la rueda a mano.",
            connection_guide: "Canal 1: Punta al terminal de señal del sensor ABS de rueda. Masa a chasis o retorno del sensor.",
            expected_waveform: "Para inductivo: señal senoidal simétrica de amplitud creciente con la velocidad. Para sensor activo Hall/MR: onda cuadrada limpia de 0.75V a 1.4V con frecuencia proporcional al giro.",
            timebase_s_per_div: 20.0e-3,
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 0.5,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Sensor Rueda ABS",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: true,
                source: 1,
                slope: Slope::Rising,
                level_volts: 0.4,
                sweep: Sweep::Auto,
            },
        },

        // ====================================================================
        // 9. ACCESORIOS Y CONSUMOS PARÁSITOS
        // ====================================================================
        AutomotivePreset {
            id: "parasitic_drain_current",
            category: PresetCategory::AccessoriesParasitic,
            name: "Consumo Parásito y Fuga en Reposo (Corriente de Descarga Batería)",
            description: "Monitoreo prolongado del consumo en reposo tras el apagado de las unidades de control (sleep mode) para localizar módulos con fuga.",
            connection_guide: "Canal 1: Pinza amperimétrica de baja corriente (60A / 100mV por Amperio) abrazando el cable negativo principal de batería.",
            expected_waveform: "Consumo inicial de 1A a 3A durante el apagado de ECUs. Al cabo de 10 a 20 minutos, la corriente debe caer a menos de 35 a 50 mA en reposo absoluto.",
            timebase_s_per_div: 2.0, // 2 s/div (registro lento)
            ch1: ChannelPreset {
                enabled: true,
                volts_per_div: 0.05, // 50 mV/div = 500 mA/div
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH1 Pinza Amperios Batería",
            },
            ch2: ChannelPreset {
                enabled: false,
                volts_per_div: 1.0,
                attenuation: 1.0,
                coupling: Coupling::Dc,
                label: "CH2",
            },
            trigger: TriggerPreset {
                enabled: false,
                source: 1,
                slope: Slope::Rising,
                level_volts: 0.1,
                sweep: Sweep::Auto,
            },
        },
    ]
}
