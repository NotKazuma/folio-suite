//! Perch: a native presentation editor. Slides are drawn by one function, used for the
//! canvas, the thumbnails and full-screen presenting alike.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Key, Modifiers, Painter, Pos2,
    Rect, Sense, Stroke, StrokeKind, vec2,
};
use wren_perch_doc::{Deck, HEIGHT, Item, Layout, Shape, Slide, Theme, WIDTH};

const MAX_UNDO: usize = 100;
const HANDLE: f32 = 14.0;

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

// ---- fonts ---------------------------------------------------------------------------------------
fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// egui ships no bold face; borrow the system's, if there is one.
fn install_fonts(ctx: &egui::Context) -> bool {
    const CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\segoeuib.ttf",
        "C:\\Windows\\Fonts\\arialbd.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ];
    let mut defs = FontDefinitions::default();
    let found = CANDIDATES.iter().find_map(|p| std::fs::read(p).ok());
    let has = found.is_some();
    let mut family = Vec::new();
    if let Some(bytes) = found {
        defs.font_data.insert("system-bold".into(), Arc::new(FontData::from_owned(bytes)));
        family.push("system-bold".to_string());
    }
    family.extend(defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    defs.families.insert(bold_family(), family);
    ctx.set_fonts(defs);
    has
}

// ---- drawing -------------------------------------------------------------------------------------
/// Draws `slide` into the rectangle at `origin` where one slide unit is `scale` pixels.
fn paint_slide(painter: &Painter, origin: Pos2, scale: f32, slide: &Slide, theme: Theme, has_bold: bool) {
    let area = Rect::from_min_size(origin, vec2(WIDTH * scale, HEIGHT * scale));
    painter.rect_filled(area, 0.0, rgb(theme.palette().background));
    let clip = painter.with_clip_rect(area);
    for it in &slide.items {
        let r = Rect::from_min_size(origin + vec2(it.x, it.y) * scale, vec2(it.w, it.h) * scale);
        match &it.shape {
            Shape::Rect { fill } => {
                clip.rect_filled(r, CornerRadius::same((8.0 * scale).min(255.0) as u8), rgb(*fill));
            }
            Shape::Ellipse { fill } => paint_ellipse(&clip, r, rgb(*fill)),
            Shape::Text { text, size, bold, italic, color } => {
                let family = if *bold && has_bold { bold_family() } else { FontFamily::Proportional };
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = r.width();
                job.append(
                    text,
                    0.0,
                    egui::TextFormat {
                        font_id: FontId::new(size * scale, family),
                        color: rgb(*color),
                        italics: *italic,
                        ..Default::default()
                    },
                );
                let galley = clip.layout_job(job);
                clip.with_clip_rect(r.intersect(area)).galley(r.min, galley, rgb(*color));
            }
        }
    }
}

fn paint_ellipse(painter: &Painter, r: Rect, fill: Color32) {
    let (c, rx, ry) = (r.center(), r.width() / 2.0, r.height() / 2.0);
    let points: Vec<Pos2> = (0..64)
        .map(|i| {
            let a = i as f32 / 64.0 * std::f32::consts::TAU;
            c + vec2(a.cos() * rx, a.sin() * ry)
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(points, fill, Stroke::NONE));
}

// ---- app -----------------------------------------------------------------------------------------
#[derive(Clone, Copy)]
enum Drag {
    Move { grab: Pos2, item_x: f32, item_y: f32 },
    Resize { grab: Pos2, w: f32, h: f32 },
}

struct App {
    deck: Deck,
    cur: usize,
    sel: Option<usize>,
    drag: Option<Drag>,
    /// Item whose text is being edited in place.
    editing: Option<usize>,
    undo: Vec<Deck>,
    redo: Vec<Deck>,
    path: Option<PathBuf>,
    dirty: bool,
    message: Option<String>,
    presenting: bool,
    has_bold: bool,
}

impl App {
    fn new(ctx: &egui::Context) -> App {
        let has_bold = install_fonts(ctx);
        App {
            deck: Deck::default(),
            cur: 0,
            sel: None,
            drag: None,
            editing: None,
            undo: Vec::new(),
            redo: Vec::new(),
            path: None,
            dirty: false,
            message: None,
            presenting: false,
            has_bold,
        }
    }

    fn snapshot(&mut self) {
        self.undo.push(self.deck.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn restore(&mut self, deck: Deck) {
        self.deck = deck;
        self.cur = self.cur.min(self.deck.slides.len() - 1);
        self.sel = None;
        self.editing = None;
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.deck.clone());
            self.restore(prev);
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.deck.clone());
            self.restore(next);
        }
    }

    fn slide_mut(&mut self) -> &mut Slide {
        let i = self.cur.min(self.deck.slides.len() - 1);
        &mut self.deck.slides[i]
    }

    fn new_item(&mut self, shape: Shape) {
        self.snapshot();
        let (w, h) = if matches!(shape, Shape::Text { .. }) { (360.0, 90.0) } else { (240.0, 160.0) };
        let item = Item { shape, x: 300.0, y: 190.0, w, h };
        let slide = self.slide_mut();
        slide.items.push(item);
        self.sel = Some(self.deck.slides[self.cur].items.len() - 1);
    }

    fn text_shape(&self) -> Shape {
        Shape::Text {
            text: "Text".into(),
            size: 36.0,
            bold: false,
            italic: false,
            color: self.deck.theme.palette().ink,
        }
    }

    fn remove_item(&mut self) {
        if let Some(i) = self.sel.take() {
            self.snapshot();
            let s = self.slide_mut();
            if i < s.items.len() {
                s.items.remove(i);
            }
            self.editing = None;
        }
    }

    // ---- files ---------------------------------------------------------------------------------
    fn new_deck(&mut self) {
        self.snapshot();
        self.deck = Deck::default();
        self.cur = 0;
        self.sel = None;
        self.path = None;
        self.dirty = false;
    }

    fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Wren presentation", &["json"]).pick_file() else { return };
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| Deck::from_json(&t)) {
            Ok(deck) => {
                self.snapshot();
                self.deck = deck;
                self.cur = 0;
                self.sel = None;
                self.path = Some(path);
                self.dirty = false;
                self.message = None;
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.path, as_new) {
            (Some(p), false) => p.clone(),
            _ => match rfd::FileDialog::new()
                .add_filter("Wren presentation", &["json"])
                .set_file_name("presentation.json")
                .save_file()
            {
                Some(p) => p,
                None => return,
            },
        };
        match std::fs::write(&path, self.deck.to_json()) {
            Ok(()) => {
                self.path = Some(path);
                self.dirty = false;
                self.message = Some("Saved.".into());
            }
            Err(e) => self.message = Some(format!("Could not save the file: {e}")),
        }
    }

    fn export_outline(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Text", &["txt"]).set_file_name("outline.txt").save_file()
        else {
            return;
        };
        self.message = Some(match std::fs::write(&path, self.deck.outline()) {
            Ok(()) => "Outline exported.".into(),
            Err(e) => format!("Could not export: {e}"),
        });
    }

    fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("Untitled".into(), |n| n.to_string_lossy().to_string());
        format!("{name}{} - Perch", if self.dirty { " •" } else { "" })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.presenting {
            self.present_ui(ctx);
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));
        self.keys(ctx);
        self.toolbar(ctx);
        self.thumbnails(ctx);
        egui::TopBottomPanel::bottom("notes").resizable(true).default_height(70.0).show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("Slide {} of {} · Notes", self.cur + 1, self.deck.slides.len())).small(),
            );
            let before = self.deck.slides[self.cur].notes.clone();
            let mut notes = before.clone();
            ui.add(
                egui::TextEdit::multiline(&mut notes)
                    .desired_width(f32::INFINITY)
                    .desired_rows(2)
                    .hint_text("Speaker notes"),
            );
            if notes != before {
                self.snapshot();
                self.slide_mut().notes = notes;
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| self.canvas(ui));
    }
}

