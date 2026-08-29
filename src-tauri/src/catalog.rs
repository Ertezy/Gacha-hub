//! Соответствие «установленная игра → идентификатор контента».
//!
//! Каталог НЕ знает, как игру запускать: это говорят манифесты магазинов.
//! Он отвечает только на вопрос, показывать ли по этой игре коды, баннеры
//! и видео, и под каким идентификатором искать их в файле хаба.
//!
//! На этапе 1 таблица встроенная. На этапе 2 она переезжает в файл хаба,
//! чтобы поддержка новой игры не требовала обновления приложения, —
//! сигнатура `content_id_for` при этом не меняется.

use crate::config::{Game, Launch};
use crate::stores::InstalledGame;

struct Known {
    content_id: &'static str,
    /// Официальные названия, как их пишут магазины. Сравниваются нормализованно.
    titles: &'static [&'static str],
    /// Steam appid, если игра там есть. Приоритетнее названия.
    steam_appids: &'static [u32],
}

static KNOWN: &[Known] = &[
    Known {
        content_id: "genshin",
        titles: &["Genshin Impact"],
        steam_appids: &[],
    },
    Known {
        content_id: "hsr",
        titles: &["Honkai: Star Rail", "Honkai Star Rail"],
        steam_appids: &[],
    },
    Known {
        content_id: "zzz",
        titles: &["Zenless Zone Zero"],
        steam_appids: &[],
    },
    Known {
        content_id: "wuthering",
        titles: &["Wuthering Waves"],
        // Проверено на реальной установке: appmanifest_3513350.acf.
        steam_appids: &[3513350],
    },
    Known {
        content_id: "endfield",
        titles: &["Arknights: Endfield", "Arknights Endfield"],
        steam_appids: &[],
    },
];

/// Приводит название к виду, устойчивому к пунктуации, регистру и пробелам.
pub fn normalize(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Идентификатор контента для найденной игры, если она нам знакома.
pub fn content_id_for(game: &InstalledGame) -> Option<&'static str> {
    if let Launch::Steam { appid } = game.launch {
        if let Some(known) = KNOWN.iter().find(|k| k.steam_appids.contains(&appid)) {
            return Some(known.content_id);
        }
    }
    let normalized = normalize(&game.title);
    KNOWN
        .iter()
        .find(|k| k.titles.iter().any(|t| normalize(t) == normalized))
        .map(|k| k.content_id)
}

/// Находит установленную игру, соответствующую записи пользователя: сперва
/// по идентификатору контента, если он есть, иначе — по нормализованному
/// названию.
fn find_installed<'a>(game: &Game, installed: &'a [InstalledGame]) -> Option<&'a InstalledGame> {
    if let Some(cid) = game.content_id.as_deref() {
        if let Some(found) = installed.iter().find(|ig| content_id_for(ig) == Some(cid)) {
            return Some(found);
        }
    }
    let normalized = normalize(&game.title);
    installed
        .iter()
        .find(|ig| normalize(&ig.title) == normalized)
}

