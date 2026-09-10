//! Команды — единственный мост между интерфейсом и системой.
//! Все обращения к файлам, реестру и оболочке живут здесь и глубже.
//!
//! Все команды асинхронные: синхронные в Tauri выполняются в главном потоке
//! и подмораживали бы окно на чтении файлов и запуске процессов.

use serde::Serialize;
use tauri::{AppHandle, Manager};

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
    /// Путь к файлу иконки на диске. `None` — рисуется заглушка с буквой.
    pub icon_path: Option<String>,
    /// Путь к фоновой картинке. `None` — рисуется сгенерированная заливка.
    pub art_path: Option<String>,
    /// Аргументы запуска. Действуют только при прямом запуске — Steam и Epic
    /// открывают ссылку магазина и передать их игре не могут (`launch.rs`).
    pub args: String,
    /// Файл или папка игры пропали с диска.
    pub missing: bool,
}

/// Часть представления игры, не требующая `AppHandle`.
///
/// Выделена отдельной функцией, чтобы её можно было проверять в модульных
/// тестах напрямую: `AppHandle` там взять неоткуда (тот же приём, что и у
/// `is_fresh` в `icons.rs` или `reject_if_newer_than_current` в `config.rs`).
/// `icon_path` и `art_path` здесь всегда `None` — их выставляет только
/// `view_of`.
fn view_of_without_icon(game: &Game) -> GameView {
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
        icon_path: None,
        art_path: None,
        args: game.args.clone(),
        missing: !config::is_present(game),
    }
}

pub fn view_of(app: &AppHandle, game: &Game) -> GameView {
    GameView {
        icon_path: crate::icons::ensure(app, game).map(|p| p.to_string_lossy().into_owned()),
        art_path: crate::art::ensure(app, game).map(|p| p.to_string_lossy().into_owned()),
        ..view_of_without_icon(game)
    }
}

#[tauri::command]
pub async fn get_games(app: AppHandle) -> Vec<GameView> {
    config::load(&app).games.iter().map(|g| view_of(&app, g)).collect()
}

#[tauri::command]
pub async fn get_config(app: AppHandle) -> config::AppConfig {
    config::load(&app)
}

#[tauri::command]
pub async fn get_config_dir(app: AppHandle) -> Result<String, String> {
    Ok(config::config_dir(&app)?.to_string_lossy().into_owned())
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
        log::error!("[config] не удалось сохранить lastPlayed: {e}");
    }

    // Прячем окно, только если значок в трее на месте: иначе спрятанное окно
    // стало бы нечем вернуть.
    if cfg.behaviour.tray_on_launch && app.tray_by_id("main").is_some() {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
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

/// Путь к картинке в локальном кеше; качает, если её там нет.
///
/// Интерфейс получает путь к файлу, а не адрес: страница в интернет не ходит.
#[tauri::command]
pub async fn cache_image(app: AppHandle, url: String) -> Result<String, String> {
    let path = crate::images::fetch(&app, &url)?;
    Ok(path.to_string_lossy().into_owned())
}

/// Найденная в магазинах игра — для экрана с галочками.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundGame {
    pub title: String,
    /// Идентификатор контента, если игру знает каталог. `None` — по ней не
    /// будет ни кодов, ни баннеров, и на экране это подписывается честно.
    pub content_id: Option<String>,
    pub source_label: String,
    /// Игра с таким путём уже есть в конфиге — галочку ставить не нужно.
    pub already_added: bool,
}

/// Что нашлось в магазинах. **Конфиг не меняется** — это только предложение,
/// человек сам решает, что добавить.
#[tauri::command]
pub async fn scan_installed(app: AppHandle) -> Vec<FoundGame> {
    let cfg = config::load(&app);
    let hub_games = crate::hub::load_local(&app).games;

    crate::stores::installed()
        .into_iter()
        .map(|found| {
            let content_id = crate::catalog::content_id_for(&found, &hub_games);
            let source_label = match found.launch {
                Launch::Steam { .. } => "Steam",
                Launch::Epic { .. } => "Epic Games",
                Launch::Exe => "напрямую",
            }
            .to_string();
            // То же сравнение, что решает, какую запись пользователя
            // дополнить данными установки (`catalog::already_configured`
            // переиспользует его же) — а не отдельное по exePath: у игр
            // Steam он всегда `None` (см. `stores::steam`), и такое
            // сравнение никогда не находило бы совпадение.
            let already_added = crate::catalog::already_configured(&found, &cfg.games, &hub_games);
            FoundGame {
                title: found.title,
                content_id,
                source_label,
                already_added,
            }
        })
        .collect()
}

