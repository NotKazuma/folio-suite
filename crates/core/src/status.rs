//! What the launcher shows for each app.

use crate::version::Version;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not installed, and a build for this computer exists.
    NotInstalled,
    /// Installed and a newer release exists.
    UpdateAvailable,
    /// Installed and up to date.
    Current,
    /// The app has no published release yet.
    Unreleased,
    /// A release exists but has no build for this computer.
    NoBuild,
    /// The latest release could not be read.
    Unknown,
}

impl Status {
    /// `latest`: `None` means the lookup failed; `Some(None)` means no release exists.
    pub fn of(installed: Option<&Version>, latest: Option<Option<&Version>>, has_build: bool) -> Status {
        match latest {
            None => Status::Unknown,
            Some(None) => Status::Unreleased,
            Some(Some(_)) if !has_build => Status::NoBuild,
            Some(Some(l)) => match installed {
                None => Status::NotInstalled,
                Some(i) if l.is_newer_than(i) => Status::UpdateAvailable,
                Some(_) => Status::Current,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn every_state() {
        let (a, b) = (v("1.0.0"), v("1.1.0"));
        assert_eq!(Status::of(None, Some(Some(&a)), true), Status::NotInstalled);
        assert_eq!(Status::of(Some(&a), Some(Some(&b)), true), Status::UpdateAvailable);
        assert_eq!(Status::of(Some(&b), Some(Some(&b)), true), Status::Current);
        assert_eq!(Status::of(None, Some(None), false), Status::Unreleased);
        assert_eq!(Status::of(None, Some(Some(&a)), false), Status::NoBuild);
        assert_eq!(Status::of(Some(&a), None, true), Status::Unknown);
    }
}
