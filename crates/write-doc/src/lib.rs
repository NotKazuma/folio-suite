//! Pinion's document model: no windows, no files, just text and structure.
//!
//! A document is a list of blocks. Each block has a kind (paragraph, heading, list item, quote)
//! and its text, where `**bold**` and `*italic*` markers stay in the text so editing is plain
//! text editing. [`spans`] turns a block's text into styled pieces for drawing; Markdown reads
//! and writes the same markers, so a saved file is an ordinary `.md`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod find;
mod history;
mod markdown;
mod spans;

pub use find::{Match, find_all, replace_all, replace_one};
pub use history::History;
pub use spans::{Span, plain_text, spans, toggle_marker};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Paragraph,
    /// Levels 1 to 3.
    Heading(u8),
    Bullet,
    Numbered,
    Quote,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Paragraph,
        Kind::Heading(1),
        Kind::Heading(2),
        Kind::Heading(3),
        Kind::Bullet,
        Kind::Numbered,
        Kind::Quote,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Paragraph => "Paragraph",
            Kind::Heading(1) => "Heading 1",
            Kind::Heading(2) => "Heading 2",
            Kind::Heading(_) => "Heading 3",
            Kind::Bullet => "Bulleted list",
            Kind::Numbered => "Numbered list",
            Kind::Quote => "Quote",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: Kind,
    pub text: String,
}

impl Block {
    pub fn new(kind: Kind, text: impl Into<String>) -> Block {
        Block { kind, text: text.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

impl Default for Document {
    fn default() -> Self {
        Document { blocks: vec![Block::new(Kind::Paragraph, "")] }
    }
}

/// Where a block's text splits when Enter is pressed, in characters.
fn byte_at(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(i, _)| i)
}

impl Document {
    pub fn from_markdown(source: &str) -> Document {
        markdown::parse(source)
    }

    pub fn to_markdown(&self) -> String {
        markdown::write(self)
    }

    /// The text with formatting markers removed, blocks separated by blank lines.
    pub fn to_plain_text(&self) -> String {
        self.blocks.iter().map(|b| plain_text(&b.text)).collect::<Vec<_>>().join("\n\n")
    }

    pub fn word_count(&self) -> usize {
        self.blocks.iter().map(|b| plain_text(&b.text).split_whitespace().count()).sum()
    }

    /// Splits block `i` at character offset `at` (Enter). The new block follows the old one; a
    /// list item or heading continues as a list item / paragraph. Returns the new block's index.
    pub fn split(&mut self, i: usize, at: usize) -> usize {
        let Some(block) = self.blocks.get_mut(i) else { return i };
        let cut = byte_at(&block.text, at);
        let tail = block.text.split_off(cut);
        let next_kind = match block.kind {
            Kind::Heading(_) => Kind::Paragraph,
            other => other,
        };
        self.blocks.insert(i + 1, Block::new(next_kind, tail));
        i + 1
    }

    /// Joins block `i` onto the previous block (Backspace at the start). Returns the join point
    /// as `(block index, character offset)`, or `None` when there is nothing to join.
    pub fn join_with_previous(&mut self, i: usize) -> Option<(usize, usize)> {
        if i == 0 || i >= self.blocks.len() {
            return None;
        }
        let removed = self.blocks.remove(i);
        let prev = self.blocks.get_mut(i - 1)?;
        let at = prev.text.chars().count();
        prev.text.push_str(&removed.text);
        Some((i - 1, at))
    }

    pub fn remove_empty_tail(&mut self) {
        while self.blocks.len() > 1 && self.blocks.last().is_some_and(|b| b.text.is_empty()) {
            self.blocks.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_join_round_trip() {
        let mut d = Document { blocks: vec![Block::new(Kind::Heading(1), "Hello world")] };
        let n = d.split(0, 5);
        assert_eq!(n, 1);
        assert_eq!(d.blocks[0].text, "Hello");
        assert_eq!(d.blocks[1], Block::new(Kind::Paragraph, " world"));
        assert_eq!(d.join_with_previous(1), Some((0, 5)));
        assert_eq!(d.blocks, vec![Block::new(Kind::Heading(1), "Hello world")]);
        assert_eq!(d.join_with_previous(0), None);
    }

    #[test]
    fn split_counts_characters_not_bytes() {
        let mut d = Document { blocks: vec![Block::new(Kind::Paragraph, "héllo wörld")] };
        d.split(0, 3);
        assert_eq!(d.blocks[0].text, "hél");
        assert_eq!(d.blocks[1].text, "lo wörld");
        // out-of-range offsets clamp instead of panicking
        d.split(1, 999);
        assert_eq!(d.blocks[2].text, "");
    }

    #[test]
    fn lists_continue_and_counts_ignore_markers() {
        let mut d = Document { blocks: vec![Block::new(Kind::Bullet, "one **two** three")] };
        d.split(0, 3);
        assert_eq!(d.blocks[1].kind, Kind::Bullet);
        assert_eq!(d.word_count(), 3);
        // the split left a leading space on the second block, and markers are not counted
        assert_eq!(d.to_plain_text(), "one\n\n two three");
    }
}
