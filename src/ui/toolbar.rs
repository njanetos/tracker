use eframe::egui;

use crate::core::action::Action;
use crate::core::state::AppState;

/// Draw the toolbar (transport controls, BPM, octave).
/// Returns actions to apply.
pub fn draw_toolbar(ui: &mut egui::Ui, state: &AppState) -> Vec<Action> {
    let mut actions = Vec::new();

    ui.horizontal(|ui| {
        // Play/Stop
        if state.is_playing {
            if ui.button("Stop").clicked() {
                actions.push(Action::Stop);
            }
        } else if ui.button("Play").clicked() {
            actions.push(Action::Play);
        }

        ui.separator();

        // BPM
        ui.label("BPM:");
        let mut bpm = state.bpm;
        let response = ui.add(
            egui::DragValue::new(&mut bpm)
                .range(20.0..=999.0)
                .speed(1.0),
        );
        if response.changed() {
            actions.push(Action::SetBpm(bpm));
        }

        ui.separator();

        // Octave
        ui.label("Oct:");
        let mut oct = state.octave;
        let response = ui.add(egui::DragValue::new(&mut oct).range(0..=8).speed(0.1));
        if response.changed() {
            actions.push(Action::SetOctave(oct));
        }

        ui.separator();

        // Edit step
        ui.label("Step:");
        let mut step = state.edit_step as u32;
        let response = ui.add(egui::DragValue::new(&mut step).range(0..=16).speed(0.1));
        if response.changed() {
            actions.push(Action::SetEditStep(step as usize));
        }
    });

    actions
}
