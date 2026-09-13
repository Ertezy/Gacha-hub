//! Чтение списков установленного из собственных манифестов магазинов.
//!
//! Магазины сами ведут машинно-читаемый учёт того, что стоит на диске, —
//! это надёжнее, чем угадывать игру по имени папки.

pub mod epic;
pub mod steam;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Чем именно запускается игра.
///
/// Живёт здесь, а не в `config.rs`, потому что источник этого знания —
/// манифест магазина. `config.rs` переэкспортирует тип, чтобы остальной код
/// обращался к нему как к `config::Launch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Launch {
    /// `steam://rungameid/<appid>`
    Steam { appid: u32 },
    /// `com.epicgames.launcher://apps/<ns>%3A<cid>%3A<app>?action=launch&silent=true`
    Epic {
        namespace: String,
        catalog_item_id: String,
        app_name: String,
    },
    /// Прямой запуск `exe_path`.
    Exe,
}

/// Откуда узнали про игру.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Steam,
    Epic,
}

/// Игра, найденная в манифесте магазина.
#[derive(Debug, Clone)]
pub struct InstalledGame {
    pub title: String,
    pub install_path: PathBuf,
    /// Известен только для Epic — там манифест прямо называет исполняемый файл.
    pub exe_path: Option<PathBuf>,
    pub launch: Launch,
    /// Пока не читается: пригодится на этапе 3 для выбора между несколькими
    /// установками одной игры (§6.6 — магазин приоритетнее найденной по имени
    /// папки).
    #[allow(dead_code)]
    pub source: Source,
}

/// Магазин, из которого установлена игра, и её номер там (спека этапа 5, §2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreRef {
    Steam { appid: u32 },
    Epic { catalog_item_id: String },
}

fn store_ref(launch: &Launch) -> Option<StoreRef> {
    match launch {
        Launch::Steam { appid } => Some(StoreRef::Steam { appid: *appid }),
        Launch::Epic { catalog_item_id, .. } => Some(StoreRef::Epic {
            catalog_item_id: catalog_item_id.clone(),
        }),
        Launch::Exe => None,
    }
}

/// Лежит ли `child` внутри `parent` или совпадает с ним.
///
/// Сравнение по частям пути и без учёта регистра латиницы: на Windows
/// `C:\Games` и `c:\games` — одна папка, а строковое «начинается с» приняло бы
/// `D:\Game2` за содержимое `D:\Game`. Пустой `parent` не содержит ничего.
#[allow(dead_code)] // подключается в задаче 6
fn is_inside(child: &Path, parent: &Path) -> bool {
    if parent.as_os_str().is_empty() {
        return false;
    }
    let mut child_parts = child.components();
    parent.components().all(|p| {
        child_parts.next().is_some_and(|c| {
            c.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&p.as_os_str().to_string_lossy())
        })
    })
}

/// Из какого магазина установлена игра.
///
/// Способ запуска говорит это прямо. Если игра запускается напрямую, её файл
/// ищется внутри папок, которые магазины записали в свои манифесты: у владельца
/// так записан Arknights: Endfield, установленный из Epic.
#[allow(dead_code)] // подключается в задаче 6
pub fn store_of(game: &crate::config::Game, installed: &[InstalledGame]) -> Option<StoreRef> {
    if let Some(found) = store_ref(&game.launch) {
        return Some(found);
    }
    let own = game.exe_path.as_deref().or(game.install_path.as_deref())?;
    installed
        .iter()
        .find(|i| is_inside(own, &i.install_path))
        .and_then(|i| store_ref(&i.launch))
}

/// Всё установленное во всех магазинах.
pub fn installed() -> Vec<InstalledGame> {
    let mut all = steam::installed();
    all.extend(epic::installed());
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Game;

    fn exe_game(exe: &str) -> Game {
        let mut g = Game::manual("g".into(), "G".into());
        g.exe_path = Some(PathBuf::from(exe));
        g
    }

    fn installed_at(path: &str, launch: Launch) -> InstalledGame {
        InstalledGame {
            title: "X".into(),
            install_path: PathBuf::from(path),
            exe_path: None,
            launch,
            source: Source::Epic,
        }
    }

    fn epic(id: &str) -> Launch {
        Launch::Epic {
            namespace: "ns".into(),
            catalog_item_id: id.into(),
            app_name: "app".into(),
        }
    }

    #[test]
    fn the_launch_kind_names_the_store_directly() {
        let mut steam = Game::manual("s".into(), "S".into());
        steam.launch = Launch::Steam { appid: 3513350 };
        assert_eq!(store_of(&steam, &[]), Some(StoreRef::Steam { appid: 3513350 }));

        let mut from_epic = Game::manual("e".into(), "E".into());
        from_epic.launch = epic("7d690c122fde4c60bed85405f343ad10");
        assert_eq!(
            store_of(&from_epic, &[]),
            Some(StoreRef::Epic { catalog_item_id: "7d690c122fde4c60bed85405f343ad10".into() })
        );
    }

    #[test]
    fn a_direct_launch_inside_an_epic_install_is_from_epic() {
        // Случай владельца: Arknights: Endfield записан как прямой запуск, хотя
        // установлен из Epic (спека §2.2).
        let game = exe_game(r"C:\Program Files\Epic Games\ArknightsEndfieldgowoU\Launcher.exe");
        let installed = [installed_at(
            r"C:\Program Files\Epic Games\ArknightsEndfieldgowoU",
            epic("6838c695288a4fcea4486285edebcfdf"),
        )];
        assert_eq!(
            store_of(&game, &installed),
            Some(StoreRef::Epic { catalog_item_id: "6838c695288a4fcea4486285edebcfdf".into() })
        );
    }

    #[test]
    fn a_direct_launch_inside_a_steam_install_is_from_steam() {
        let game = exe_game(r"D:\SteamLibrary\steamapps\common\Some Game\game.exe");
        let installed = [installed_at(
            r"D:\SteamLibrary\steamapps\common\Some Game",
            Launch::Steam { appid: 42 },
        )];
        assert_eq!(store_of(&game, &installed), Some(StoreRef::Steam { appid: 42 }));
    }

    #[test]
    fn letter_case_in_the_path_does_not_matter() {
        let game = exe_game(r"c:\program files\epic games\zzz\launcher_epic.exe");
        let installed = [installed_at(r"C:\Program Files\Epic Games\ZZZ", epic("id"))];
        assert_eq!(
            store_of(&game, &installed),
            Some(StoreRef::Epic { catalog_item_id: "id".into() })
        );
    }

    #[test]
    fn a_folder_with_a_longer_name_is_not_inside() {
        // Строковое «начинается с» приняло бы Game2 за содержимое Game.
        let game = exe_game(r"D:\Games\Game2\run.exe");
        let installed = [installed_at(r"D:\Games\Game", Launch::Steam { appid: 1 })];
        assert_eq!(store_of(&game, &installed), None);
    }

    #[test]
    fn a_game_from_an_arbitrary_folder_has_no_store() {
        let game = exe_game(r"E:\Portable\thing.exe");
        let installed = [installed_at(r"D:\Games\Game", Launch::Steam { appid: 1 })];
        assert_eq!(store_of(&game, &installed), None);
    }

    #[test]
    fn an_empty_install_path_matches_nothing() {
        // Пустой путь не имеет частей, и без отдельной проверки «лежит внутри»
        // было бы правдой для любого файла на диске.
        let game = exe_game(r"D:\Games\Any\run.exe");
        let installed = [installed_at("", Launch::Steam { appid: 1 })];
        assert_eq!(store_of(&game, &installed), None);
    }
}
