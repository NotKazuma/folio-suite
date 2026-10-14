//! Undo and redo by snapshots. Typing in bursts collapses into one step; structural changes
//! (splitting, joining, changing a block's kind) always get their own.

use crate::Document;

const MAX_STEPS: usize = 200;
/// Edits closer together than this belong to the same typing burst.
const BURST_MS: u64 = 900;

#[derive(Clone, Debug)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
    /// The document as of the last observation.
    baseline: Document,
    last_change_ms: u64,
}

impl History {
    pub fn new(doc: &Document) -> History {
        History { undo: Vec::new(), redo: Vec::new(), baseline: doc.clone(), last_change_ms: 0 }
    }

    /// Call once per frame with the current document. Returns whether it had changed.
    pub fn observe(&mut self, doc: &Document, now_ms: u64) -> bool {
        if *doc == self.baseline {
            return false;
        }
        let structural = doc.blocks.len() != self.baseline.blocks.len()
            || doc.blocks.iter().zip(&self.baseline.blocks).any(|(a, b)| a.kind != b.kind);
        let new_burst = now_ms.saturating_sub(self.last_change_ms) > BURST_MS;
        if structural || new_burst || self.undo.is_empty() {
            self.undo.push(self.baseline.clone());
            if self.undo.len() > MAX_STEPS {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.baseline = doc.clone();
        self.last_change_ms = now_ms;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The document to go back to, if any. `current` becomes available to redo.
    pub fn undo(&mut self, current: &Document) -> Option<Document> {
        let prev = self.undo.pop()?;
        self.redo.push(current.clone());
        self.baseline = prev.clone();
        self.last_change_ms = 0;
        Some(prev)
    }

    pub fn redo(&mut self, current: &Document) -> Option<Document> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        self.baseline = next.clone();
        self.last_change_ms = 0;
        Some(next)
    }

    /// Forget everything (a different document was opened).
    pub fn reset(&mut self, doc: &Document) {
        *self = History::new(doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, Kind};

    fn doc(text: &str) -> Document {
        Document { blocks: vec![Block::new(Kind::Paragraph, text)] }
    }

    #[test]
    fn a_typing_burst_is_one_step() {
        let mut h = History::new(&doc(""));
        let mut now = 10_000;
        for (i, t) in ["h", "he", "hel", "hell", "hello"].iter().enumerate() {
            now += 100 * (i as u64 + 1) / 2;
            assert!(h.observe(&doc(t), now));
        }
        assert!(!h.observe(&doc("hello"), now + 1));
        assert_eq!(h.undo(&doc("hello")), Some(doc("")));
        assert!(!h.can_undo());
    }

    #[test]
    fn a_pause_starts_a_new_step() {
        let mut h = History::new(&doc(""));
        h.observe(&doc("one"), 10_000);
        h.observe(&doc("one two"), 20_000);
        assert_eq!(h.undo(&doc("one two")), Some(doc("one")));
        assert_eq!(h.undo(&doc("one")), Some(doc("")));
    }

    #[test]
    fn redo_follows_undo_and_new_edits_clear_it() {
        let mut h = History::new(&doc(""));
        h.observe(&doc("a"), 10_000);
        let back = h.undo(&doc("a")).unwrap();
        assert!(h.can_redo());
        assert_eq!(h.redo(&back), Some(doc("a")));
        h.undo(&doc("a"));
        h.observe(&doc("b"), 30_000);
        assert!(!h.can_redo());
    }

    #[test]
    fn structure_changes_always_get_a_step() {
        let mut h = History::new(&doc("x"));
        h.observe(
            &Document { blocks: vec![Block::new(Kind::Paragraph, "x"), Block::new(Kind::Paragraph, "")] },
            10_000,
        );
        h.observe(&Document { blocks: vec![Block::new(Kind::Paragraph, "x"), Block::new(Kind::Bullet, "")] }, 10_050);
        assert_eq!(h.undo.len(), 2);
    }

    #[test]
    fn history_is_capped() {
        let mut h = History::new(&doc(""));
        for i in 0..(MAX_STEPS as u64 + 50) {
            h.observe(&doc(&format!("{i}")), 10_000 + i * 5_000);
        }
        assert_eq!(h.undo.len(), MAX_STEPS);
    }
}
