//! Sources, in priority order:
//!   1. remote JSON if `hubUrl` is set in the user config — fetched by
//!      Rust (https only, 5 s timeout), then cached to
//!      `%APPDATA%\<id>\hub_cache.json`;
//!   2. `%APPDATA%\<id>\hub.json` — manual local override: drop/update this
//!      file to change codes/news without a rebuild;
//!   3. `%APPDATA%\<id>\hub_cache.json` — last successful remote fetch
//!      (only consulted when a remote URL is set and the fetch failed);
//!   4. bundled `src-tauri/resources/hub.json` (demo data) from
//!      `resource_dir()/resources/hub.json`.
//!
//! The schema mirrors the TS types in `src/types.ts`; the UI only depends
//! on the `HubData` shape, so the source can change without UI changes.

pub mod schema;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HubData {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub promo_codes: Vec<PromoCode>,
    #[serde(default)]
    pub news: Vec<NewsItem>,
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Human-readable note, e.g. "demo data" (JSON field `_note`).
    #[serde(rename = "_note", default)]
    pub note: Option<String>,
    /// Where this data came from: "remote" | "override" | "cache" | "bundled".
    /// Set at load time (JSON field `_source`).
    #[serde(rename = "_source", default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PromoCode {
    pub game_id: String,
    pub code: String,
    pub rewards: String,
    /// ISO date `YYYY-MM-DD`: the code is valid through this day inclusive.
    pub expired: String,
    #[serde(default)]
    pub source: Option<String>,
}

impl Default for PromoCode {
    fn default() -> Self {
        Self {
            game_id: String::new(),
            code: String::new(),
            rewards: String::new(),
            expired: String::new(),
            source: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct NewsItem {
    pub game_id: String,
    pub title: String,
    pub date: String,
    pub summary: String,
    pub url: String,
}

impl Default for NewsItem {
    fn default() -> Self {
        Self {
            game_id: String::new(),
            title: String::new(),
            date: String::new(),
            summary: String::new(),
            url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Guide {
    pub game_id: String,
    pub title: String,
    /// Character the guide is about (optional).
    pub character: Option<String>,
    pub date: Option<String>,
    pub url: String,
}

impl Default for Guide {
    fn default() -> Self {
        Self {
            game_id: String::new(),
            title: String::new(),
            character: None,
            date: None,
            url: String::new(),
        }
    }
}

const REMOTE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn read_json_file(path: &Path) -> Option<HubData> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Не больше двух мегабайт: файл хаба — это текст, всё что крупнее либо
/// ошибка, либо попытка занять нам память.
const MAX_HUB_BYTES: usize = 2 * 1024 * 1024;

/// Блокирующая загрузка по https с коротким таймаутом.
/// Редиректы запрещены: иначе ответ https-адреса мог бы увести нас на http.
fn fetch_remote(url: &str) -> Option<HubData> {
    let resp = ureq::builder()
        .redirects(0)
        .build()
        .get(url)
        .timeout(REMOTE_TIMEOUT)
        .call()
        .ok()?;

    let mut body = String::new();
    resp.into_reader()
        .take(MAX_HUB_BYTES as u64)
        .read_to_string(&mut body)
        .ok()?;

    serde_json::from_str(&body).ok()
}

/// Load hub data (see the module docs for the source priority).
pub fn load(app: &AppHandle, hub_url: Option<&str>) -> Result<HubData, String> {
    let cfg_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    let cache_path = cfg_dir.join("hub_cache.json");
    let override_path = cfg_dir.join("hub.json");

    if let Some(url) = hub_url.filter(|u| u.starts_with("https://")) {
        if let Some(mut data) = fetch_remote(url) {
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let _ = fs::write(&cache_path, json);
            }
            data.source = Some("remote".to_string());
            return Ok(data);
        }
        // Remote failed: manual override first, then the last good cache.
        if let Some(mut data) = read_json_file(&override_path) {
            data.source = Some("override".to_string());
            return Ok(data);
        }
        if let Some(mut data) = read_json_file(&cache_path) {
            data.source = Some("cache".to_string());
            return Ok(data);
        }
    } else if let Some(mut data) = read_json_file(&override_path) {
        data.source = Some("override".to_string());
        return Ok(data);
    }

    let dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource dir: {e}"))?;
    let path = dir.join("resources/hub.json");
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut data: HubData =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    data.source = Some("bundled".to_string());
    Ok(data)
}
