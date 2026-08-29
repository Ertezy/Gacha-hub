//! Соответствие «установленная игра → идентификатор контента».
//!
//! Каталог НЕ знает, как игру запускать: это говорят манифесты магазинов.
//! Он отвечает только на вопрос, показывать ли по этой игре коды, баннеры
//! и видео, и под каким идентификатором искать их в файле хаба.
//!
//! На этапе 1 таблица встроенная. На этапе 2 она переезжает в файл хаба,
//! чтобы поддержка новой игры не требовала обновления приложения, —
//! сигнатура `content_id_for` при этом не меняется.

use crate::config::Launch;
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
}
