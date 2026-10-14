//! Downloading a release file, proving it is what the release published, and opening it.

use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use wren_core::checksum::parse_sums;
use wren_core::{ORG, Release, release::Asset};

use crate::{github, state};

/// Largest file we will accept.
pub const MAX_BYTES: u64 = 1_500_000_000;

#[derive(Debug)]
pub enum Error {
    Refused(String),
    Network(String),
    TooLarge,
    Checksum,
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refused(h) => write!(f, "Refusing to download from {h}."),
            Error::Network(e) => write!(f, "Download failed ({e})."),
            Error::TooLarge => write!(f, "File is larger than the allowed limit."),
            Error::Checksum => write!(f, "Checksum mismatch: the file was not kept."),
            Error::Io(e) => write!(f, "Could not save the file ({e})."),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// https only; github.com URLs must sit under our org; GitHub's file CDN is allowed.
pub fn check_url(url: &str) -> Result<(), Error> {
    let rest = url.strip_prefix("https://").ok_or_else(|| Error::Refused(url.to_string()))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.to_ascii_lowercase();
    let own = host == "github.com" && path.to_ascii_lowercase().starts_with(&format!("{}/", ORG.to_ascii_lowercase()));
    let cdn = matches!(host.as_str(), "objects.githubusercontent.com" | "release-assets.githubusercontent.com");
    if own || cdn { Ok(()) } else { Err(Error::Refused(host)) }
}

/// Downloads `asset` into the downloads folder, hashing as it streams. When the release
/// publishes a checksum for it the file must match; on any failure nothing is left behind.
pub fn fetch_verified(release: &Release, asset: &Asset, progress: &dyn Fn(u8)) -> Result<PathBuf, Error> {
    if Path::new(&asset.name).file_name().and_then(|n| n.to_str()) != Some(asset.name.as_str()) {
        return Err(Error::Refused("an unsafe file name".into()));
    }
    check_url(&asset.url)?;
    let expected = expected_digest(release, &asset.name)?;

    let dir = state::data_dir().join("downloads");
    fs::create_dir_all(&dir)?;
    let dest = dir.join(&asset.name);
    let res = github::agent().get(&asset.url).call().map_err(|e| Error::Network(e.to_string()))?;
    check_url(res.get_url())?; // the final address after redirects
    let total: u64 = res.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if total > MAX_BYTES {
        return Err(Error::TooLarge);
    }

    let result = (|| {
        let mut reader = res.into_reader();
        let mut out = File::create(&dest)?;
        let mut hasher = Sha256::new();
        let (mut buf, mut got, mut last) = (vec![0u8; 64 * 1024], 0u64, 0u8);
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            got += n as u64;
            if got > MAX_BYTES {
                return Err(Error::TooLarge);
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            if let Some(pct) = (got * 100).checked_div(total).map(|p| p.min(100) as u8)
                && pct != last
            {
                last = pct;
                progress(pct);
            }
        }
        out.flush()?;
        match expected {
            Some(want) if hex(&hasher.finalize()) != want => Err(Error::Checksum),
            _ => Ok(()),
        }
    })();
    match result {
        Ok(()) => Ok(dest),
        Err(e) => {
            let _ = fs::remove_file(&dest);
            Err(e)
        }
    }
}

fn expected_digest(release: &Release, name: &str) -> Result<Option<String>, Error> {
    let Some(sums) = release.checksum_asset() else { return Ok(None) };
    check_url(&sums.url)?;
    let res = github::agent().get(&sums.url).call().map_err(|e| Error::Network(e.to_string()))?;
    let text = github::read_capped(res.into_reader(), 1_000_000).unwrap_or_default();
    Ok(parse_sums(&text).remove(name))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hands the verified file to the operating system; the user finishes the install there.
pub fn open_installer(path: &Path) -> std::io::Result<()> {
    use std::process::Command;
    if cfg!(target_os = "windows") {
        // `start` resolves .msi/.exe through the shell's file association.
        Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn().map(|_| ())
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(path).spawn().map(|_| ())
    } else {
        Command::new("xdg-open").arg(path).spawn().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_org_and_github_cdn() {
        assert!(check_url("https://github.com/NotKazuma/wren-pinion/releases/download/v1/a.msi").is_ok());
        assert!(check_url("https://objects.githubusercontent.com/x").is_ok());
        assert!(check_url("https://github.com/evil/app/releases/download/v1/a.msi").is_err());
        assert!(check_url("http://github.com/NotKazuma/a").is_err());
        assert!(check_url("https://example.com/a.msi").is_err());
        assert!(check_url("https://github.com.evil.io/NotKazuma/a").is_err());
    }

    #[test]
    fn hex_encodes() {
        assert_eq!(hex(&[0, 15, 255]), "000fff");
    }
}
