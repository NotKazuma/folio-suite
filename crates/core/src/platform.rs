//! Which computer we are on, and which release file suits it.

use crate::release::Asset;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    X64,
    Arm64,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    /// The platform this binary was built for.
    pub fn current() -> Platform {
        let os = if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        };
        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Arm64
        } else {
            Arch::Other
        };
        Platform { os, arch }
    }

    /// File endings this OS can install, best first.
    fn kinds(self) -> &'static [&'static str] {
        match self.os {
            Os::Windows => &[".msi", ".exe"],
            Os::MacOs => &[".dmg", ".pkg"],
            Os::Linux => &[".appimage", ".deb", ".rpm", ".tar.gz"],
            Os::Other => &[],
        }
    }

    fn os_words(self) -> &'static [&'static str] {
        match self.os {
            Os::Windows => &["windows", "win", "win64"],
            Os::MacOs => &["macos", "mac", "osx", "darwin"],
            Os::Linux => &["linux"],
            Os::Other => &[],
        }
    }
}

const X64_WORDS: &[&str] = &["x64", "x86_64", "amd64"];
const ARM_WORDS: &[&str] = &["arm64", "aarch64"];

fn arch_words(a: Arch) -> &'static [&'static str] {
    match a {
        Arch::X64 => X64_WORDS,
        Arch::Arm64 => ARM_WORDS,
        Arch::Other => &[],
    }
}

/// `folio-write-1.0.0-x64.exe` belongs to `folio-write`; `folio-writer-...` does not.
fn is_own_file(lower_name: &str, stem: &str) -> bool {
    lower_name[stem.len()..].starts_with(['-', '_', '.'])
}

fn words(name: &str) -> Vec<String> {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_any(ws: &[String], list: &[&str]) -> bool {
    ws.iter().any(|w| list.contains(&w.as_str()))
}

/// The best file for `platform` belonging to the app whose files start with `stem`, or `None`.
/// Checksum and signature files are never candidates, and a file built for the other CPU family
/// is skipped.
pub fn pick_asset<'a>(assets: &'a [Asset], platform: Platform, stem: &str) -> Option<&'a Asset> {
    let kinds = platform.kinds();
    let mine = arch_words(platform.arch);
    let other: Vec<&str> =
        [X64_WORDS, ARM_WORDS].iter().filter(|set| **set != mine).flat_map(|set| set.iter().copied()).collect();
    let mut best: Option<(&Asset, i32)> = None;
    for asset in assets {
        let lower = asset.name.to_lowercase();
        if !lower.starts_with(&stem.to_lowercase()) || !is_own_file(&lower, stem) {
            continue;
        }
        let Some(kind) = kinds.iter().position(|k| lower.ends_with(k)) else { continue };
        if lower.contains("sha256") || lower.ends_with(".sig") || lower.ends_with(".asc") {
            continue;
        }
        let ws = words(&asset.name);
        let is_mine = has_any(&ws, mine);
        if has_any(&ws, &other) && !is_mine {
            continue;
        }
        let mut score = 10 - kind as i32;
        if is_mine {
            score += 20;
        }
        if has_any(&ws, platform.os_words()) {
            score += 5;
        }
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((asset, score));
        }
    }
    best.map(|(a, _)| a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets(names: &[&str]) -> Vec<Asset> {
        names
            .iter()
            .map(|n| Asset { name: (*n).into(), url: format!("https://github.com/folio-suite/x/{n}"), size: 1 })
            .collect()
    }

    const WIN_X64: Platform = Platform { os: Os::Windows, arch: Arch::X64 };

    #[test]
    fn picks_by_os_and_cpu() {
        let a = assets(&[
            "folio-write-1.0.0-windows-x64.msi",
            "folio-write-1.0.0-windows-arm64.msi",
            "folio-write-1.0.0-macos-arm64.dmg",
            "folio-write-1.0.0-linux-x86_64.AppImage",
            "SHA256SUMS.txt",
        ]);
        assert_eq!(pick_asset(&a, WIN_X64, "folio-write").unwrap().name, "folio-write-1.0.0-windows-x64.msi");
        let arm = Platform { os: Os::Windows, arch: Arch::Arm64 };
        assert_eq!(pick_asset(&a, arm, "folio-write").unwrap().name, "folio-write-1.0.0-windows-arm64.msi");
        let mac_intel = Platform { os: Os::MacOs, arch: Arch::X64 };
        assert!(pick_asset(&a, mac_intel, "folio-write").is_none());
        let linux = Platform { os: Os::Linux, arch: Arch::X64 };
        assert!(pick_asset(&a, linux, "folio-write").unwrap().name.ends_with(".AppImage"));
    }

    #[test]
    fn installer_beats_bare_exe_and_checksums_never_win() {
        let a = assets(&["folio-x64.exe", "folio-x64.msi", "folio-x64.msi.sha256"]);
        assert_eq!(pick_asset(&a, WIN_X64, "folio").unwrap().name, "folio-x64.msi");
    }

    #[test]
    fn each_app_only_sees_its_own_files() {
        let a = assets(&[
            "folio-write-1.0.0-windows-x64.exe",
            "folio-sheet-1.0.0-windows-x64.exe",
            "folio-writer-1.0.0-windows-x64.exe",
        ]);
        assert_eq!(pick_asset(&a, WIN_X64, "folio-write").unwrap().name, "folio-write-1.0.0-windows-x64.exe");
        assert_eq!(pick_asset(&a, WIN_X64, "folio-sheet").unwrap().name, "folio-sheet-1.0.0-windows-x64.exe");
        assert!(pick_asset(&a, WIN_X64, "folio-slide").is_none());
        assert!(pick_asset(&a, WIN_X64, "folio-wri").is_none());
    }

    #[test]
    fn unknown_os_gets_nothing() {
        let a = assets(&["folio.msi"]);
        assert!(pick_asset(&a, Platform { os: Os::Other, arch: Arch::X64 }, "folio").is_none());
    }
}