/// Дополняет игры пользователя данными из манифестов магазинов.
/// Возвращает true, если что-то изменилось.
///
/// Пишет только то, что пусто: способ запуска и пути подставляются лишь для
/// записи, у которой ещё нет `exePath` (например, только что перенесённой из
/// v1, где эти данные взять было неоткуда). Если пользователь уже указал
/// свой exe — в том числе намеренно направив его в папку другой игры к
/// общему лаунчеру — это решение не трогаем.
pub fn enrich_from_stores(games: &mut [Game], installed: &[InstalledGame]) -> bool {
    let mut changed = false;

    for game in games.iter_mut() {
        let Some(found) = find_installed(game, installed) else {
            continue;
        };

        // Отпечаток записи, перенесённой из v1: title там равен id, потому
        // что ключ словаря стал и тем, и другим.
        if game.title == game.id && game.title != found.title {
            game.title = found.title.clone();
            changed = true;
        }

        if matches!(game.launch, Launch::Exe) && game.exe_path.is_none() {
            let install_path = Some(found.install_path.clone());
            if game.launch != found.launch
                || game.install_path != install_path
                || game.exe_path != found.exe_path
            {
                game.launch = found.launch.clone();
                game.install_path = install_path;
                game.exe_path = found.exe_path.clone();
                changed = true;
            }
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::{InstalledGame, Source};
    use std::path::PathBuf;

    fn game(title: &str, launch: Launch, source: Source) -> InstalledGame {
        InstalledGame {
            title: title.to_string(),
            install_path: PathBuf::from(r"C:\x"),
            exe_path: None,
            launch,
            source,
        }
    }

    #[test]
    fn normalizes_punctuation_and_case() {
        assert_eq!(normalize("Honkai: Star Rail"), "honkaistarrail");
        assert_eq!(normalize("Arknights: Endfield"), "arknightsendfield");
        assert_eq!(normalize("  ZENLESS  ZONE  ZERO "), "zenlesszonezero");
    }

    #[test]
    fn matches_a_known_game_by_title() {
        let g = game(
            "Honkai: Star Rail",
            Launch::Epic {
                namespace: "n".into(),
                catalog_item_id: "c".into(),
                app_name: "a".into(),
            },
            Source::Epic,
        );
        assert_eq!(content_id_for(&g), Some("hsr"));
    }

    #[test]
    fn matches_wuthering_waves_by_steam_appid_even_if_renamed() {
        let g = game("Wuthering  Waves", Launch::Steam { appid: 3513350 }, Source::Steam);
        assert_eq!(content_id_for(&g), Some("wuthering"));
    }

    #[test]
    fn returns_none_for_a_game_we_know_nothing_about() {
        let g = game("Limbus Company", Launch::Steam { appid: 1973530 }, Source::Steam);
        assert_eq!(content_id_for(&g), None);
    }

    #[test]
    fn enrich_fills_launch_and_paths_for_a_migrated_game_matched_by_content_id() {
        // «title == id» — отпечаток записи, перенесённой из v1: там ключ
        // словаря стал одновременно id и title.
        let mut games = vec![crate::config::Game {
            id: "hsr".into(),
            title: "hsr".into(),
            content_id: Some("hsr".into()),
            launch: Launch::Exe,
            install_path: None,
            exe_path: None,
            args: String::new(),
            background: None,
        }];
        let installed = vec![InstalledGame {
            title: "Honkai: Star Rail".into(),
            install_path: PathBuf::from(r"C:\Games\HSR"),
            exe_path: Some(PathBuf::from(r"C:\Games\HSR\launcher_epic.exe")),
            launch: Launch::Epic {
                namespace: "n".into(),
                catalog_item_id: "c".into(),
                app_name: "a".into(),
            },
            source: Source::Epic,
        }];

        let changed = enrich_from_stores(&mut games, &installed);

        assert!(changed);
        assert_eq!(games[0].title, "Honkai: Star Rail");
        assert!(matches!(games[0].launch, Launch::Epic { .. }));
        assert_eq!(games[0].install_path, Some(PathBuf::from(r"C:\Games\HSR")));
        assert_eq!(
            games[0].exe_path,
            Some(PathBuf::from(r"C:\Games\HSR\launcher_epic.exe"))
        );
    }

    #[test]
    fn enrich_leaves_a_game_with_an_exe_path_already_set_untouched() {
        // Пользователь намеренно указал путь — возможно, на общий лаунчер
        // в папке другой игры. Перезаписывать это молча нельзя.
        let mut games = vec![crate::config::Game {
            id: "wuthering".into(),
            title: "Wuthering Waves".into(),
            content_id: Some("wuthering".into()),
            launch: Launch::Exe,
            install_path: Some(PathBuf::from(r"D:\Games\WutheringWaves")),
            exe_path: Some(PathBuf::from(r"D:\Games\WutheringWaves\launcher_epic.exe")),
            args: String::new(),
            background: None,
        }];
        let installed = vec![InstalledGame {
            title: "Wuthering Waves".into(),
            install_path: PathBuf::from(r"C:\SteamLibrary\WutheringWaves"),
            exe_path: None,
            launch: Launch::Steam { appid: 3513350 },
            source: Source::Steam,
        }];

        let changed = enrich_from_stores(&mut games, &installed);

        assert!(!changed);
        assert!(matches!(games[0].launch, Launch::Exe));
        assert_eq!(
            games[0].install_path,
            Some(PathBuf::from(r"D:\Games\WutheringWaves"))
        );
        assert_eq!(
            games[0].exe_path,
            Some(PathBuf::from(r"D:\Games\WutheringWaves\launcher_epic.exe"))
        );
    }

    #[test]
    fn enrich_leaves_an_unmatched_game_completely_untouched() {
        let mut games = vec![crate::config::Game {
            id: "limbus".into(),
            title: "limbus".into(),
            content_id: Some("limbus".into()),
            launch: Launch::Exe,
            install_path: None,
            exe_path: None,
            args: String::new(),
            background: None,
        }];
        let installed: Vec<InstalledGame> = Vec::new();

        let changed = enrich_from_stores(&mut games, &installed);

        assert!(!changed);
        assert_eq!(games[0].title, "limbus");
        assert!(matches!(games[0].launch, Launch::Exe));
        assert_eq!(games[0].exe_path, None);
    }

    #[test]
    fn enrich_returns_false_when_a_matched_game_needs_no_changes() {
        let mut games = vec![crate::config::Game {
            id: "wuthering".into(),
            title: "Wuthering Waves".into(),
            content_id: Some("wuthering".into()),
            launch: Launch::Steam { appid: 3513350 },
            install_path: Some(PathBuf::from(r"C:\SteamLibrary\WutheringWaves")),
            exe_path: None,
            args: String::new(),
            background: None,
        }];
        let installed = vec![InstalledGame {
            title: "Wuthering Waves".into(),
            install_path: PathBuf::from(r"C:\SteamLibrary\WutheringWaves"),
            exe_path: None,
            launch: Launch::Steam { appid: 3513350 },
            source: Source::Steam,
        }];

        assert!(!enrich_from_stores(&mut games, &installed));
    }
}
