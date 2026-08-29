//! Config Manager: user settings stored as JSON in the OS config dir.
//!
//! Location on Windows: %APPDATA%\<identifier>\config.json
//! (resolved via Tauri's `app.path().app_config_dir()`).
//!
//! Writes are atomic: data is written to `config.json.tmp` in the same
//! directory and then `fs::rename`d over the target. A rename within the
//! same volume is atomic on NTFS, so a crash mid-write can never leave a
//! torn/corrupt config file.

use crate::catalog::GameMeta;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppConfig {
    /// Schema version. Bump when the shape of the settings changes.
    pub version: u32,
    /// Per-game overrides, keyed by catalog game id.
    pub games: HashMap<String, GameLaunchConfig>,
    /// Optional https URL of a remote hub.json (news/codes/guides).
    /// When set, it is fetched on load and cached locally.
    #[serde(default)]
    pub hub_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            games: HashMap::new(),
            hub_url: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMode {
    /// Opens `steam://rungameid/<appid>`; the Steam client takes over.
    Steam,
    /// Opens `egstore://launch/<product-id>`; the Epic Games Store launches.
    Epic,
    /// Direct .exe launch (official launcher or game binary) with user args.
    /// `#[serde(other)]`: fallback for unrecognized values, so one bad
    /// per-game field cannot nuke the whole config file.
    #[serde(other)]
    Exe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct GameLaunchConfig {
    #[serde(default = "default_mode")]
    pub launch_mode: LaunchMode,
    /// Path to the .exe (used by Exe mode: official launcher or game binary).
    #[serde(default)]
    pub exe_path: Option<PathBuf>,
    /// Arbitrary launch flags as a string, e.g. "-dx12". Tokenized with the
    /// Windows CommandLineToArgvW rules before being passed to the process
    /// (see `split_args` in launch.rs).
    #[serde(default)]
    pub args: String,
    /// Epic internal product id used for `egstore://launch/<id>` (Epic mode).
    #[serde(default)]
    pub epic_product_id: Option<String>,
}

fn default_mode() -> LaunchMode {
    LaunchMode::Exe
}

impl Default for GameLaunchConfig {
    fn default() -> Self {
        Self {
            launch_mode: LaunchMode::Exe,
            exe_path: None,
            args: String::new(),
            epic_product_id: None,
        }
    }
}

impl GameLaunchConfig {
    /// Sensible default for a game the user hasn't configured yet:
    /// Steam when the catalog has an appid, otherwise direct .exe.
    pub fn default_for(meta: &GameMeta) -> Self {
        Self {
            launch_mode: meta.default_mode(),
            ..Self::default()
        }
    }
}

/// %APPDATA%\<identifier> — created if missing.
pub fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("failed to resolve config dir: {e}"))?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create {}: {e}", dir.display()))?;
    Ok(dir)
}

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("config.json"))
}

/// Load settings. Lenient on purpose:
///   * a syntactically broken JSON file → renamed to `config.json.broken`,
///     defaults returned;
///   * a bad *per-game* entry → dropped with a log line, the rest of the
///     config is preserved (one typo must not nuke all user settings).
pub fn load(app: &AppHandle) -> AppConfig {
    let Ok(p) = config_path(app) else {
        return AppConfig::default();
    };
    if !p.exists() {
        return AppConfig::default();
    }
    let Ok(text) = fs::read_to_string(&p) else {
        return AppConfig::default();
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) else {
        eprintln!("[config] {} is not valid JSON — using defaults (renaming old file)", p.display());
        let _ = fs::rename(&p, p.with_file_name("config.json.broken"));
        return AppConfig::default();
    };
    let version = raw.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    let mut games = HashMap::new();
    if let Some(obj) = raw.get("games").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            match serde_json::from_value::<GameLaunchConfig>(v.clone()) {
                Ok(c) => {
                    games.insert(k.clone(), c);
                }
                Err(e) => eprintln!("[config] dropping bad config for game «{k}»: {e}"),
            }
        }
    }
    let hub_url = raw.get("hubUrl").and_then(|v| v.as_str()).map(str::to_string);
    AppConfig { version, games, hub_url }
}

/// Atomic save: write to `config.json.tmp` in the same directory, then
/// rename over the target (atomic within one volume on NTFS).
pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let final_path = config_path(app)?;
    let tmp_path = final_path.with_file_name("config.json.tmp");
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&tmp_path, json).map_err(|e| format!("write {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, &final_path)
        .map_err(|e| format!("atomic rename {} -> {}: {e}", tmp_path.display(), final_path.display()))?;
    Ok(())
}
