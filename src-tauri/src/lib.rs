mod catalog;
mod commands;
mod config;
mod hub;
mod images;
mod launch;
mod stores;
mod vdf;

use config::Game;

/// При первом запуске список игр пуст — заполняем его тем, что нашли в
/// манифестах магазинов, чтобы человек сразу увидел свои игры, а не пустоту.
/// Экран с галочками появится на этапе 3 и заменит это автозаполнение.
fn seed_games(cfg: &mut config::AppConfig, installed: &[stores::InstalledGame], hub_games: &[hub::HubGame]) {
    let mut used_ids = std::collections::HashSet::new();
    for found in installed {
        let content_id = catalog::content_id_for(found, hub_games);
        let base = content_id
            .clone()
            .unwrap_or_else(|| catalog::normalize(&found.title));
        let mut id = base.clone();
        let mut n = 2;
        while !used_ids.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }

        cfg.games.push(Game {
            id,
            title: found.title.clone(),
            content_id,
            launch: found.launch.clone(),
            install_path: Some(found.install_path.clone()),
            exe_path: found.exe_path.clone(),
            args: String::new(),
            background: None,
        });
    }

    // Знакомые игры вперёд: ради них лаунчер и открывают.
    cfg.games.sort_by_key(|g| g.content_id.is_none());
    cfg.last_played = cfg.games.first().map(|g| g.id.clone());
}

/// Синхронизирует конфиг со списком установленного: на первом запуске
/// заполняет список игр, на последующих — дополняет уже существующие записи
/// (важно для тех, что перенеслись из v1 без данных о запуске: см.
/// `catalog::enrich_from_stores`). `stores::installed()` вызывается ровно
/// один раз, поэтому оба сценария собраны в одну функцию.
fn sync_games_with_stores(app: &tauri::AppHandle) {
    let mut cfg = config::load(app);
    let installed = stores::installed();
    // Локальная копия, без сети: setup() выполняется до появления окна,
    // и сетевой запрос отсюда заставил бы окно ждать сеть (спека §3.4).
    let hub_games = hub::load_local(app).games;

    let changed = if cfg.games.is_empty() {
        seed_games(&mut cfg, &installed, &hub_games);
        true
    } else {
        catalog::enrich_from_stores(&mut cfg.games, &installed, &hub_games)
    };

    if changed {
        if let Err(e) = config::save(app, &cfg) {
            eprintln!("[setup] не удалось сохранить конфиг: {e}");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            sync_games_with_stores(&app.handle().clone());
            // Уборка кеша картинок при запуске: дёшево, и без неё папка
            // за год превращается в свалку.
            if let Ok(dir) = images::cache_dir(&app.handle().clone()) {
                let removed = images::evict(&dir, images::MAX_AGE);
                if removed > 0 {
                    eprintln!("[images] убрано из кеша: {removed}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::get_config,
            commands::get_config_dir,
            commands::launch_game,
            commands::get_hub,
            commands::get_last_played,
            commands::cache_image
        ])
        .run(tauri::generate_context!())
        .expect("ошибка при запуске приложения");
}
