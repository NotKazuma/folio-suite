//! Document -> .docx bytes.

use std::io::{Cursor, Write};

use wren_pinion_doc::{Document, Kind, spans};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::Error;

const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

/// numbering.xml ids: 1 = bullets, 2 = decimal.
const BULLET_NUM: u32 = 1;
const DECIMAL_NUM: u32 = 2;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn paragraph(kind: Kind, text: &str) -> String {
    let mut ppr = String::new();
    match kind {
        Kind::Heading(n) => ppr.push_str(&format!(r#"<w:pStyle w:val="Heading{}"/>"#, n.clamp(1, 3))),
        Kind::Quote => ppr.push_str(r#"<w:pStyle w:val="Quote"/>"#),
        Kind::Bullet => ppr.push_str(&format!(
            r#"<w:pStyle w:val="ListParagraph"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="{BULLET_NUM}"/></w:numPr>"#
        )),
        Kind::Numbered => ppr.push_str(&format!(
            r#"<w:pStyle w:val="ListParagraph"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="{DECIMAL_NUM}"/></w:numPr>"#
        )),
        Kind::Paragraph => {}
    }
    let mut runs = String::new();
    for s in spans(text).into_iter().filter(|s| !s.marker) {
        let mut rpr = String::new();
        if s.bold {
            rpr.push_str("<w:b/>");
        }
        if s.italic {
            rpr.push_str("<w:i/>");
        }
        let rpr = if rpr.is_empty() { String::new() } else { format!("<w:rPr>{rpr}</w:rPr>") };
        runs.push_str(&format!(r#"<w:r>{rpr}<w:t xml:space="preserve">{}</w:t></w:r>"#, esc(&s.text)));
    }
    format!("<w:p><w:pPr>{ppr}</w:pPr>{runs}</w:p>")
}

fn document_xml(doc: &Document) -> String {
    let body: String = doc.blocks.iter().map(|b| paragraph(b.kind, &b.text)).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {NS}><w:body>{body}</w:body></w:document>"#
    )
}

fn style(id: &str, name: &str, based: Option<&str>, ppr: &str, rpr: &str) -> String {
    let based = based.map_or(String::new(), |b| format!(r#"<w:basedOn w:val="{b}"/>"#));
    format!(
        r#"<w:style w:type="paragraph" w:styleId="{id}"><w:name w:val="{name}"/>{based}<w:qFormat/><w:pPr>{ppr}</w:pPr><w:rPr>{rpr}</w:rPr></w:style>"#
    )
}

fn styles_xml() -> String {
    let heading = |n: u8, size: u32| {
        style(
            &format!("Heading{n}"),
            &format!("heading {n}"),
            Some("Normal"),
            &format!(r#"<w:keepNext/><w:spacing w:before="240" w:after="80"/><w:outlineLvl w:val="{}"/>"#, n - 1),
            &format!(r#"<w:b/><w:sz w:val="{size}"/>"#),
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles {NS}><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/><w:sz w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120" w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>{}{}{}{}{}</w:styles>"#,
        heading(1, 40),
        heading(2, 32),
        heading(3, 26),
        style("ListParagraph", "List Paragraph", Some("Normal"), r#"<w:ind w:left="720"/>"#, ""),
        style("Quote", "Quote", Some("Normal"), r#"<w:ind w:left="720"/>"#, "<w:i/>"),
    )
}

fn numbering_xml() -> String {
    let level = |fmt: &str, text: &str| {
        format!(
            r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="{fmt}"/><w:lvlText w:val="{text}"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl>"#
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering {NS}><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="singleLevel"/>{}</w:abstractNum><w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="singleLevel"/>{}</w:abstractNum><w:num w:numId="{BULLET_NUM}"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="{DECIMAL_NUM}"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#,
        level("bullet", "\u{2022}"),
        level("decimal", "%1."),
    )
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/></Relationships>"#;

pub fn export(doc: &Document) -> Result<Vec<u8>, Error> {
    let werr = |e: &dyn std::fmt::Display| Error::Write(e.to_string());
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, body) in [
        ("[Content_Types].xml", CONTENT_TYPES.to_string()),
        ("_rels/.rels", ROOT_RELS.to_string()),
        ("word/_rels/document.xml.rels", DOC_RELS.to_string()),
        ("word/document.xml", document_xml(doc)),
        ("word/styles.xml", styles_xml()),
        ("word/numbering.xml", numbering_xml()),
    ] {
        zip.start_file(name, opts).map_err(|e| werr(&e))?;
        zip.write_all(body.as_bytes()).map_err(|e| werr(&e))?;
    }
    Ok(zip.finish().map_err(|e| werr(&e))?.into_inner())
}
