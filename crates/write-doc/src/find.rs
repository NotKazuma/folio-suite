//! Find and replace over a document's block text (offsets are in characters).

use crate::Document;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub block: usize,
    pub start: usize,
    pub end: usize,
}

fn fold(c: char, case_sensitive: bool) -> char {
    if case_sensitive { c } else { c.to_lowercase().next().unwrap_or(c) }
}

/// Every non-overlapping match of `query`, in reading order. An empty query matches nothing.
pub fn find_all(doc: &Document, query: &str, case_sensitive: bool) -> Vec<Match> {
    let q: Vec<char> = query.chars().map(|c| fold(c, case_sensitive)).collect();
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    for (block, b) in doc.blocks.iter().enumerate() {
        let text: Vec<char> = b.text.chars().map(|c| fold(c, case_sensitive)).collect();
        let mut i = 0;
        while i + q.len() <= text.len() {
            if text[i..i + q.len()] == q[..] {
                out.push(Match { block, start: i, end: i + q.len() });
                i += q.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Replaces every match; returns how many were replaced.
pub fn replace_all(doc: &mut Document, query: &str, with: &str, case_sensitive: bool) -> usize {
    let matches = find_all(doc, query, case_sensitive);
    let count = matches.len();
    // Work from the end of each block so earlier offsets stay valid.
    for m in matches.into_iter().rev() {
        replace_range(doc, m, with);
    }
    count
}

/// Replaces one match.
pub fn replace_one(doc: &mut Document, m: Match, with: &str) {
    replace_range(doc, m, with);
}

fn replace_range(doc: &mut Document, m: Match, with: &str) {
    let Some(block) = doc.blocks.get_mut(m.block) else { return };
    let chars: Vec<char> = block.text.chars().collect();
    if m.start > m.end || m.end > chars.len() {
        return;
    }
    let mut out: String = chars[..m.start].iter().collect();
    out.push_str(with);
    out.extend(&chars[m.end..]);
    block.text = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, Kind};

    fn doc() -> Document {
        Document {
            blocks: vec![
                Block::new(Kind::Paragraph, "The cat sat. The CAT ran."),
                Block::new(Kind::Bullet, "no match here"),
                Block::new(Kind::Paragraph, "cat"),
            ],
        }
    }

    #[test]
    fn finds_in_reading_order() {
        let m = find_all(&doc(), "cat", false);
        assert_eq!(m.len(), 3);
        assert_eq!((m[0].block, m[0].start, m[0].end), (0, 4, 7));
        assert_eq!(m[2].block, 2);
        assert_eq!(find_all(&doc(), "cat", true).len(), 2);
        assert!(find_all(&doc(), "", false).is_empty());
    }

    #[test]
    fn replaces_everywhere_with_a_different_length() {
        let mut d = doc();
        assert_eq!(replace_all(&mut d, "cat", "wombat", false), 3);
        assert_eq!(d.blocks[0].text, "The wombat sat. The wombat ran.");
        assert_eq!(d.blocks[2].text, "wombat");
        assert_eq!(replace_all(&mut d, "wombat", "", false), 3);
        assert_eq!(d.blocks[2].text, "");
    }

    #[test]
    fn unicode_offsets_are_characters() {
        let mut d = Document { blocks: vec![Block::new(Kind::Paragraph, "héllo wörld wörld")] };
        let m = find_all(&d, "wörld", true);
        assert_eq!((m[0].start, m[0].end), (6, 11));
        replace_one(&mut d, m[1], "x");
        assert_eq!(d.blocks[0].text, "héllo wörld x");
    }
}
