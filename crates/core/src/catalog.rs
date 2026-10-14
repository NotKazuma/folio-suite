//! The Wren app suite. The list is data, so a later version can fetch it instead.

/// The GitHub account the suite's apps live under. The launcher downloads from nowhere else.
pub const ORG: &str = "NotKazuma";

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

/// The section an app is listed under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Office,
    Creative,
    PublishingPhoto,
    MotionWeb3d,
}

impl Category {
    pub const ALL: [Category; 4] =
        [Category::Office, Category::Creative, Category::PublishingPhoto, Category::MotionWeb3d];

    pub fn label(self) -> &'static str {
        match self {
            Category::Office => "Office",
            Category::Creative => "Creative",
            Category::PublishingPhoto => "Publishing and photo",
            Category::MotionWeb3d => "Video, audio, web and 3D",
        }
    }
}

/// One app in the suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct App {
    /// Machine name: repository, asset stem and install folder (`wren-pinion`).
    pub id: &'static str,
    /// Display name (`Pinion`).
    pub name: &'static str,
    /// What it is for, in a few words.
    pub tagline: &'static str,
    /// The tool it stands in for.
    pub replaces: &'static str,
    /// Brand colour (sRGB) for the app's tile.
    pub color: [u8; 3],
    pub stage: Stage,
    pub category: Category,
}

impl App {
    /// `owner/repo` on GitHub: every app has a repository of its own, named after its id.
    pub fn repo(&self) -> String {
        format!("{ORG}/{}", self.id)
    }

    pub fn releases_url(&self) -> String {
        format!("https://github.com/{}/releases", self.repo())
    }

    /// Two letters for the tile: `Pinion` becomes `Pi`.
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

use Category::{Creative, MotionWeb3d, Office, PublishingPhoto};
use Stage::{Alpha, InDevelopment};

const fn app(
    id: &'static str,
    name: &'static str,
    tagline: &'static str,
    replaces: &'static str,
    color: [u8; 3],
    stage: Stage,
    category: Category,
) -> App {
    App { id, name, tagline, replaces, color, stage, category }
}

/// The suite in display order: grouped by category, categories in [`Category::ALL`] order.
pub const SUITE: [App; 21] = [
    // Office
    app("wren-pinion", "Pinion", "Documents and writing", "Word", [0x25, 0x63, 0xeb], Alpha, Office),
    app("wren-trellis", "Trellis", "Spreadsheets and data", "Excel", [0x16, 0xa3, 0x4a], Alpha, Office),
    app("wren-perch", "Perch", "Presentations", "PowerPoint", [0xea, 0x58, 0x0c], Alpha, Office),
    app("wren-nest", "Nest", "Notes and notebooks", "OneNote", [0x7e, 0x22, 0xce], InDevelopment, Office),
    app("wren-dove", "Dove", "Mail and calendar", "Outlook", [0x02, 0x84, 0xc7], InDevelopment, Office),
    app("wren-heron", "Heron", "Diagrams and flowcharts", "Visio", [0x0d, 0x94, 0x88], InDevelopment, Office),
    app("wren-godwit", "Godwit", "Project plans and timelines", "Project", [0x15, 0x80, 0x3d], InDevelopment, Office),
    app("wren-jackdaw", "Jackdaw", "Databases and forms", "Access", [0x9f, 0x12, 0x39], InDevelopment, Office),
    app("wren-magpie", "Magpie", "Read, edit and sign PDFs", "Acrobat", [0xdc, 0x26, 0x26], InDevelopment, Office),
    // Creative
    app(
        "wren-osprey",
        "Osprey",
        "Image editing and retouching",
        "Photoshop",
        [0x7c, 0x3a, 0xed],
        InDevelopment,
        Creative,
    ),
    app("wren-egret", "Egret", "Vector illustration", "Illustrator", [0xdb, 0x27, 0x77], InDevelopment, Creative),
    app("wren-goldfinch", "Goldfinch", "Digital painting", "Fresco", [0xca, 0x8a, 0x04], InDevelopment, Creative),
    app(
        "wren-lorikeet",
        "Lorikeet",
        "Quick designs from templates",
        "Express",
        [0xea, 0x58, 0x0c],
        InDevelopment,
        Creative,
    ),
    app(
        "wren-tanager",
        "Tanager",
        "Interface design and prototyping",
        "XD",
        [0xc0, 0x26, 0xd3],
        InDevelopment,
        Creative,
    ),
    // Publishing and photo
    app(
        "wren-bower",
        "Bower",
        "Page layout for print and PDF",
        "InDesign",
        [0xbe, 0x18, 0x5d],
        InDevelopment,
        PublishingPhoto,
    ),
    app(
        "wren-peregrine",
        "Peregrine",
        "Photo library and RAW",
        "Lightroom",
        [0x1d, 0x4e, 0xd8],
        InDevelopment,
        PublishingPhoto,
    ),
    // Video, audio, web and 3D
    app("wren-starling", "Starling", "Video editing", "Premiere Pro", [0x08, 0x91, 0xb2], InDevelopment, MotionWeb3d),
    app("wren-warbler", "Warbler", "Recording and mixing", "Audition", [0xb4, 0x53, 0x09], InDevelopment, MotionWeb3d),
    app("wren-colibri", "Colibri", "Motion graphics", "After Effects", [0x4f, 0x46, 0xe5], InDevelopment, MotionWeb3d),
    app("wren-weaver", "Weaver", "Web page design", "Dreamweaver", [0x0f, 0x76, 0x6e], InDevelopment, MotionWeb3d),
    app("wren-condor", "Condor", "3D modelling and rendering", "Maya", [0x47, 0x55, 0x69], InDevelopment, MotionWeb3d),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_wren_prefixed_and_lowercase() {
        let mut ids: Vec<_> = SUITE.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), SUITE.len());
        assert!(SUITE.iter().all(|a| a.id == a.id.to_lowercase() && a.id.starts_with("wren-")));
    }

    #[test]
    fn every_app_has_a_repository_of_its_own() {
        let a = App::find("wren-pinion").unwrap();
        assert_eq!(a.repo(), "NotKazuma/wren-pinion");
        assert_eq!(a.releases_url(), "https://github.com/NotKazuma/wren-pinion/releases");
        for app in &SUITE {
            assert!(app.repo().ends_with(app.id), "{}", app.id);
        }
        let mut repos: Vec<_> = SUITE.iter().map(App::repo).collect();
        repos.sort();
        repos.dedup();
        assert_eq!(repos.len(), SUITE.len());
    }

    #[test]
    fn initials_are_two_letters() {
        assert_eq!(App::find("wren-pinion").unwrap().initials(), "Pi");
        assert_eq!(App::find("wren-peregrine").unwrap().initials(), "Pe");
        assert!(SUITE.iter().all(|a| a.initials().chars().count() == 2), "initials");
        assert!(App::find("nope").is_none());
    }

    #[test]
    fn display_name_matches_the_id() {
        for a in &SUITE {
            assert_eq!(format!("wren-{}", a.name.to_lowercase()), a.id);
        }
    }

    #[test]
    fn grouped_in_category_order_with_three_built_apps() {
        let order = |c: Category| Category::ALL.iter().position(|x| *x == c).unwrap();
        assert!(SUITE.windows(2).all(|w| order(w[0].category) <= order(w[1].category)), "grouped");
        assert!(Category::ALL.iter().all(|c| SUITE.iter().any(|a| a.category == *c)), "no empty section");
        assert_eq!(SUITE.iter().filter(|a| a.stage == Stage::Alpha).count(), 3);
        assert_eq!(SUITE.len(), 21);
    }
}
