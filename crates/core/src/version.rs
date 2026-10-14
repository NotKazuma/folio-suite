//! Version numbers as people write them on release tags: `v1.2.3`, `1.2`, `1.0.0-beta.2`.

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Pre-release label (`beta.2`); a release outranks its own pre-releases.
    pub pre: Option<String>,
}

impl Version {
    /// Parses leniently; `None` when the text does not start with a number.
    pub fn parse(text: &str) -> Option<Version> {
        let t = text.trim().trim_start_matches(['v', 'V']);
        let (core, pre) = match t.split_once('-') {
            Some((c, p)) => (c, Some(p.split('+').next().unwrap_or(p).to_string())),
            None => (t.split('+').next().unwrap_or(t), None),
        };
        let mut nums = core.split('.').map(|p| p.parse::<u64>().ok());
        let major = nums.next()??;
        let minor = nums.next().unwrap_or(Some(0))?;
        let patch = nums.next().unwrap_or(Some(0))?;
        Some(Version { major, minor, patch, pre: pre.filter(|p| !p.is_empty()) })
    }

    /// Is `self` a newer version than `other`?
    pub fn is_newer_than(&self, other: &Version) -> bool {
        self > other
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch)).then_with(|| {
            match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            }
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(p) = &self.pre {
            write!(f, "-{p}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn compares_numerically_not_as_text() {
        assert!(v("v1.10.0") > v("1.9.9"));
        assert_eq!(v("1.2"), v("1.2.0"));
    }

    #[test]
    fn release_outranks_prerelease() {
        assert!(v("1.0.0") > v("1.0.0-beta.1"));
        assert!(v("1.0.0-beta.2") > v("1.0.0-beta.1"));
    }

    #[test]
    fn rejects_junk() {
        assert!(Version::parse("latest").is_none());
        assert!(Version::parse("").is_none());
        assert!(Version::parse("1.x.3").is_none());
        assert_eq!(v("v2.0.1+build5").to_string(), "2.0.1");
    }
}
