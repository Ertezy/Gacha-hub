//! Чтение списков установленного из собственных манифестов магазинов.
//!
//! Магазины сами ведут машинно-читаемый учёт того, что стоит на диске, —
//! это надёжнее, чем угадывать игру по имени папки.

pub mod epic;
pub mod steam;

use std::path::PathBuf;

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

impl Source {
    /// Подпись под названием игры на главном экране.
    pub fn label(&self) -> &'static str {
        match self {
            Source::Steam => "Steam",
            Source::Epic => "Epic Games",
        }
    }
}

/// Игра, найденная в манифесте магазина.
#[derive(Debug, Clone)]
pub struct InstalledGame {
    pub title: String,
    pub install_path: PathBuf,
    /// Известен только для Epic — там манифест прямо называет исполняемый файл.
    pub exe_path: Option<PathBuf>,
    pub launch: Launch,
    pub source: Source,
}

/// Всё установленное во всех магазинах.
pub fn installed() -> Vec<InstalledGame> {
    let mut all = steam::installed();
    all.extend(epic::installed());
    all
}
