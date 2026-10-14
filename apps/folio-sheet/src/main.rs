//! Folio Sheet: a native spreadsheet. Only the cells on screen are drawn, so a big sheet stays
//! fast; formulas are calculated by `folio-sheet-engine`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui::{self, Align2, Color32, FontId, Key, Modifiers, Pos2, Rect, Sense, Stroke, StrokeKind, vec2};
use folio_sheet_engine::{Addr, Sheet, Value, col_name, from_csv, to_csv};

const COLS: u32 = 52;
const ROWS: u32 = 1000;
const CELL_W: f32 = 100.0;
const CELL_H: f32 = 25.0;
const HEAD_W: f32 = 48.0;
const HEAD_H: f32 = 26.0;
const MAX_UNDO: usize = 100;

type Cells = HashMap<Addr, String>;

struct App {
    sheet: Sheet,
    sel: Addr,
    /// Text being typed into a cell, with the cell it belongs to.
    editing: Option<(Addr, String)>,
    /// Set for one frame after editing starts so the editor grabs focus.
    focus_editor: bool,
    path: Option<PathBuf>,
    dirty: bool,
    message: Option<String>,
    undo: Vec<Cells>,
    redo: Vec<Cells>,
    formula_text: String,
    formula_for: Option<Addr>,
}

impl App {
    fn new() -> App {
        let mut sheet = Sheet::new();
        for (a, t) in [
            ((0, 0), "Item"),
            ((1, 0), "Qty"),
            ((2, 0), "Price"),
            ((3, 0), "Total"),
            ((0, 1), "Coffee"),
            ((1, 1), "3"),
            ((2, 1), "4.5"),
            ((3, 1), "=B2*C2"),
            ((0, 2), "Lunch"),
            ((1, 2), "2"),
            ((2, 2), "12"),
            ((3, 2), "=B3*C3"),
            ((0, 3), "Total"),
            ((3, 3), "=SUM(D2:D3)"),
        ] {
            sheet.set(Addr::new(a.0, a.1), t);
        }
        App {
            sheet,
            sel: Addr::new(0, 0),
            editing: None,
            focus_editor: false,
            path: None,
            dirty: false,
            message: None,
            undo: Vec::new(),
            redo: Vec::new(),
            formula_text: String::new(),
            formula_for: None,
        }
    }

    fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("Untitled".into(), |n| n.to_string_lossy().to_string());
        format!("{name}{} - Folio Sheet", if self.dirty { " •" } else { "" })
    }

    // ---- editing -------------------------------------------------------------------------------
    fn snapshot(&mut self) {
        self.undo.push(self.sheet.cells().clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn set_cell(&mut self, at: Addr, text: &str) {
        if self.sheet.raw(at) == text {
            return;
        }
        self.snapshot();
        self.sheet.set(at, text);
        self.dirty = true;
        self.formula_for = None;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.sheet.cells().clone());
            self.sheet = Sheet::from_cells(prev);
            self.dirty = true;
            self.formula_for = None;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.sheet.cells().clone());
            self.sheet = Sheet::from_cells(next);
            self.dirty = true;
            self.formula_for = None;
        }
    }

    fn move_sel(&mut self, dc: i64, dr: i64) {
        let c = (i64::from(self.sel.col) + dc).clamp(0, i64::from(COLS) - 1) as u32;
        let r = (i64::from(self.sel.row) + dr).clamp(0, i64::from(ROWS) - 1) as u32;
        self.sel = Addr::new(c, r);
    }

    fn start_edit(&mut self, initial: Option<String>) {
        let text = initial.unwrap_or_else(|| self.sheet.raw(self.sel).to_string());
        self.editing = Some((self.sel, text));
        self.focus_editor = true;
    }

    fn commit_edit(&mut self) {
        if let Some((at, text)) = self.editing.take() {
            self.set_cell(at, &text);
        }
    }

    // ---- files ---------------------------------------------------------------------------------
    fn new_sheet(&mut self) {
        self.snapshot();
        self.sheet.clear();
        self.path = None;
        self.dirty = false;
        self.sel = Addr::new(0, 0);
    }

    fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file() else { return };
        match std::fs::read(&path) {
            Ok(bytes) => {
                self.snapshot();
                self.sheet = Sheet::from_cells(from_csv(&String::from_utf8_lossy(&bytes)));
                self.path = Some(path);
                self.dirty = false;
                self.sel = Addr::new(0, 0);
                self.message = None;
            }
            Err(e) => self.message = Some(format!("Could not open the file: {e}")),
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.path, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new().add_filter("CSV", &["csv"]).set_file_name("sheet.csv").save_file() {
                Some(p) => p,
                None => return,
            },
        };
        match std::fs::write(&path, to_csv(&self.sheet)) {
            Ok(()) => {
                self.path = Some(path);
                self.dirty = false;
                self.message = Some("Saved (values only; formulas are not stored in CSV).".into());
            }
            Err(e) => self.message = Some(format!("Could not save the file: {e}")),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));
        self.shortcuts(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    self.new_sheet();
                }
                if ui.button("Open").clicked() {
                    self.open();
                }
                if ui.button("Save").clicked() {
                    self.save(false);
                }
                if ui.button("Save as…").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo")).clicked() {
                    self.undo();
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo")).clicked() {
                    self.redo();
                }
            });
            ui.add_space(4.0);
            // formula bar
            ui.horizontal(|ui| {
                ui.add_sized([56.0, 22.0], egui::Label::new(egui::RichText::new(self.sel.label()).strong()));
                if self.formula_for != Some(self.sel) && self.editing.is_none() {
                    self.formula_text = self.sheet.raw(self.sel).to_string();
                    self.formula_for = Some(self.sel);
                }
                let editing_here = self.editing.as_ref().is_some_and(|(a, _)| *a == self.sel);
                if editing_here {
                    if let Some((_, buf)) = self.editing.as_mut() {
                        ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY).interactive(false));
                    }
                } else {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.formula_text)
                            .desired_width(f32::INFINITY)
                            .hint_text("Value or =formula"),
                    );
                    if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        let (at, text) = (self.sel, self.formula_text.clone());
                        self.set_cell(at, &text);
                        self.move_sel(0, 1);
                    }
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let v = self.sheet.value(self.sel);
                let text = match &v {
                    Value::Empty => "Empty".to_string(),
                    other => other.to_string(),
                };
                ui.label(egui::RichText::new(format!("{} = {text}", self.sel.label())).small());
                if let Some(m) = &self.message {
                    ui.separator();
                    ui.label(egui::RichText::new(m).small());
                }
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ctx.style().visuals.extreme_bg_color))
            .show(ctx, |ui| self.grid(ui));
    }
}

