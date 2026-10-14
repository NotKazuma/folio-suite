//! Pinion: a native word processor. Each paragraph, heading or list item is a block of
//! text; `**bold**` and `*italic*` are drawn styled with their markers dimmed.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod editor;
mod fonts;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pinion")
            .with_inner_size([1000.0, 760.0])
            .with_min_inner_size([480.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native("Pinion", options, Box::new(|cc| Ok(Box::new(editor::Editor::new(&cc.egui_ctx)))))
}
