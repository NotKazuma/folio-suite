//! The Folio app suite. The list is data, so a later version can fetch it instead.

/// The GitHub account the suite's releases live under. The launcher downloads from nowhere else.
pub const ORG: &str = "NotKazuma";

/// The repository holding the suite's releases. Every app publishes its files there, named
/// `<app id>-<version>-<platform>.<ext>`, until each app has a repository of its own.
pub const SUITE_REPO: &str = "folio-suite";

/// How far along an app is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    InDevelopment,
    Alpha,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::InDevelopment => "In development",
            Stage::Alpha => "Early alpha",
        }
    }
}

/// One app in the suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct App {
    /// Machine name: repository, asset stem and install folder (`folio-write`).
    pub id: &'static str,
    /// Display name (`Folio Write`).
    pub name: &'static str,
    /// What it is for, in a few words.
    pub tagline: &'static str,
    /// The tool it stands in for.
    pub replaces: &'static str,
    /// Brand colour (sRGB) for the app's tile.
    pub color: [u8; 3],
    pub stage: Stage,
}

impl App {
    /// `owner/repo` on GitHub that publishes this app.
    pub fn repo(&self) -> String {
        format!("{ORG}/{SUITE_REPO}")
    }

    pub fn releases_url(&self) -> String {
        format!("https://github.com/{}/releases", self.repo())
    }

    /// Two letters for the tile: `Folio Write` becomes `Wr`.
    pub fn initials(&self) -> String {
        let word = self.name.split_whitespace().last().unwrap_or(self.name);
        let mut chars = word.chars();
        let first = chars.next().map(|c| c.to_ascii_uppercase());
        let second = chars.next().map(|c| c.to_ascii_lowercase());
        first.into_iter().chain(second).collect()
    }

    pub fn find(id: &str) -> Option<&'static App> {
        SUITE.iter().find(|a| a.id == id)
    }
}

const fn app(
    id: &'static str,
    name: &'static str,
    tagline: &'static str,
    replaces: &'static str,
    color: [u8; 3],
    stage: Stage,
) -> App {
    App { id, name, tagline, replaces, color, stage }
}

/// The suite in display order.
pub const SUITE: [App; 8] = [
    app("folio-write", "Folio Write", "Documents and writing", "Word", [0x25, 0x63, 0xeb], Stage::Alpha),
    app("folio-sheet", "Folio Sheet", "Spreadsheets and data", "Excel", [0x16, 0xa3, 0x4a], Stage::Alpha),
    app("folio-slide", "Folio Slide", "Presentations", "PowerPoint", [0xea, 0x58, 0x0c], Stage::Alpha),
    app(
        "folio-photo",
        "Folio Photo",
        "Image editing and retouching",
        "Photoshop",
        [0x7c, 0x3a, 0xed],
        Stage::InDevelopment,
    ),
    app("folio-vector", "Folio Vector", "Vector illustration", "Illustrator", [0xdb, 0x27, 0x77], Stage::InDevelopment),
    app("folio-pdf", "Folio PDF", "Read, edit and sign PDFs", "Acrobat", [0xdc, 0x26, 0x26], Stage::InDevelopment),
    app("folio-video", "Folio Video", "Video editing", "Premiere Pro", [0x08, 0x91, 0xb2], Stage::InDevelopment),
    app("folio-audio", "Folio Audio", "Recording and mixing", "Audition", [0xca, 0x8a, 0x04], Stage::InDevelopment),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_lowercase() {
        let mut ids: Vec<_> = SUITE.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), SUITE.len());
        assert!(SUITE.iter().all(|a| a.id == a.id.to_lowercase() && a.id.starts_with("folio-")));
    }

    #[test]
    fn repo_and_initials() {
        let a = App::find("folio-write").unwrap();
        assert_eq!(a.repo(), "NotKazuma/folio-suite");
        assert_eq!(a.initials(), "Wr");
        assert_eq!(App::find("folio-pdf").unwrap().initials(), "Pd");
        assert!(App::find("nope").is_none());
    }
}
