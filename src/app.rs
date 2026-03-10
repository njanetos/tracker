use eframe::egui;
use std::sync::atomic::Ordering;

use crate::audio::engine::{AudioCommand, AudioEngine};
use crate::audio::instrument::{InstrumentFactory, SineInstrumentFactory};
use crate::core::action::SideEffect;
use crate::core::state::AppState;
use crate::ui::{chunk_sidebar, pattern_editor, toolbar};

pub struct TrackerApp {
    state: AppState,
    audio_engine: Option<AudioEngine>,
    audio_error: Option<String>,
    instrument_factories: Vec<Box<dyn InstrumentFactory>>,
}

impl TrackerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let state = AppState::new();
        let audio_engine = match AudioEngine::new(
            state.pattern.clone(),
            state.bpm,
            state.rows_per_beat,
            state.time_signature.denominator,
        ) {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("Failed to initialize audio: {}", e);
                None
            }
        };

        Self {
            state,
            audio_engine,
            audio_error: None,
            instrument_factories: vec![Box::new(SineInstrumentFactory::new())],
        }
    }

    fn process_side_effects(&mut self, effects: Vec<SideEffect>) {
        let engine = match &mut self.audio_engine {
            Some(e) => e,
            None => return,
        };

        for effect in effects {
            match effect {
                SideEffect::StartAudio => {
                    engine.send(AudioCommand::Play);
                }
                SideEffect::StopAudio => {
                    engine.send(AudioCommand::Stop);
                }
                SideEffect::SendPatternToAudio(pattern) => {
                    engine.send(AudioCommand::UpdatePattern(pattern));
                }
                SideEffect::SendTimingToAudio {
                    rows_per_beat,
                    beat_value,
                } => {
                    engine.send(AudioCommand::SetTiming {
                        rows_per_beat,
                        beat_value,
                    });
                }
                SideEffect::SetChannelInstrument {
                    channel,
                    instrument_id,
                } => {
                    // Find the factory and create the instrument
                    if let Some(factory) = self
                        .instrument_factories
                        .iter()
                        .find(|f| f.id() == &instrument_id)
                    {
                        let instrument = factory.create();
                        engine.send(AudioCommand::SetInstrument {
                            channel,
                            instrument,
                        });
                    }
                }
            }
        }
    }
}

impl eframe::App for TrackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Get playback row from audio thread
        let playback_row = self
            .audio_engine
            .as_ref()
            .map(|e| e.playback_row.load(Ordering::Relaxed))
            .unwrap_or(0);

        // Request repaint while playing for smooth playback indicator
        if self.state.is_playing {
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            let actions = toolbar::draw_toolbar(ui, &self.state);
            for action in actions {
                let effects = self.state.apply(action);
                self.process_side_effects(effects);
            }

            // Show audio error if any
            if let Some(ref err) = self.audio_error {
                ui.colored_label(egui::Color32::RED, format!("Audio error: {}", err));
            }
        });

        egui::SidePanel::left("chunk_sidebar")
            .resizable(false)
            .exact_width(60.0)
            .show(ctx, |ui| {
                let actions = chunk_sidebar::draw_chunk_sidebar(ui, &self.state);
                for action in actions {
                    let effects = self.state.apply(action);
                    self.process_side_effects(effects);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let actions = pattern_editor::draw_pattern_editor(ui, &self.state, playback_row);
            for action in actions {
                let effects = self.state.apply(action);
                self.process_side_effects(effects);
            }
        });
    }
}
