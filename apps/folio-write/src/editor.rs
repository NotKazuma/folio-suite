//! The editor window: toolbar, status bar and the page of blocks.

use std::path::PathBuf;

use eframe::egui::{
    self, Color32, FontId, Key, Modifiers, RichText, Stroke, TextFormat,
    text::{CCursor, CCursorRange, LayoutJob},
};
use folio_write_doc::{Block, Document, Kind, spans, toggle_marker};

use crate::fonts;

const PAGE_WIDTH: f32 = 780.0;

/// Something a block asked for while it was being drawn; applied after the frame's layout.
enum Action {
    Split { block: usize, at: usize },
    JoinPrevious { block: usize },
}

pub struct Editor {
    doc: Document,
    path: Option<PathBuf>,
    dirty: bool,
    message: Option<String>,
    has_bold_font: bool,
    /// Block that has the keyboard focus, if any.
    focus: Option<usize>,
    /// Block and character offset to focus on the next frame.
    pending_focus: Option<(usize, usize)>,
    last_title: String,
}

impl Editor {
    pub fn new(ctx: &egui::Context) -> Editor {
        let has_bold_font = fonts::install(ctx);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 6.0);
            s.visuals.selection.bg_fill = Color32::from_rgb(0x4f, 0x46, 0xe5).gamma_multiply(0.45);
        });
        let doc = Document::from_markdown(WELCOME);
        Editor {
            doc,
            path: None,
            dirty: false,
            message: None,
            has_bold_font,
            focus: None,
            pending_focus: Some((1, 0)),
            last_title: String::new(),
        }
    }

    fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("Untitled".to_string(), |n| n.to_string_lossy().into());
        format!("{}{} - Folio Write", name, if self.dirty { " •" } else { "" })
    }

    // ---- files ---------------------------------------------------------------------------------
    fn new_document(&mut self) {
        self.doc = Document::default();
        self.path = None;
        self.dirty = false;
        self.pending_focus = Some((0, 0));
    }

    fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Markdown or text", &["md", "markdown", "txt"]).pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.doc = Document::from_markdown(&text);
                self.path = Some(path);
                self.dirty = false;
                self.message = None;
                self.pending_focus = Some((0, 0));
            }
            Err(e) => self.message = Some(format!("Could not open the file: {e}")),
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.path, as_new) {
            (Some(p), false) => p.clone(),
            _ => {
                match rfd::FileDialog::new().add_filter("Markdown", &["md"]).set_file_name("document.md").save_file() {
                    Some(p) => p,
                    None => return,
                }
            }
        };
        match std::fs::write(&path, self.doc.to_markdown()) {
            Ok(()) => {
                self.path = Some(path);
                self.dirty = false;
                self.message = Some("Saved.".into());
            }
            Err(e) => self.message = Some(format!("Could not save the file: {e}")),
        }
    }

    // ---- editing -------------------------------------------------------------------------------
    /// Wrap or unwrap the focused block's selection in `marker`.
    fn toggle(&mut self, ctx: &egui::Context, marker: &str) {
        let Some(i) = self.focus else { return };
        let id = block_id(i);
        let Some(mut state) = egui::TextEdit::load_state(ctx, id) else { return };
        let Some(range) = state.cursor.char_range() else { return };
        let (a, b) = (range.primary.index, range.secondary.index);
        let Some(block) = self.doc.blocks.get_mut(i) else { return };
        let (text, from, to) = toggle_marker(&block.text, a.min(b), a.max(b), marker);
        block.text = text;
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(from), CCursor::new(to))));
        state.store(ctx, id);
        self.dirty = true;
        ctx.memory_mut(|m| m.request_focus(id));
    }

    fn set_kind(&mut self, kind: Kind) {
        if let Some(b) = self.focus.and_then(|i| self.doc.blocks.get_mut(i)) {
            b.kind = kind;
            self.dirty = true;
        }
    }

    fn apply(&mut self, action: Action) {
        self.dirty = true;
        match action {
            Action::Split { block, at } => {
                let n = self.doc.split(block, at);
                self.pending_focus = Some((n, 0));
            }
            Action::JoinPrevious { block } => {
                if let Some((i, at)) = self.doc.join_with_previous(block) {
                    self.pending_focus = Some((i, at));
                }
            }
        }
    }
}

