//! Drawing: an editorial header (eyebrow, headline, icon strip) and a responsive grid of
//! numbered app cards.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, ProgressBar, RichText, Sense, Stroke, StrokeKind, vec2,
};
use folio_core::Status;
use folio_native::Latest;

use crate::model::{AppState, Job};
use crate::{Launcher, theme};

const CARD_MIN: f32 = 260.0;
const GAP: f32 = 16.0;

pub fn show(ctx: &egui::Context, ui_state: &mut Launcher) {
    let t = theme::tokens(ui_state.dark);

    egui::TopBottomPanel::top("header")
        .frame(
            egui::Frame::new()
                .fill(t.panel)
                .stroke(Stroke::new(1.0_f32, t.line))
                .inner_margin(egui::Margin::symmetric(28, 18)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("01 / THE SUITE").small().color(t.accent).strong());
                    ui.label(RichText::new("Eight tools. One home.").heading().strong());
                    ui.label(RichText::new("Eight apps · Open source · Pure Rust").small().color(t.muted));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for s in &ui_state.model.apps {
                            mini_tile(ui, s);
                        }
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ui.button(if ui_state.dark { "Light" } else { "Dark" }).clicked() {
                        ui_state.toggle_theme(ctx);
                    }
                    if ui.button("Check again").clicked() {
                        ui_state.backend.refresh_all(&folio_core::SUITE);
                    }
                });
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("02 / PICK AN APP").small().color(t.muted).strong());
            ui.add_space(4.0);
            let width = ui.available_width() - 24.0;
            let cols = (((width + GAP) / (CARD_MIN + GAP)).floor() as usize).max(1);
            let card_w = ((width - GAP * (cols as f32 - 1.0)) / cols as f32).max(200.0);
            let mut action: Option<&'static folio_core::App> = None;
            egui::Grid::new("apps").spacing(vec2(GAP, GAP)).show(ui, |ui| {
                for (i, state) in ui_state.model.apps.iter().enumerate() {
                    let status = ui_state.model.status(state);
                    if card(ui, &t, state, i + 1, status, card_w) {
                        action = Some(state.app);
                    }
                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(16.0);
            ui.label(RichText::new("More apps are on the way.").color(t.muted));
            if let Some(app) = action {
                if let Some(s) = ui_state.model.apps.iter_mut().find(|s| s.app.id == app.id) {
                    s.job = Job::Working(0);
                }
                ui_state.backend.install(app);
            }
        });
    });
}

/// One numbered app card. Returns true when the install/update button was pressed.
fn card(ui: &mut egui::Ui, t: &theme::Tokens, s: &AppState, number: usize, status: Status, width: f32) -> bool {
    let mut clicked = false;
    egui::Frame::new()
        .fill(t.panel)
        .stroke(Stroke::new(1.0_f32, t.line))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            let inner = width - 34.0;
            ui.set_width(inner);
            ui.set_min_height(196.0);
            ui.vertical(|ui| {
                ui.set_width(inner);
                ui.horizontal(|ui| {
                    tile(ui, s, 44.0);
                    ui.add_space((inner - 44.0 - 22.0).max(0.0));
                    ui.label(RichText::new(format!("{number:02}")).monospace().color(t.muted));
                });
                ui.add_space(2.0);
                ui.label(RichText::new(s.app.name).strong().size(17.0));
                ui.label(
                    RichText::new(format!("{} · an alternative to {}", s.app.tagline, s.app.replaces))
                        .color(t.muted)
                        .small(),
                );

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                    badge(ui, t, s.app.stage.label(), t.muted);
                    if matches!(status, Status::NotInstalled | Status::UpdateAvailable | Status::Current) {
                        badge(ui, t, "Installer ready", t.accent);
                    }
                });

                let working = matches!(s.job, Job::Working(_));
                let line = match status {
                    Status::Unreleased => "No release yet.".to_string(),
                    Status::NoBuild => "No build for this computer yet.".to_string(),
                    Status::Unknown => match &s.latest {
                        Some(Latest::Error(e)) => e.clone(),
                        _ => "Checking…".to_string(),
                    },
                    _ => {
                        let mut parts = Vec::new();
                        if let Some(v) = &s.installed {
                            parts.push(format!("Installed {v}"));
                        }
                        if let Some(Latest::Release(r)) = &s.latest {
                            parts.push(format!("Latest {}", r.tag));
                        }
                        parts.join(" · ")
                    }
                };
                ui.label(RichText::new(line).color(t.muted).small());
                if let Job::Failed(e) = &s.job {
                    ui.label(RichText::new(e).color(t.danger).small());
                }
                if let Job::Working(p) = s.job {
                    ui.add(ProgressBar::new(f32::from(p) / 100.0).desired_height(8.0));
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let action = match status {
                        Status::NotInstalled => Some("Install"),
                        Status::UpdateAvailable => Some("Update"),
                        Status::Current => Some("Reinstall"),
                        _ => None,
                    };
                    if let Some(label) = action {
                        let button = egui::Button::new(
                            RichText::new(if working { "Working…" } else { label }).color(t.panel).strong(),
                        )
                        .fill(t.ink);
                        if ui.add_enabled(!working, button).clicked() {
                            clicked = true;
                        }
                    }
                    let url = match &s.latest {
                        Some(Latest::Release(r)) if !r.page_url.is_empty() => r.page_url.clone(),
                        _ => s.app.releases_url(),
                    };
                    ui.hyperlink_to(format!("Explore {}", s.app.name.trim_start_matches("Folio ")), url);
                });
            });
        });
    clicked
}

/// A small outlined pill of text.
fn badge(ui: &mut egui::Ui, t: &theme::Tokens, text: &str, color: Color32) {
    egui::Frame::new()
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.55)))
        .corner_radius(CornerRadius::same(20))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(if color == t.muted { t.muted } else { color }));
        });
}

fn tile(ui: &mut egui::Ui, s: &AppState, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    paint_tile(ui, rect, s, size);
}

/// The 28 px tile used in the header's icon strip; hovering names the app.
fn mini_tile(ui: &mut egui::Ui, s: &AppState) {
    let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
    paint_tile(ui, rect, s, 28.0);
    resp.on_hover_text(s.app.name);
}

fn paint_tile(ui: &egui::Ui, rect: egui::Rect, s: &AppState, size: f32) {
    let [r, g, b] = s.app.color;
    let color = Color32::from_rgb(r, g, b);
    let radius = CornerRadius::same((size * 0.23) as u8);
    ui.painter().rect_filled(rect, radius, color);
    ui.painter().rect_stroke(rect, radius, Stroke::new(1.0_f32, color.gamma_multiply(0.6)), StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        s.app.initials(),
        FontId::proportional(size * 0.4),
        Color32::WHITE,
    );
}
