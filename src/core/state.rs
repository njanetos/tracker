use super::action::{Action, Direction, NoteKey, SideEffect};
use super::pattern::{Note, Pattern};

pub const DEFAULT_ROWS: usize = 64;
pub const DEFAULT_CHANNELS: usize = 4;
pub const DEFAULT_BPM: f64 = 120.0;
pub const DEFAULT_OCTAVE: u8 = 4;
pub const DEFAULT_EDIT_STEP: usize = 1;

#[derive(Clone, Debug)]
pub struct AppState {
    pub pattern: Pattern,
    pub cursor_row: usize,
    pub cursor_channel: usize,
    pub is_playing: bool,
    pub bpm: f64,
    pub octave: u8,
    pub edit_step: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pattern: Pattern::new(DEFAULT_ROWS, DEFAULT_CHANNELS),
            cursor_row: 0,
            cursor_channel: 0,
            is_playing: false,
            bpm: DEFAULT_BPM,
            octave: DEFAULT_OCTAVE,
            edit_step: DEFAULT_EDIT_STEP,
        }
    }

    /// Apply an action, mutate state, and return any side effects.
    pub fn apply(&mut self, action: Action) -> Vec<SideEffect> {
        let mut effects = Vec::new();

        match action {
            Action::SetNote { row, channel, note } => {
                self.pattern.set(row, channel, note);
                effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
            }
            Action::ClearNote { row, channel } => {
                self.pattern.clear(row, channel);
                effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
            }
            Action::MoveCursor(dir) => {
                self.move_cursor(dir);
            }
            Action::NoteKeyPress(key) => {
                let pitch = self.note_key_to_pitch(key);
                if pitch <= 127 {
                    let note = Note {
                        pitch,
                        instrument: 0,
                        velocity: 127,
                    };
                    self.pattern.set(self.cursor_row, self.cursor_channel, note);
                    effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                    // Advance cursor by edit_step
                    for _ in 0..self.edit_step {
                        self.move_cursor(Direction::Down);
                    }
                }
            }
            Action::NoteOff => {
                self.pattern
                    .set(self.cursor_row, self.cursor_channel, Note::note_off());
                effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                for _ in 0..self.edit_step {
                    self.move_cursor(Direction::Down);
                }
            }
            Action::Delete => {
                self.pattern.clear(self.cursor_row, self.cursor_channel);
                effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
            }
            Action::Play => {
                self.is_playing = true;
                effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                effects.push(SideEffect::StartAudio);
            }
            Action::Stop => {
                self.is_playing = false;
                effects.push(SideEffect::StopAudio);
            }
            Action::TogglePlayback => {
                if self.is_playing {
                    return self.apply(Action::Stop);
                } else {
                    return self.apply(Action::Play);
                }
            }
            Action::SetCursorPosition { row, channel } => {
                if row < self.pattern.num_rows && channel < self.pattern.num_channels {
                    self.cursor_row = row;
                    self.cursor_channel = channel;
                }
            }
            Action::SetBpm(bpm) => {
                self.bpm = bpm.clamp(20.0, 999.0);
            }
            Action::SetOctave(oct) => {
                self.octave = oct.min(8);
            }
            Action::SetEditStep(step) => {
                self.edit_step = step;
            }
        }

        effects
    }

    fn move_cursor(&mut self, dir: Direction) {
        match dir {
            Direction::Up => {
                if self.cursor_row == 0 {
                    self.cursor_row = self.pattern.num_rows - 1;
                } else {
                    self.cursor_row -= 1;
                }
            }
            Direction::Down => {
                self.cursor_row = (self.cursor_row + 1) % self.pattern.num_rows;
            }
            Direction::Left => {
                if self.cursor_channel == 0 {
                    self.cursor_channel = self.pattern.num_channels - 1;
                } else {
                    self.cursor_channel -= 1;
                }
            }
            Direction::Right => {
                self.cursor_channel = (self.cursor_channel + 1) % self.pattern.num_channels;
            }
        }
    }

    fn note_key_to_pitch(&self, key: NoteKey) -> u8 {
        let offset = key.semitone_offset();
        let base = (self.octave as u16) * 12;
        let pitch = base + offset as u16;
        pitch.min(127) as u8
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::NoteKey;

    #[test]
    fn test_new_state_defaults() {
        let state = AppState::new();
        assert_eq!(state.cursor_row, 0);
        assert_eq!(state.cursor_channel, 0);
        assert!(!state.is_playing);
        assert_eq!(state.bpm, 120.0);
        assert_eq!(state.octave, 4);
        assert_eq!(state.edit_step, 1);
        assert_eq!(state.pattern.num_rows, 64);
        assert_eq!(state.pattern.num_channels, 4);
    }

    #[test]
    fn test_set_note() {
        let mut state = AppState::new();
        let note = Note {
            pitch: 60,
            instrument: 0,
            velocity: 100,
        };
        let effects = state.apply(Action::SetNote {
            row: 0,
            channel: 0,
            note,
        });
        assert_eq!(state.pattern.get(0, 0), &note);
        assert!(effects
            .iter()
            .any(|e| matches!(e, SideEffect::SendPatternToAudio(_))));
    }

    #[test]
    fn test_clear_note() {
        let mut state = AppState::new();
        state.apply(Action::SetNote {
            row: 0,
            channel: 0,
            note: Note {
                pitch: 60,
                instrument: 0,
                velocity: 100,
            },
        });
        state.apply(Action::ClearNote { row: 0, channel: 0 });
        assert!(state.pattern.get(0, 0).is_empty());
    }

    #[test]
    fn test_move_cursor_wraps_down() {
        let mut state = AppState::new();
        state.cursor_row = 63;
        state.apply(Action::MoveCursor(Direction::Down));
        assert_eq!(state.cursor_row, 0);
    }

    #[test]
    fn test_move_cursor_wraps_up() {
        let mut state = AppState::new();
        state.cursor_row = 0;
        state.apply(Action::MoveCursor(Direction::Up));
        assert_eq!(state.cursor_row, 63);
    }

    #[test]
    fn test_move_cursor_wraps_left() {
        let mut state = AppState::new();
        state.cursor_channel = 0;
        state.apply(Action::MoveCursor(Direction::Left));
        assert_eq!(state.cursor_channel, 3);
    }

    #[test]
    fn test_move_cursor_wraps_right() {
        let mut state = AppState::new();
        state.cursor_channel = 3;
        state.apply(Action::MoveCursor(Direction::Right));
        assert_eq!(state.cursor_channel, 0);
    }

    #[test]
    fn test_note_key_press_sets_note_and_advances() {
        let mut state = AppState::new();
        state.octave = 4;
        // A = C, so pitch = 4*12 + 0 = 48
        state.apply(Action::NoteKeyPress(NoteKey::A));
        assert_eq!(state.pattern.get(0, 0).pitch, 48);
        assert_eq!(state.cursor_row, 1); // advanced by edit_step=1
    }

    #[test]
    fn test_note_key_chromatic_layout() {
        let mut state = AppState::new();
        state.octave = 4;
        // Semicolon = A, offset 9, so pitch = 4*12 + 9 = 57
        state.apply(Action::NoteKeyPress(NoteKey::Semicolon));
        assert_eq!(state.pattern.get(0, 0).pitch, 57);
    }

    #[test]
    fn test_note_key_upper_row() {
        let mut state = AppState::new();
        state.octave = 4;
        // Q = C+1oct, offset 12, so pitch = 4*12 + 12 = 60
        state.apply(Action::NoteKeyPress(NoteKey::Q));
        assert_eq!(state.pattern.get(0, 0).pitch, 60);
    }

    #[test]
    fn test_edit_step_advances_multiple_rows() {
        let mut state = AppState::new();
        state.edit_step = 4;
        state.apply(Action::NoteKeyPress(NoteKey::A));
        assert_eq!(state.cursor_row, 4);
    }

    #[test]
    fn test_delete_clears_current_cell() {
        let mut state = AppState::new();
        state.apply(Action::NoteKeyPress(NoteKey::A));
        state.cursor_row = 0; // go back
        state.apply(Action::Delete);
        assert!(state.pattern.get(0, 0).is_empty());
    }

    #[test]
    fn test_note_off_inserts_and_advances() {
        let mut state = AppState::new();
        state.apply(Action::NoteOff);
        assert!(state.pattern.get(0, 0).is_note_off());
        assert_eq!(state.cursor_row, 1);
    }

    #[test]
    fn test_play_returns_start_audio() {
        let mut state = AppState::new();
        let effects = state.apply(Action::Play);
        assert!(state.is_playing);
        assert!(effects.iter().any(|e| matches!(e, SideEffect::StartAudio)));
    }

    #[test]
    fn test_stop_returns_stop_audio() {
        let mut state = AppState::new();
        state.apply(Action::Play);
        let effects = state.apply(Action::Stop);
        assert!(!state.is_playing);
        assert!(effects.iter().any(|e| matches!(e, SideEffect::StopAudio)));
    }

    #[test]
    fn test_set_bpm_clamps() {
        let mut state = AppState::new();
        state.apply(Action::SetBpm(10.0));
        assert_eq!(state.bpm, 20.0);
        state.apply(Action::SetBpm(5000.0));
        assert_eq!(state.bpm, 999.0);
    }

    #[test]
    fn test_set_octave_clamps() {
        let mut state = AppState::new();
        state.apply(Action::SetOctave(10));
        assert_eq!(state.octave, 8);
    }

    #[test]
    fn test_pitch_clamps_to_127() {
        let mut state = AppState::new();
        state.octave = 8;
        // Upper row key at octave 8: 8*12 + 23 = 119, which is fine
        // But let's test octave 10 (clamped to 8) with high offset
        state.apply(Action::SetOctave(10));
        state.apply(Action::NoteKeyPress(NoteKey::P)); // offset 21
                                                       // octave clamped to 8, pitch = 8*12 + 21 = 117
        assert!(state.pattern.get(0, 0).pitch <= 127);
    }
}
