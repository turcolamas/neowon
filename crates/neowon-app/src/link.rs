//! The instrument link: the supervisor handle with the scope-side state it
//! feeds, and the per-frame systems that drain its events and push config back.

use bevy::prelude::*;
use neowon_backend::{Capabilities, Event, InstrumentConfig, MultiMode, ScopeConfig, Supervisor};
use neowon_core::SharedFrame;

use crate::sdr;

#[derive(Resource)]
pub struct Link {
    pub sup: Supervisor,
    /// The connected instrument's capabilities, `None` while disconnected.
    /// The variant *is* the instrument, so there is one of these, not
    /// one `Option` per mode (readers: `Link::scope_caps`/`sdr_caps`).
    pub caps: Option<Capabilities>,
    pub status: String,
    pub latest: Option<SharedFrame>,
    pub config: ScopeConfig,
    pub dirty: bool,
    pub frames_seen: u64,
    pub multi: MultiMode,
    /// Elapsed time when the last frame arrived — the WAIT indicator in the
    /// menu bar compares against it (starved Normal/Single trigger).
    pub last_frame_at: f64,
    /// Every frame that arrived this update, oldest first. `latest` is the
    /// newest of them; consumers that need one record read that, while the
    /// recorder takes them all — otherwise the scrollback is capped at the
    /// render rate no matter how fast the instrument captures.
    pub arrived: Vec<SharedFrame>,
    /// Name of the active stimulus (generating backends only).
    pub stimulus: String,
    /// The channel pointer gestures and scroll steps act on.
    pub selected: usize,
    /// Path of the last written screenshot (`shot`/`shotplot`), for the
    /// status line and `get status`.
    pub last_shot: Option<String>,
}

impl Link {
    /// The link as the app starts it: connecting, nothing acquired yet.
    pub fn new(sup: Supervisor, config: ScopeConfig) -> Self {
        Self {
            sup,
            caps: None,
            status: "connecting…".into(),
            latest: None,
            config,
            dirty: false,
            frames_seen: 0,
            multi: MultiMode::TriggerOut,
            last_frame_at: 0.0,
            arrived: Vec::new(),
            stimulus: "probe-comp".into(),
            selected: 0,
            last_shot: None,
        }
    }

    /// Dispara inmediatamente una captura forzada por hardware (SET_FORCETRG)
    pub fn force_trigger(&self) {
        self.sup.force_trigger();
    }

    /// Indica si la fuente de adquisición activa es el Simulador Demo (`SimBackend`).
    pub fn is_sim_mode(&self) -> bool {
        self.caps
            .as_ref()
            .is_some_and(|c| c.name() == "Simulated")
            || self.status.starts_with("Simulated")
            || self.status.contains("Simulador")
    }

    /// Conmuta en caliente entre el Osciloscopio USB Hardware (OWON VDS1022) y el
    /// Generador/Simulador Demo en vivo sin reiniciar la aplicación (Cap. 1.7 Connect Device).
    pub fn switch_device_source(&mut self, use_simulator: bool) {
        self.sup.shutdown();
        self.caps = None;
        self.frames_seen = 0;
        if use_simulator {
            self.status = "Iniciando Simulador Demo Automotriz…".into();
            self.sup = neowon_backend::spawn(|| {
                Ok(Box::new(neowon_sim::SimBackend::new()) as Box<dyn neowon_backend::Backend>)
            });
            self.sup.apply(self.config.clone());
            if self.stimulus.is_empty() {
                self.stimulus = "auto-ckp-60-2".into();
            }
            let _ = self
                .sup
                .commands
                .send(neowon_backend::Command::Stimulus(self.stimulus.clone()));
        } else {
            self.status = "Conectando USB OWON VDS1022…".into();
            self.sup = neowon_backend::spawn(neowon_vds1022::backend::factory(None));
            self.sup.apply(self.config.clone());
        }
    }

    /// Cambia la señal patrón generada por el Simulador Demo en vivo.
    pub fn set_sim_stimulus(&mut self, stim_name: &str) {
        self.stimulus = stim_name.to_string();
        let _ = self
            .sup
            .commands
            .send(neowon_backend::Command::Stimulus(self.stimulus.clone()));
    }
}

