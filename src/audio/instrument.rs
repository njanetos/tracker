use crate::core::pattern::pitch_to_freq;

pub trait InstrumentPlugin: Send {
    fn note_on(&mut self, pitch: u8, velocity: u8);
    fn note_off(&mut self, pitch: u8);
    /// Render mono audio into the buffer (additive — adds to existing contents).
    fn render(&mut self, buffer: &mut [f32], sample_rate: f32);
}

const MAX_VOICES: usize = 16;
const MASTER_GAIN: f32 = 0.2;

#[derive(Clone, Debug)]
struct Voice {
    pitch: u8,
    phase: f32,
    amplitude: f32,
    active: bool,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            pitch: 0,
            phase: 0.0,
            amplitude: 0.0,
            active: false,
        }
    }
}

/// A simple polyphonic sine wave synthesizer for testing.
pub struct TestSineInstrument {
    voices: Vec<Voice>,
}

impl TestSineInstrument {
    pub fn new() -> Self {
        Self {
            voices: vec![Voice::default(); MAX_VOICES],
        }
    }
}

impl Default for TestSineInstrument {
    fn default() -> Self {
        Self::new()
    }
}

impl InstrumentPlugin for TestSineInstrument {
    fn note_on(&mut self, pitch: u8, velocity: u8) {
        // First, try to reuse a voice already playing this pitch
        if let Some(v) = self
            .voices
            .iter_mut()
            .find(|v| v.active && v.pitch == pitch)
        {
            v.amplitude = (velocity as f32 / 127.0) * MASTER_GAIN;
            v.phase = 0.0;
            return;
        }
        // Find an inactive voice
        if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
            v.pitch = pitch;
            v.phase = 0.0;
            v.amplitude = (velocity as f32 / 127.0) * MASTER_GAIN;
            v.active = true;
            return;
        }
        // Voice stealing: take the first voice
        let v = &mut self.voices[0];
        v.pitch = pitch;
        v.phase = 0.0;
        v.amplitude = (velocity as f32 / 127.0) * MASTER_GAIN;
        v.active = true;
    }

    fn note_off(&mut self, pitch: u8) {
        for v in &mut self.voices {
            if v.active && v.pitch == pitch {
                v.active = false;
            }
        }
    }

    fn render(&mut self, buffer: &mut [f32], sample_rate: f32) {
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let freq = pitch_to_freq(v.pitch);
            let phase_inc = freq / sample_rate;
            for sample in buffer.iter_mut() {
                *sample += v.amplitude * (v.phase * std::f32::consts::TAU).sin();
                v.phase += phase_inc;
                if v.phase >= 1.0 {
                    v.phase -= 1.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silent_by_default() {
        let mut inst = TestSineInstrument::new();
        let mut buf = vec![0.0_f32; 256];
        inst.render(&mut buf, 44100.0);
        assert!(buf.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_note_on_produces_output() {
        let mut inst = TestSineInstrument::new();
        inst.note_on(69, 127); // A4
        let mut buf = vec![0.0_f32; 256];
        inst.render(&mut buf, 44100.0);
        assert!(buf.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_note_off_silences() {
        let mut inst = TestSineInstrument::new();
        inst.note_on(69, 127);
        let mut buf = vec![0.0_f32; 256];
        inst.render(&mut buf, 44100.0);
        assert!(buf.iter().any(|&s| s != 0.0));

        inst.note_off(69);
        let mut buf2 = vec![0.0_f32; 256];
        inst.render(&mut buf2, 44100.0);
        assert!(buf2.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_frequency_correct_a4() {
        let mut inst = TestSineInstrument::new();
        inst.note_on(69, 127); // A4 = 440 Hz
        let sample_rate = 44100.0;
        let num_samples = 44100; // 1 second
        let mut buf = vec![0.0_f32; num_samples];
        inst.render(&mut buf, sample_rate);

        // Count zero crossings (positive -> negative)
        let mut crossings = 0;
        for i in 0..buf.len() - 1 {
            if buf[i] >= 0.0 && buf[i + 1] < 0.0 {
                crossings += 1;
            }
        }
        // Should be approximately 440 crossings in 1 second
        assert!(
            (crossings as i32 - 440).abs() <= 2,
            "Expected ~440 zero crossings, got {}",
            crossings
        );
    }

    #[test]
    fn test_polyphony() {
        let mut inst = TestSineInstrument::new();
        inst.note_on(60, 100); // C4
        inst.note_on(64, 100); // E4
        inst.note_on(67, 100); // G4

        let mut buf = vec![0.0_f32; 256];
        inst.render(&mut buf, 44100.0);

        // With three notes playing, amplitude should be higher than single note
        let mut single_inst = TestSineInstrument::new();
        single_inst.note_on(60, 100);
        let mut single_buf = vec![0.0_f32; 256];
        single_inst.render(&mut single_buf, 44100.0);

        let multi_peak: f32 = buf.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        let single_peak: f32 = single_buf.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(multi_peak > single_peak);
    }

    #[test]
    fn test_voice_stealing() {
        let mut inst = TestSineInstrument::new();
        // Fill all voices
        for i in 0..MAX_VOICES {
            inst.note_on(60 + i as u8, 100);
        }
        // One more should steal
        inst.note_on(90, 100);
        // Should not panic, and the stolen voice should play pitch 90
        let mut buf = vec![0.0_f32; 256];
        inst.render(&mut buf, 44100.0);
        assert!(buf.iter().any(|&s| s != 0.0));
    }
}
