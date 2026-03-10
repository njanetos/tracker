use serde::{Deserialize, Serialize};

/// Sentinel value: no note in this cell.
pub const NOTE_EMPTY: u8 = 0;
/// Sentinel value: explicit note-off.
pub const NOTE_OFF: u8 = 255;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// MIDI note number 1-127, or NOTE_EMPTY (0), or NOTE_OFF (255).
    pub pitch: u8,
    /// Instrument index.
    pub instrument: u8,
    /// Velocity 0-127.
    pub velocity: u8,
}

impl Note {
    pub fn is_empty(&self) -> bool {
        self.pitch == NOTE_EMPTY
    }

    pub fn is_note_off(&self) -> bool {
        self.pitch == NOTE_OFF
    }

    pub fn note_off() -> Self {
        Self {
            pitch: NOTE_OFF,
            instrument: 0,
            velocity: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub num_rows: usize,
    pub num_channels: usize,
    /// data[row][channel]
    pub data: Vec<Vec<Note>>,
}

impl Pattern {
    pub fn new(num_rows: usize, num_channels: usize) -> Self {
        Self {
            num_rows,
            num_channels,
            data: vec![vec![Note::default(); num_channels]; num_rows],
        }
    }

    pub fn get(&self, row: usize, channel: usize) -> &Note {
        &self.data[row][channel]
    }

    pub fn set(&mut self, row: usize, channel: usize, note: Note) {
        self.data[row][channel] = note;
    }

    pub fn clear(&mut self, row: usize, channel: usize) {
        self.data[row][channel] = Note::default();
    }
}

/// Format a MIDI pitch as tracker notation (e.g. "C-4", "D#5").
pub fn format_note(pitch: u8) -> String {
    if pitch == NOTE_EMPTY {
        return "---".to_string();
    }
    if pitch == NOTE_OFF {
        return "OFF".to_string();
    }
    let names = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];
    let octave = pitch / 12;
    let name = names[(pitch % 12) as usize];
    format!("{}{}", name, octave)
}

/// Convert a MIDI pitch number to frequency in Hz.
pub fn pitch_to_freq(pitch: u8) -> f32 {
    440.0 * 2.0_f32.powf((pitch as f32 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pattern_is_empty() {
        let p = Pattern::new(64, 4);
        assert_eq!(p.num_rows, 64);
        assert_eq!(p.num_channels, 4);
        for row in 0..64 {
            for ch in 0..4 {
                assert!(p.get(row, ch).is_empty());
            }
        }
    }

    #[test]
    fn test_set_and_get_note() {
        let mut p = Pattern::new(64, 4);
        let note = Note {
            pitch: 60,
            instrument: 0,
            velocity: 100,
        };
        p.set(5, 2, note);
        assert_eq!(p.get(5, 2), &note);
        assert!(p.get(5, 1).is_empty());
    }

    #[test]
    fn test_clear_note() {
        let mut p = Pattern::new(64, 4);
        p.set(
            0,
            0,
            Note {
                pitch: 60,
                instrument: 0,
                velocity: 127,
            },
        );
        p.clear(0, 0);
        assert!(p.get(0, 0).is_empty());
    }

    #[test]
    fn test_format_note_display() {
        assert_eq!(format_note(0), "---");
        assert_eq!(format_note(60), "C-5");
        assert_eq!(format_note(69), "A-5");
        assert_eq!(format_note(61), "C#5");
        assert_eq!(format_note(48), "C-4");
        assert_eq!(format_note(NOTE_OFF), "OFF");
    }

    #[test]
    fn test_note_off() {
        let off = Note::note_off();
        assert!(off.is_note_off());
        assert!(!off.is_empty());
    }

    #[test]
    fn test_pitch_to_freq() {
        let freq = pitch_to_freq(69);
        assert!((freq - 440.0).abs() < 0.01);

        // A5 = 880 Hz
        let freq = pitch_to_freq(81);
        assert!((freq - 880.0).abs() < 0.1);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut p = Pattern::new(16, 2);
        p.set(
            0,
            0,
            Note {
                pitch: 60,
                instrument: 1,
                velocity: 100,
            },
        );
        p.set(
            4,
            1,
            Note {
                pitch: 72,
                instrument: 0,
                velocity: 80,
            },
        );

        let bytes = rmp_serde::to_vec(&p).unwrap();
        let restored: Pattern = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(p, restored);
    }
}
