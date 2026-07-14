// Hide the console window in release builds (keep it in debug for logs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fonts;
mod markup;
mod storage;
mod theme;

use app::RustPadApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([620.0, 420.0])
            .with_title("RustPad"),
        ..Default::default()
    };

    eframe::run_native(
        "RustPad",
        options,
        Box::new(|cc| Ok(Box::new(RustPadApp::new(cc)))),
    )
}
