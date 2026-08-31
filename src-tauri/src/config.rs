//! Пользовательские настройки: %APPDATA%\<identifier>\config.json
//!
//! Схема версии 3. От версии 1 к версии 2 игра стала записью в конфиге,
//! а не константой в коде, и `games` — упорядоченный массив, а не словарь.
//! Порядок значим: он задаёт полку на главном экране, а словарь давал бы
//! случайный порядок при каждом запуске. Версия 3 добавляет `seeded` (был ли
//! уже первый запуск) и `behaviour` (настройки поведения окна).
//!
//! Запись атомарная: во временный файл рядом, затем переименование поверх.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const CURRENT_VERSION: u32 = 3;

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
    /// Пока не вызывается за пределами тестов: конструктор для экрана
    /// добавления вручную из §6.4, который появится на этапе 3.
    #[allow(dead_code)]
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

/// Настройки поведения окна.
///
/// Оба значения по умолчанию — `true`: это ровно то, как приложение вело себя
/// до появления переключателей, поэтому обновление ничего не меняет под
/// человеком.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Behaviour {
    /// Крестик прячет окно в трей, а не закрывает приложение.
    pub close_to_tray: bool,
    /// После удачного запуска игры окно уходит в трей.
    pub tray_on_launch: bool,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            tray_on_launch: true,
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
    /// Первый запуск уже состоялся. После этого пустой список игр остаётся
    /// пустым: человек, удаливший всё, сделал это осознанно, и возвращать
    /// игры автоматически — значит не слушаться его.
    #[serde(default)]
    pub seeded: bool,
    #[serde(default)]
    pub behaviour: Behaviour,
    #[serde(default)]
    pub games: Vec<Game>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            hub_url: None,
            last_played: None,
            // Свежая установка: первого запуска ещё не было.
            seeded: false,
            behaviour: Behaviour::default(),
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

/// Перенос конфига версии 1 сразу до текущей версии.
///
/// В версии 1 игры лежали словарём, ключ был идентификатором из зашитого
/// каталога, а способ запуска — строкой `launchMode`. Appid и идентификаторы
/// Epic в конфиге не хранились, поэтому режимы `steam` и `epic` при переносе
/// падают до `Exe`: конкретика подтянется заново из манифестов магазинов.
///
/// Функция пишет `version: CURRENT_VERSION` напрямую, минуя промежуточную
/// версию 2: у человека, пришедшего с версии 1, уже есть игры, так что поля
/// уровня приложения, добавленные в версии 3, получают те же значения, что и
/// при переносе с версии 2 (см. `migrate_v2`).
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
        // У человека уже есть игры — первый запуск для него состоялся.
        seeded: true,
        behaviour: Behaviour::default(),
        games,
    })
}

/// Перенос конфига со второй версии на третью.
///
/// Игры переносятся дословно — это главное требование этапа. Прибавляются
/// только два поля уровня приложения, и оба получают значения, при которых
/// приложение ведёт себя так же, как до обновления.
pub fn migrate_v2(raw: &serde_json::Value) -> Option<AppConfig> {
    let games: Vec<Game> = serde_json::from_value(raw.get("games")?.clone()).ok()?;

    Some(AppConfig {
        version: CURRENT_VERSION,
        hub_url: raw
            .get("hubUrl")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_played: raw
            .get("lastPlayed")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        // У человека уже есть игры — первый запуск для него состоялся.
        seeded: true,
        behaviour: Behaviour::default(),
        games,
    })
}

