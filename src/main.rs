// Hide the console window that Windows would otherwise attach to a release
// build. Debug builds keep it for diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod document;
mod persistence;
mod theme;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("Darkboard"),
        ..Default::default()
    };

    eframe::run_native(
        "Darkboard",
        options,
        Box::new(|cc| Ok(Box::new(app::DarkboardApp::new(cc)))),
    )
}
