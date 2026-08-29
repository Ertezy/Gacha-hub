mod catalog;
mod commands;
mod config;
mod hub;
mod launch;
mod stores;
mod vdf;

use config::Game;

/// При первом запуске список игр пуст. Заполняем его тем, что нашли в
/// манифестах магазинов, чтобы человек сразу увидел свои игры, а не пустоту.
/// Экран с галочками появится на этапе 3 и заменит это автозаполнение.
fn seed_games_if_empty(app: &tauri::AppHandle) {
    let mut cfg = config::load(app);
    if !cfg.games.is_empty() {
        return;
    }

    let mut used_ids = std::collections::HashSet::new();
    for found in stores::installed() {
        let content_id = catalog::content_id_for(&found);
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
            title: found.title,
            content_id: content_id.map(str::to_string),
            launch: found.launch,
            install_path: Some(found.install_path),
            exe_path: found.exe_path,
            args: String::new(),
            background: None,
        });
    }

    // Знакомые игры вперёд: ради них лаунчер и открывают.
    cfg.games.sort_by_key(|g| g.content_id.is_none());
    cfg.last_played = cfg.games.first().map(|g| g.id.clone());

    if let Err(e) = config::save(app, &cfg) {
        eprintln!("[setup] не удалось сохранить начальный конфиг: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            seed_games_if_empty(&app.handle().clone());
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
