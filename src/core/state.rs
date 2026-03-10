use super::action::{Action, Direction, NoteKey, SideEffect};
use super::pattern::{Note, Pattern, TimeSignature};

pub const DEFAULT_CHANNELS: usize = 4;
pub const DEFAULT_BPM: f64 = 120.0;
pub const DEFAULT_OCTAVE: u8 = 4;
pub const DEFAULT_EDIT_STEP: usize = 1;
pub const DEFAULT_ROWS_PER_BEAT: usize = 4;
pub const DEFAULT_BARS: usize = 4;

/// A chunk slot: either empty or contains a numbered chunk referencing a pattern.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// The display number for this chunk.
    pub number: usize,
    /// The pattern data for this chunk.
    pub pattern: Pattern,
}

#[derive(Clone, Debug)]
pub struct AppState {
    /// The currently displayed/edited pattern (loaded from the selected chunk).
    pub pattern: Pattern,
    pub cursor_row: usize,
    pub cursor_channel: usize,
    pub is_playing: bool,
    pub bpm: f64,
    pub octave: u8,
    pub edit_step: usize,
    pub time_signature: TimeSignature,
    pub bars: usize,
    pub rows_per_beat: usize,
    /// Ordered list of chunk slots. `None` means empty slot.
    pub chunks: Vec<Option<Chunk>>,
    /// Index of the currently selected chunk slot (if any).
    pub selected_chunk: Option<usize>,
}

impl AppState {
    pub fn new() -> Self {
        let time_sig = TimeSignature::default(); // 4/4
        let bars = DEFAULT_BARS;
        let rows_per_beat = DEFAULT_ROWS_PER_BEAT;
        let num_rows = time_sig.total_rows(bars, rows_per_beat);
        Self {
            pattern: Pattern::new(num_rows, DEFAULT_CHANNELS),
            cursor_row: 0,
            cursor_channel: 0,
            is_playing: false,
            bpm: DEFAULT_BPM,
            octave: DEFAULT_OCTAVE,
            edit_step: DEFAULT_EDIT_STEP,
            time_signature: time_sig,
            bars,
            rows_per_beat,
            chunks: vec![None; 8], // Start with 8 empty slots
            selected_chunk: None,
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
            Action::SetTimeSignature {
                numerator,
                denominator,
            } => {
                if numerator > 0 && denominator > 0 && denominator.is_power_of_two() {
                    self.time_signature = TimeSignature::new(numerator, denominator);
                    self.resize_pattern();
                    effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                    effects.push(SideEffect::SendTimingToAudio {
                        rows_per_beat: self.rows_per_beat,
                        beat_value: self.time_signature.denominator,
                    });
                }
            }
            Action::SetBars(bars) => {
                if bars > 0 {
                    self.bars = bars;
                    self.resize_pattern();
                    effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                }
            }
            Action::SetRowsPerBeat(rpb) => {
                if rpb > 0 {
                    self.rows_per_beat = rpb;
                    self.resize_pattern();
                    effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                    effects.push(SideEffect::SendTimingToAudio {
                        rows_per_beat: self.rows_per_beat,
                        beat_value: self.time_signature.denominator,
                    });
                }
            }
            Action::NewChunk { slot } => {
                if slot < self.chunks.len() && self.chunks[slot].is_none() {
                    let number = self.smallest_unused_chunk_number();
                    let num_rows = self
                        .time_signature
                        .total_rows(self.bars, self.rows_per_beat);
                    self.chunks[slot] = Some(Chunk {
                        number,
                        pattern: Pattern::new(num_rows, DEFAULT_CHANNELS),
                    });
                    // Auto-select the newly created chunk
                    self.selected_chunk = Some(slot);
                    self.pattern = self.chunks[slot].as_ref().unwrap().pattern.clone();
                    effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                }
            }
            Action::DeleteChunk { slot } => {
                if slot < self.chunks.len() && self.chunks[slot].is_some() {
                    self.chunks[slot] = None;
                    if self.selected_chunk == Some(slot) {
                        self.selected_chunk = None;
                        // Clear the editor pattern
                        let num_rows = self
                            .time_signature
                            .total_rows(self.bars, self.rows_per_beat);
                        self.pattern = Pattern::new(num_rows, DEFAULT_CHANNELS);
                        effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                    }
                }
            }
            Action::SelectChunk { slot } => {
                if slot < self.chunks.len() {
                    // Save current pattern back to previously selected chunk
                    self.save_current_chunk();
                    if self.chunks[slot].is_some() {
                        self.selected_chunk = Some(slot);
                        self.pattern = self.chunks[slot].as_ref().unwrap().pattern.clone();
                        self.cursor_row = 0;
                        self.cursor_channel = 0;
                        effects.push(SideEffect::SendPatternToAudio(self.pattern.clone()));
                    }
                }
            }
            Action::MoveChunk { from_slot, to_slot } => {
                if from_slot < self.chunks.len()
                    && to_slot < self.chunks.len()
                    && from_slot != to_slot
                {
                    // Remove from source and insert at destination
                    let chunk = self.chunks.remove(from_slot);
                    self.chunks.insert(to_slot, chunk);
                    // Update selected_chunk to follow the moved chunk
                    if let Some(sel) = self.selected_chunk {
                        if sel == from_slot {
                            self.selected_chunk = Some(to_slot);
                        } else if from_slot < to_slot {
                            if sel > from_slot && sel <= to_slot {
                                self.selected_chunk = Some(sel - 1);
                            }
                        } else if sel >= to_slot && sel < from_slot {
                            self.selected_chunk = Some(sel + 1);
                        }
                    }
                }
            }
        }

        // Keep the selected chunk's pattern in sync with edits
        self.save_current_chunk();

        effects
    }

