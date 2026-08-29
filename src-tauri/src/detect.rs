//! Auto-detection of game install locations.
//!
//! Scanned (in priority order):
//!   * Epic Games library folders on **all** drives: `<drive>\Epic Games`,
//!     `<drive>\Program Files\Epic Games`, `<drive>\Program Files (x86)\Epic Games`;
//!   * Steam `steamapps\common` roots: registry `InstallPath` + defaults +
//!     **every additional library from `steamapps\libraryfolders.vdf`**;
//!   * "official"/manual installs: `Program Files` on every drive, plus one
//!     level of nesting on every drive root (`D:\Games\GenshinImpact`).
//!
//! Matching is deliberately fuzzy:
//!   * folder names — case-insensitive *substring* match, because users
//!     rename install folders (observed: "Genshin Impact game",
//!     "HonkaiStarRail", "ArknightsEndfieldgowoU");
//!   * exe names — *exact* stem match from a per-game priority list.
//!
//! Epic-installed HoYoverse games use generic launcher names
//! (`launcher_epic.exe` for both HSR and ZZZ), so the *folder name* is the
//! primary signal. Results are deduplicated by path (first / highest-priority
//! root wins) and cached per app session (see [`Cache`]) — a full scan
//! touches every drive root and should not run on every "Играть" click.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

/// A found install: what to launch and where it was found.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedInstall {
    pub game_id: String,
    /// Path to the launcher .exe.
    pub path: String,
    /// "epic" | "steam" | "official"
    pub source: String,
}

/// Fuzzy folder-name patterns + launcher exe priority list per game.
struct Patterns {
    game_id: &'static str,
    /// Substrings (lowercased) that identify the install folder.
    folder: &'static [&'static str],
    /// Launcher exe stems (without `.exe`), in priority order.
    exes: &'static [&'static str],
    /// Exe stems that *unambiguously* name this game (cross-check used by
    /// the launch guard). Generic names are NOT listed here.
    strong: &'static [&'static str],
}

static PATTERNS: &[Patterns] = &[
    Patterns {
        game_id: "genshin",
        folder: &["genshin"],
        exes: &["GenshinImpact", "launcher_epic", "launcher"],
        strong: &["GenshinImpact"],
    },
    Patterns {
        game_id: "hsr",
        folder: &["honkaistarrail", "star rail", "star_rail", "hysr"],
        exes: &["launcher_epic", "launcher"],
        strong: &[],
    },
    Patterns {
        game_id: "zzz",
        folder: &["zenlesszonezero", "zenless zone zero", "zzz"],
        exes: &["launcher_epic", "launcher"],
        strong: &[],
    },
    Patterns {
        game_id: "wuthering",
        folder: &["wutheringwaves", "wuthering waves"],
        exes: &["Wuthering Waves", "launcher", "launcher_epic"],
        strong: &["Wuthering Waves"],
    },
    Patterns {
        game_id: "endfield",
        folder: &["endfield", "arknightsendfield", "arknights endfield"],
        exes: &["Launcher", "launcher", "launcher_epic"],
        strong: &[],
    },
];

fn patterns_for(game_id: &str) -> Option<&'static Patterns> {
    PATTERNS.iter().find(|p| p.game_id == game_id)
}

/// Which catalog game (if any) does this *folder name* belong to?
pub fn game_for_folder(folder_name: &str) -> Option<&'static str> {
    let lowered = folder_name.to_lowercase();
    for p in PATTERNS {
        if p.folder.iter().any(|f| lowered.contains(f)) {
            return Some(p.game_id);
        }
    }
    None
}

/// Which catalog game (if any) is *unambiguously* named by this exe stem?
pub fn strong_game_for_exe(exe_path: &str) -> Option<&'static str> {
    let file = Path::new(exe_path).file_stem()?.to_string_lossy();
    for p in PATTERNS {
        if p.strong.iter().any(|s| s.eq_ignore_ascii_case(&file)) {
            return Some(p.game_id);
        }
    }
    None
}

