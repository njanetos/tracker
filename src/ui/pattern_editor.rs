use eframe::egui;

use crate::core::action::{Action, Direction, NoteKey};
use crate::core::pattern::{format_note, NOTE_OFF};
use crate::core::state::AppState;

/// Determine the visual weight of a row based on time signature structure.
/// Returns 2 for bar lines, 1 for beat lines, 0 for normal rows.
fn row_weight(row: usize, state: &AppState) -> u8 {
    let rows_per_beat = state.rows_per_beat;
    let rows_per_bar = state.time_signature.rows_per_bar(rows_per_beat);

    if rows_per_bar > 0 && row.is_multiple_of(rows_per_bar) {
        2 // bar boundary
    } else if rows_per_beat > 0 && row.is_multiple_of(rows_per_beat) {
        1 // beat boundary
    } else {
        0
    }
}

const CELL_WIDTH: f32 = 64.0;
const CELL_HEIGHT: f32 = 20.0;
const ROW_NUM_WIDTH: f32 = 32.0;

/// Draw the pattern editor grid and handle keyboard input.
/// Returns a list of actions to apply.
pub fn draw_pattern_editor(
    ui: &mut egui::Ui,
    state: &AppState,
    playback_row: usize,
) -> Vec<Action> {
    let mut actions = Vec::new();

    // Handle keyboard input
    actions.extend(handle_keyboard(ui, state));

    let total_w = ROW_NUM_WIDTH + state.pattern.num_channels as f32 * CELL_WIDTH;
    let total_h = state.pattern.num_rows as f32 * CELL_HEIGHT;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (response, painter) =
                ui.allocate_painter(egui::vec2(total_w, total_h), egui::Sense::click());
            let origin = response.rect.min;

            // Handle click to select cell
            if let Some(pos) = response.interact_pointer_pos() {
                let rel = pos - origin;
                let col = ((rel.x - ROW_NUM_WIDTH) / CELL_WIDTH) as isize;
                let row = (rel.y / CELL_HEIGHT) as isize;
                if col >= 0
                    && (col as usize) < state.pattern.num_channels
                    && row >= 0
                    && (row as usize) < state.pattern.num_rows
                {
                    // We can't directly set cursor, so we'll generate appropriate move actions
                    // For simplicity, use SetNote-style targeting via a click action
                    // Actually, let's just move cursor to clicked position
                    // This is a bit of a hack but keeps things simple for MVP
                    actions.push(Action::SetCursorPosition {
                        row: row as usize,
                        channel: col as usize,
                    });
                }
            }

            for row in 0..state.pattern.num_rows {
                let y = origin.y + row as f32 * CELL_HEIGHT;

                // Row number
                let row_rect = egui::Rect::from_min_size(
                    egui::pos2(origin.x, y),
                    egui::vec2(ROW_NUM_WIDTH, CELL_HEIGHT),
                );
                let weight = row_weight(row, state);
                let row_bg = if weight == 2 {
                    egui::Color32::from_rgb(50, 50, 50)
                } else if weight == 1 {
                    egui::Color32::from_rgb(40, 40, 40)
                } else {
                    egui::Color32::from_rgb(30, 30, 30)
                };
                painter.rect_filled(row_rect, 0.0, row_bg);
                painter.text(
                    row_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:02X}", row),
                    egui::FontId::monospace(13.0),
                    egui::Color32::from_rgb(120, 120, 120),
                );

                for ch in 0..state.pattern.num_channels {
                    let x = origin.x + ROW_NUM_WIDTH + ch as f32 * CELL_WIDTH;
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(x, y),
                        egui::vec2(CELL_WIDTH, CELL_HEIGHT),
                    );

                    // Background
                    let bg = if row == state.cursor_row && ch == state.cursor_channel {
                        egui::Color32::from_rgb(60, 60, 140) // cursor
                    } else if state.is_playing && row == playback_row {
                        egui::Color32::from_rgb(40, 80, 40) // playback
                    } else if weight == 2 {
                        egui::Color32::from_rgb(50, 50, 50)
                    } else if weight == 1 {
                        egui::Color32::from_rgb(40, 40, 40)
                    } else {
                        egui::Color32::from_rgb(30, 30, 30)
                    };
                    painter.rect_filled(rect, 0.0, bg);

                    // Border
                    painter.rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
                        egui::StrokeKind::Outside,
                    );

                    // Note text
                    let note = state.pattern.get(row, ch);
                    let text = format_note(note.pitch);
                    let text_color = if note.is_empty() {
                        egui::Color32::from_rgb(80, 80, 80)
                    } else if note.pitch == NOTE_OFF {
                        egui::Color32::from_rgb(200, 100, 100)
                    } else {
                        egui::Color32::from_rgb(220, 220, 220)
                    };
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::monospace(13.0),
                        text_color,
                    );
                }
            }
        });

    actions
}

fn handle_keyboard(ui: &mut egui::Ui, _state: &AppState) -> Vec<Action> {
    let mut actions = Vec::new();

    ui.input(|input| {
        // Navigation
        if input.key_pressed(egui::Key::ArrowUp) {
            actions.push(Action::MoveCursor(Direction::Up));
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            actions.push(Action::MoveCursor(Direction::Down));
        }
        if input.key_pressed(egui::Key::ArrowLeft) {
            actions.push(Action::MoveCursor(Direction::Left));
        }
        if input.key_pressed(egui::Key::ArrowRight) {
            actions.push(Action::MoveCursor(Direction::Right));
        }

        // Delete
        if input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace) {
            actions.push(Action::Delete);
        }

        // Transport
        if input.key_pressed(egui::Key::Space) {
            actions.push(Action::TogglePlayback);
        }

        // Note keys (only when no modifier is held)
        if !input.modifiers.ctrl && !input.modifiers.command && !input.modifiers.alt {
            // Tilde/backtick = note off
            if input.key_pressed(egui::Key::Backtick) {
                actions.push(Action::NoteOff);
            }

            let note_mappings: &[(egui::Key, NoteKey)] = &[
                // Home row: A=C, S=C#, D=D, F=D#, G=E, H=F, J=F#, K=G, L=G#, ;=A, '=A#
                (egui::Key::A, NoteKey::A),
                (egui::Key::S, NoteKey::S),
                (egui::Key::D, NoteKey::D),
                (egui::Key::F, NoteKey::F),
                (egui::Key::G, NoteKey::G),
                (egui::Key::H, NoteKey::H),
                (egui::Key::J, NoteKey::J),
                (egui::Key::K, NoteKey::K),
                (egui::Key::L, NoteKey::L),
                (egui::Key::Semicolon, NoteKey::Semicolon),
                (egui::Key::Quote, NoteKey::Quote),
                // Upper row: Q=C+oct, W=C#+oct, ...
                (egui::Key::Q, NoteKey::Q),
                (egui::Key::W, NoteKey::W),
                (egui::Key::E, NoteKey::E),
                (egui::Key::R, NoteKey::R),
                (egui::Key::T, NoteKey::T),
                (egui::Key::Y, NoteKey::Y),
                (egui::Key::U, NoteKey::U),
                (egui::Key::I, NoteKey::I),
                (egui::Key::O, NoteKey::O),
                (egui::Key::P, NoteKey::P),
            ];

            for &(egui_key, note_key) in note_mappings {
                if input.key_pressed(egui_key) {
                    actions.push(Action::NoteKeyPress(note_key));
                }
            }
        }
    });

    actions
}