    /// Recompute the pattern length from time signature, bars, and rows_per_beat,
    /// then resize the pattern (preserving existing data).
    fn resize_pattern(&mut self) {
        let new_rows = self
            .time_signature
            .total_rows(self.bars, self.rows_per_beat);
        self.pattern.resize_rows(new_rows);
        // Clamp cursor
        if self.cursor_row >= self.pattern.num_rows {
            self.cursor_row = self.pattern.num_rows.saturating_sub(1);
        }
    }

    /// Save the current editor pattern back to the selected chunk.
    fn save_current_chunk(&mut self) {
        if let Some(slot) = self.selected_chunk {
            if let Some(Some(chunk)) = self.chunks.get_mut(slot) {
                chunk.pattern = self.pattern.clone();
            }
        }
    }

    /// Find the smallest chunk number not already used.
    fn smallest_unused_chunk_number(&self) -> usize {
        let used: std::collections::HashSet<usize> = self
            .chunks
            .iter()
            .filter_map(|c| c.as_ref().map(|c| c.number))
            .collect();
        let mut n = 0;
        while used.contains(&n) {
            n += 1;
        }
        n
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

    #[test]
    fn test_default_time_signature_and_bars() {
        let state = AppState::new();
        assert_eq!(state.time_signature.numerator, 4);
        assert_eq!(state.time_signature.denominator, 4);
        assert_eq!(state.bars, 4);
        assert_eq!(state.rows_per_beat, 4);
        // 4/4 time, 4 bars, 4 rows/beat = 4 * 4 * 4 = 64 rows
        assert_eq!(state.pattern.num_rows, 64);
    }

    #[test]
    fn test_set_time_signature_resizes_pattern() {
        let mut state = AppState::new();
        // Change to 3/4 time: 3 * 4 * 4 = 48 rows
        state.apply(Action::SetTimeSignature {
            numerator: 3,
            denominator: 4,
        });
        assert_eq!(state.time_signature.numerator, 3);
        assert_eq!(state.pattern.num_rows, 48);
    }

    #[test]
    fn test_set_bars_resizes_pattern() {
        let mut state = AppState::new();
        // 4/4 time, 2 bars: 4 * 2 * 4 = 32 rows
        state.apply(Action::SetBars(2));
        assert_eq!(state.bars, 2);
        assert_eq!(state.pattern.num_rows, 32);
    }

    #[test]
    fn test_set_rows_per_beat_resizes_pattern() {
        let mut state = AppState::new();
        // 4/4 time, 4 bars, 8 rows/beat: 4 * 4 * 8 = 128 rows
        state.apply(Action::SetRowsPerBeat(8));
        assert_eq!(state.rows_per_beat, 8);
        assert_eq!(state.pattern.num_rows, 128);
    }

    #[test]
    fn test_resize_preserves_notes() {
        let mut state = AppState::new();
        let note = Note {
            pitch: 60,
            instrument: 0,
            velocity: 100,
        };
        state.apply(Action::SetNote {
            row: 0,
            channel: 0,
            note,
        });
        // Shrink then grow
        state.apply(Action::SetBars(1)); // 16 rows
        assert_eq!(state.pattern.get(0, 0), &note);
        state.apply(Action::SetBars(4)); // back to 64 rows
        assert_eq!(state.pattern.get(0, 0), &note);
    }

    #[test]
    fn test_shrink_clamps_cursor() {
        let mut state = AppState::new();
        state.cursor_row = 60;
        state.apply(Action::SetBars(1)); // 16 rows, cursor was at 60
        assert_eq!(state.cursor_row, 15); // clamped to last row
    }

    #[test]
    fn test_six_eight_time() {
        let mut state = AppState::new();
        // 6/8 time, 4 bars, 4 rows/beat: 6 * 4 * 4 = 96 rows
        state.apply(Action::SetTimeSignature {
            numerator: 6,
            denominator: 8,
        });
        assert_eq!(state.pattern.num_rows, 96);
    }

    #[test]
    fn test_set_time_signature_emits_timing_effect() {
        let mut state = AppState::new();
        let effects = state.apply(Action::SetTimeSignature {
            numerator: 3,
            denominator: 4,
        });
        assert!(effects.iter().any(|e| matches!(
            e,
            SideEffect::SendTimingToAudio {
                rows_per_beat: 4,
                beat_value: 4
            }
        )));
    }

    #[test]
    fn test_set_rows_per_beat_emits_timing_effect() {
        let mut state = AppState::new();
        let effects = state.apply(Action::SetRowsPerBeat(8));
        assert!(effects.iter().any(|e| matches!(
            e,
            SideEffect::SendTimingToAudio {
                rows_per_beat: 8,
                beat_value: 4
            }
        )));
    }

    #[test]
    fn test_invalid_time_signature_ignored() {
        let mut state = AppState::new();
        // denominator 0
        state.apply(Action::SetTimeSignature {
            numerator: 4,
            denominator: 0,
        });
        assert_eq!(state.time_signature.denominator, 4); // unchanged
                                                         // denominator not power of 2
        state.apply(Action::SetTimeSignature {
            numerator: 4,
            denominator: 3,
        });
        assert_eq!(state.time_signature.denominator, 4); // unchanged
                                                         // numerator 0
        state.apply(Action::SetTimeSignature {
            numerator: 0,
            denominator: 4,
        });
        assert_eq!(state.time_signature.numerator, 4); // unchanged
    }

    #[test]
    fn test_zero_bars_ignored() {
        let mut state = AppState::new();
        state.apply(Action::SetBars(0));
        assert_eq!(state.bars, 4); // unchanged
    }

    #[test]
    fn test_zero_rows_per_beat_ignored() {
        let mut state = AppState::new();
        state.apply(Action::SetRowsPerBeat(0));
        assert_eq!(state.rows_per_beat, 4); // unchanged
    }

    #[test]
    fn test_new_chunk_assigns_smallest_number() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        assert_eq!(state.chunks[0].as_ref().unwrap().number, 0);
        state.apply(Action::NewChunk { slot: 1 });
        assert_eq!(state.chunks[1].as_ref().unwrap().number, 1);
    }

