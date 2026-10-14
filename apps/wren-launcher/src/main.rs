//! Wren Launcher: the desktop window around the shared UI.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use wren_ui::Launcher;

struct Shell {
    launcher: Launcher,
}

impl eframe::App for Shell {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.launcher.ui(ctx);
    }
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Wren Launcher")
        .with_inner_size([1040.0, 700.0])
        .with_min_inner_size([520.0, 420.0]);
    // The Wren mark (an original bird drawing) as the window icon, if it decodes.
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "Wren Launcher",
        options,
        Box::new(|cc| Ok(Box::new(Shell { launcher: Launcher::new(&cc.egui_ctx) }))),
    )
}
