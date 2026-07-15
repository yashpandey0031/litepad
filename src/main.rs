// Hide the console window in release builds (keep it in debug for logs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fonts;
mod markup;
mod storage;
mod theme;

use app::LitePadApp;

/// The app icon, decoded from the PNG bundled into the binary.
fn load_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("bundled icon should be a valid PNG")
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([620.0, 420.0])
            .with_title("LitePad")
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "LitePad",
        options,
        Box::new(|cc| Ok(Box::new(LitePadApp::new(cc)))),
    )
}
