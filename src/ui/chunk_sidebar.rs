use eframe::egui;

use crate::core::action::Action;
use crate::core::state::AppState;

const CHUNK_SIZE: f32 = 40.0;
const CHUNK_MARGIN: f32 = 4.0;

/// Draw the chunk sidebar. Returns a list of actions to apply.
pub fn draw_chunk_sidebar(ui: &mut egui::Ui, state: &AppState) -> Vec<Action> {
    let mut actions = Vec::new();

    ui.vertical(|ui| {
        ui.label("Chunks");
        ui.add_space(4.0);

        // Track drag-and-drop state
        let drag_id = egui::Id::new("chunk_drag_source");

        for (slot, chunk_opt) in state.chunks.iter().enumerate() {
            let size = egui::vec2(CHUNK_SIZE, CHUNK_SIZE);

            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
            ui.add_space(CHUNK_MARGIN);

            let is_selected = state.selected_chunk == Some(slot);

            // Draw the chunk square
            let bg_color = if is_selected {
                egui::Color32::from_rgb(60, 60, 140)
            } else if response.hovered() {
                egui::Color32::from_rgb(55, 55, 55)
            } else {
                egui::Color32::from_rgb(40, 40, 40)
            };

            ui.painter().rect_filled(rect, 4.0, bg_color);
            ui.painter().rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                egui::StrokeKind::Outside,
            );

            // Draw the label
            let label = match chunk_opt {
                Some(chunk) => format!("{}", chunk.number),
                None => "-".to_string(),
            };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::monospace(16.0),
                egui::Color32::from_rgb(200, 200, 200),
            );

            // Left-click to select a chunk
            if response.clicked() && chunk_opt.is_some() {
                actions.push(Action::SelectChunk { slot });
            }

            // Drag handling
            if chunk_opt.is_some() && response.drag_started() {
                ui.memory_mut(|mem| mem.data.insert_temp(drag_id, slot));
            }

            // Drop handling
            if response.hovered() && ui.input(|i| i.pointer.any_released()) {
                let dragged_from: Option<usize> = ui.memory(|mem| mem.data.get_temp(drag_id));
                if let Some(from_slot) = dragged_from {
                    if from_slot != slot {
                        actions.push(Action::MoveChunk {
                            from_slot,
                            to_slot: slot,
                        });
                    }
                    ui.memory_mut(|mem| mem.data.remove::<usize>(drag_id));
                }
            }

            // Right-click context menu
            response.context_menu(|ui| {
                if chunk_opt.is_none() {
                    if ui.button("New Chunk").clicked() {
                        actions.push(Action::NewChunk { slot });
                        ui.close();
                    }
                } else if ui.button("Delete Chunk").clicked() {
                    actions.push(Action::DeleteChunk { slot });
                    ui.close();
                }
            });
        }
    });

    actions
}
