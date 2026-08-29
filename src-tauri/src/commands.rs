//! Команды — единственный мост между интерфейсом и системой.
//! Все обращения к файлам, реестру и оболочке живут здесь и глубже.
//!
//! Все команды асинхронные: синхронные в Tauri выполняются в главном потоке
//! и подмораживали бы окно на чтении файлов и запуске процессов.

use serde::Serialize;
use tauri::AppHandle;

use crate::config::{self, Game, Launch};
use crate::hub;
use crate::launch;

/// Игра в том виде, в каком её рисует интерфейс.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub id: String,
    pub title: String,
    pub content_id: Option<String>,
    /// Подпись «запустится через …».
    pub source_label: String,
    /// Файл или папка игры пропали с диска.
    pub missing: bool,
}

pub fn view_of(game: &Game) -> GameView {
    let source_label = match game.launch {
        Launch::Steam { .. } => "Steam",
        Launch::Epic { .. } => "Epic Games",
        Launch::Exe => "напрямую",
    }
    .to_string();

    GameView {
        id: game.id.clone(),
        title: game.title.clone(),
        content_id: game.content_id.clone(),
        source_label,
        missing: !config::is_present(game),
    }
}

#[tauri::command]
pub async fn get_games(app: AppHandle) -> Vec<GameView> {
    config::load(&app).games.iter().map(view_of).collect()
}

#[tauri::command]
pub async fn get_config(app: AppHandle) -> config::AppConfig {
    config::load(&app)
}

#[tauri::command]
pub async fn get_config_dir(app: AppHandle) -> Result<String, String> {
    Ok(config::config_dir(&app)?.to_string_lossy().into_owned())
}

/// Запомнить выбранную игру, не запуская её.
#[tauri::command]
pub async fn select_game(app: AppHandle, game_id: String) -> Result<(), String> {
    let mut cfg = config::load(&app);
    if !cfg.games.iter().any(|g| g.id == game_id) {
        return Err(format!("нет такой игры: {game_id}"));
    }
    cfg.last_played = Some(game_id);
    config::save(&app, &cfg)
}

/// Запустить игру. `lastPlayed` пишется только после удачного старта.
#[tauri::command]
pub async fn launch_game(app: AppHandle, game_id: String) -> Result<String, String> {
    let mut cfg = config::load(&app);
    let game = cfg
        .games
        .iter()
        .find(|g| g.id == game_id)
        .cloned()
        .ok_or_else(|| format!("нет такой игры: {game_id}"))?;

    launch::launch(&app, &game)?;

    cfg.last_played = Some(game_id.clone());
    if let Err(e) = config::save(&app, &cfg) {
        eprintln!("[config] не удалось сохранить lastPlayed: {e}");
    }
    Ok(game_id)
}

#[tauri::command]
pub async fn get_hub(app: AppHandle) -> Result<hub::HubData, String> {
    let cfg = config::load(&app);
    hub::load(&app, cfg.hub_url.as_deref())
}

#[tauri::command]
pub async fn get_last_played(app: AppHandle) -> Option<String> {
    config::load(&app).last_played
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Game, Launch};

    fn steam_game() -> Game {
        Game {
            id: "wuthering".into(),
            title: "Wuthering Waves".into(),
            content_id: Some("wuthering".into()),
            launch: Launch::Steam { appid: 3513350 },
            install_path: Some(std::path::PathBuf::from(r"C:\nope\never")),
            exe_path: None,
            args: String::new(),
            background: None,
        }
    }

    #[test]
    fn view_labels_the_store_a_game_starts_through() {
        assert_eq!(view_of(&steam_game()).source_label, "Steam");
    }

    #[test]
    fn view_marks_a_game_whose_folder_disappeared() {
        assert!(view_of(&steam_game()).missing);
    }

    #[test]
    fn exe_games_are_labelled_as_direct() {
        let mut g = steam_game();
        g.launch = Launch::Exe;
        assert_eq!(view_of(&g).source_label, "напрямую");
    }
}