/// Добавить игру, найденную в магазинах, по её названию.
///
/// Отдельная команда, а не `add_game`: у найденной игры уже известны способ
/// запуска и пути из манифеста, и терять их, сводя всё к `Launch::Exe`,
/// нельзя — тогда игра из Steam перестала бы запускаться через Steam.
#[tauri::command]
pub async fn add_game_from_scan(app: AppHandle, title: String) -> Result<String, String> {
    let hub_games = crate::hub::load_local(&app).games;
    let found = crate::stores::installed()
        .into_iter()
        .find(|g| g.title == title)
        .ok_or_else(|| format!("игра больше не найдена: {title}"))?;

    let content_id = crate::catalog::content_id_for(&found, &hub_games);
    let mut cfg = config::load(&app);
    let id = crate::library::add_found(
        &mut cfg,
        found.title,
        content_id,
        found.launch,
        found.install_path,
        found.exe_path,
    )?;
    config::save(&app, &cfg)?;
    Ok(id)
}

/// Пометить, что первый запуск состоялся — независимо от того, добавил ли
/// человек что-то с экрана предложений или нажал «Пропустить». Пропуск это
/// тоже осознанное решение, и повторно спрашивать нельзя.
#[tauri::command]
pub async fn mark_seeded(app: AppHandle) -> Result<(), String> {
    let mut cfg = config::load(&app);
    cfg.seeded = true;
    config::save(&app, &cfg)
}

/// Нужно ли показать экран первого запуска вместо главного экрана.
#[tauri::command]
pub async fn needs_first_run(app: AppHandle) -> bool {
    !config::load(&app).seeded
}

/// Общий помощник: прочитать конфиг, изменить, записать.
fn with_config<F>(app: &AppHandle, f: F) -> Result<(), String>
where
    F: FnOnce(&mut config::AppConfig) -> bool,
{
    let mut cfg = config::load(app);
    if !f(&mut cfg) {
        return Err("игра не найдена или правка невозможна".to_string());
    }
    config::save(app, &cfg)
}

#[tauri::command]
pub async fn add_game(app: AppHandle, title: String, exe: String) -> Result<String, String> {
    let exe = std::path::PathBuf::from(exe);
    // Путь вводит человек, значит это недоверенный ввод: проверяем, что файл
    // существует, до того как записать его в конфиг.
    if !exe.is_file() {
        return Err(format!("файла нет: {}", exe.display()));
    }
    // Название тоже вводит человек — проверяем перед записью в конфиг.
    crate::library::validate_title(&title)?;
    let mut cfg = config::load(&app);
    let id = crate::library::add(&mut cfg, title, exe);
    config::save(&app, &cfg)?;
    Ok(id)
}

#[tauri::command]
pub async fn update_game(
    app: AppHandle,
    game_id: String,
    title: Option<String>,
    exe: Option<String>,
    background: Option<String>,
    // Пустая строка означает «убрать свою картинку»: null при переходе из
    // JSON неотличим от «поле не передали».
    icon: Option<String>,
    // Пустая строка означает «стереть привязку»: null для этого не годится,
    // потому что при переходе из JSON он неотличим от «поле не передали».
    content_id: Option<String>,
    // Один уровень, как у `title`: `None` — не трогать, `Some` — заменить
    // целиком. Пустая строка здесь не признак стирания, а обычное значение
    // «нет аргументов».
    args: Option<String>,
) -> Result<(), String> {
    let exe_path = match exe {
        Some(p) => {
            let p = std::path::PathBuf::from(p);
            if !p.is_file() {
                return Err(format!("файла нет: {}", p.display()));
            }
            Some(p)
        }
        None => None,
    };
    // Название вводит человек — проверяем перед записью в конфиг.
    if let Some(title_val) = &title {
        crate::library::validate_title(title_val)?;
    }
    let patch = crate::library::GamePatch {
        title,
        exe_path,
        background: background.map(std::path::PathBuf::from),
        icon: icon.map(|s| (!s.is_empty()).then(|| std::path::PathBuf::from(s))),
        content_id: content_id.map(|s| (!s.is_empty()).then_some(s)),
        args,
    };
    with_config(&app, |cfg| crate::library::update(cfg, &game_id, patch))
}

#[tauri::command]
pub async fn remove_game(app: AppHandle, game_id: String) -> Result<(), String> {
    with_config(&app, |cfg| crate::library::remove(cfg, &game_id))
}

#[tauri::command]
pub async fn reorder_games(app: AppHandle, ids: Vec<String>) -> Result<(), String> {
    with_config(&app, |cfg| crate::library::reorder(cfg, &ids))
}

#[tauri::command]
pub async fn relocate_game(app: AppHandle, game_id: String) -> Result<(), String> {
    let hub_games = crate::hub::load_local(&app).games;
    let installed = crate::stores::installed();
    with_config(&app, |cfg| {
        cfg.games
            .iter_mut()
            .find(|g| g.id == game_id)
            .map(|g| crate::catalog::relocate(g, &installed, &hub_games))
            .unwrap_or(false)
    })
}

#[tauri::command]
pub async fn get_behaviour(app: AppHandle) -> config::Behaviour {
    config::load(&app).behaviour
}

#[tauri::command]
pub async fn set_behaviour(
    app: AppHandle,
    close_to_tray: bool,
    tray_on_launch: bool,
) -> Result<(), String> {
    let mut cfg = config::load(&app);
    cfg.behaviour = config::Behaviour {
        close_to_tray,
        tray_on_launch,
    };
    config::save(&app, &cfg)
}