/// Чтение настроек. Терпимое к поломкам: битая запись по одной игре
/// отбрасывается с записью в лог, остальной конфиг живёт. В `.broken`
/// уезжает только синтаксически сломанный файл.
///
/// Файл версии выше `CURRENT_VERSION` (например, записанный более новой
/// сборкой) читается как есть, без понижения `version` до `CURRENT_VERSION`:
/// `save` откажется писать такой конфиг обратно (см. ниже), так что файл
/// остаётся на диске нетронутым — как и подобает записи, которая не сломана
/// и не подлежит миграции.
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
        return match migrate_v1(&raw) {
            Some(cfg) => {
                // Миграция должна пережить перезапуск сама по себе, а не
                // ждать, пока её случайно сохранит другой вызов (например,
                // sync_games_with_stores, если сверка с магазинами ничего
                // не поменяла). Без этого файл на диске остаётся версии 1
                // и мигрирует заново при каждом старте.
                if let Err(e) = save(app, &cfg) {
                    eprintln!("[config] не удалось сохранить перенесённый конфиг: {e}");
                }
                cfg
            }
            None => {
                eprintln!(
                    "[config] {} — не удалось перенести версию 1 (нет «games» или это не объект), начинаю с чистого",
                    path.display()
                );
                AppConfig::default()
            }
        };
    }

    if version < 3 {
        return match migrate_v2(&raw) {
            Some(cfg) => {
                // Та же причина, что и для версии 1: миграция должна
                // пережить перезапуск сама по себе.
                if let Err(e) = save(app, &cfg) {
                    eprintln!("[config] не удалось сохранить перенесённый конфиг: {e}");
                }
                cfg
            }
            None => {
                eprintln!(
                    "[config] {} — не удалось перенести версию 2 (нет «games» или записи не соответствуют схеме), начинаю с чистого",
                    path.display()
                );
                AppConfig::default()
            }
        };
    }

    if version > CURRENT_VERSION {
        eprintln!(
            "[config] {} — версия файла ({version}) новее, чем понимает эта сборка (умеет до {CURRENT_VERSION}); читаю как есть, не понижаю версию",
            path.display()
        );
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
        version,
        hub_url: raw
            .get("hubUrl")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_played: raw
            .get("lastPlayed")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        seeded: raw.get("seeded").and_then(|v| v.as_bool()).unwrap_or(false),
        behaviour: raw
            .get("behaviour")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        games,
    }
}

/// Выделена из `save` отдельной чистой функцией, чтобы проверять её без
/// `AppHandle` — в модульных тестах его взять неоткуда.
fn reject_if_newer_than_current(version: u32) -> Result<(), String> {
    if version > CURRENT_VERSION {
        return Err(format!(
            "конфиг версии {version} новее, чем понимает эта сборка (умеет до версии {CURRENT_VERSION}); отказываюсь перезаписывать файл"
        ));
    }
    Ok(())
}