impl App {
    fn keys(&mut self, ctx: &egui::Context) {
        let focused = ctx.memory(|m| m.focused().is_some());
        let (undo, redo_a, redo_b, save, open, present) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::Z),
                i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z),
                i.consume_key(Modifiers::COMMAND, Key::Y),
                i.consume_key(Modifiers::COMMAND, Key::S),
                i.consume_key(Modifiers::COMMAND, Key::O),
                i.consume_key(Modifiers::NONE, Key::F5),
            )
        });
        if !focused {
            if undo {
                self.undo();
            }
            if redo_a || redo_b {
                self.redo();
            }
        }
        if save {
            self.save(false);
        }
        if open {
            self.open();
        }
        if present {
            self.presenting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        if focused || self.editing.is_some() {
            return;
        }
        let presses: Vec<Key> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| if let egui::Event::Key { key, pressed: true, .. } = e { Some(*key) } else { None })
                .collect()
        });
        for key in presses {
            match key {
                Key::Delete | Key::Backspace => self.remove_item(),
                Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown => {
                    if let Some(i) = self.sel {
                        let (dx, dy) = match key {
                            Key::ArrowLeft => (-4.0, 0.0),
                            Key::ArrowRight => (4.0, 0.0),
                            Key::ArrowUp => (0.0, -4.0),
                            _ => (0.0, 4.0),
                        };
                        self.snapshot();
                        if let Some(it) = self.slide_mut().items.get_mut(i) {
                            it.x += dx;
                            it.y += dy;
                        }
                    } else if matches!(key, Key::ArrowLeft | Key::ArrowUp) {
                        self.cur = self.cur.saturating_sub(1);
                    } else {
                        self.cur = (self.cur + 1).min(self.deck.slides.len() - 1);
                    }
                }
                _ => {}
            }
        }
    }

    // Nested `if let` blocks read better here than let-chains, and keep the borrows short.
    #[allow(clippy::collapsible_if)]
    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("New").clicked() {
                    self.new_deck();
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
                if ui.button("Export outline").clicked() {
                    self.export_outline();
                }
                ui.separator();
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo")).clicked() {
                    self.undo();
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo")).clicked() {
                    self.redo();
                }
                ui.separator();
                for (label, layout) in
                    [("+ Title", Layout::Title), ("+ Content", Layout::Content), ("+ Blank", Layout::Blank)]
                {
                    if ui.button(label).clicked() {
                        self.snapshot();
                        self.cur = self.deck.add_slide(self.cur, layout);
                        self.sel = None;
                    }
                }
                if ui.button("Duplicate").clicked() {
                    self.snapshot();
                    if let Some(n) = self.deck.duplicate_slide(self.cur) {
                        self.cur = n;
                    }
                }
                if ui.button("Delete slide").clicked() {
                    self.snapshot();
                    if self.deck.remove_slide(self.cur) {
                        self.cur = self.cur.min(self.deck.slides.len() - 1);
                        self.sel = None;
                    }
                }
                if ui.button("Move up").clicked() && self.cur > 0 {
                    self.snapshot();
                    self.deck.move_slide(self.cur, self.cur - 1);
                    self.cur -= 1;
                }
                if ui.button("Move down").clicked() && self.cur + 1 < self.deck.slides.len() {
                    self.snapshot();
                    self.deck.move_slide(self.cur, self.cur + 1);
                    self.cur += 1;
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Text").clicked() {
                    let s = self.text_shape();
                    self.new_item(s);
                }
                if ui.button("Rectangle").clicked() {
                    let fill = self.deck.theme.palette().accent;
                    self.new_item(Shape::Rect { fill });
                }
                if ui.button("Ellipse").clicked() {
                    let fill = self.deck.theme.palette().accent;
                    self.new_item(Shape::Ellipse { fill });
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("Remove")).clicked() {
                    self.remove_item();
                }
                if ui.add_enabled(self.sel.is_some(), egui::Button::new("Bring to front")).clicked() {
                    if let Some(i) = self.sel {
                        self.snapshot();
                        self.sel = self.deck.bring_to_front(self.cur, i);
                    }
                }
                ui.separator();
                let mut theme = self.deck.theme;
                egui::ComboBox::from_id_salt("theme").selected_text(theme.name()).width(90.0).show_ui(ui, |ui| {
                    for t in Theme::ALL {
                        ui.selectable_value(&mut theme, t, t.name());
                    }
                });
                if theme != self.deck.theme {
                    self.snapshot();
                    self.deck.theme = theme;
                }
                self.item_props(ui);
                ui.separator();
                if ui.button("▶ Present (F5)").clicked() {
                    self.presenting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
            });
            ui.add_space(4.0);
        });
    }

    /// Size, bold, italic and colour controls for the selected item.
    fn item_props(&mut self, ui: &mut egui::Ui) {
        let Some(i) = self.sel else { return };
        let Some(item) = self.deck.slides[self.cur].items.get(i).cloned() else { return };
        let mut changed = item.clone();
        match &mut changed.shape {
            Shape::Text { size, bold, italic, color, .. } => {
                ui.add(egui::DragValue::new(size).range(6.0..=400.0).suffix(" pt"));
                ui.toggle_value(bold, egui::RichText::new("B").strong());
                ui.toggle_value(italic, egui::RichText::new("I").italics());
                ui.color_edit_button_srgb(color);
            }
            Shape::Rect { fill } | Shape::Ellipse { fill } => {
                ui.color_edit_button_srgb(fill);
            }
        }
        if changed != item {
            self.snapshot();
            if let Some(slot) = self.slide_mut().items.get_mut(i) {
                *slot = changed;
            }
        }
    }

    fn thumbnails(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("thumbs").exact_width(190.0).resizable(false).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let width = ui.available_width() - 8.0;
                let scale = width / WIDTH;
                for i in 0..self.deck.slides.len() {
                    let (rect, resp) = ui.allocate_exact_size(vec2(width, HEIGHT * scale), Sense::click());
                    paint_slide(ui.painter(), rect.min, scale, &self.deck.slides[i], self.deck.theme, self.has_bold);
                    let color = if i == self.cur {
                        Color32::from_rgb(0x4f, 0x46, 0xe5)
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke.color
                    };
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(if i == self.cur { 3.0_f32 } else { 1.0_f32 }, color),
                        StrokeKind::Outside,
                    );
                    ui.painter().text(
                        rect.left_bottom() + vec2(6.0, -4.0),
                        Align2::LEFT_BOTTOM,
                        (i + 1).to_string(),
                        FontId::proportional(12.0),
                        Color32::GRAY,
                    );
                    if resp.clicked() {
                        self.cur = i;
                        self.sel = None;
                        self.editing = None;
                    }
                    ui.add_space(10.0);
                }
            });
        });
    }

    // Nested `if let` blocks read better here than let-chains, and keep the borrows short.
    #[allow(clippy::collapsible_if)]
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap().shrink(20.0);
        let scale = (avail.width() / WIDTH).min(avail.height() / HEIGHT).max(0.05);
        let size = vec2(WIDTH, HEIGHT) * scale;
        let origin = avail.center() - size / 2.0;
        let area = Rect::from_min_size(origin, size);
        let resp = ui.allocate_rect(area, Sense::click_and_drag());
        let theme = self.deck.theme;
        let has_bold = self.has_bold;
        let accent = Color32::from_rgb(0x4f, 0x46, 0xe5);

        ui.painter().rect_stroke(area.expand(1.0), 0.0, Stroke::new(1.0_f32, Color32::GRAY), StrokeKind::Outside);
        paint_slide(ui.painter(), origin, scale, &self.deck.slides[self.cur], theme, has_bold);

        let item_rect = |it: &Item| Rect::from_min_size(origin + vec2(it.x, it.y) * scale, vec2(it.w, it.h) * scale);

        // selection frame and resize handle
        if let Some(it) = self.sel.and_then(|i| self.deck.slides[self.cur].items.get(i)) {
            let r = item_rect(it);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(2.0_f32, accent), StrokeKind::Outside);
            ui.painter().rect_filled(Rect::from_center_size(r.right_bottom(), vec2(HANDLE, HANDLE)), 3.0, accent);
        }

        // pointer: select, move, resize
        if resp.drag_started() {
            if let Some(p) = resp.interact_pointer_pos() {
                let on_handle = self
                    .sel
                    .and_then(|i| self.deck.slides[self.cur].items.get(i))
                    .map(|it| item_rect(it).right_bottom().distance(p) <= HANDLE);
                if on_handle == Some(true) {
                    let size = self.sel.and_then(|i| self.deck.slides[self.cur].items.get(i)).map(|it| (it.w, it.h));
                    if let Some((w, h)) = size {
                        self.snapshot();
                        self.drag = Some(Drag::Resize { grab: p, w, h });
                    }
                } else {
                    let hit = self.deck.slides[self.cur].items.iter().rposition(|it| item_rect(it).contains(p));
                    self.sel = hit;
                    if let Some(i) = hit {
                        let it = self.deck.slides[self.cur].items[i].clone();
                        self.snapshot();
                        self.drag = Some(Drag::Move { grab: p, item_x: it.x, item_y: it.y });
                    }
                }
            }
        }
        if let (Some(d), Some(p), Some(i)) = (self.drag, resp.interact_pointer_pos(), self.sel) {
            if let Some(it) = self.slide_mut().items.get_mut(i) {
                match d {
                    Drag::Move { grab, item_x, item_y } => {
                        it.x = item_x + (p.x - grab.x) / scale;
                        it.y = item_y + (p.y - grab.y) / scale;
                    }
                    Drag::Resize { grab, w, h } => {
                        it.w = (w + (p.x - grab.x) / scale).max(20.0);
                        it.h = (h + (p.y - grab.y) / scale).max(20.0);
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                self.sel = self.deck.slides[self.cur].items.iter().rposition(|it| item_rect(it).contains(p));
                self.editing = None;
            }
        }
        if resp.double_clicked() {
            if let Some(i) = self.sel.filter(|i| {
                matches!(self.deck.slides[self.cur].items.get(*i).map(|it| &it.shape), Some(Shape::Text { .. }))
            }) {
                self.editing = Some(i);
            }
        }

        // in-place text editing
        if let Some(i) = self.editing {
            let Some(item) = self.deck.slides[self.cur].items.get(i).cloned() else {
                self.editing = None;
                return;
            };
            if let Shape::Text { text, size, color, .. } = &item.shape {
                let mut buf = text.clone();
                let r = item_rect(&item);
                ui.painter().rect_filled(r, 0.0, rgb(theme.palette().background));
                let out = ui.put(
                    r,
                    egui::TextEdit::multiline(&mut buf)
                        .frame(false)
                        .font(FontId::proportional(size * scale))
                        .text_color(rgb(*color))
                        .desired_width(r.width()),
                );
                out.request_focus();
                if buf != *text {
                    self.snapshot();
                    if let Some(Shape::Text { text, .. }) = self.slide_mut().items.get_mut(i).map(|it| &mut it.shape) {
                        *text = buf;
                    }
                }
                if ui.input(|inp| inp.key_pressed(Key::Escape)) || out.lost_focus() {
                    self.editing = None;
                }
            }
        }
    }

    fn present_ui(&mut self, ctx: &egui::Context) {
        let (next, prev, esc) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowRight)
                    || i.key_pressed(Key::Space)
                    || i.key_pressed(Key::PageDown)
                    || i.pointer.primary_clicked(),
                i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::PageUp),
                i.key_pressed(Key::Escape),
            )
        });
        if next {
            self.cur = (self.cur + 1).min(self.deck.slides.len() - 1);
        }
        if prev {
            self.cur = self.cur.saturating_sub(1);
        }
        if esc {
            self.presenting = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::BLACK)).show(ctx, |ui| {
            let avail = ui.available_rect_before_wrap();
            let scale = (avail.width() / WIDTH).min(avail.height() / HEIGHT);
            let origin = avail.center() - vec2(WIDTH, HEIGHT) * scale / 2.0;
            paint_slide(ui.painter(), origin, scale, &self.deck.slides[self.cur], self.deck.theme, self.has_bold);
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Perch")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([640.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native("Perch", options, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))))
}