impl eframe::App for Editor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Global shortcuts.
        let (cmd_s, cmd_o, cmd_b, cmd_i) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::S),
                i.consume_key(Modifiers::COMMAND, Key::O),
                i.consume_key(Modifiers::COMMAND, Key::B),
                i.consume_key(Modifiers::COMMAND, Key::I),
            )
        });
        if cmd_s {
            self.save(false);
        }
        if cmd_o {
            self.open();
        }
        if cmd_b {
            self.toggle(ctx, "**");
        }
        if cmd_i {
            self.toggle(ctx, "*");
        }

        self.toolbar(ctx);
        self.status_bar(ctx);
        self.page(ctx);

        let title = self.title();
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }
}

impl Editor {
    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("New").clicked() {
                    self.new_document();
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
                let current = self.focus.and_then(|i| self.doc.blocks.get(i)).map(|b| b.kind);
                egui::ComboBox::from_id_salt("style")
                    .selected_text(current.map_or("Style", Kind::label))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for kind in Kind::ALL {
                            if ui.selectable_label(current == Some(kind), kind.label()).clicked() {
                                self.set_kind(kind);
                            }
                        }
                    });
                if ui.button(RichText::new("B").strong()).on_hover_text("Bold (Ctrl+B)").clicked() {
                    self.toggle(ctx, "**");
                }
                if ui.button(RichText::new("I").italics()).on_hover_text("Italic (Ctrl+I)").clicked() {
                    self.toggle(ctx, "*");
                }
            });
            ui.add_space(4.0);
        });
    }

    fn status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let words = self.doc.word_count();
                ui.label(RichText::new(format!("{words} word{}", if words == 1 { "" } else { "s" })).small());
                if let Some(m) = &self.message {
                    ui.separator();
                    ui.label(RichText::new(m).small());
                }
            });
        });
    }

    fn page(&mut self, ctx: &egui::Context) {
        let mut action = None;
        let has_bold = self.has_bold_font;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ctx.style().visuals.extreme_bg_color.gamma_multiply(0.8)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let side = ((ui.available_width() - PAGE_WIDTH) / 2.0).clamp(8.0, 400.0);
                    ui.add_space(16.0);
                    egui::Frame::new()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0_f32, Color32::from_gray(210)))
                        .inner_margin(egui::Margin::symmetric(48, 56))
                        .outer_margin(egui::Margin::symmetric(side as i8, 0))
                        .show(ui, |ui| {
                            ui.set_min_height(900.0);
                            ui.set_max_width(PAGE_WIDTH - 96.0);
                            let mut number = 0;
                            let count = self.doc.blocks.len();
                            for i in 0..count {
                                let kind = self.doc.blocks[i].kind;
                                number = if kind == Kind::Numbered { number + 1 } else { 0 };
                                if let Some(a) =
                                    block_ui(ui, &mut self.doc.blocks[i], i, number, has_bold, &mut self.dirty)
                                {
                                    action = Some(a);
                                }
                            }
                            // Clicking the empty page below the text focuses the last block.
                            let rest = ui.available_rect_before_wrap();
                            if ui.interact(rest, egui::Id::new("page-rest"), egui::Sense::click()).clicked() {
                                self.pending_focus = Some((count.saturating_sub(1), usize::MAX));
                            }
                        });
                    ui.add_space(24.0);
                });
            });

        if let Some(a) = action {
            self.apply(a);
        }
        // Track which block has focus, and honour a requested focus move.
        self.focus = (0..self.doc.blocks.len()).find(|&i| ctx.memory(|m| m.has_focus(block_id(i)))).or(self.focus);
        if let Some((i, at)) = self.pending_focus.take() {
            let i = i.min(self.doc.blocks.len().saturating_sub(1));
            let id = block_id(i);
            let len = self.doc.blocks.get(i).map_or(0, |b| b.text.chars().count());
            let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(at.min(len)))));
            state.store(ctx, id);
            ctx.memory_mut(|m| m.request_focus(id));
            self.focus = Some(i);
            ctx.request_repaint();
        }
    }
}