/// Атомарная запись: временный файл рядом, затем переименование поверх.
///
/// Отказывается писать конфиг версии выше `CURRENT_VERSION`: у этой сборки
/// нет схемы для полей, которых она не знает, и молча их отбросить значило
/// бы необратимо стереть данные более новой версии приложения.
pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    reject_if_newer_than_current(cfg.version)?;
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

        assert_eq!(cfg.version, CURRENT_VERSION);
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
    fn save_refuses_a_config_newer_than_this_build_understands() {
        // `save` delegates the check to `reject_if_newer_than_current`, since
        // `save` itself needs an `AppHandle` that unit tests have no way to
        // construct. This exercises the exact guard `save` runs first, before
        // any file is touched.
        let newer = CURRENT_VERSION + 1;
        let err = reject_if_newer_than_current(newer).expect_err("должен отказать");
        assert!(
            err.contains(&newer.to_string()),
            "ошибка должна называть версию файла: {err}"
        );
        assert!(
            err.contains(&CURRENT_VERSION.to_string()),
            "ошибка должна называть версию, которую понимает сборка: {err}"
        );

        assert!(reject_if_newer_than_current(CURRENT_VERSION).is_ok());
    }

    #[test]
    fn games_keep_their_order_across_a_save_round_trip() {
        let cfg = AppConfig {
            version: 2,
            hub_url: None,
            last_played: None,
            seeded: false,
            behaviour: Behaviour::default(),
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

    const V2_SAMPLE: &str = r#"{
      "version": 2,
      "hubUrl": null,
      "lastPlayed": "genshin",
      "games": [
        {
          "id": "genshin",
          "title": "Genshin Impact",
          "contentId": "genshin",
          "launch": { "kind": "epic", "namespace": "n", "catalogItemId": "c", "appName": "a" },
          "installPath": "C:\\Games\\Genshin",
          "exePath": "C:\\Games\\Genshin\\launcher.exe",
          "args": "",
          "background": null
        },
        {
          "id": "wuthering",
          "title": "Wuthering Waves",
          "contentId": "wuthering",
          "launch": { "kind": "steam", "appid": 3513350 },
          "installPath": "D:\\Steam\\WW",
          "exePath": null,
          "args": "-window",
          "background": "C:\\my\\art.png"
        }
      ]
    }"#;

    #[test]
    fn migration_keeps_every_game_untouched() {
        // Это самая важная проверка этапа: у человека настроенные игры с
        // вручную выверенными путями, и потерять их нельзя.
        let raw: serde_json::Value = serde_json::from_str(V2_SAMPLE).unwrap();
        let cfg = migrate_v2(&raw).expect("миграция не должна проваливаться");

        assert_eq!(cfg.version, 3);
        assert_eq!(cfg.games.len(), 2);

        let g = &cfg.games[0];
        assert_eq!(g.id, "genshin");
        assert_eq!(g.title, "Genshin Impact");
        assert_eq!(g.content_id.as_deref(), Some("genshin"));
        assert_eq!(g.exe_path, Some(std::path::PathBuf::from(r"C:\Games\Genshin\launcher.exe")));

        let w = &cfg.games[1];
        assert_eq!(w.launch, Launch::Steam { appid: 3513350 });
        assert_eq!(w.args, "-window");
        assert_eq!(w.background, Some(std::path::PathBuf::from(r"C:\my\art.png")));
    }

    #[test]
    fn migration_marks_an_existing_config_as_already_seeded() {
        // У человека уже есть игры — первый запуск для него состоялся.
        // Показать ему экран с галочками после обновления было бы враньём.
        let raw: serde_json::Value = serde_json::from_str(V2_SAMPLE).unwrap();
        let cfg = migrate_v2(&raw).unwrap();
        assert!(cfg.seeded);
    }

    #[test]
    fn migration_defaults_behaviour_to_how_the_app_already_behaved() {
        // Обе настройки по умолчанию true — это ровно то, как приложение
        // вело себя до появления переключателей. Обновление не должно
        // менять поведение под человеком.
        let raw: serde_json::Value = serde_json::from_str(V2_SAMPLE).unwrap();
        let cfg = migrate_v2(&raw).unwrap();
        assert!(cfg.behaviour.close_to_tray);
        assert!(cfg.behaviour.tray_on_launch);
    }

    #[test]
    fn migration_preserves_last_played_and_hub_url() {
        let raw: serde_json::Value = serde_json::from_str(V2_SAMPLE).unwrap();
        let cfg = migrate_v2(&raw).unwrap();
        assert_eq!(cfg.last_played.as_deref(), Some("genshin"));
        assert_eq!(cfg.hub_url, None);
    }

    #[test]
    fn a_v3_config_round_trips_without_loss() {
        let cfg = AppConfig {
            version: 3,
            hub_url: Some("https://example.test/hub.json".into()),
            last_played: Some("zzz".into()),
            seeded: true,
            behaviour: Behaviour { close_to_tray: false, tray_on_launch: true },
            games: vec![],
        };
        let text = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.version, 3);
        assert!(!back.behaviour.close_to_tray);
        assert!(back.behaviour.tray_on_launch);
        assert!(back.seeded);
    }

    #[test]
    fn a_config_missing_the_new_fields_still_loads() {
        // Файл, записанный до появления полей, не должен ронять разбор.
        let json = r#"{"version":3,"games":[]}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.seeded);
        assert!(cfg.behaviour.close_to_tray);
    }
}
