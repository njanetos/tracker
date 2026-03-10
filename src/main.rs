mod app;
mod audio;
mod core;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 600.0])
            .with_title("Tracker"),
        ..Default::default()
    };

    eframe::run_native(
        "Tracker",
        options,
        Box::new(|cc| Ok(Box::new(app::TrackerApp::new(cc)))),
    )
}
