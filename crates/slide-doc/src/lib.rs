//! Folio Slide's deck model: slides made of positioned items on a 960 x 540 canvas.
//! Pure data and operations; drawing and windows live in the app.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use serde::{Deserialize, Serialize};

pub const WIDTH: f32 = 960.0;
pub const HEIGHT: f32 = 540.0;
/// Decks larger than this are refused when loading (a hostile file cannot make us allocate).
pub const MAX_SLIDES: usize = 500;
pub const MAX_ITEMS: usize = 300;
pub const MAX_TEXT: usize = 20_000;
pub const MAX_FILE_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Indigo,
    Night,
    Sunrise,
    Forest,
}

/// A theme's colours as `[r, g, b]`.
pub struct Palette {
    pub background: [u8; 3],
    pub ink: [u8; 3],
    pub accent: [u8; 3],
}

impl Theme {
    pub const ALL: [Theme; 4] = [Theme::Indigo, Theme::Night, Theme::Sunrise, Theme::Forest];

    pub fn name(self) -> &'static str {
        match self {
            Theme::Indigo => "Indigo",
            Theme::Night => "Night",
            Theme::Sunrise => "Sunrise",
            Theme::Forest => "Forest",
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Theme::Indigo => Palette { background: [255, 255, 255], ink: [27, 27, 47], accent: [79, 70, 229] },
            Theme::Night => Palette { background: [20, 20, 28], ink: [242, 242, 246], accent: [34, 211, 238] },
            Theme::Sunrise => Palette { background: [255, 247, 237], ink: [59, 29, 11], accent: [234, 88, 12] },
            Theme::Forest => Palette { background: [240, 253, 244], ink: [16, 48, 28], accent: [22, 163, 74] },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    Text { text: String, size: f32, bold: bool, italic: bool, color: [u8; 3] },
    Rect { fill: [u8; 3] },
    Ellipse { fill: [u8; 3] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub shape: Shape,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    pub items: Vec<Item>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Title,
    Content,
    Blank,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Deck {
    pub theme: Theme,
    pub slides: Vec<Slide>,
}

/// A text item at `[x, y, w, h]`.
fn text_item(text: &str, [x, y, w, h]: [f32; 4], size: f32, bold: bool, color: [u8; 3]) -> Item {
    Item { shape: Shape::Text { text: text.into(), size, bold, italic: false, color }, x, y, w, h }
}

impl Slide {
    pub fn from_layout(layout: Layout, theme: Theme) -> Slide {
        let ink = theme.palette().ink;
        let items = match layout {
            Layout::Title => vec![
                text_item("Presentation title", [80.0, 180.0, 800.0, 100.0], 60.0, true, ink),
                text_item("Subtitle", [80.0, 300.0, 800.0, 60.0], 28.0, false, ink),
            ],
            Layout::Content => vec![
                text_item("Slide title", [60.0, 40.0, 840.0, 80.0], 44.0, true, ink),
                text_item("First point\nSecond point\nThird point", [60.0, 150.0, 840.0, 320.0], 30.0, false, ink),
            ],
            Layout::Blank => vec![],
        };
        Slide { items, notes: String::new() }
    }
}

impl Default for Deck {
    fn default() -> Self {
        Deck {
            theme: Theme::Indigo,
            slides: vec![
                Slide::from_layout(Layout::Title, Theme::Indigo),
                Slide::from_layout(Layout::Content, Theme::Indigo),
            ],
        }
    }
}

impl Deck {
    /// Insert a new slide after `after` (or at the end). Returns its index.
    pub fn add_slide(&mut self, after: usize, layout: Layout) -> usize {
        let at = (after + 1).min(self.slides.len());
        self.slides.insert(at, Slide::from_layout(layout, self.theme));
        at
    }

    pub fn duplicate_slide(&mut self, i: usize) -> Option<usize> {
        let copy = self.slides.get(i)?.clone();
        self.slides.insert(i + 1, copy);
        Some(i + 1)
    }

    /// Removes slide `i`; a deck always keeps at least one slide.
    pub fn remove_slide(&mut self, i: usize) -> bool {
        if self.slides.len() <= 1 || i >= self.slides.len() {
            return false;
        }
        self.slides.remove(i);
        true
    }

    pub fn move_slide(&mut self, from: usize, to: usize) {
        if from == to || from >= self.slides.len() || to >= self.slides.len() {
            return;
        }
        let s = self.slides.remove(from);
        self.slides.insert(to, s);
    }

    /// Bring item `i` of slide `s` to the front (last drawn). Returns its new index.
    pub fn bring_to_front(&mut self, s: usize, i: usize) -> Option<usize> {
        let items = &mut self.slides.get_mut(s)?.items;
        if i >= items.len() {
            return None;
        }
        let it = items.remove(i);
        items.push(it);
        Some(items.len() - 1)
    }

    /// Slide titles and text as plain outline text, one slide per paragraph block.
    pub fn outline(&self) -> String {
        let mut out = String::new();
        for (n, s) in self.slides.iter().enumerate() {
            out.push_str(&format!("Slide {}\n", n + 1));
            for it in &s.items {
                if let Shape::Text { text, .. } = &it.shape {
                    for line in text.lines().filter(|l| !l.trim().is_empty()) {
                        out.push_str(&format!("  {}\n", line.trim()));
                    }
                }
            }
            out.push('\n');
        }
        out
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Loads a deck from untrusted JSON: oversized files are refused and every number and
    /// string is clamped to something safe to draw.
    pub fn from_json(text: &str) -> Result<Deck, String> {
        if text.len() > MAX_FILE_BYTES {
            return Err("This presentation is too large to open.".into());
        }
        let mut deck: Deck =
            serde_json::from_str(text).map_err(|_| "This is not a Folio Slide presentation.".to_string())?;
        if deck.slides.is_empty() || deck.slides.len() > MAX_SLIDES {
            return Err("This presentation has an unusable number of slides.".into());
        }
        for s in &mut deck.slides {
            s.items.truncate(MAX_ITEMS);
            s.notes = s.notes.chars().take(MAX_TEXT).collect();
            for it in &mut s.items {
                it.x = finite(it.x, -2000.0, 2000.0, 0.0);
                it.y = finite(it.y, -2000.0, 2000.0, 0.0);
                it.w = finite(it.w, 8.0, 4000.0, 100.0);
                it.h = finite(it.h, 8.0, 4000.0, 50.0);
                if let Shape::Text { text, size, .. } = &mut it.shape {
                    *size = finite(*size, 6.0, 400.0, 32.0);
                    *text = text.chars().take(MAX_TEXT).collect();
                }
            }
        }
        Ok(deck)
    }
}

fn finite(v: f32, lo: f32, hi: f32, fallback: f32) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let mut d = Deck::default();
        d.slides[0].notes = "say hello".into();
        d.slides[0].items.push(Item { shape: Shape::Rect { fill: [1, 2, 3] }, x: 10.0, y: 20.0, w: 30.0, h: 40.0 });
        assert_eq!(Deck::from_json(&d.to_json()).unwrap(), d);
    }

    #[test]
    fn hostile_json_is_refused_or_clamped() {
        assert!(Deck::from_json("not json").is_err());
        assert!(Deck::from_json(r#"{"theme":"Indigo","slides":[]}"#).is_err());
        let d = Deck::from_json(
            r#"{"theme":"Night","slides":[{"items":[{"shape":{"Text":{"text":"hi","size":-5.0,"bold":false,"italic":false,"color":[0,0,0]}},"x":1e30,"y":-1e30,"w":0.0,"h":1e9}]}]}"#,
        )
        .unwrap();
        let it = &d.slides[0].items[0];
        assert_eq!((it.x, it.y, it.w, it.h), (2000.0, -2000.0, 8.0, 4000.0));
        assert!(matches!(&it.shape, Shape::Text { size, .. } if *size == 6.0));
        assert!(Deck::from_json(&" ".repeat(MAX_FILE_BYTES + 1)).is_err());
    }

    #[test]
    fn slide_operations() {
        let mut d = Deck::default();
        let n = d.add_slide(0, Layout::Blank);
        assert_eq!(n, 1);
        assert_eq!(d.slides.len(), 3);
        assert!(d.slides[1].items.is_empty());
        assert_eq!(d.duplicate_slide(0), Some(1));
        assert_eq!(d.slides[0], d.slides[1]);
        d.move_slide(0, 3);
        d.move_slide(0, 99); // out of range: ignored
        assert_eq!(d.slides.len(), 4);
        while d.remove_slide(0) {}
        assert_eq!(d.slides.len(), 1);
        assert!(!d.remove_slide(0));
    }

    #[test]
    fn z_order_and_outline() {
        let mut d = Deck::default();
        assert_eq!(d.bring_to_front(0, 0), Some(1));
        assert_eq!(d.bring_to_front(0, 9), None);
        let o = d.outline();
        assert!(o.contains("Slide 1") && o.contains("Presentation title") && o.contains("Slide 2"));
    }

    #[test]
    fn every_theme_has_contrast() {
        for t in Theme::ALL {
            let p = t.palette();
            let lum = |c: [u8; 3]| 0.2126 * f32::from(c[0]) + 0.7152 * f32::from(c[1]) + 0.0722 * f32::from(c[2]);
            assert!((lum(p.background) - lum(p.ink)).abs() > 120.0, "{}", t.name());
        }
    }
}
