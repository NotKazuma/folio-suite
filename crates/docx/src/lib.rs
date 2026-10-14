//! Reading and writing Word `.docx` files for Folio Write.
//!
//! Supported: paragraphs, Heading 1-3, bulleted and numbered lists, quotes, bold and italic.
//! Everything else in a foreign file (tables, pictures, headers) is flattened to its text or
//! skipped, never an error. Files from the outside are hostile: every part is size-capped.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod read;
mod write;

use std::fmt;

pub use read::import;
pub use write::export;

/// Largest uncompressed part we will read (guards against zip bombs).
pub const MAX_PART_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    /// Not a zip file, or a damaged one.
    NotDocx,
    /// A part is larger than [`MAX_PART_BYTES`].
    TooLarge,
    /// The main document part is missing or is not valid XML.
    Malformed,
    Write(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotDocx => write!(f, "This is not a Word (.docx) file."),
            Error::TooLarge => write!(f, "This document is too large to open."),
            Error::Malformed => write!(f, "The document's contents could not be read."),
            Error::Write(e) => write!(f, "Could not create the file ({e})."),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_write_doc::{Block, Document, Kind};

    fn sample() -> Document {
        Document {
            blocks: vec![
                Block::new(Kind::Heading(1), "Plan <&> \"quoted\""),
                Block::new(Kind::Paragraph, "Some **bold**, *italic* and plain text."),
                Block::new(Kind::Bullet, "first"),
                Block::new(Kind::Bullet, "second **b**"),
                Block::new(Kind::Numbered, "one"),
                Block::new(Kind::Numbered, "two"),
                Block::new(Kind::Quote, "a *wise* quote"),
                Block::new(Kind::Heading(3), "Small heading"),
            ],
        }
    }

    #[test]
    fn export_then_import_round_trips() {
        let doc = sample();
        let bytes = export(&doc).unwrap();
        assert_eq!(&bytes[..2], b"PK");
        assert_eq!(import(&bytes).unwrap(), doc);
    }

    #[test]
    fn empty_document_round_trips() {
        let doc = Document::default();
        assert_eq!(import(&export(&doc).unwrap()).unwrap(), doc);
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(matches!(import(b"hello"), Err(Error::NotDocx)));
        assert!(matches!(import(&[]), Err(Error::NotDocx)));
        let mut half = export(&sample()).unwrap();
        half.truncate(half.len() / 2);
        assert!(import(&half).is_err());
    }
}
