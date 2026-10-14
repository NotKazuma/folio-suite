//! Asking GitHub for an app's latest release, with ETag caching to spare the rate limit.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use wren_core::Release;

use crate::Latest;

/// What we remember about a repository: the ETag GitHub gave and the answer it went with.
type Entry = (Option<String>, Latest);

#[derive(Default)]
pub struct Cache {
    entries: HashMap<String, Entry>,
}

pub(crate) fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent("wren-launcher")
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
    // ureq reports 4xx/5xx as errors but 304 as a success with an empty body, so both paths
    // end in the same place: a status, an optional ETag and an optional body.
    let outcome = match req.call() {
        Ok(res) => {
            let (status, etag) = (res.status(), res.header("ETag").map(str::to_string));
            let body = read_capped(res.into_reader(), wren_core::release::MAX_JSON_BYTES);
            Some(interpret(status, etag, body, previous.as_ref()))
        }
        Err(ureq::Error::Status(code, _)) => Some(interpret(code, None, None, previous.as_ref())),
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

/// Turns one HTTP answer into what we remember. Pure, so every status can be tested.
fn interpret(status: u16, etag: Option<String>, body: Option<String>, previous: Option<&Entry>) -> Entry {
    match status {
        // Not modified: nothing new, keep what we had (or admit we have nothing to show).
        304 => previous
            .cloned()
            .unwrap_or_else(|| (None, Latest::Error("GitHub sent an answer Wren could not read.".into()))),
        404 => (None, Latest::None),
        403 | 429 => (None, Latest::Error("GitHub rate limit reached; try again later.".into())),
        200..=299 => match body.map(|t| Release::parse(&t)) {
            Some(Ok(r)) => (etag, Latest::Release(r)),
            _ => (None, Latest::Error("GitHub sent an answer Wren could not read.".into())),
        },
        code => (None, Latest::Error(format!("GitHub answered {code}."))),
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

    const RELEASE: &str = r#"{"tag_name":"v1.0.0","html_url":"https://github.com/x","assets":[]}"#;

    fn release_entry() -> Entry {
        interpret(200, Some("\"abc\"".into()), Some(RELEASE.into()), None)
    }

    #[test]
    fn caps_and_decodes() {
        assert_eq!(read_capped(&b"hello"[..], 10).as_deref(), Some("hello"));
        assert!(read_capped(&b"hello world"[..], 5).is_none());
        assert!(read_capped(&[0xff, 0xfe][..], 10).is_none());
    }

    #[test]
    fn ok_answer_becomes_a_release_and_remembers_the_etag() {
        let (etag, latest) = release_entry();
        assert_eq!(etag.as_deref(), Some("\"abc\""));
        assert!(matches!(latest, Latest::Release(r) if r.tag == "v1.0.0"));
    }

    /// The bug the real release exposed: a second lookup gets 304 with an empty body and must
    /// keep the earlier answer instead of failing to parse nothing.
    #[test]
    fn not_modified_keeps_the_previous_answer() {
        let prev = release_entry();
        let again = interpret(304, None, Some(String::new()), Some(&prev));
        assert_eq!(again, prev);
        // with nothing remembered, a 304 is an error rather than a panic
        assert!(matches!(interpret(304, None, None, None).1, Latest::Error(_)));
    }

    #[test]
    fn other_statuses() {
        assert_eq!(interpret(404, None, None, None).1, Latest::None);
        assert!(matches!(interpret(403, None, None, None).1, Latest::Error(e) if e.contains("rate limit")));
        assert!(matches!(interpret(429, None, None, None).1, Latest::Error(e) if e.contains("rate limit")));
        assert!(matches!(interpret(502, None, None, None).1, Latest::Error(e) if e.contains("502")));
        assert!(matches!(interpret(200, None, Some("not json".into()), None).1, Latest::Error(_)));
        assert!(matches!(interpret(200, None, None, None).1, Latest::Error(_)));
    }
}
