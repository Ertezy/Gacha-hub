mod catalog;
mod commands;
mod config;
mod hub;
mod launch;
mod stores;
mod vdf;

use config::Game;

/// При первом запуске список игр пуст — заполняем его тем, что нашли в
/// манифестах магазинов, чтобы человек сразу увидел свои игры, а не пустоту.
/// Экран с галочками появится на этапе 3 и заменит это автозаполнение.
fn seed_games(cfg: &mut config::AppConfig, installed: &[stores::InstalledGame]) {
    let mut used_ids = std::collections::HashSet::new();
    for found in installed {
        let content_id = catalog::content_id_for(found);
        let base = content_id
            .map(str::to_string)
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
            content_id: content_id.map(str::to_string),
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

    let changed = if cfg.games.is_empty() {
        seed_games(&mut cfg, &installed);
        true
    } else {
        catalog::enrich_from_stores(&mut cfg.games, &installed)
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
        .setup(|app| {
            sync_games_with_stores(&app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::get_config,
            commands::get_config_dir,
            commands::select_game,
            commands::launch_game,
            commands::get_hub
        ])
        .run(tauri::generate_context!())
        .expect("ошибка при запуске приложения");
}
