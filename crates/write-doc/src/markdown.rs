//! Reading and writing Markdown: headings, bullets, numbered items, quotes, paragraphs.
//! Inline `**` / `*` markers pass through untouched because they are already the document's text.

use crate::{Block, Document, Kind};

pub fn parse(source: &str) -> Document {
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::new(Kind::Paragraph, paragraph.join(" ")));
            paragraph.clear();
        }
    };
    for line in source.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        if let Some(block) = block_line(line) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(block);
        } else {
            paragraph.push(line.trim_start());
        }
    }
    flush(&mut paragraph, &mut blocks);
    if blocks.is_empty() {
        return Document::default();
    }
    Document { blocks }
}

/// A line that is a block by itself: heading, list item or quote.
fn block_line(line: &str) -> Option<Block> {
    for (prefix, level) in [("### ", 3), ("## ", 2), ("# ", 1)] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(Block::new(Kind::Heading(level), rest.trim()));
        }
    }
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")).or_else(|| line.strip_prefix("+ "))
    {
        return Some(Block::new(Kind::Bullet, rest.trim()));
    }
    if let Some(rest) = line.strip_prefix("> ") {
        return Some(Block::new(Kind::Quote, rest.trim()));
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(rest) = line[digits..].strip_prefix(". ")
    {
        return Some(Block::new(Kind::Numbered, rest.trim()));
    }
    None
}

pub fn write(doc: &Document) -> String {
    let mut out = String::new();
    let mut number = 0;
    let mut prev: Option<Kind> = None;
    for block in &doc.blocks {
        if let Some(p) = prev {
            // list items of the same kind stay together; everything else gets a blank line
            let tight = p == block.kind && matches!(p, Kind::Bullet | Kind::Numbered);
            out.push_str(if tight { "\n" } else { "\n\n" });
        }
        number = if block.kind == Kind::Numbered { number + 1 } else { 0 };
        match block.kind {
            Kind::Paragraph => out.push_str(&block.text),
            Kind::Heading(level) => {
                out.push_str(&"#".repeat(usize::from(level.clamp(1, 3))));
                out.push(' ');
                out.push_str(&block.text);
            }
            Kind::Bullet => {
                out.push_str("- ");
                out.push_str(&block.text);
            }
            Kind::Numbered => {
                out.push_str(&format!("{number}. {}", block.text));
            }
            Kind::Quote => {
                out.push_str("> ");
                out.push_str(&block.text);
            }
        }
        prev = Some(block.kind);
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_block_kind() {
        let d =
            parse("# Title\n\nSome **bold** text\nsecond line\n\n- one\n- two\n\n1. first\n2. second\n\n> quoted\n");
        let kinds: Vec<_> = d.blocks.iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Kind::Heading(1),
                Kind::Paragraph,
                Kind::Bullet,
                Kind::Bullet,
                Kind::Numbered,
                Kind::Numbered,
                Kind::Quote
            ]
        );
        assert_eq!(d.blocks[1].text, "Some **bold** text second line");
    }

    #[test]
    fn write_then_parse_is_stable() {
        let d = Document {
            blocks: vec![
                Block::new(Kind::Heading(2), "Plan"),
                Block::new(Kind::Paragraph, "Intro with *italic*."),
                Block::new(Kind::Bullet, "a"),
                Block::new(Kind::Bullet, "b"),
                Block::new(Kind::Numbered, "x"),
                Block::new(Kind::Numbered, "y"),
                Block::new(Kind::Quote, "wise words"),
            ],
        };
        let md = write(&d);
        assert_eq!(parse(&md), d);
        assert_eq!(write(&parse(&md)), md);
    }

    #[test]
    fn empty_and_odd_input_never_panic() {
        assert_eq!(parse(""), Document::default());
        assert_eq!(parse("\n\n  \n"), Document::default());
        assert_eq!(parse("#nospace\n* *not a list*").blocks.len(), 2);
        assert_eq!(parse("99999999999999999999. big").blocks[0].kind, Kind::Numbered);
    }
}