#[tauri::command]
pub async fn set_hub_url(app: AppHandle, url: Option<String>) -> Result<(), String> {
    // Пустая строка означает «нет адреса», а не адрес из пустой строки.
    let url = url.filter(|u| !u.trim().is_empty());
    if let Some(u) = &url {
        if !crate::hub::is_safe_https(u) {
            return Err("адрес должен начинаться с https://".to_string());
        }
    }
    let mut cfg = config::load(&app);
    cfg.hub_url = url;
    config::save(&app, &cfg)
}

/// Размер файлов в одной папке кеша, в байтах. Общая часть для картинок хаба
/// и добытых иконок — они лежат в разных папках (`icons::cache_dir`
/// отдельно от `images::cache_dir` намеренно, см. комментарий там), но с
/// точки зрения человека это один кеш с одной кнопкой очистки и одним
/// показанным размером.
fn dir_size(dir: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Удаляет файлы из одной папки кеша. Возвращает количество удалённых.
fn clear_dir(dir: &std::path::Path) -> Result<usize, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("не читается кеш: {e}"))?;
    let mut removed = 0;
    for entry in entries.flatten() {
        if entry.metadata().map(|m| m.is_file()).unwrap_or(false)
            && std::fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Размер кеша картинок в байтах — чтобы показать его рядом с кнопкой очистки.
/// Считает и картинки хаба, и добытые иконки: иначе смена своей иконки
/// оставляла бы прежнюю копию на диске навсегда, и её никто бы не увидел и
/// не удалил.
#[tauri::command]
pub async fn image_cache_size(app: AppHandle) -> u64 {
    [crate::images::cache_dir(&app), crate::icons::cache_dir(&app)]
        .into_iter()
        .filter_map(Result::ok)
        .map(|dir| dir_size(&dir))
        .sum()
}

/// Очищает кеш картинок хаба и кеш добытых иконок. После очистки иконки
/// добываются заново сами при следующем обращении к списку игр —
/// `icons::ensure` каждый раз проверяет, что файл в кеше есть и свеж, и
/// пересоздаёт его, если нет.
#[tauri::command]
pub async fn clear_image_cache(app: AppHandle) -> Result<usize, String> {
    let mut removed = 0;
    for dir in [crate::images::cache_dir(&app)?, crate::icons::cache_dir(&app)?] {
        removed += clear_dir(&dir)?;
    }
    Ok(removed)
}

/// Сведения для раздела «О программе».
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct About {
    pub version: String,
    pub log_path: String,
}

#[tauri::command]
pub async fn get_about(app: AppHandle) -> About {
    let log_path = app
        .path()
        .app_log_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    About {
        version: app.package_info().version.to_string(),
        log_path,
    }
}

/// Открыть папку журнала в проводнике.
#[tauri::command]
pub async fn open_log_folder(app: AppHandle) -> Result<(), String> {
    // `open_path` — метод на `Opener`, а не свободная функция; добраться до
    // него можно только через расширение `OpenerExt`.
    use tauri_plugin_opener::OpenerExt;

    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("не найдена папка журнала: {e}"))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("не удалось открыть папку: {e}"))
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
            icon: None,
        }
    }

    #[test]
    fn view_labels_the_store_a_game_starts_through() {
        assert_eq!(view_of_without_icon(&steam_game()).source_label, "Steam");
    }

    #[test]
    fn view_marks_a_game_whose_folder_disappeared() {
        assert!(view_of_without_icon(&steam_game()).missing);
    }

    #[test]
    fn exe_games_are_labelled_as_direct() {
        let mut g = steam_game();
        g.launch = Launch::Exe;
        assert_eq!(view_of_without_icon(&g).source_label, "напрямую");
    }

    /// Отдельная папка на тег теста, а не общий путь: тесты этого файла пишут
    /// на диск и выполняются в одном процессе параллельно.
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gh-cmd-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dir_size_sums_the_files_in_a_folder() {
        // Ровно то, что раньше не считало иконки: если бы папка иконок не
        // попадала в подсчёт, эта сумма не изменилась бы от файлов внутри.
        let dir = temp_dir("dirsize");
        std::fs::write(dir.join("a.png"), vec![0u8; 10]).unwrap();
        std::fs::write(dir.join("b.png"), vec![0u8; 5]).unwrap();
        assert_eq!(dir_size(&dir), 15);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dir_size_of_a_missing_folder_is_zero() {
        assert_eq!(dir_size(std::path::Path::new(r"C:\nope\never\missing")), 0);
    }

    #[test]
    fn clear_dir_removes_files_and_reports_the_count() {
        let dir = temp_dir("cleardir");
        std::fs::write(dir.join("a.png"), b"x").unwrap();
        std::fs::write(dir.join("b.png"), b"y").unwrap();
        assert_eq!(clear_dir(&dir).unwrap(), 2);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clear_dir_on_a_missing_folder_is_an_error_not_a_panic() {
        let err = clear_dir(std::path::Path::new(r"C:\nope\never\missing"));
        assert!(err.is_err());
    }
}
