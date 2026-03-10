use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::instrument::{InstrumentPlugin, TestSineInstrument};
use super::render::{render_block, SequencerState};
use crate::core::pattern::Pattern;
use crate::core::state::DEFAULT_CHANNELS;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AudioCommand {
    Play,
    Stop,
    UpdatePattern(Pattern),
    SetBpm(f64),
    /// Update timing parameters (rows_per_beat, beat_value/denominator).
    SetTiming {
        rows_per_beat: usize,
        beat_value: u8,
    },
}

pub struct AudioEngine {
    producer: Producer<AudioCommand>,
    /// Shared playback row for UI to read.
    pub playback_row: Arc<AtomicUsize>,
    _stream: Stream,
}

impl AudioEngine {
    pub fn new(
        initial_pattern: Pattern,
        bpm: f64,
        rows_per_beat: usize,
        beat_value: u8,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output audio device found")?;
        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {}", e))?;

        let sample_rate = config.sample_rate() as f64;
        let channels = config.channels();

        let (producer, consumer) = RingBuffer::<AudioCommand>::new(64);
        let playback_row = Arc::new(AtomicUsize::new(0));
        let playback_row_writer = playback_row.clone();

        let stream = Self::build_stream(
            &device,
            &config.into(),
            consumer,
            initial_pattern,
            bpm,
            sample_rate,
            channels,
            playback_row_writer,
            rows_per_beat,
            beat_value,
        )?;

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {}", e))?;

        Ok(Self {
            producer,
            playback_row,
            _stream: stream,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn build_stream(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        mut consumer: Consumer<AudioCommand>,
        initial_pattern: Pattern,
        bpm: f64,
        sample_rate: f64,
        channels: u16,
        playback_row: Arc<AtomicUsize>,
        rows_per_beat: usize,
        beat_value: u8,
    ) -> Result<Stream, String> {
        let mut sequencer = SequencerState::new(bpm, sample_rate, rows_per_beat, beat_value);
        let mut current_bpm = bpm;
        let mut current_rows_per_beat = rows_per_beat;
        let mut current_beat_value = beat_value;
        let mut pattern = initial_pattern;

        // Create one instrument per channel
        let mut instruments: Vec<Box<dyn InstrumentPlugin>> = (0..DEFAULT_CHANNELS)
            .map(|_| Box::new(TestSineInstrument::new()) as Box<dyn InstrumentPlugin>)
            .collect();

        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Drain pending commands
                    while let Ok(cmd) = consumer.pop() {
                        match cmd {
                            AudioCommand::Play => {
                                sequencer.playing = true;
                                sequencer.current_row = 0;
                                sequencer.samples_since_tick = 0.0;
                            }
                            AudioCommand::Stop => {
                                sequencer.playing = false;
                                // Silence all instruments
                                for inst in &mut instruments {
                                    for pitch in 0..128u8 {
                                        inst.note_off(pitch);
                                    }
                                }
                            }
                            AudioCommand::UpdatePattern(new_pattern) => {
                                pattern = new_pattern;
                            }
                            AudioCommand::SetBpm(new_bpm) => {
                                current_bpm = new_bpm;
                                sequencer.set_bpm(
                                    new_bpm,
                                    sample_rate,
                                    current_rows_per_beat,
                                    current_beat_value,
                                );
                            }
                            AudioCommand::SetTiming {
                                rows_per_beat: rpb,
                                beat_value: bv,
                            } => {
                                current_rows_per_beat = rpb;
                                current_beat_value = bv;
                                sequencer.set_bpm(current_bpm, sample_rate, rpb, bv);
                            }
                        }
                    }

                    // Clear buffer
                    for s in data.iter_mut() {
                        *s = 0.0;
                    }

                    render_block(
                        data,
                        channels,
                        sample_rate as f32,
                        &mut sequencer,
                        &pattern,
                        &mut instruments,
                    );

                    playback_row.store(sequencer.current_row, Ordering::Relaxed);
                },
                |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build output stream: {}", e))?;

        Ok(stream)
    }

    /// Send a command to the audio thread. Drops the command if the ring buffer is full.
    pub fn send(&mut self, cmd: AudioCommand) {
        let _ = self.producer.push(cmd);
    }
}
