use super::pattern::{Note, Pattern};

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// A keyboard key that maps to a chromatic semitone offset.
/// Layout starting from A: A=C, S=C#, D=D, F=D#, G=E, H=F, J=F#, K=G, L=G#,
/// Semicolon=A, Quote=A#
/// Upper row Q=C+12, W=C#+12, E=D+12, R=D#+12, T=E+12, Y=F+12, U=F#+12,
/// I=G+12, O=G#+12, P=A+12
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKey {
    // Home row: A..Quote = C..A#
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    Semicolon,
    Quote,
    // Upper row: Q..P = C+12..A+12
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
}

impl NoteKey {
    /// Returns the semitone offset from C in the base octave.
    /// Home row keys return 0..10.
    /// Upper row keys return 12..21.
    pub fn semitone_offset(self) -> u8 {
        match self {
            NoteKey::A => 0,         // C
            NoteKey::S => 1,         // C#
            NoteKey::D => 2,         // D
            NoteKey::F => 3,         // D#
            NoteKey::G => 4,         // E
            NoteKey::H => 5,         // F
            NoteKey::J => 6,         // F#
            NoteKey::K => 7,         // G
            NoteKey::L => 8,         // G#
            NoteKey::Semicolon => 9, // A
            NoteKey::Quote => 10,    // A#
            NoteKey::Q => 12,        // C (+1 oct)
            NoteKey::W => 13,        // C#
            NoteKey::E => 14,        // D
            NoteKey::R => 15,        // D#
            NoteKey::T => 16,        // E
            NoteKey::Y => 17,        // F
            NoteKey::U => 18,        // F#
            NoteKey::I => 19,        // G
            NoteKey::O => 20,        // G#
            NoteKey::P => 21,        // A
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum Action {
    /// Set a note at a specific position.
    SetNote {
        row: usize,
        channel: usize,
        note: Note,
    },
    /// Clear the note at a specific position.
    ClearNote { row: usize, channel: usize },
    /// Move the cursor.
    MoveCursor(Direction),
    /// A note key was pressed — map to MIDI pitch using current octave.
    NoteKeyPress(NoteKey),
    /// Insert a note-off at the current cursor position.
    NoteOff,
    /// Delete the note at the current cursor position.
    Delete,
    /// Start playback.
    Play,
    /// Stop playback.
    Stop,
    /// Toggle play/stop.
    TogglePlayback,
    /// Set the cursor position directly (e.g. from a mouse click).
    SetCursorPosition { row: usize, channel: usize },
    /// Set tempo.
    SetBpm(f64),
    /// Change the current octave (clamped to 0-8).
    SetOctave(u8),
    /// Set how many rows the cursor advances after entering a note.
    SetEditStep(usize),
    /// Set the time signature (numerator, denominator).
    SetTimeSignature { numerator: u8, denominator: u8 },
    /// Set the pattern length in bars.
    SetBars(usize),
    /// Set how many rows represent one beat (denominator-unit).
    SetRowsPerBeat(usize),
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum SideEffect {
    StartAudio,
    StopAudio,
    SendPatternToAudio(Pattern),
    SendTimingToAudio {
        rows_per_beat: usize,
        beat_value: u8,
    },
}
