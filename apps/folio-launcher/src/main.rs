//! Folio Launcher: the desktop window around the shared UI.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use folio_ui::Launcher;

struct Shell {
    launcher: Launcher,
}

impl eframe::App for Shell {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.launcher.ui(ctx);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Folio")
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([520.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native("Folio", options, Box::new(|cc| Ok(Box::new(Shell { launcher: Launcher::new(&cc.egui_ctx) }))))
}
