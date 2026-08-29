//! Tauri commands — the only bridge between the React UI and the backend.
//! All system calls (URI schemes, process spawn, filesystem) happen on this
//! side; the frontend never touches the OS directly.

use crate::catalog::{GameMeta, GAMES};
use crate::config::{self, GameLaunchConfig, LaunchMode};
use crate::detect;
use crate::hub;
use crate::launch;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};

/// One game for the UI: catalog meta + the user's launch config.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub steam_appid: Option<u32>,
    pub epic_supported: bool,
    pub has_official_launcher: bool,
    pub store_url: String,
    pub config: GameLaunchConfig,
}

fn view(g: &GameMeta, cfg: &config::AppConfig) -> GameView {
    let user_cfg = cfg
        .games
        .get(g.id)
        .cloned()
        .unwrap_or_else(|| GameLaunchConfig::default_for(g));
    GameView {
        id: g.id.to_string(),
        name: g.name.to_string(),
        publisher: g.publisher.to_string(),
        steam_appid: g.steam_appid,
        epic_supported: g.epic_supported,
        has_official_launcher: g.has_official_launcher,
        store_url: g.store_url.to_string(),
        config: user_cfg,
    }
}

// All commands are async: in Tauri, sync commands run on the main thread
// and would briefly freeze the window (file reads, shell spawn); async
// commands run on the async runtime instead.

#[tauri::command]
pub async fn get_games(app: AppHandle) -> Vec<GameView> {
    let cfg = config::load(&app);
    GAMES.iter().map(|g| view(g, &cfg)).collect()
}

#[tauri::command]
pub async fn get_config(app: AppHandle) -> config::AppConfig {
    config::load(&app)
}

#[tauri::command]
pub async fn get_config_dir(app: AppHandle) -> Result<String, String> {
    Ok(config::config_dir(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn save_config(app: AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::save(&app, &config)
}

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    detect: State<'_, detect::Cache>,
    game_id: String,
) -> Result<(), String> {
    let meta = GAMES
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("unknown game: {game_id}"))?;

    let cfg = config::load(&app);
    let user_cfg = cfg
        .games
        .get(&game_id)
        .cloned()
        .unwrap_or_else(|| GameLaunchConfig::default_for(meta));

    let target = match user_cfg.launch_mode {
        LaunchMode::Steam => {
            let appid = meta
                .steam_appid
                .ok_or_else(|| format!("no Steam appid for «{}» in the catalog", meta.name))?;
            launch::LaunchTarget::Steam(appid)
        }
        LaunchMode::Epic => {
            let id = user_cfg
                .epic_product_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            match id {
                Some(id) => launch::LaunchTarget::Epic(id),
                // No product id: try to find the game in the Epic Games
                // library folder and launch its .exe directly.
                None => match detect::pick_from(&detect.get(), &meta.id, Some("epic")) {
                    Some(found) => launch::LaunchTarget::Exe(
                        PathBuf::from(found.path),
                        user_cfg.args.clone(),
                    ),
                    None => {
                        return Err(format!(
                            "{}: Epic product id not set and the game was not found in the \
                             Epic Games folder. Fill the product id in the settings, or switch \
                             to \"direct .exe\" (press \"Scan\" there).",
                            meta.name
                        ))
                    }
                },
            }
        }
        LaunchMode::Exe => {
            let exe = user_cfg
                .exe_path
                .clone()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or_else(|| "game path not set — pick an .exe in the settings (or press \"Scan\")".to_string())?;
            launch::LaunchTarget::Exe(exe, user_cfg.args.clone())
        }
    };

    launch::launch(&app, &game_id, target)
}

#[tauri::command]
pub async fn detect_installs(
    detect: State<'_, detect::Cache>,
) -> Result<Vec<detect::DetectedInstall>, String> {
    Ok(detect.get())
}

/// Force a full rescan (the "Скан" button in settings).
#[tauri::command]
pub async fn rescan_installs(
    detect: State<'_, detect::Cache>,
) -> Result<Vec<detect::DetectedInstall>, String> {
    Ok(detect.refresh())
}

#[tauri::command]
pub async fn get_hub(app: AppHandle) -> Result<hub::HubData, String> {
    let cfg = config::load(&app);
    hub::load(&app, cfg.hub_url.as_deref())
}