    #[test]
    fn test_new_chunk_auto_selects() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 2 });
        assert_eq!(state.selected_chunk, Some(2));
    }

    #[test]
    fn test_new_chunk_fills_gap_in_numbers() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 }); // number 0
        state.apply(Action::NewChunk { slot: 1 }); // number 1
        state.apply(Action::DeleteChunk { slot: 0 }); // free number 0
        state.apply(Action::NewChunk { slot: 2 }); // should get number 0
        assert_eq!(state.chunks[2].as_ref().unwrap().number, 0);
    }

    #[test]
    fn test_new_chunk_on_occupied_slot_ignored() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        let num = state.chunks[0].as_ref().unwrap().number;
        state.apply(Action::NewChunk { slot: 0 }); // should be ignored
        assert_eq!(state.chunks[0].as_ref().unwrap().number, num);
    }

    #[test]
    fn test_delete_chunk() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        assert!(state.chunks[0].is_some());
        state.apply(Action::DeleteChunk { slot: 0 });
        assert!(state.chunks[0].is_none());
    }

    #[test]
    fn test_delete_selected_chunk_clears_selection() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        assert_eq!(state.selected_chunk, Some(0));
        state.apply(Action::DeleteChunk { slot: 0 });
        assert_eq!(state.selected_chunk, None);
    }

    #[test]
    fn test_select_chunk_loads_pattern() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        // Edit a note in chunk 0
        state.apply(Action::NoteKeyPress(NoteKey::A));
        let chunk0_note = state.pattern.get(0, 0).pitch;

        state.apply(Action::NewChunk { slot: 1 });
        // Chunk 1 should be empty
        assert!(state.pattern.get(0, 0).is_empty());

        // Switch back to chunk 0
        state.apply(Action::SelectChunk { slot: 0 });
        assert_eq!(state.pattern.get(0, 0).pitch, chunk0_note);
    }

    #[test]
    fn test_select_empty_slot_ignored() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        state.apply(Action::SelectChunk { slot: 3 }); // empty slot
        assert_eq!(state.selected_chunk, Some(0)); // unchanged
    }

    #[test]
    fn test_move_chunk_reorders() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 }); // number 0
        state.apply(Action::NewChunk { slot: 1 }); // number 1
        state.apply(Action::MoveChunk {
            from_slot: 0,
            to_slot: 1,
        });
        // Chunk with number 0 should now be at slot 1
        assert_eq!(state.chunks[1].as_ref().unwrap().number, 0);
    }

    #[test]
    fn test_move_chunk_updates_selection() {
        let mut state = AppState::new();
        state.apply(Action::NewChunk { slot: 0 });
        assert_eq!(state.selected_chunk, Some(0));
        state.apply(Action::MoveChunk {
            from_slot: 0,
            to_slot: 2,
        });
        assert_eq!(state.selected_chunk, Some(2));
    }

    #[test]
    fn test_initial_chunks_are_empty() {
        let state = AppState::new();
        assert_eq!(state.chunks.len(), 8);
        assert!(state.chunks.iter().all(|c| c.is_none()));
        assert_eq!(state.selected_chunk, None);
    }
}
