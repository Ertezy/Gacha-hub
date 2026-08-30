//! Данные хаба: где их взять и в каком порядке.
//!
//! Источники по убыванию приоритета:
//!   1. удалённый файл по `hubUrl` (только https, 5 с, редиректы запрещены),
//!      результат кешируется в `%APPDATA%\<id>\hub_cache.json`;
//!   2. `%APPDATA%\<id>\hub.json` — ручная подмена без пересборки;
//!   3. `%APPDATA%\<id>\hub_cache.json` — последняя удачная загрузка;
//!   4. `resources/hub.json` из комплекта.
//!
//! Схема живёт в `schema.rs`; здесь только доставка.

pub mod schema;

// Плоский реэкспорт — часть публичного интерфейса модуля для команд и кеша
// картинок, которые появятся в следующих задачах этапа. Здесь их ещё никто
// не использует, поэтому без `allow` анализ мёртвого кода принял бы это за
// неиспользуемый импорт.
#[allow(unused_imports)]
pub use schema::{Banner, Code, HubData, HubGame, Match, Video};

use std::fs;
use std::io::Read;
use std::path::Path;
use tauri::{AppHandle, Manager};

const REMOTE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Не больше двух мегабайт: файл хаба — текст, всё крупнее либо ошибка,
/// либо попытка занять нам память.
const MAX_HUB_BYTES: usize = 2 * 1024 * 1024;

/// Годится ли адрес для запроса из приложения.
///
/// Проверяется на месте перед каждым запросом, а не подразумевается: адреса
/// приходят из недоверенного файла (спека §8.1), и `file://` прочитал бы
/// с диска человека всё, на что укажут.
pub fn is_safe_https(url: &str) -> bool {
    // Схема нечувствительна к регистру по RFC 3986, остальное — нет.
    // Ведущие пробелы не обрезаются: адрес с ними — уже подозрительный.
    url.len() > 8 && url[..8].eq_ignore_ascii_case("https://")
}

fn read_json_file(path: &Path) -> Option<HubData> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Блокирующая загрузка по https с коротким таймаутом.
/// Редиректы запрещены: иначе ответ https-адреса мог бы увести нас на http.
fn fetch_remote(url: &str) -> Option<HubData> {
    if !is_safe_https(url) {
        return None;
    }
    let resp = ureq::builder()
        .redirects(0)
        .build()
        .get(url)
        .timeout(REMOTE_TIMEOUT)
        .call()
        .ok()?;

    let mut body = String::new();
    // Потолок соблюдается при чтении: Content-Length пишет отправитель.
    resp.into_reader()
        .take(MAX_HUB_BYTES as u64)
        .read_to_string(&mut body)
        .ok()?;

    serde_json::from_str(&body).ok()
}

fn bundled(app: &AppHandle) -> Result<HubData, String> {
    let dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource dir: {e}"))?;
    let path = dir.join("resources/hub.json");
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut data: HubData =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    data.source = Some("bundled".to_string());
    Ok(data)
}

/// Данные хаба без единого сетевого запроса.
///
/// Нужна в `setup()`: каталог для сопоставления игр лежит в этом же файле, а
/// `setup()` выполняется до появления окна. Сетевой запрос оттуда заставил бы
/// окно ждать сеть там, где сейчас оно не ждёт.
///
/// В `setup()` вызывается начиная со следующей задачи этапа (перенос каталога
/// игр на данные хаба); пока не подключена — без `allow` это дало бы
/// предупреждение о неиспользуемой функции.
#[allow(dead_code)]
pub fn load_local(app: &AppHandle) -> HubData {
    if let Ok(cfg_dir) = app.path().app_config_dir() {
        for (name, label) in [("hub.json", "override"), ("hub_cache.json", "cache")] {
            if let Some(mut data) = read_json_file(&cfg_dir.join(name)) {
                data.source = Some(label.to_string());
                return data;
            }
        }
    }
    bundled(app).unwrap_or_default()
}

/// Данные хаба по полной цепочке источников (см. описание модуля).
pub fn load(app: &AppHandle, hub_url: Option<&str>) -> Result<HubData, String> {
    let cfg_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    let cache_path = cfg_dir.join("hub_cache.json");
    let override_path = cfg_dir.join("hub.json");

    if let Some(url) = hub_url.filter(|u| is_safe_https(u)) {
        if let Some(mut data) = fetch_remote(url) {
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let _ = fs::write(&cache_path, json);
            }
            data.source = Some("remote".to_string());
            return Ok(data);
        }
        if let Some(mut data) = read_json_file(&override_path) {
            data.source = Some("override".to_string());
            return Ok(data);
        }
        if let Some(mut data) = read_json_file(&cache_path) {
            data.source = Some("cache".to_string());
            return Ok(data);
        }
    } else if let Some(mut data) = read_json_file(&override_path) {
        data.source = Some("override".to_string());
        return Ok(data);
    }

    bundled(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_https() {
        assert!(is_safe_https("https://raw.githubusercontent.com/u/r/main/hub.json"));
    }

    #[test]
    fn rejects_http() {
        assert!(!is_safe_https("http://example.test/hub.json"));
    }

    #[test]
    fn rejects_local_files() {
        // Поле image приходит из недоверенного файла; file:// прочитал бы
        // с диска человека всё, на что укажут.
        assert!(!is_safe_https("file:///C:/Windows/win.ini"));
        assert!(!is_safe_https(r"C:\Windows\win.ini"));
    }

    #[test]
    fn rejects_schemeless_and_empty() {
        assert!(!is_safe_https("example.test/hub.json"));
        assert!(!is_safe_https(""));
    }

    #[test]
    fn rejects_case_tricks_around_the_scheme() {
        // Схема сравнивается без учёта регистра, но всё, что за ней, — нет.
        assert!(is_safe_https("HTTPS://example.test/a.png"));
        assert!(!is_safe_https(" https://example.test/a.png"));
    }
}
