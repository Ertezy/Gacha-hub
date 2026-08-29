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

/// Всё установленное во всех магазинах.
pub fn installed() -> Vec<InstalledGame> {
    let mut all = steam::installed();
    all.extend(epic::installed());
    all
}