pub(crate) fn ingest(
    time: Res<Time>,
    mut link: ResMut<Link>,
    mut sdr: ResMut<sdr::SdrState>,
    mut auto: Option<ResMut<crate::ui::automotive_dock::AutomotiveUiState>>,
) {
    link.arrived.clear();
    while let Ok(event) = link.sup.events.try_recv() {
        match event {
            Event::Connected(caps) => {
                link.status = match &caps {
                    Capabilities::Scope(c) => format!("{} {}", c.name, c.serial),
                    Capabilities::Sdr(c) => format!("{} {} ({})", c.name, c.serial, c.tuner),
                };
                // The mode follows the instrument that answered.
                sdr.active |= caps.sdr().is_some();
                link.caps = Some(caps);
            }
            Event::Disconnected(e) => {
                link.status = format!("disconnected: {e}");
                link.caps = None;
            }
            // Complex frames are SDR data; the scope consumers never see them.
            Event::Frame(f) if f.layout() == neowon_core::SampleLayout::Complex => {
                link.last_frame_at = time.elapsed_secs_f64();
                sdr.note_frame(&f, time.elapsed_secs_f64());
                // Streaming consumers get every frame here; the display path
                // only ever sees the latest one.
                sdr::dab::feed(&mut sdr, &f);
                sdr::iqdump::write(&mut sdr, &f);
                sdr.latest = Some(f);
            }
            Event::Frame(mut f) => {
                link.frames_seen += 1;
                link.last_frame_at = time.elapsed_secs_f64();

                // Evaluación del Calificador de Disparo Automotriz Avanzado (Fase 2 — Cap. 4)
                let mut accept_for_display = true;
                if let Some(a) = auto.as_deref_mut() {
                    if a.trigger_qualifier.mode != picowon_auto::AdvancedTriggerMode::StandardHardware {
                        let src = link.config.trigger.source.min(1);
                        let to_volts = |cap: &neowon_core::ChannelCapture| -> Vec<f64> {
                            cap.data
                                .iter()
                                .map(|&r| (r as f64) * cap.cal.scale_i + cap.cal.offset_i)
                                .collect()
                        };
                        let primary_v = f
                            .channels
                            .iter()
                            .find(|c| c.ch == src)
                            .or_else(|| f.channels.first())
                            .map(to_volts)
                            .unwrap_or_default();
                        let ch1_v = f.channels.iter().find(|c| c.ch == 0).map(to_volts);
                        let ch2_v = f.channels.iter().find(|c| c.ch == 1).map(to_volts);

                        let full_scale_v = link
                            .config
                            .channels
                            .get(src)
                            .map(|c| c.volts_div * 10.0 * c.probe)
                            .unwrap_or(10.0);
                        let hyst_v = (link.config.trigger.hysteresis_pct as f64 / 100.0) * full_scale_v;

                        let q_res = picowon_auto::evaluate_trigger_qualifier(
                            &a.trigger_qualifier,
                            &primary_v,
                            ch1_v.as_deref(),
                            ch2_v.as_deref(),
                            f.sample_rate,
                            link.config.trigger.level,
                            hyst_v,
                        );

                        if q_res.matched {
                            a.trigger_qualifier_rt.matched_frames += 1;
                            if a.trigger_qualifier.realign_to_trigger_pos {
                                if let Some(ev_idx) = q_res.event_sample_idx {
                                    let len = primary_v.len().max(1);
                                    let target_idx = (link.config.position.clamp(0.05, 0.95)
                                        * (len as f64))
                                        .round() as usize;
                                    let mut shifted = (*f).clone();
                                    for cap in shifted.channels.iter_mut() {
                                        cap.data = picowon_auto::shift_samples_to_trigger(
                                            &cap.data,
                                            ev_idx,
                                            target_idx,
                                        );
                                    }
                                    f = std::sync::Arc::new(shifted);
                                }
                            }
                            a.trigger_qualifier_rt.last_result = q_res;
                            if link.config.trigger.sweep == neowon_core::Sweep::Single {
                                link.config.running = false;
                                link.dirty = true;
                            }
                        } else {
                            a.trigger_qualifier_rt.rejected_frames += 1;
                            a.trigger_qualifier_rt.last_result = q_res;
                            if a.trigger_qualifier.hold_until_qualified && link.latest.is_some() {
                                accept_for_display = false;
                            }
                        }
                    }
                }

                link.arrived.push(f.clone());
                if link.frames_seen == 1 || link.frames_seen.is_multiple_of(500) {
                    tracing::info!(frames = link.frames_seen, "acquiring");
                }
                // Si la adquisición está detenida (revisando captura histórica o en pausa), no pisar link.latest
                if link.config.running && accept_for_display {
                    link.latest = Some(f);
                }
            }
            Event::ConfigUpdated(InstrumentConfig::Scope(cfg)) => {
                link.config = cfg;
            }
            Event::ConfigUpdated(InstrumentConfig::Sdr(cfg)) => sdr.config = cfg,
            Event::Error(e) => link.status = format!("error: {e}"),
        }
    }
    // A DAB receiver whose stream has stopped is looking at nothing. The
    // hardware-moving actions reset at once; this catches a stopped backend,
    // a disconnect or a stalled link, where no frame will arrive to carry the
    // receiver's own expiry.
    let stalled = |at: f64| time.elapsed_secs_f64() - at > sdr::dab::NO_INPUT_TIMEOUT_S;
    if sdr.dab.on() && sdr.dab.last_frame_at.is_some_and(stalled) {
        sdr.dab_no_input();
    }
}

pub(crate) fn flush(mut link: ResMut<Link>) {
    if link.dirty {
        link.dirty = false;
        let cfg = link.config.clone();
        link.sup.apply(cfg);
    }
}