impl App {
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let editing = self.editing.is_some();
        // Typing or pasting a character while a cell is selected starts editing it.
        let typed: Option<String> = if editing || ctx.memory(|m| m.focused().is_some()) {
            None
        } else {
            // all text typed this frame, so a fast "20" is not cut down to "2"
            let text: String = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Text(t) if !t.chars().all(char::is_control) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect()
            });
            (!text.is_empty()).then_some(text)
        };
        let (undo, redo_a, redo_b, save, open, copy, cut, paste) = ctx.input_mut(|i| {
            let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
            (
                i.consume_key(Modifiers::COMMAND, Key::Z),
                i.consume_key(shift_cmd, Key::Z),
                i.consume_key(Modifiers::COMMAND, Key::Y),
                i.consume_key(Modifiers::COMMAND, Key::S),
                i.consume_key(Modifiers::COMMAND, Key::O),
                i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                i.events.iter().find_map(|e| if let egui::Event::Paste(t) = e { Some(t.clone()) } else { None }),
            )
        });
        if !editing {
            if undo {
                self.undo();
            }
            if redo_a || redo_b {
                self.redo();
            }
            if copy || cut {
                ctx.copy_text(self.sheet.raw(self.sel).to_string());
                if cut {
                    let at = self.sel;
                    self.set_cell(at, "");
                }
            }
            if let Some(text) = paste {
                self.paste(&text);
            }
        }
        if save {
            self.save(false);
        }
        if open {
            self.open();
        }
        if let Some(t) = typed {
            self.start_edit(Some(t));
            return;
        }
        if editing || ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        // Every key press this frame, in order, so quick sequences are never dropped.
        let presses: Vec<(Key, Modifiers)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                    _ => None,
                })
                .collect()
        });
        for (key, mods) in presses {
            match key {
                Key::ArrowUp => self.move_sel(0, -1),
                Key::ArrowDown => self.move_sel(0, 1),
                Key::ArrowLeft => self.move_sel(-1, 0),
                Key::ArrowRight => self.move_sel(1, 0),
                Key::Tab => self.move_sel(if mods.shift { -1 } else { 1 }, 0),
                Key::Enter | Key::F2 => {
                    self.start_edit(None);
                    break;
                }
                Key::Delete | Key::Backspace => {
                    let at = self.sel;
                    self.set_cell(at, "");
                }
                _ => {}
            }
        }
    }

    /// Pastes text into the selected cell; tab/newline separated text fills a block.
    fn paste(&mut self, text: &str) {
        let rows: Vec<&str> = text.lines().take(1000).collect();
        if rows.len() <= 1 && !text.contains('\t') {
            let at = self.sel;
            self.set_cell(at, text.trim_end_matches(['\r', '\n']));
            return;
        }
        self.snapshot();
        for (dr, line) in rows.iter().enumerate() {
            for (dc, field) in line.split('\t').take(100).enumerate() {
                let (c, r) = (self.sel.col + dc as u32, self.sel.row + dr as u32);
                if c < COLS && r < ROWS {
                    self.sheet.set(Addr::new(c, r), field);
                }
            }
        }
        self.dirty = true;
        self.formula_for = None;
    }

    fn grid(&mut self, ui: &mut egui::Ui) {
        let total = vec2(HEAD_W + CELL_W * COLS as f32, HEAD_H + CELL_H * ROWS as f32);
        let visuals = ui.visuals().clone();
        let line = visuals.widgets.noninteractive.bg_stroke.color;
        let ink = visuals.text_color();
        let head_fill = visuals.faint_bg_color;
        let accent = Color32::from_rgb(0x4f, 0x46, 0xe5);
        let mut clicked: Option<Addr> = None;
        let mut double: bool = false;

        egui::ScrollArea::both().auto_shrink([false, false]).show_viewport(ui, |ui, viewport| {
            let origin = ui.min_rect().min;
            let (rect, resp) = ui.allocate_exact_size(total, Sense::click());
            let painter = ui.painter_at(ui.clip_rect());
            let font = FontId::proportional(14.0);

            let first_col = (((viewport.min.x - HEAD_W) / CELL_W).floor().max(0.0)) as u32;
            let last_col = ((((viewport.max.x - HEAD_W) / CELL_W).ceil()).max(0.0) as u32).min(COLS - 1);
            let first_row = (((viewport.min.y - HEAD_H) / CELL_H).floor().max(0.0)) as u32;
            let last_row = ((((viewport.max.y - HEAD_H) / CELL_H).ceil()).max(0.0) as u32).min(ROWS - 1);
            let cell_rect = |c: u32, r: u32| {
                Rect::from_min_size(
                    origin + vec2(HEAD_W + c as f32 * CELL_W, HEAD_H + r as f32 * CELL_H),
                    vec2(CELL_W, CELL_H),
                )
            };

            // cells
            for r in first_row..=last_row {
                for c in first_col..=last_col {
                    let at = Addr::new(c, r);
                    let cr = cell_rect(c, r);
                    painter.rect_stroke(cr, 0.0, Stroke::new(1.0_f32, line), StrokeKind::Inside);
                    if self.editing.as_ref().is_some_and(|(a, _)| *a == at) {
                        continue;
                    }
                    let v = self.sheet.value(at);
                    if v == Value::Empty {
                        continue;
                    }
                    let text = v.to_string();
                    let (color, align_right) = match &v {
                        Value::Error(_) => (Color32::from_rgb(0xdc, 0x26, 0x26), false),
                        Value::Num(_) => (ink, true),
                        _ => (ink, false),
                    };
                    let clip = painter.with_clip_rect(cr.shrink(1.0));
                    if align_right {
                        clip.text(cr.right_center() - vec2(6.0, 0.0), Align2::RIGHT_CENTER, text, font.clone(), color);
                    } else {
                        clip.text(cr.left_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, text, font.clone(), color);
                    }
                }
            }

            // selection
            let sel = cell_rect(self.sel.col, self.sel.row);
            painter.rect_stroke(sel, 0.0, Stroke::new(2.0_f32, accent), StrokeKind::Inside);

            // sticky headers, drawn last at the viewport's top-left so they stay in view
            let top = origin.y + viewport.min.y;
            let left = origin.x + viewport.min.x;
            for c in first_col..=last_col {
                let hr =
                    Rect::from_min_size(Pos2::new(origin.x + HEAD_W + c as f32 * CELL_W, top), vec2(CELL_W, HEAD_H));
                let active = c == self.sel.col;
                painter.rect_filled(hr, 0.0, if active { accent.gamma_multiply(0.25) } else { head_fill });
                painter.rect_stroke(hr, 0.0, Stroke::new(1.0_f32, line), StrokeKind::Inside);
                painter.text(hr.center(), Align2::CENTER_CENTER, col_name(c), font.clone(), ink);
            }
            for r in first_row..=last_row {
                let hr =
                    Rect::from_min_size(Pos2::new(left, origin.y + HEAD_H + r as f32 * CELL_H), vec2(HEAD_W, CELL_H));
                let active = r == self.sel.row;
                painter.rect_filled(hr, 0.0, if active { accent.gamma_multiply(0.25) } else { head_fill });
                painter.rect_stroke(hr, 0.0, Stroke::new(1.0_f32, line), StrokeKind::Inside);
                painter.text(hr.center(), Align2::CENTER_CENTER, (r + 1).to_string(), font.clone(), ink);
            }
            let corner = Rect::from_min_size(Pos2::new(left, top), vec2(HEAD_W, HEAD_H));
            painter.rect_filled(corner, 0.0, head_fill);
            painter.rect_stroke(corner, 0.0, Stroke::new(1.0_f32, line), StrokeKind::Inside);

            // clicks
            if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.clicked() || resp.double_clicked()) {
                let rel = p - rect.min;
                if rel.x >= HEAD_W && rel.y >= HEAD_H {
                    let c = ((rel.x - HEAD_W) / CELL_W) as u32;
                    let r = ((rel.y - HEAD_H) / CELL_H) as u32;
                    if c < COLS && r < ROWS {
                        clicked = Some(Addr::new(c, r));
                        double = resp.double_clicked();
                    }
                }
            }

            // in-cell editor
            if let Some((at, buf)) = self.editing.as_mut() {
                let er = cell_rect(at.col, at.row);
                let width = (er.width()).max(120.0);
                let editor = Rect::from_min_size(er.min, vec2(width, er.height()));
                painter.rect_filled(editor, 0.0, visuals.extreme_bg_color);
                painter.rect_stroke(editor, 0.0, Stroke::new(2.0_f32, accent), StrokeKind::Inside);
                let r = ui.put(
                    editor.shrink2(vec2(4.0, 1.0)),
                    egui::TextEdit::singleline(buf).frame(false).font(font.clone()),
                );
                if self.focus_editor {
                    r.request_focus();
                    self.focus_editor = false;
                }
                let (enter, tab, esc) =
                    ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Tab), i.key_pressed(Key::Escape)));
                if esc {
                    self.editing = None;
                } else if enter || tab {
                    let at = *at;
                    self.commit_edit();
                    self.sel = at;
                    if tab { self.move_sel(1, 0) } else { self.move_sel(0, 1) }
                } else if r.lost_focus() && !self.focus_editor {
                    // clicked elsewhere: keep what was typed
                    self.commit_edit();
                }
            }
        });

        if let Some(at) = clicked {
            if self.editing.is_some() {
                self.commit_edit();
            }
            self.sel = at;
            if double {
                self.start_edit(None);
            }
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Folio Sheet")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([480.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native("Folio Sheet", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}
