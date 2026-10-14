//! egui ships no bold face, so Pinion borrows the system's: Segoe UI Bold on Windows, and
//! a few common names elsewhere. If none is found, bold text falls back to a stronger colour.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const CANDIDATES: &[&str] = &[
    "C:\\Windows\\Fonts\\segoeuib.ttf",
    "C:\\Windows\\Fonts\\arialbd.ttf",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
];

pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Registers the bold family; returns whether a real bold face was found.
pub fn install(ctx: &egui::Context) -> bool {
    let mut defs = FontDefinitions::default();
    let found = CANDIDATES.iter().find_map(|p| std::fs::read(p).ok());
    let has_bold = found.is_some();
    let mut family = vec![];
    if let Some(bytes) = found {
        defs.font_data.insert("system-bold".into(), Arc::new(FontData::from_owned(bytes)));
        family.push("system-bold".to_string());
    }
    // Always end with the regular proportional faces so missing glyphs still draw.
    family.extend(defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    defs.families.insert(bold_family(), family);
    ctx.set_fonts(defs);
    has_bold
}