fn is_exe(p: &Path) -> bool {
    p.extension()
        .and_then(|x| x.to_str())
        .map(|s| s.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

/// Look for a launcher inside `folder` (root, then typical subfolders).
fn find_launcher(folder: &Path, specific: &[&str]) -> Option<PathBuf> {
    let candidates = [
        folder.to_path_buf(),
        folder.join("Binaries").join("Win64"),
        folder.join("Client").join("Binaries").join("Win64"),
    ];
    for dir in &candidates {
        if !dir.is_dir() {
            continue;
        }
        for name in specific {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_exe(&path)
                    && entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("launcher.exe")
                {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// All existing drive roots (A:\..Z:\) that pass a cheap existence check.
fn drive_roots() -> Vec<PathBuf> {
    (b'A'..=b'Z')
        .filter_map(|b| {
            let p = PathBuf::from(format!("{}:\\", b as char));
            p.is_dir().then_some(p)
        })
        .collect()
}

fn epic_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for d in drive_roots() {
        v.push(d.join("Epic Games"));
        v.push(d.join("Program Files").join("Epic Games"));
        v.push(d.join("Program Files (x86)").join("Epic Games"));
    }
    v
}

/// Steam install roots (the folders that contain `steamapps`).
fn steam_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hk = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hk.open_subkey(r"SOFTWARE\Valve\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                v.push(PathBuf::from(path));
            }
        }
    }
    v.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    v.push(PathBuf::from(r"C:\Program Files\Steam"));
    v
}

/// Minimal VDF value extraction: pull out all `"path"` values
/// (libraryfolders.vdf is simple enough for a line scan).
fn vdf_paths(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("\"path\"")?;
            let rest = rest.trim();
            let val = rest.strip_prefix('"')?;
            let end = val.find('"')?;
            Some(val[..end].replace("\\\\", "\\"))
        })
        .collect()
}

/// Every `<library>\steamapps\common` root, including Steam's additional
/// library folders (second-disk installs) from `libraryfolders.vdf`.
fn steam_common_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for root in steam_roots() {
        v.push(root.join("steamapps").join("common"));
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(&vdf) {
            for p in vdf_paths(&text) {
                v.push(PathBuf::from(p).join("steamapps").join("common"));
            }
        }
    }
    let mut seen = HashSet::new();
    v.into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// Top-level drive dirs that are never a games location.
const SKIPPED_TOP_DIRS: &[&str] = &[
    "windows",
    "programdata",
    "users",
    "perflogs",
    "recovery",
    "$recycle.bin",
    "system volume information",
    "old microsoft visual studio",
    "msocache",
    "config.msi",
    "shared",
];

fn is_skipped_top(name_lower: &str) -> bool {
    // `Program Files*` is scanned explicitly as a level-0 root already.
    SKIPPED_TOP_DIRS.contains(&name_lower) || name_lower.starts_with("program files")
}

/// Scan one directory's *children* for game folders.
fn scan_dir_for_games(dir: &Path, source: &'static str, out: &mut Vec<DetectedInstall>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let d = entry.path();
        if !d.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Some(game_id) = game_for_folder(&name) else {
            continue;
        };
        let Some(pat) = patterns_for(game_id) else {
            continue;
        };
        if let Some(exe) = find_launcher(&d, pat.exes) {
            out.push(DetectedInstall {
                game_id: game_id.to_string(),
                path: exe.to_string_lossy().into_owned(),
                source: source.to_string(),
            });
        }
    }
}

