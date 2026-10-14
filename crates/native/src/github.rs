//! Asking GitHub for an app's latest release, with ETag caching to spare the rate limit.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use folio_core::Release;

use crate::Latest;

#[derive(Default)]
pub struct Cache {
    entries: HashMap<String, (Option<String>, Latest)>,
}

pub(crate) fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent("folio-launcher")
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .build()
}

pub fn latest(cache: &Mutex<Cache>, repo: &str) -> Latest {
    let previous = cache.lock().ok().and_then(|c| c.entries.get(repo).cloned());
    let mut req = agent()
        .get(&format!("https://api.github.com/repos/{repo}/releases/latest"))
        .set("Accept", "application/vnd.github+json");
    if let Some((Some(etag), _)) = &previous {
        req = req.set("If-None-Match", etag);
    }
    let outcome = match req.call() {
        Ok(res) => {
            let etag = res.header("ETag").map(str::to_string);
            let text = read_capped(res.into_reader(), folio_core::release::MAX_JSON_BYTES);
            match text.map(|t| Release::parse(&t)) {
                Some(Ok(r)) => Some((etag, Latest::Release(r))),
                _ => Some((None, Latest::Error("GitHub sent an answer Folio could not read.".into()))),
            }
        }
        Err(ureq::Error::Status(304, _)) => previous.clone(),
        Err(ureq::Error::Status(404, _)) => Some((None, Latest::None)),
        Err(ureq::Error::Status(403 | 429, _)) => {
            Some((None, Latest::Error("GitHub rate limit reached; try again later.".into())))
        }
        Err(ureq::Error::Status(code, _)) => Some((None, Latest::Error(format!("GitHub answered {code}.")))),
        Err(_) => None,
    };
    match outcome {
        Some(entry) => {
            let latest = entry.1.clone();
            if let Ok(mut c) = cache.lock() {
                c.entries.insert(repo.to_string(), entry);
            }
            latest
        }
        // Network trouble: show the last answer we had, or say we could not reach GitHub.
        None => previous
            .map(|(_, l)| l)
            .unwrap_or_else(|| Latest::Error("Could not reach GitHub. Check your connection.".into())),
    }
}

/// Reads at most `cap` bytes as UTF-8; `None` if it is longer or not text.
pub(crate) fn read_capped(reader: impl std::io::Read, cap: usize) -> Option<String> {
    use std::io::Read;
    let mut buf = Vec::new();
    reader.take(cap as u64 + 1).read_to_end(&mut buf).ok()?;
    if buf.len() > cap {
        return None;
    }
    String::from_utf8(buf).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_and_decodes() {
        assert_eq!(read_capped(&b"hello"[..], 10).as_deref(), Some("hello"));
        assert!(read_capped(&b"hello world"[..], 5).is_none());
        assert!(read_capped(&[0xff, 0xfe][..], 10).is_none());
    }
}
