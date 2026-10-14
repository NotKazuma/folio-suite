//! .docx bytes -> Document.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use folio_write_doc::{Block, Document, Kind};
use roxmltree::{Document as Xml, Node};
use zip::ZipArchive;

use crate::{Error, MAX_PART_BYTES};

fn part(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<String>, Error> {
    let Ok(file) = zip.by_name(name) else { return Ok(None) };
    let mut buf = Vec::new();
    file.take(MAX_PART_BYTES + 1).read_to_end(&mut buf).map_err(|_| Error::Malformed)?;
    if buf.len() as u64 > MAX_PART_BYTES {
        return Err(Error::TooLarge);
    }
    String::from_utf8(buf).map(Some).map_err(|_| Error::Malformed)
}

fn is(node: Node, local: &str) -> bool {
    node.is_element() && node.tag_name().name() == local
}

fn child<'a>(node: Node<'a, 'a>, local: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|c| is(*c, local))
}

fn attr<'a>(node: Node<'a, 'a>, local: &str) -> Option<&'a str> {
    node.attributes().find(|a| a.name() == local).map(|a| a.value())
}

/// numId -> is it a bullet list? (anything else counts as numbered)
fn numbering_kinds(xml: &str) -> HashMap<String, bool> {
    let Ok(doc) = Xml::parse(xml) else { return HashMap::new() };
    let mut abstract_bullet: HashMap<&str, bool> = HashMap::new();
    for a in doc.descendants().filter(|n| is(*n, "abstractNum")) {
        let Some(id) = attr(a, "abstractNumId") else { continue };
        let fmt = a
            .descendants()
            .find(|n| is(*n, "lvl") && attr(*n, "ilvl") == Some("0"))
            .and_then(|l| child(l, "numFmt"))
            .and_then(|f| attr(f, "val"));
        abstract_bullet.insert(id, fmt == Some("bullet"));
    }
    let mut out = HashMap::new();
    for n in doc.descendants().filter(|n| is(*n, "num")) {
        let (Some(num), Some(abs)) = (attr(n, "numId"), child(n, "abstractNumId").and_then(|a| attr(a, "val"))) else {
            continue;
        };
        if let Some(b) = abstract_bullet.get(abs) {
            out.insert(num.to_string(), *b);
        }
    }
    out
}

/// One run of text with its formatting.
struct Piece {
    text: String,
    bold: bool,
    italic: bool,
}

fn flag(rpr: Option<Node>, name: &str) -> bool {
    rpr.and_then(|r| child(r, name)).is_some_and(|n| !matches!(attr(n, "val"), Some("0" | "false" | "off")))
}

fn collect_pieces(p: Node, out: &mut Vec<Piece>) {
    for node in p.children().filter(|c| c.is_element()) {
        match node.tag_name().name() {
            "r" => {
                let rpr = child(node, "rPr");
                let (bold, italic) = (flag(rpr, "b"), flag(rpr, "i"));
                let mut text = String::new();
                for c in node.children().filter(|c| c.is_element()) {
                    match c.tag_name().name() {
                        "t" => text.push_str(c.text().unwrap_or("")),
                        "tab" => text.push('\t'),
                        "br" | "cr" => text.push(' '),
                        _ => {}
                    }
                }
                if !text.is_empty() {
                    out.push(Piece { text, bold, italic });
                }
            }
            // links, smart tags and the like wrap more runs
            "hyperlink" | "smartTag" | "sdt" | "sdtContent" | "ins" => collect_pieces(node, out),
            _ => {}
        }
    }
}

/// Merge adjacent pieces with the same formatting, then write the `**`/`*` markers.
fn to_marked_text(pieces: Vec<Piece>) -> String {
    let mut merged: Vec<Piece> = Vec::new();
    for p in pieces {
        match merged.last_mut() {
            Some(last) if last.bold == p.bold && last.italic == p.italic => last.text.push_str(&p.text),
            _ => merged.push(p),
        }
    }
    let mut out = String::new();
    for p in merged {
        // Markers must hug the words: keep surrounding spaces outside them.
        let trimmed = p.text.trim();
        if trimmed.is_empty() || (!p.bold && !p.italic) {
            out.push_str(&p.text);
            continue;
        }
        let lead = &p.text[..p.text.len() - p.text.trim_start().len()];
        let trail = &p.text[p.text.trim_end().len()..];
        let mark = match (p.bold, p.italic) {
            (true, true) => "***",
            (true, false) => "**",
            _ => "*",
        };
        out.push_str(&format!("{lead}{mark}{trimmed}{mark}{trail}"));
    }
    out
}

fn kind_of(p: Node, kinds: &HashMap<String, bool>) -> Kind {
    let ppr = child(p, "pPr");
    let style = ppr.and_then(|x| child(x, "pStyle")).and_then(|s| attr(s, "val")).unwrap_or("");
    let lower = style.to_ascii_lowercase().replace([' ', '-', '_'], "");
    if lower == "title" {
        return Kind::Heading(1);
    }
    if let Some(level) = lower.strip_prefix("heading").and_then(|n| n.parse::<u8>().ok()) {
        return Kind::Heading(level.clamp(1, 3));
    }
    if matches!(lower.as_str(), "quote" | "intensequote" | "blocktext") {
        return Kind::Quote;
    }
    let num_id = ppr.and_then(|x| child(x, "numPr")).and_then(|n| child(n, "numId")).and_then(|n| attr(n, "val"));
    match num_id {
        Some("0") | None => Kind::Paragraph,
        Some(id) => match kinds.get(id) {
            Some(true) => Kind::Bullet,
            Some(false) => Kind::Numbered,
            // a list style whose numbering we cannot read: bullets are the safe guess
            None => Kind::Bullet,
        },
    }
}

fn walk_body(node: Node, kinds: &HashMap<String, bool>, blocks: &mut Vec<Block>) {
    for c in node.children().filter(|c| c.is_element()) {
        match c.tag_name().name() {
            "p" => {
                let mut pieces = Vec::new();
                collect_pieces(c, &mut pieces);
                blocks.push(Block::new(kind_of(c, kinds), to_marked_text(pieces)));
            }
            // tables and content controls: keep their paragraphs, drop the grid
            "tbl" | "tr" | "tc" | "sdt" | "sdtContent" => walk_body(c, kinds, blocks),
            _ => {}
        }
    }
}

pub fn import(bytes: &[u8]) -> Result<Document, Error> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| Error::NotDocx)?;
    let main = part(&mut zip, "word/document.xml")?.ok_or(Error::NotDocx)?;
    let kinds = part(&mut zip, "word/numbering.xml")?.map(|x| numbering_kinds(&x)).unwrap_or_default();
    let xml = Xml::parse(&main).map_err(|_| Error::Malformed)?;
    let body = xml.descendants().find(|n| is(*n, "body")).ok_or(Error::Malformed)?;
    let mut blocks = Vec::new();
    walk_body(body, &kinds, &mut blocks);
    if blocks.is_empty() {
        return Ok(Document::default());
    }
    Ok(Document { blocks })
}
