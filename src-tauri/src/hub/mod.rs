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

/// Правда, если прочитанный текст не влез в потолок размера.
///
/// Вынесена отдельно, как и `images::exceeds_ceiling`: читаем `MAX_HUB_BYTES
/// + 1` байт и проверяем результат этой функции, а не наоборот — тогда
/// граница проверяется без файла размером в потолок на диске у теста.
fn exceeds_hub_ceiling(len: usize) -> bool {
    len as u64 > MAX_HUB_BYTES as u64
}

/// Читает и разбирает местный файл хаба (`hub.json`, `hub_cache.json`).
///
/// Потолок размера тот же, что и у сетевой загрузки: это те же данные,
/// разница только в источнике. Без потолка человек, подменивший `hub.json`
/// огромным файлом (случайно или нет), заставил бы приложение вычитывать его
/// целиком в память при каждом запуске.
fn read_json_file(path: &Path) -> Option<HubData> {
    let file = fs::File::open(path).ok()?;
    let mut text = String::new();
    file.take(MAX_HUB_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .ok()?;
    if exceeds_hub_ceiling(text.len()) {
        log::error!(
            "[hub] {} больше потолка в {MAX_HUB_BYTES} байт, файл пропущен",
            path.display()
        );
        return None;
    }
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

/// Местная цепочка без сети: сначала `override_path` (ручная подмена),
/// затем `cache_path` (последняя удачная загрузка). `None` — ни один файл не
/// прочитался, и вызывающему коду пора переходить на встроенный запасной
/// вариант.
///
/// Не берёт `AppHandle` и не решает, что делать при `None` — вынесена именно
/// поэтому: обе стороны цепочки, зависящие от `AppHandle` (сборка путей,
/// встроенный файл из комплекта), проверить обычным тестом нельзя, а саму
/// логику выбора источника — можно, на временных файлах.
fn read_local_chain(override_path: &Path, cache_path: &Path) -> Option<HubData> {
    for (path, label) in [(override_path, "override"), (cache_path, "cache")] {
        if let Some(mut data) = read_json_file(path) {
            data.source = Some(label.to_string());
            return Some(data);
        }
    }
    None
}

/// Данные хаба без единого сетевого запроса.
///
/// Нужна в `setup()`: каталог для сопоставления игр лежит в этом же файле, а
/// `setup()` выполняется до появления окна. Сетевой запрос оттуда заставил бы
/// окно ждать сеть там, где сейчас оно не ждёт.
pub fn load_local(app: &AppHandle) -> HubData {
    if let Ok(cfg_dir) = app.path().app_config_dir() {
        let override_path = cfg_dir.join("hub.json");
        let cache_path = cfg_dir.join("hub_cache.json");
        if let Some(data) = read_local_chain(&override_path, &cache_path) {
            return data;
        }
    }
    match bundled(app) {
        Ok(data) => data,
        Err(e) => {
            // Именно этот отказ оставляет каталог пустым на первом запуске.
            // Поведение не меняем — setup() не имеет права провалиться, — но
            // причина обязана быть видна, иначе следующий такой случай снова
            // будет выглядеть как «панель почему-то пустая».
            log::error!("[hub] не удалось прочитать файл из комплекта: {e}");
            HubData::default()
        }
    }
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
        if let Some(data) = read_local_chain(&override_path, &cache_path) {
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

    #[test]
    fn a_hub_file_exactly_at_the_ceiling_is_accepted() {
        // Проверяет ровно ту границу, на которой легко ошибиться на один
        // байт: `read_json_file` читает `MAX_HUB_BYTES + 1` байт и передаёт
        // длину сюда, а не наоборот. Ровно потолок обязан пройти.
        assert!(!exceeds_hub_ceiling(MAX_HUB_BYTES));
    }

    #[test]
    fn a_hub_file_one_byte_over_the_ceiling_is_rejected() {
        assert!(exceeds_hub_ceiling(MAX_HUB_BYTES + 1));
    }

    fn hub_json(version: u32) -> String {
        format!(r#"{{"version":{version}}}"#)
    }

    /// Отдельная временная папка на тест: тесты в этом файле пишут местные
    /// файлы на диск и выполняются в одном процессе параллельно, общий путь
    /// привёл бы к тому, что один тест читал бы файл, оставленный другим.
    fn temp_hub_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gh-hub-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn read_json_file_refuses_a_local_file_larger_than_the_ceiling() {
        // До этой правки потолок применялся только к сетевой загрузке:
        // `hub.json`/`hub_cache.json` читались целиком, без ограничения.
        let dir = temp_hub_dir("oversized");
        let path = dir.join("hub.json");
        std::fs::write(&path, "0".repeat(MAX_HUB_BYTES + 1)).unwrap();

        assert!(read_json_file(&path).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_json_file_accepts_a_local_file_within_the_ceiling() {
        let dir = temp_hub_dir("within-cap");
        let path = dir.join("hub.json");
        std::fs::write(&path, hub_json(7)).unwrap();

        let data = read_json_file(&path).expect("файл в пределах потолка обязан прочитаться");
        assert_eq!(data.version, 7);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_chain_prefers_the_override_file_over_the_cache() {
        let dir = temp_hub_dir("override-over-cache");
        let override_path = dir.join("hub.json");
        let cache_path = dir.join("hub_cache.json");
        std::fs::write(&override_path, hub_json(11)).unwrap();
        std::fs::write(&cache_path, hub_json(22)).unwrap();

        let data =
            read_local_chain(&override_path, &cache_path).expect("должен найтись override");
        assert_eq!(data.version, 11);
        assert_eq!(data.source.as_deref(), Some("override"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_chain_falls_back_to_the_cache_when_there_is_no_override() {
        let dir = temp_hub_dir("cache-fallback");
        let override_path = dir.join("hub.json");
        let cache_path = dir.join("hub_cache.json");
        std::fs::write(&cache_path, hub_json(22)).unwrap();

        let data = read_local_chain(&override_path, &cache_path).expect("должен найтись cache");
        assert_eq!(data.version, 22);
        assert_eq!(data.source.as_deref(), Some("cache"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_chain_skips_a_broken_override_and_falls_back_to_the_cache() {
        // Подмена может оказаться не JSON (человек редактировал руками и
        // ошибся) — это не повод остаться совсем без данных, пока рабочий
        // кеш есть.
        let dir = temp_hub_dir("broken-override");
        let override_path = dir.join("hub.json");
        let cache_path = dir.join("hub_cache.json");
        std::fs::write(&override_path, "это не json").unwrap();
        std::fs::write(&cache_path, hub_json(5)).unwrap();

        let data = read_local_chain(&override_path, &cache_path).expect("должен найтись cache");
        assert_eq!(data.version, 5);
        assert_eq!(data.source.as_deref(), Some("cache"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_chain_is_none_when_neither_file_exists() {
        // `None` — сигнал вызывающему коду перейти на встроенный запасной
        // вариант из комплекта. Ни один местный файл не обязан существовать:
        // это обычное состояние на свежей установке.
        let dir = temp_hub_dir("neither-exists");
        let override_path = dir.join("hub.json");
        let cache_path = dir.join("hub_cache.json");

        assert!(read_local_chain(&override_path, &cache_path).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
