mod catalog;
mod commands;
mod config;
mod detect;
mod hub;
mod launch;
mod vdf;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(detect::Cache::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_games,
            commands::get_config,
            commands::get_config_dir,
            commands::save_config,
            commands::launch_game,
            commands::get_hub,
            commands::detect_installs,
            commands::rescan_installs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
