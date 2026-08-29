//! Пользовательские настройки: %APPDATA%\<identifier>\config.json
//!
//! Схема версии 2. Главное отличие от версии 1: игра — это запись в конфиге,
//! а не константа в коде, и `games` — упорядоченный массив, а не словарь.
//! Порядок значим: он задаёт полку на главном экране, а словарь давал бы
//! случайный порядок при каждом запуске.
//!
//! Запись атомарная: во временный файл рядом, затем переименование поверх.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const CURRENT_VERSION: u32 = 2;

/// Способ запуска определён в `stores`, потому что узнаём мы его из манифеста
/// магазина. Переэкспортируем, чтобы остальной код писал `config::Launch`.
pub use crate::stores::Launch;

/// Одна игра в списке пользователя.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    /// Стабильный идентификатор внутри конфига.
    pub id: String,
    pub title: String,
    /// Связь с контентом в файле хаба. `None` — игра добавлена вручную,
    /// новостей и кодов по ней не будет.
    #[serde(default)]
    pub content_id: Option<String>,
    pub launch: Launch,
    /// Папка установки — по ней проверяем, что игра не переехала.
    #[serde(default)]
    pub install_path: Option<PathBuf>,
    /// Исполняемый файл. Обязателен для `Launch::Exe`.
    #[serde(default)]
    pub exe_path: Option<PathBuf>,
    #[serde(default)]
    pub args: String,
    /// Картинка фона, подставленная пользователем.
    #[serde(default)]
    pub background: Option<PathBuf>,
}

impl Game {
    /// Игра без привязки к каталогу контента — добавленная руками.
    pub fn manual(id: String, title: String) -> Self {
        Self {
            id,
            title,
            content_id: None,
            launch: Launch::Exe,
            install_path: None,
            exe_path: None,
            args: String::new(),
            background: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub hub_url: Option<String>,
    /// Игра, открытая при следующем запуске. Пишется в момент удачного старта.
    #[serde(default)]
    pub last_played: Option<String>,
    #[serde(default)]
    pub games: Vec<Game>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            hub_url: None,
            last_played: None,
            games: Vec::new(),
        }
    }
}

/// %APPDATA%\<identifier> — создаётся, если её нет.
pub fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("не удалось определить папку конфига: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("не удалось создать {}: {e}", dir.display()))?;
    Ok(dir)
}

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("config.json"))
}

/// Перенос конфига версии 1 на версию 2.
///
/// В версии 1 игры лежали словарём, ключ был идентификатором из зашитого
/// каталога, а способ запуска — строкой `launchMode`. Appid и идентификаторы
/// Epic в конфиге не хранились, поэтому режимы `steam` и `epic` при переносе
/// падают до `Exe`: конкретика подтянется заново из манифестов магазинов.
pub fn migrate_v1(raw: &serde_json::Value) -> Option<AppConfig> {
    let games_obj = raw.get("games")?.as_object()?;
    let mut games = Vec::new();

    for (key, value) in games_obj {
        let Some(entry) = value.as_object() else {
            eprintln!("[config] пропускаю «{key}»: запись не объект");
            continue;
        };
        let args = entry
            .get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let exe_path = entry
            .get("exePath")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        let mode = entry.get("launchMode").and_then(|v| v.as_str());
        if entry.contains_key("launchMode") && mode.is_none() {
            eprintln!("[config] пропускаю «{key}»: launchMode не строка");
            continue;
        }

        games.push(Game {
            id: key.clone(),
            title: key.clone(),
            content_id: Some(key.clone()),
            launch: Launch::Exe,
            install_path: None,
            exe_path,
            args,
            background: None,
        });
    }

    games.sort_by(|a, b| a.id.cmp(&b.id));

    Some(AppConfig {
        version: CURRENT_VERSION,
        hub_url: raw
            .get("hubUrl")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_played: None,
        games,
    })
}

/// Чтение настроек. Терпимое к поломкам: битая запись по одной игре
/// отбрасывается с записью в лог, остальной конфиг живёт. В `.broken`
/// уезжает только синтаксически сломанный файл.
pub fn load(app: &AppHandle) -> AppConfig {
    let Ok(path) = config_path(app) else {
        return AppConfig::default();
    };
    if !path.exists() {
        return AppConfig::default();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) else {
        eprintln!("[config] {} — не JSON, начинаю с чистого", path.display());
        let _ = fs::rename(&path, path.with_file_name("config.json.broken"));
        return AppConfig::default();
    };

    let version = raw.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    if version < 2 {
        return migrate_v1(&raw).unwrap_or_default();
    }

    let mut games = Vec::new();
    if let Some(list) = raw.get("games").and_then(|v| v.as_array()) {
        for item in list {
            match serde_json::from_value::<Game>(item.clone()) {
                Ok(game) => games.push(game),
                Err(e) => eprintln!("[config] отбрасываю запись игры: {e}"),
            }
        }
    }

    AppConfig {
        version: CURRENT_VERSION,
        hub_url: raw
            .get("hubUrl")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_played: raw
            .get("lastPlayed")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        games,
    }
}

