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

        ui.separator();

        // Time signature
        ui.label("Time:");
        let mut num = state.time_signature.numerator;
        let response = ui.add(egui::DragValue::new(&mut num).range(1..=32).speed(0.1));
        if response.changed() {
            actions.push(Action::SetTimeSignature {
                numerator: num,
                denominator: state.time_signature.denominator,
            });
        }
        ui.label("/");
        let mut denom = state.time_signature.denominator;
        let denom_options = [1, 2, 4, 8, 16, 32];
        let mut denom_idx = denom_options.iter().position(|&d| d == denom).unwrap_or(2); // default to 4
        egui::ComboBox::from_id_salt("time_sig_denom")
            .width(36.0)
            .selected_text(format!("{}", denom_options[denom_idx]))
            .show_ui(ui, |ui| {
                for (i, &val) in denom_options.iter().enumerate() {
                    if ui
                        .selectable_value(&mut denom_idx, i, format!("{}", val))
                        .changed()
                    {
                        denom = val;
                        actions.push(Action::SetTimeSignature {
                            numerator: state.time_signature.numerator,
                            denominator: denom,
                        });
                    }
                }
            });

        ui.separator();

        // Bars
        ui.label("Bars:");
        let mut bars = state.bars as u32;
        let response = ui.add(egui::DragValue::new(&mut bars).range(1..=256).speed(0.1));
        if response.changed() {
            actions.push(Action::SetBars(bars as usize));
        }

        ui.separator();

        // Rows per beat
        ui.label("Rows/Beat:");
        let mut rpb = state.rows_per_beat as u32;
        let response = ui.add(egui::DragValue::new(&mut rpb).range(1..=16).speed(0.1));
        if response.changed() {
            actions.push(Action::SetRowsPerBeat(rpb as usize));
        }
    });

    actions
}
