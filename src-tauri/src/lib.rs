mod catalog;
mod commands;
mod config;
mod hub;
mod images;
mod launch;
mod library;
mod stores;
mod tray;
mod vdf;

use tauri::Manager;

/// Дополняет уже существующие записи данными из манифестов магазинов (важно
/// для тех, что перенеслись из v1 без данных о запуске: см.
/// `catalog::enrich_from_stores`). Список игр больше не заполняется здесь
/// автоматически — на первом запуске это делает экран с галочками, и
/// человек сам решает, что добавить.
fn sync_games_with_stores(app: &tauri::AppHandle) {
    let mut cfg = config::load(app);
    let installed = stores::installed();
    // Локальная копия, без сети: setup() выполняется до появления окна,
    // и сетевой запрос отсюда заставил бы окно ждать сеть (спека §3.4).
    let hub_games = hub::load_local(app).games;

    if catalog::enrich_from_stores(&mut cfg.games, &installed, &hub_games) {
        if let Err(e) = config::save(app, &cfg) {
            log::error!("[setup] не удалось сохранить конфиг: {e}");
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
        .plugin(
            tauri_plugin_log::Builder::new()
                // В файл и в терминал сразу: файл нужен людям, терминал — нам
                // при разработке.
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: None },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                // Журнал попадает к посторонним при разборе жалобы, поэтому
                // в нём только техническое: пути, коды ошибок, счётчики.
                .level(log::LevelFilter::Info)
                .max_file_size(512 * 1024)
                .build(),
        )
        .setup(|app| {
            sync_games_with_stores(&app.handle().clone());
            // Уборка кеша картинок при запуске: дёшево, и без неё папка
            // за год превращается в свалку.
            if let Ok(dir) = images::cache_dir(&app.handle().clone()) {
                let removed = images::evict(&dir, images::MAX_AGE);
                if removed > 0 {
                    log::info!("[images] убрано из кеша: {removed}");
                }
            }
            // Провал трея не должен мешать запуску: без значка крестик просто
            // закроет приложение вместо того, чтобы прятать его (см. tray.rs
            // и обработчик CloseRequested ниже).
            if let Err(e) = tray::setup(&app.handle().clone()) {
                log::error!("[tray] не удалось создать значок в трее: {e}");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Крестик прячет окно, а не закрывает приложение — но только
                // если значок в трее на месте. Иначе спрятанное окно стало бы
                // нечем вернуть.
                let has_tray = window.app_handle().tray_by_id("main").is_some();
                let cfg = crate::config::load(window.app_handle());
                if has_tray && cfg.behaviour.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::get_config,
            commands::get_config_dir,
            commands::launch_game,
            commands::get_hub,
            commands::get_last_played,
            commands::cache_image,
            commands::scan_installed,
            commands::add_game,
            commands::add_game_from_scan,
            commands::mark_seeded,
            commands::needs_first_run,
            commands::update_game,
            commands::remove_game,
            commands::reorder_games,
            commands::relocate_game,
            commands::get_behaviour,
            commands::set_behaviour,
            commands::set_hub_url,
            commands::image_cache_size,
            commands::clear_image_cache,
            commands::get_about,
            commands::open_log_folder
        ])
        .run(tauri::generate_context!())
        .expect("ошибка при запуске приложения");
}
