//! What the window shows, separated from how it is drawn so it can be tested.

use wren_core::{App, Platform, Status, Version, platform::pick_asset};
use wren_native::{Event, Latest, state};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Job {
    Idle,
    Working(u8),
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub app: &'static App,
    pub installed: Option<String>,
    /// `None` until the first lookup finishes.
    pub latest: Option<Latest>,
    pub job: Job,
}

pub struct Model {
    pub apps: Vec<AppState>,
    platform: Platform,
}

impl Model {
    pub fn new(platform: Platform) -> Model {
        let installed = state::installed();
        Model {
            platform,
            apps: wren_core::SUITE
                .iter()
                .map(|app| AppState { app, installed: installed.get(app.id).cloned(), latest: None, job: Job::Idle })
                .collect(),
        }
    }

    pub fn any_working(&self) -> bool {
        self.apps.iter().any(|a| matches!(a.job, Job::Working(_)))
    }

    /// Applies backend events; returns whether anything changed.
    pub fn apply(&mut self, events: Vec<Event>) -> bool {
        let changed = !events.is_empty();
        for e in events {
            let id = match &e {
                Event::Latest { id, .. }
                | Event::Progress { id, .. }
                | Event::Installed { id, .. }
                | Event::Failed { id, .. } => *id,
            };
            let Some(s) = self.apps.iter_mut().find(|a| a.app.id == id) else { continue };
            match e {
                Event::Latest { latest, .. } => s.latest = Some(latest),
                Event::Progress { percent, .. } => s.job = Job::Working(percent),
                Event::Installed { version, .. } => {
                    s.installed = Some(version);
                    s.job = Job::Idle;
                }
                Event::Failed { error, .. } => s.job = Job::Failed(error),
            }
        }
        changed
    }

    pub fn status(&self, s: &AppState) -> Status {
        let installed = s.installed.as_deref().and_then(Version::parse);
        let (latest, has_build) = match &s.latest {
            None => return Status::Unknown,
            Some(Latest::Error(_)) => (None, false),
            Some(Latest::None) => (Some(None), false),
            Some(Latest::Release(r)) => {
                (Some(Version::parse(&r.tag)), pick_asset(&r.assets, self.platform, s.app.id).is_some())
            }
        };
        match latest {
            None => Status::Unknown,
            Some(None) if matches!(s.latest, Some(Latest::None)) => Status::Unreleased,
            Some(None) => Status::Unknown, // a tag we cannot read as a version
            Some(Some(v)) => Status::of(installed.as_ref(), Some(Some(&v)), has_build),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wren_core::{Arch, Os, Release, release::Asset};

    fn model() -> Model {
        Model::new(Platform { os: Os::Windows, arch: Arch::X64 })
    }

    fn release(tag: &str, files: &[&str]) -> Latest {
        Latest::Release(Release {
            tag: tag.into(),
            notes: String::new(),
            page_url: String::new(),
            assets: files
                .iter()
                .map(|n| Asset { name: (*n).into(), url: "https://github.com/NotKazuma/wren-pinion".into(), size: 1 })
                .collect(),
        })
    }

    #[test]
    fn events_drive_status() {
        let mut m = model();
        assert_eq!(m.status(&m.apps[0].clone()), Status::Unknown);
        m.apply(vec![Event::Latest { id: "wren-pinion", latest: release("v1.0.0", &["wren-pinion-1.0.0-x64.msi"]) }]);
        assert_eq!(m.status(&m.apps[0].clone()), Status::NotInstalled);
        m.apply(vec![Event::Progress { id: "wren-pinion", percent: 40 }]);
        assert!(m.any_working());
        m.apply(vec![Event::Installed { id: "wren-pinion", version: "v1.0.0".into() }]);
        assert_eq!(m.status(&m.apps[0].clone()), Status::Current);
        assert!(!m.any_working());
    }

    #[test]
    fn unreleased_and_no_build() {
        let mut m = model();
        m.apply(vec![
            Event::Latest { id: "wren-pinion", latest: Latest::None },
            Event::Latest { id: "wren-trellis", latest: release("v1.0.0", &["wren-trellis-1.0.0-linux.AppImage"]) },
        ]);
        assert_eq!(m.status(&m.apps[0].clone()), Status::Unreleased);
        assert_eq!(m.status(&m.apps[1].clone()), Status::NoBuild);
    }
}
