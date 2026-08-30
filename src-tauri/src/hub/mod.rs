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

// Плоский реэкспорт — только то, что называют по имени за пределами модуля
// (не считая тестов, которым виден весь крейт и которые обращаются к
// остальным типам схемы через полный путь `hub::schema::…`): `HubData`
// возвращает команда `get_hub`, `HubGame` собирает `catalog` при
// сопоставлении установленного с каталогом хаба.
pub use schema::{HubData, HubGame};

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
    //
    // Сравнение идёт по БАЙТАМ, а не срезом строки. `url[..8]` на строке,
    // где восьмой байт попадает внутрь многобайтового символа, не вернёт
    // false, а вызовет панику: «https:/日本» — опечатка в один слеш плюс
    // неASCII-хост — роняет приложение. Адреса приходят из недоверенного
    // файла, так что вход не гипотетический.
    let Some(head) = url.as_bytes().get(..8) else {
        return false;
    };
    if !head.eq_ignore_ascii_case(b"https://") {
        return false;
    }
    // После схемы обязан быть хотя бы один символ хоста: «https:///путь»
    // иначе проходит воротами насквозь.
    url.as_bytes().get(8).is_some_and(|c| *c != b'/')
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
    fn the_scheme_is_case_insensitive_but_nothing_else_is() {
        assert!(is_safe_https("HTTPS://example.test/a.png"));
        assert!(!is_safe_https(" https://example.test/a.png"));
    }

    #[test]
    fn a_non_ascii_url_is_rejected_and_does_not_panic() {
        // Единственный способ этой функции сломаться: сравнение по срезу
        // строки паникует, когда восьмой байт попадает внутрь символа.
        assert!(!is_safe_https("https:/日本"));
        assert!(!is_safe_https("日日日x"));
        assert!(!is_safe_https("ааа€x"));
        // А правильный адрес с неASCII-хостом проходить обязан.
        assert!(is_safe_https("https://日本.test/арт.png"));
    }

    #[test]
    fn an_empty_host_is_rejected() {
        assert!(!is_safe_https("https:///etc/passwd"));
        assert!(!is_safe_https("https://"));
        assert!(is_safe_https("https://a"));
    }
}