/// Scan all standard locations. At most: level-0 library roots + one level of
/// nesting on each drive root — a few dozen `read_dir` calls total.
pub fn detect_all() -> Vec<DetectedInstall> {
    let mut found: Vec<DetectedInstall> = Vec::new();

    // Exact library locations first (they win on duplicates).
    for r in epic_roots() {
        if r.is_dir() {
            scan_dir_for_games(&r, "epic", &mut found);
        }
    }
    for r in steam_common_roots() {
        if r.is_dir() {
            scan_dir_for_games(&r, "steam", &mut found);
        }
    }
    for d in drive_roots() {
        for pf in [d.join("Program Files"), d.join("Program Files (x86)")] {
            if pf.is_dir() {
                scan_dir_for_games(&pf, "official", &mut found);
            }
        }
        // One level of nesting: <drive>\<Game> and <drive>\<GamesRoot>\<Game>.
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let mid = entry.path();
            if !mid.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if is_skipped_top(&name.to_lowercase()) {
                continue;
            }
            if game_for_folder(&name).is_some() {
                // The top dir itself is a game folder.
                if let Some(pat) = patterns_for(game_for_folder(&name).unwrap()) {
                    if let Some(exe) = find_launcher(&mid, pat.exes) {
                        found.push(DetectedInstall {
                            game_id: pat.game_id.to_string(),
                            path: exe.to_string_lossy().into_owned(),
                            source: "official".to_string(),
                        });
                    }
                }
            } else {
                // e.g. D:\Games\GenshinImpact
                scan_dir_for_games(&mid, "official", &mut found);
            }
        }
    }

    // Deduplicate by path; the first (higher-priority) root wins.
    let mut seen = HashSet::new();
    found.retain(|d| seen.insert(d.path.clone()));
    found
}

/// Pick the best detection for one game from a list; a hit with
/// `prefer_source` (e.g. "epic" for the Epic-mode fallback) wins.
pub fn pick_from(
    list: &[DetectedInstall],
    game_id: &str,
    prefer_source: Option<&str>,
) -> Option<DetectedInstall> {
    let mut exact: Option<DetectedInstall> = None;
    let mut other: Option<DetectedInstall> = None;
    for d in list.iter().filter(|d| d.game_id == game_id) {
        match prefer_source {
            Some(src) if d.source == src => {
                if exact.is_none() {
                    exact = Some(d.clone());
                }
            }
            _ => {
                if other.is_none() {
                    other = Some(d.clone());
                }
            }
        }
    }
    exact.or(other)
}

/// Per-session cache so a full rescan happens at most once per app start
/// (or on demand via `refresh` — the "Скан" button).
#[derive(Default)]
pub struct Cache {
    inner: Mutex<Option<Vec<DetectedInstall>>>,
}

impl Cache {
    pub fn get(&self) -> Vec<DetectedInstall> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(v) = guard.as_ref() {
            return v.clone();
        }
        let v = detect_all();
        *guard = Some(v.clone());
        v
    }

    pub fn refresh(&self) -> Vec<DetectedInstall> {
        *self.inner.lock().unwrap() = None;
        self.get()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdf_parses_paths() {
        let vdf = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}"#;
        let paths = vdf_paths(vdf);
        assert_eq!(
            paths,
            vec![
                r"C:\Program Files (x86)\Steam".to_string(),
                r"D:\SteamLibrary".to_string(),
            ]
        );
    }

    #[test]
    fn extension_is_case_insensitive() {
        assert!(is_exe(Path::new(r"C:\tmp\GAME.EXE")));
        assert!(is_exe(Path::new(r"C:\tmp\game.exe")));
        assert!(!is_exe(Path::new(r"C:\tmp\game.dll")));
    }

    /// Machine-specific smoke test: this PC has all five games installed
    /// (four via Epic on C:, one via Steam, plus nested copies on D:).
    #[test]
    fn detect_all_finds_local_games() {
        let all = detect_all();
        let ids: Vec<&str> = all.iter().map(|d| d.game_id.as_str()).collect();
        for expected in ["genshin", "hsr", "zzz", "endfield", "wuthering"] {
            assert!(
                ids.iter().any(|i| *i == expected),
                "missing {expected}; got {ids:?}"
            );
        }
        // Dedup: every path is unique.
        let unique: HashSet<&str> = all.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(all.len(), unique.len(), "duplicate paths: {all:?}");
    }
}
