//! The launcher's own files: what it installed and where downloads go.
//! A missing or corrupt state file means "nothing recorded", never a crash.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// `%LOCALAPPDATA%\Wren` (or the platform's equivalent). `WREN_HOME` overrides it.
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("WREN_HOME") {
        return PathBuf::from(p);
    }
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("Wren")
}

fn state_file() -> PathBuf {
    data_dir().join("state.json")
}

/// app id -> installed version tag.
pub fn installed() -> BTreeMap<String, String> {
    read_installed(&state_file())
}

pub fn record_install(id: &str, version: &str, verified: bool) -> io::Result<()> {
    write_install(&state_file(), id, version, verified)
}

fn read_installed(file: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("installed").cloned())
        .and_then(|v| v.as_object().cloned())
        .map(|m| {
            m.into_iter()
                .filter_map(|(k, v)| v.get("version").and_then(Value::as_str).map(|s| (k, s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn write_install(file: &Path, id: &str, version: &str, verified: bool) -> io::Result<()> {
    let mut all = read_installed(file);
    all.insert(id.to_string(), version.to_string());
    let body: serde_json::Map<String, Value> = all
        .into_iter()
        .map(|(k, v)| {
            let was_verified = verified || k != id;
            (k, json!({ "version": v, "verified": was_verified }))
        })
        .collect();
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&json!({ "installed": body }))?)?;
    fs::rename(tmp, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wren-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn records_and_reads_back() {
        let file = scratch("state").join("state.json");
        assert!(read_installed(&file).is_empty());
        write_install(&file, "wren-pinion", "v1.0.0", true).unwrap();
        write_install(&file, "wren-trellis", "v0.2.0", true).unwrap();
        write_install(&file, "wren-pinion", "v1.1.0", true).unwrap();
        let all = read_installed(&file);
        assert_eq!(all.get("wren-pinion").map(String::as_str), Some("v1.1.0"));
        assert_eq!(all.get("wren-trellis").map(String::as_str), Some("v0.2.0"));
        let _ = fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn corrupt_file_means_nothing_recorded() {
        let dir = scratch("corrupt");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("state.json");
        fs::write(&file, "{ not json").unwrap();
        assert!(read_installed(&file).is_empty());
        // and a later install still works
        write_install(&file, "wren-magpie", "v1.0.0", false).unwrap();
        assert_eq!(read_installed(&file).len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
