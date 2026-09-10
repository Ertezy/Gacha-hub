//! Фоновая картинка игры.
//!
//! Отдельно от иконок, потому что это другая вещь с другим жизненным циклом,
//! и класть их в одну папку значило бы путать при чистке.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::config::Game;

/// Папка кеша фонов.
///
/// Отдельно от кеша картинок хаба: тот чистится по возрасту при каждом
/// запуске, и копия фона оттуда вылетала бы, а исходник человека мог к тому
/// времени уже переехать.
pub fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("нет папки кеша: {e}"))?
        .join("art");
    std::fs::create_dir_all(&dir).map_err(|e| format!("не создать папку кеша арта: {e}"))?;
    Ok(dir)
}

/// Путь к фоновой картинке игры, готовый к показу в окне.
///
/// `None` — картинки нет, экран рисует сгенерированную заливку.
pub fn ensure(app: &AppHandle, game: &Game) -> Option<PathBuf> {
    let own = game.background.as_ref()?;
    let dir = cache_dir(app).ok()?;
    crate::localcopy::copy_into(&dir, own, "")
}
