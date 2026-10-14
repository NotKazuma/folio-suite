//! Wren's look: calm neutrals, one indigo accent, soft corners, hairline borders, no shadows.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

pub struct Tokens {
    pub bg: Color32,
    pub panel: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub danger: Color32,
}

pub fn tokens(dark: bool) -> Tokens {
    if dark {
        Tokens {
            bg: Color32::from_rgb(0x10, 0x10, 0x14),
            panel: Color32::from_rgb(0x1a, 0x1a, 0x21),
            ink: Color32::from_rgb(0xf2, 0xf2, 0xf6),
            muted: Color32::from_rgb(0x9a, 0x9a, 0xa8),
            line: Color32::from_rgb(0x2c, 0x2c, 0x36),
            accent: Color32::from_rgb(0x81, 0x8c, 0xf8),
            danger: Color32::from_rgb(0xf8, 0x71, 0x71),
        }
    } else {
        Tokens {
            bg: Color32::from_rgb(0xf4, 0xf4, 0xf7),
            panel: Color32::WHITE,
            ink: Color32::from_rgb(0x15, 0x15, 0x1c),
            muted: Color32::from_rgb(0x66, 0x66, 0x76),
            line: Color32::from_rgb(0xdc, 0xdc, 0xe6),
            accent: Color32::from_rgb(0x4f, 0x46, 0xe5),
            danger: Color32::from_rgb(0xdc, 0x26, 0x26),
        }
    }
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let t = tokens(dark);
    let mut visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    visuals.panel_fill = t.bg;
    visuals.window_fill = t.panel;
    visuals.extreme_bg_color = t.panel;
    visuals.override_text_color = Some(t.ink);
    visuals.hyperlink_color = t.accent;
    visuals.selection.bg_fill = t.accent;
    visuals.window_stroke = Stroke::new(1.0_f32, t.line);
    let radius = CornerRadius::same(8);
    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        w.corner_radius = radius;
    }
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, t.line);
    visuals.widgets.inactive.bg_fill = t.line;
    visuals.widgets.inactive.weak_bg_fill = t.line;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, t.line);
    visuals.widgets.hovered.weak_bg_fill = t.line.gamma_multiply(1.4);
    visuals.widgets.hovered.bg_fill = t.line.gamma_multiply(1.4);
    visuals.widgets.active.weak_bg_fill = t.accent;
    for w in [&mut visuals.widgets.inactive, &mut visuals.widgets.hovered, &mut visuals.widgets.active] {
        w.fg_stroke = Stroke::new(1.0_f32, t.ink);
    }
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;
    ctx.set_visuals(visuals);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
    });
}
