use super::instrument::InstrumentPlugin;
use crate::core::pattern::Pattern;

const MAX_CHANNELS: usize = 64;

#[derive(Clone, Debug)]
pub struct SequencerState {
    pub current_row: usize,
    pub samples_since_tick: f64,
    pub samples_per_tick: f64,
    pub playing: bool,
    /// Last note playing on each channel (for monophonic note-off before new note).
    pub active_note: [u8; MAX_CHANNELS],
}

impl SequencerState {
    pub fn new(bpm: f64, sample_rate: f64, rows_per_beat: usize, beat_value: u8) -> Self {
        Self {
            current_row: 0,
            samples_since_tick: 0.0,
            samples_per_tick: Self::compute_samples_per_tick(
                bpm,
                sample_rate,
                rows_per_beat,
                beat_value,
            ),
            playing: false,
            active_note: [0; MAX_CHANNELS],
        }
    }

    /// Compute samples per row tick.
    ///
    /// BPM defines quarter notes per minute. Each row represents 1/rows_per_beat
    /// of a denominator-unit beat. The beat_value (denominator) determines the
    /// relationship to quarter notes: beat_value/4 denominator-beats per quarter note.
    pub fn compute_samples_per_tick(
        bpm: f64,
        sample_rate: f64,
        rows_per_beat: usize,
        beat_value: u8,
    ) -> f64 {
        let rows_per_second = (bpm / 60.0) * (beat_value as f64 / 4.0) * rows_per_beat as f64;
        sample_rate / rows_per_second
    }

    pub fn set_bpm(&mut self, bpm: f64, sample_rate: f64, rows_per_beat: usize, beat_value: u8) {
        self.samples_per_tick =
            Self::compute_samples_per_tick(bpm, sample_rate, rows_per_beat, beat_value);
    }
}