fn block_id(i: usize) -> egui::Id {
    egui::Id::new(("block", i))
}

fn base_size(kind: Kind) -> f32 {
    match kind {
        Kind::Heading(1) => 30.0,
        Kind::Heading(2) => 24.0,
        Kind::Heading(_) => 19.0,
        _ => 16.5,
    }
}

/// Draws one block and reports Enter / Backspace-at-start as actions.
fn block_ui(
    ui: &mut egui::Ui,
    block: &mut Block,
    i: usize,
    number: usize,
    has_bold: bool,
    dirty: &mut bool,
) -> Option<Action> {
    let id = block_id(i);
    let kind = block.kind;
    let size = base_size(kind);
    let ink = Color32::from_rgb(0x1b, 0x1b, 0x24);
    let was_focused = ui.memory(|m| m.has_focus(id));

    // Keys that change structure are taken before the TextEdit sees them.
    let mut action = None;
    if was_focused {
        let cursor = egui::TextEdit::load_state(ui.ctx(), id).and_then(|s| s.cursor.char_range());
        if let Some(r) = cursor {
            let at = r.primary.index;
            if ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Enter)) {
                action = Some(Action::Split { block: i, at });
            } else if r.is_empty()
                && at == 0
                && i > 0
                && ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Backspace))
            {
                action = Some(Action::JoinPrevious { block: i });
            }
        }
    }

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let gutter = match kind {
            Kind::Bullet => Some("•".to_string()),
            Kind::Numbered => Some(format!("{number}.")),
            Kind::Quote => Some("|".to_string()),
            _ => None,
        };
        if let Some(g) = gutter {
            ui.add_sized(
                [22.0, size + 6.0],
                egui::Label::new(RichText::new(g).size(size).color(Color32::from_gray(120))),
            );
        }
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = LayoutJob::default();
            job.wrap.max_width = wrap_width;
            for s in spans(text.as_str()) {
                let heading = matches!(kind, Kind::Heading(_));
                let bold = s.bold || heading;
                let family = if bold && has_bold { fonts::bold_family() } else { egui::FontFamily::Proportional };
                let color = if s.marker {
                    Color32::from_gray(170)
                } else if kind == Kind::Quote {
                    Color32::from_gray(90)
                } else if bold && !has_bold {
                    Color32::BLACK
                } else {
                    ink
                };
                job.append(
                    &s.text,
                    0.0,
                    TextFormat {
                        font_id: FontId::new(size, family),
                        color,
                        italics: s.italic || kind == Kind::Quote,
                        ..Default::default()
                    },
                );
            }
            ui.painter().layout_job(job)
        };
        let out = egui::TextEdit::multiline(&mut block.text)
            .id(id)
            .frame(false)
            .desired_width(f32::INFINITY)
            .desired_rows(1)
            .layouter(&mut layouter)
            .show(ui);
        if out.response.changed() {
            *dirty = true;
        }
    });
    ui.add_space(if matches!(kind, Kind::Bullet | Kind::Numbered) { 2.0 } else { 8.0 });
    action
}

const WELCOME: &str = "# Welcome to Folio Write\n\nThis is a **native**, *lightweight* word processor. Click anywhere and start typing.\n\n## What works today\n\n- Headings, paragraphs, quotes and lists (choose from the style menu)\n- **Bold** and *italic* with Ctrl+B and Ctrl+I, or the toolbar buttons\n- Open and save plain Markdown files\n\n1. Press Enter to start a new block\n2. Press Backspace at the start of a block to join it with the one above\n\n> Your documents are ordinary Markdown files, so you always own them.\n";