/// Атомарная запись: временный файл рядом, затем переименование поверх.
pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let final_path = config_path(app)?;
    let tmp_path = final_path.with_file_name("config.json.tmp");
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("сериализация: {e}"))?;
    fs::write(&tmp_path, json)
        .map_err(|e| format!("запись {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        format!(
            "переименование {} -> {}: {e}",
            tmp_path.display(),
            final_path.display()
        )
    })?;
    Ok(())
}

/// Существует ли файл, которым игра запускается.
pub fn is_present(game: &Game) -> bool {
    match &game.launch {
        Launch::Exe => game
            .exe_path
            .as_deref()
            .map(Path::is_file)
            .unwrap_or(false),
        _ => game
            .install_path
            .as_deref()
            .map(Path::is_dir)
            .unwrap_or(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_serialises_with_a_kind_tag() {
        let json = serde_json::to_string(&Launch::Steam { appid: 3513350 }).unwrap();
        assert_eq!(json, r#"{"kind":"steam","appid":3513350}"#);
    }

    #[test]
    fn epic_launch_uses_camel_case_fields() {
        let json = serde_json::to_string(&Launch::Epic {
            namespace: "ns".into(),
            catalog_item_id: "cid".into(),
            app_name: "app".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"kind":"epic","namespace":"ns","catalogItemId":"cid","appName":"app"}"#
        );
    }

    #[test]
    fn migrates_a_version_one_config() {
        let v1 = serde_json::json!({
            "version": 1,
            "hubUrl": "https://example.com/hub.json",
            "games": {
                "wuthering": { "launchMode": "steam", "exePath": null, "args": "" },
                "hsr": {
                    "launchMode": "exe",
                    "exePath": "C:\\Games\\HSR\\launcher.exe",
                    "args": "-dx12"
                }
            }
        });

        let cfg = migrate_v1(&v1).expect("миграция должна пройти");

        assert_eq!(cfg.version, 2);
        assert_eq!(cfg.hub_url.as_deref(), Some("https://example.com/hub.json"));
        assert_eq!(cfg.games.len(), 2);

        let hsr = cfg.games.iter().find(|g| g.id == "hsr").unwrap();
        assert_eq!(hsr.content_id.as_deref(), Some("hsr"));
        assert_eq!(hsr.args, "-dx12");
        assert!(matches!(hsr.launch, Launch::Exe));
        assert_eq!(hsr.exe_path.as_deref(), Some(Path::new(r"C:\Games\HSR\launcher.exe")));
    }

    #[test]
    fn migration_drops_a_broken_game_but_keeps_the_rest() {
        let v1 = serde_json::json!({
            "version": 1,
            "games": {
                "hsr": { "launchMode": "exe", "exePath": "C:\\ok.exe", "args": "" },
                "broken": { "launchMode": 42 }
            }
        });
        let cfg = migrate_v1(&v1).unwrap();
        assert_eq!(cfg.games.len(), 1);
        assert_eq!(cfg.games[0].id, "hsr");
    }

    #[test]
    fn v1_steam_mode_without_an_appid_becomes_exe() {
        // В v1 appid жил в коде, а не в конфиге. При миграции его взять
        // неоткуда, поэтому режим падает до «свой .exe» — путь пользователь
        // либо уже указал, либо укажет заново.
        let v1 = serde_json::json!({
            "version": 1,
            "games": { "wuthering": { "launchMode": "steam", "args": "" } }
        });
        let cfg = migrate_v1(&v1).unwrap();
        assert!(matches!(cfg.games[0].launch, Launch::Exe));
    }

    #[test]
    fn games_keep_their_order_across_a_save_round_trip() {
        let cfg = AppConfig {
            version: 2,
            hub_url: None,
            last_played: None,
            games: vec![
                Game::manual("b".into(), "Второй".into()),
                Game::manual("a".into(), "Первый".into()),
            ],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.games[0].id, "b");
        assert_eq!(back.games[1].id, "a");
    }
}
