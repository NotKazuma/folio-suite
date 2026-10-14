//! Folio native: the launcher's contact with the outside world (desktop only).
//!
//! HTTP to GitHub, verified downloads, the record of what is installed, and handing an
//! installer to the operating system. The UI never blocks on any of it: it asks the [`Backend`]
//! to do something and later drains [`Event`]s.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod download;
pub mod github;
pub mod state;

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use folio_core::{App, Platform, Release, platform::pick_asset};

/// What a lookup of an app's latest release found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Latest {
    Release(Release),
    /// The app has never published a release.
    None,
    /// The lookup failed; the text is safe to show.
    Error(String),
}

#[derive(Clone, Debug)]
pub enum Event {
    Latest { id: &'static str, latest: Latest },
    Progress { id: &'static str, percent: u8 },
    Installed { id: &'static str, version: String },
    Failed { id: &'static str, error: String },
}

pub struct Backend {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    cache: Arc<Mutex<github::Cache>>,
    platform: Platform,
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend {
    pub fn new() -> Backend {
        let (tx, rx) = mpsc::channel();
        Backend { tx, rx, cache: Arc::default(), platform: Platform::current() }
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Looks up every app's latest release in the background.
    pub fn refresh_all(&self, apps: &'static [App]) {
        for app in apps {
            self.refresh(app);
        }
    }

    pub fn refresh(&self, app: &'static App) {
        let (tx, cache) = (self.tx.clone(), Arc::clone(&self.cache));
        std::thread::spawn(move || {
            let latest = github::latest(&cache, &app.repo());
            let _ = tx.send(Event::Latest { id: app.id, latest });
        });
    }

    /// Downloads, verifies and opens the installer for `app`'s latest release.
    pub fn install(&self, app: &'static App) {
        let (tx, cache, platform) = (self.tx.clone(), Arc::clone(&self.cache), self.platform);
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let release = match github::latest(&cache, &app.repo()) {
                    Latest::Release(r) => r,
                    Latest::None => return Err("No release has been published yet.".into()),
                    Latest::Error(e) => return Err(e),
                };
                let asset =
                    pick_asset(&release.assets, platform).ok_or("This release has no build for your computer.")?;
                let progress = |percent| {
                    let _ = tx.send(Event::Progress { id: app.id, percent });
                };
                let file = download::fetch_verified(&release, asset, &progress).map_err(|e| e.to_string())?;
                download::open_installer(&file).map_err(|e| e.to_string())?;
                state::record_install(app.id, &release.tag, true).map_err(|e| e.to_string())?;
                Ok(release.tag)
            })();
            let _ = tx.send(match result {
                Ok(version) => Event::Installed { id: app.id, version },
                Err(error) => Event::Failed { id: app.id, error },
            });
        });
    }

    /// Everything that happened since the last call.
    pub fn drain(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }
}
