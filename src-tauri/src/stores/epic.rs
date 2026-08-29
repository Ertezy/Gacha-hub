//! Epic Games Launcher ведёт по файлу `*.item` на каждую установленную вещь
//! в `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests`.
//!
//! Записи с пустым `LaunchExecutable` — это DLC, а не игры; их отбрасываем.
//! Три идентификатора из манифеста складываются в официальную ссылку запуска,
//! поэтому пользователю не нужно ничего вводить руками.

use std::path::PathBuf;

use serde::Deserialize;

use super::{InstalledGame, Launch, Source};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Manifest {
    display_name: String,
    install_location: String,
    launch_executable: String,
    catalog_namespace: String,
    catalog_item_id: String,
    app_name: String,
}

/// Одна игра из содержимого файла `.item`.
pub fn game_from_manifest(json: &str) -> Option<InstalledGame> {
    let m: Manifest = serde_json::from_str(json).ok()?;
    if m.launch_executable.trim().is_empty() {
        return None;
    }
    let install_path = PathBuf::from(&m.install_location);
    Some(InstalledGame {
        title: m.display_name,
        exe_path: Some(install_path.join(&m.launch_executable)),
        install_path,
        launch: Launch::Epic {
            namespace: m.catalog_namespace,
            catalog_item_id: m.catalog_item_id,
            app_name: m.app_name,
        },
        source: Source::Epic,
    })
}

fn manifests_dir() -> Option<PathBuf> {
    let program_data = std::env::var_os("PROGRAMDATA")?;
    Some(
        PathBuf::from(program_data)
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests"),
    )
}

/// Всё, что Epic считает установленным.
pub fn installed() -> Vec<InstalledGame> {
    let Some(dir) = manifests_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut games = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_item = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("item"))
            .unwrap_or(false);
        if !is_item {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(game) = game_from_manifest(&text) {
            games.push(game);
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENSHIN: &str = r#"{
      "DisplayName": "Genshin Impact",
      "InstallLocation": "D:\\Games\\GenshinImpact",
      "LaunchExecutable": "launcher_epic.exe",
      "CatalogNamespace": "879b0d8776ab46a59a129983ba78f0ce",
      "CatalogItemId": "7d690c122fde4c60bed85405f343ad10",
      "AppName": "41869934302e4b8cafac2d3c0e7c293d"
    }"#;

    const DLC: &str = r#"{
      "DisplayName": "Civilization VI : Aztec DLC",
      "InstallLocation": "C:\\Program Files\\Epic Games\\SidMeiersCivilizationVI",
      "LaunchExecutable": "",
      "CatalogNamespace": "cd14dcaa4f3443f19f7169a980559c62",
      "CatalogItemId": "cd9e44a9d1b14b8d84923bb985bc1636",
      "AppName": "KingletAztec"
    }"#;

    #[test]
    fn parses_a_real_manifest() {
        let game = game_from_manifest(GENSHIN).expect("должен разобраться");
        assert_eq!(game.title, "Genshin Impact");
        assert_eq!(game.install_path, PathBuf::from(r"D:\Games\GenshinImpact"));
        assert_eq!(
            game.exe_path,
            Some(PathBuf::from(r"D:\Games\GenshinImpact\launcher_epic.exe"))
        );
        assert!(matches!(game.source, Source::Epic));
    }

    #[test]
    fn keeps_all_three_launch_identifiers() {
        let game = game_from_manifest(GENSHIN).unwrap();
        match game.launch {
            Launch::Epic {
                namespace,
                catalog_item_id,
                app_name,
            } => {
                assert_eq!(namespace, "879b0d8776ab46a59a129983ba78f0ce");
                assert_eq!(catalog_item_id, "7d690c122fde4c60bed85405f343ad10");
                assert_eq!(app_name, "41869934302e4b8cafac2d3c0e7c293d");
            }
            other => panic!("ожидался Epic, получено {other:?}"),
        }
    }

    #[test]
    fn skips_dlc_entries_that_have_no_executable() {
        assert!(game_from_manifest(DLC).is_none());
    }

    #[test]
    fn skips_malformed_json() {
        assert!(game_from_manifest("{ not json").is_none());
    }
}