/// Pure audio rendering function. No hardware dependency.
///
/// Fills `output` with interleaved stereo audio.
/// Advances the sequencer, triggers notes, and renders instruments.
pub fn render_block(
    output: &mut [f32],
    num_channels: u16,
    sample_rate: f32,
    sequencer: &mut SequencerState,
    pattern: &Pattern,
    instruments: &mut [Box<dyn InstrumentPlugin>],
) {
    let frames = output.len() / num_channels as usize;

    for frame in 0..frames {
        // Check for tick boundary
        if sequencer.playing && sequencer.samples_since_tick >= sequencer.samples_per_tick {
            sequencer.samples_since_tick -= sequencer.samples_per_tick;

            // Trigger notes for current row (monophonic per channel)
            for ch in 0..pattern.num_channels.min(instruments.len()) {
                let note = pattern.get(sequencer.current_row, ch);
                if note.is_note_off() {
                    // Explicit note-off
                    if sequencer.active_note[ch] != 0 {
                        instruments[ch].note_off(sequencer.active_note[ch]);
                        sequencer.active_note[ch] = 0;
                    }
                } else if !note.is_empty() {
                    // Stop previous note on this channel before starting new one
                    if sequencer.active_note[ch] != 0 {
                        instruments[ch].note_off(sequencer.active_note[ch]);
                    }
                    instruments[ch].note_on(note.pitch, note.velocity);
                    sequencer.active_note[ch] = note.pitch;
                }
            }

            sequencer.current_row = (sequencer.current_row + 1) % pattern.num_rows;
        }

        // Render one sample from all instruments
        let mut sample = 0.0_f32;
        let mut mono_buf = [0.0_f32; 1];
        for inst in instruments.iter_mut() {
            mono_buf[0] = 0.0;
            inst.render(&mut mono_buf, sample_rate);
            sample += mono_buf[0];
        }

        sample = sample.clamp(-1.0, 1.0);

        // Write to all output channels
        for ch in 0..num_channels as usize {
            output[frame * num_channels as usize + ch] = sample;
        }

        if sequencer.playing {
            sequencer.samples_since_tick += 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::instrument::TestSineInstrument;
    use crate::core::pattern::{Note, Pattern};

    fn make_instruments(count: usize) -> Vec<Box<dyn InstrumentPlugin>> {
        (0..count)
            .map(|_| Box::new(TestSineInstrument::new()) as Box<dyn InstrumentPlugin>)
            .collect()
    }

    #[test]
    fn test_silent_when_stopped() {
        let mut seq = SequencerState::new(120.0, 44100.0, 4, 4);
        seq.playing = false;
        let pattern = Pattern::new(64, 4);
        let mut instruments = make_instruments(4);
        let mut output = vec![0.0_f32; 512];

        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        assert!(output.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_sequencer_advances_rows() {
        let mut seq = SequencerState::new(120.0, 44100.0, 4, 4);
        seq.playing = true;
        // At 120 BPM, 4 rows/beat: ~5512.5 samples per tick
        // Render 12000 samples (> 2 ticks), should advance 2 rows
        let pattern = Pattern::new(64, 1);
        let mut instruments = make_instruments(1);
        let mut output = vec![0.0_f32; 24000]; // 12000 frames * 2 channels

        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        assert!(
            seq.current_row >= 2,
            "Expected row >= 2, got {}",
            seq.current_row
        );
    }

    #[test]
    fn test_note_triggers_nonzero_output() {
        let mut seq = SequencerState::new(120.0, 44100.0, 4, 4);
        seq.playing = true;

        let mut pattern = Pattern::new(64, 1);
        pattern.set(
            0,
            0,
            Note {
                pitch: 69,
                instrument: 0,
                velocity: 127,
            },
        );

        let mut instruments = make_instruments(1);
        // Render enough for at least one tick + some audio
        let mut output = vec![0.0_f32; 12000]; // 6000 frames stereo

        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        assert!(
            output.iter().any(|&s| s != 0.0),
            "Expected non-zero output after note trigger"
        );
    }

    #[test]
    fn test_timing_accuracy() {
        let bpm = 120.0;
        let sample_rate = 44100.0;
        let mut seq = SequencerState::new(bpm, sample_rate, 4, 4);
        seq.playing = true;

        let pattern = Pattern::new(64, 1);
        let mut instruments = make_instruments(1);

        // samples_per_tick = 44100 / 8 = 5512.5
        // After 5512 samples, should still be on row 0 (hasn't crossed yet)
        // Initial tick fires at sample 0 (samples_since_tick starts at 0,
        // which is < samples_per_tick, so no tick fires until we accumulate enough)
        let mut output = vec![0.0_f32; 5512 * 2]; // 5512 frames stereo
        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        // The first tick fires when samples_since_tick >= 5512.5
        // After 5512 samples, samples_since_tick = 5512.0 < 5512.5
        assert_eq!(
            seq.current_row, 0,
            "Should not have advanced yet after 5512 samples"
        );

        // Render 2 more samples (total 5514)
        let mut output2 = vec![0.0_f32; 4]; // 2 frames stereo
        render_block(
            &mut output2,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        // Now samples_since_tick = 5514 >= 5512.5, should have advanced
        assert_eq!(
            seq.current_row, 1,
            "Should have advanced to row 1 after ~5513 samples"
        );
    }

    #[test]
    fn test_monophonic_new_note_stops_previous() {
        let mut seq = SequencerState::new(120.0, 44100.0, 4, 4);
        seq.playing = true;

        let mut pattern = Pattern::new(64, 1);
        // Row 0: play C5 (60)
        pattern.set(
            0,
            0,
            Note {
                pitch: 60,
                instrument: 0,
                velocity: 127,
            },
        );
        // Row 1: play E5 (64) — should stop C5 first
        pattern.set(
            1,
            0,
            Note {
                pitch: 64,
                instrument: 0,
                velocity: 127,
            },
        );

        let mut instruments = make_instruments(1);
        // Render past both rows
        let mut output = vec![0.0_f32; 24000];
        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        // After both rows fire, only the second note (64) should be active
        assert_eq!(seq.active_note[0], 64);
    }

    #[test]
    fn test_note_off_silences_channel() {
        let mut seq = SequencerState::new(120.0, 44100.0, 4, 4);
        seq.playing = true;

        let mut pattern = Pattern::new(64, 1);
        // Row 0: play a note
        pattern.set(
            0,
            0,
            Note {
                pitch: 69,
                instrument: 0,
                velocity: 127,
            },
        );
        // Row 1: note off
        pattern.set(1, 0, Note::note_off());

        let mut instruments = make_instruments(1);

        // Render past both rows (2 ticks = ~11025 samples, render 14000 to be safe)
        let mut output = vec![0.0_f32; 28000]; // 14000 frames stereo
        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        assert_eq!(seq.active_note[0], 0, "Note should be off after OFF event");

        // Render more — should now be silent
        let mut output2 = vec![0.0_f32; 2048];
        render_block(
            &mut output2,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );
        assert!(
            output2.iter().all(|&s| s == 0.0),
            "Output should be silent after note-off"
        );
    }

    #[test]
    fn test_pattern_wraps() {
        let mut seq = SequencerState::new(120.0, 44100.0, 4, 4);
        seq.playing = true;
        seq.current_row = 63; // last row of a 64-row pattern

        let pattern = Pattern::new(64, 1);
        let mut instruments = make_instruments(1);

        // Render enough for 2 ticks to wrap around
        let mut output = vec![0.0_f32; 24000];
        render_block(
            &mut output,
            2,
            44100.0,
            &mut seq,
            &pattern,
            &mut instruments,
        );

        // Should have wrapped past row 63
        assert!(
            seq.current_row < 63,
            "Pattern should have wrapped, row={}",
            seq.current_row
        );
    }

    #[test]
    fn test_beat_value_affects_timing() {
        let sample_rate = 44100.0;
        let bpm = 120.0;

        // In 4/4 time with 4 rows/beat: rows/sec = (120/60) * (4/4) * 4 = 8
        let spt_44 = SequencerState::compute_samples_per_tick(bpm, sample_rate, 4, 4);
        assert!((spt_44 - 5512.5).abs() < 0.1);

        // In 6/8 time with 4 rows/beat: rows/sec = (120/60) * (8/4) * 4 = 16
        // So samples_per_tick should be half
        let spt_68 = SequencerState::compute_samples_per_tick(bpm, sample_rate, 4, 8);
        assert!((spt_68 - 2756.25).abs() < 0.1);

        // Verify the ratio
        assert!((spt_44 / spt_68 - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_rows_per_beat_affects_timing() {
        let sample_rate = 44100.0;
        let bpm = 120.0;

        let spt_4 = SequencerState::compute_samples_per_tick(bpm, sample_rate, 4, 4);
        let spt_8 = SequencerState::compute_samples_per_tick(bpm, sample_rate, 8, 4);

        // Doubling rows_per_beat should halve samples_per_tick
        assert!((spt_4 / spt_8 - 2.0).abs() < 0.01);
    }
}
