//! Картинки игр, которые магазины уже держат у человека (спека этапа 5, §3).
//!
//! Кеши Steam и Epic не рассчитаны на чужие программы и могут поменять
//! устройство после обновления магазина. Отсюда правило всего модуля: любое
//! расхождение с ожидаемым видом даёт `None`, а не панику и не ошибку наружу.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

// подключается в задаче 6
#[allow(dead_code)]
const STEAM_HERO: &str = "library_hero.jpg";

/// Широкая картинка библиотеки Steam для игры, самая свежая из найденных.
///
/// Раскладок кеша две, поддерживаются обе (спека §3.1): новая — папка по номеру
/// игры с подпапками внутри, старая — файл с номером в имени прямо в
/// `librarycache`.
// подключается в задаче 6
#[allow(dead_code)]
pub fn steam_hero(steam_roots: &[PathBuf], appid: u32) -> Option<PathBuf> {
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    for root in steam_roots {
        let cache = root.join("appcache").join("librarycache");
        remember(&mut found, cache.join(format!("{appid}_{STEAM_HERO}")));
        let Ok(entries) = std::fs::read_dir(cache.join(appid.to_string())) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remember(&mut found, path.join(STEAM_HERO));
            } else if path.file_name().is_some_and(|n| n == STEAM_HERO) {
                remember(&mut found, path);
            }
        }
    }
    found.into_iter().max_by_key(|(t, _)| *t).map(|(_, p)| p)
}

/// Запоминает файл вместе с датой изменения, если он есть и это файл.
// подключается в задаче 6
#[allow(dead_code)]
fn remember(found: &mut Vec<(SystemTime, PathBuf)>, path: PathBuf) {
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.is_file() {
            found.push((meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), path));
        }
    }
}

/// Потолок чтения каталога Epic. У владельца файл весит 600 КБ на четыре сотни
/// позиций; тридцать два мегабайта — запас на библиотеку в десятки раз больше, а
/// не приглашение держать в памяти всё, что туда положат.
// подключается в задаче 6
#[allow(dead_code)]
const MAX_CATALOG_BYTES: u64 = 32 * 1024 * 1024;

/// Где Epic хранит каталог магазина.
// подключается в задаче 6
#[allow(dead_code)]
pub fn epic_catalog_path() -> Option<PathBuf> {
    let program_data = std::env::var_os("PROGRAMDATA")?;
    Some(
        PathBuf::from(program_data)
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Catalog")
            .join("catcache.bin"),
    )
}

/// Каталог Epic с диска, с потолком размера.
// подключается в задаче 6
#[allow(dead_code)]
pub fn read_epic_catalog(path: &Path) -> Option<serde_json::Value> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_CATALOG_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_CATALOG_BYTES {
        log::warn!("[storeart] каталог Epic больше потолка, не читаю");
        return None;
    }
    parse_epic_catalog(&bytes)
}

/// Каталог Epic — это JSON-массив, записанный в base64.
// подключается в задаче 6
#[allow(dead_code)]
pub fn parse_epic_catalog(bytes: &[u8]) -> Option<serde_json::Value> {
    let json = decode_base64(bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&json).ok()?;
    value.is_array().then_some(value)
}

/// Адрес широкой картинки игры (тип `DieselGameBox`, 2560 на 1440) — или `None`,
/// если игры в каталоге нет, картинки нет или адресу нельзя доверять.
// подключается в задаче 6
#[allow(dead_code)]
pub fn epic_key_image_url(catalog: &serde_json::Value, catalog_item_id: &str) -> Option<String> {
    let item = catalog
        .as_array()?
        .iter()
        .find(|i| i.get("id").and_then(|v| v.as_str()) == Some(catalog_item_id))?;
    let url = item
        .get("keyImages")?
        .as_array()?
        .iter()
        .find(|k| k.get("type").and_then(|v| v.as_str()) == Some("DieselGameBox"))?
        .get("url")?
        .as_str()?;
    is_epic_image_url(url).then(|| url.to_string())
}

/// Адрес годен, только если он https и ведёт на хост Epic (спека §3.2).
///
/// Хост берётся до первого `/`, `?` или `#` и обязан состоять только из
/// латинских букв, цифр, дефиса и точки, а порт — только из цифр. Разрешённый
/// набор, а не список запрещённых символов: при скачивании адрес читается по
/// стандарту WHATWG, и любой символ, который эта функция и настоящий разборщик
/// поймут по-разному, превращается в подмену хоста. Так `\` работает как `/`
/// в `https://evil.test\wide.epicgames.com/`, а двоеточие с `@` делают из
/// `https://cdn1.epicgames.com:x@evil.test/` учётные данные и хост `evil.test`.
// подключается в задаче 6
#[allow(dead_code)]
fn is_epic_image_url(url: &str) -> bool {
    if !crate::hub::is_safe_https(url) {
        return false;
    }
    let Some(rest) = url.get("https://".len()..) else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if port.is_some_and(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    if !host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.') {
        return false;
    }
    const SUFFIX: &str = ".epicgames.com";
    let host = host.to_ascii_lowercase();
    host.len() > SUFFIX.len() && host.ends_with(SUFFIX)
}

/// Разбор base64 по RFC 4648, стандартный алфавит.
///
/// Пробелы и переводы строк пропускаются. Любой другой посторонний символ,
/// неполная четвёрка или `=` не в конце — `None`. Своя функция, а не
/// библиотека: новых зависимостей в проекте не добавляется.
// подключается в задаче 6
#[allow(dead_code)]
fn decode_base64(input: &[u8]) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let clean: Vec<u8> = input.iter().copied().filter(|c| !c.is_ascii_whitespace()).collect();
    if clean.len() % 4 != 0 {
        return None;
    }
    let groups = clean.len() / 4;
    let mut out = Vec::with_capacity(groups * 3);
    for (i, chunk) in clean.chunks(4).enumerate() {
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && i + 1 != groups) {
            return None;
        }
        let mut n: u32 = 0;
        for &c in &chunk[..4 - pad] {
            n = (n << 6) | value(c)?;
        }
        n <<= 6 * pad as u32;
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gh-storeart-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn put(path: &Path, secs: i64) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"jpg").unwrap();
        filetime::set_file_mtime(path, filetime::FileTime::from_unix_time(secs, 0)).unwrap();
    }

    #[test]
    fn finds_the_hero_in_the_new_layout() {
        let root = tempdir("new");
        let hero = root.join(r"appcache\librarycache\3513350\de0f5fcf\library_hero.jpg");
        put(&hero, 100);
        assert_eq!(steam_hero(&[root.clone()], 3513350), Some(hero));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn finds_the_hero_in_the_old_layout() {
        let root = tempdir("old");
        let hero = root.join(r"appcache\librarycache\3513350_library_hero.jpg");
        put(&hero, 100);
        assert_eq!(steam_hero(&[root.clone()], 3513350), Some(hero));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_newest_of_several_wins() {
        let root = tempdir("newest");
        let older = root.join(r"appcache\librarycache\7\aaaa\library_hero.jpg");
        let newer = root.join(r"appcache\librarycache\7\bbbb\library_hero.jpg");
        put(&older, 100);
        put(&newer, 200);
        assert_eq!(steam_hero(&[root.clone()], 7), Some(newer));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn another_games_hero_is_not_taken() {
        let root = tempdir("other");
        put(&root.join(r"appcache\librarycache\1\aaaa\library_hero.jpg"), 100);
        assert_eq!(steam_hero(&[root.clone()], 2), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_missing_steam_folder_gives_nothing() {
        assert_eq!(steam_hero(&[PathBuf::from(r"C:\nope\never")], 1), None);
    }

    #[test]
    fn base64_matches_the_rfc_examples() {
        // Примеры из RFC 4648, раздел 10.
        for (enc, dec) in [
            ("", ""),
            ("Zg==", "f"),
            ("Zm8=", "fo"),
            ("Zm9v", "foo"),
            ("Zm9vYg==", "foob"),
            ("Zm9vYmE=", "fooba"),
            ("Zm9vYmFy", "foobar"),
        ] {
            assert_eq!(decode_base64(enc.as_bytes()).as_deref(), Some(dec.as_bytes()), "{enc}");
        }
    }

    #[test]
    fn base64_skips_line_breaks() {
        assert_eq!(decode_base64(b"Zm9v\r\nYmFy").as_deref(), Some(&b"foobar"[..]));
    }

    #[test]
    fn base64_refuses_broken_input() {
        for bad in ["Zg=", "Z===", "Zg==Zm9v", "Zm9v!", "Z=g="] {
            assert_eq!(decode_base64(bad.as_bytes()), None, "{bad}");
        }
    }

    fn catalog() -> serde_json::Value {
        serde_json::json!([
            { "id": "other", "keyImages": [
                { "type": "DieselGameBox", "url": "https://cdn1.epicgames.com/other.jpg" }
            ]},
            { "id": "6838c695288a4fcea4486285edebcfdf", "keyImages": [
                { "type": "DieselGameBoxTall", "url": "https://cdn1.epicgames.com/tall.jpg" },
                { "type": "DieselGameBox", "url": "https://cdn1.epicgames.com/wide.jpg" }
            ]},
            { "id": "no-wide", "keyImages": [
                { "type": "DieselGameBoxTall", "url": "https://cdn1.epicgames.com/tall.jpg" }
            ]},
            { "id": "foreign", "keyImages": [
                { "type": "DieselGameBox", "url": "https://evil.test/wide.jpg" }
            ]}
        ])
    }

    #[test]
    fn the_wide_image_of_the_right_game_is_chosen() {
        assert_eq!(
            epic_key_image_url(&catalog(), "6838c695288a4fcea4486285edebcfdf"),
            Some("https://cdn1.epicgames.com/wide.jpg".to_string())
        );
    }

    #[test]
    fn an_unknown_game_or_one_without_the_wide_image_has_none() {
        assert_eq!(epic_key_image_url(&catalog(), "missing"), None);
        assert_eq!(epic_key_image_url(&catalog(), "no-wide"), None);
    }

    #[test]
    fn a_foreign_address_in_the_catalog_is_ignored() {
        // Файл каталога лежит в общей папке, писать туда может любая программа.
        assert_eq!(epic_key_image_url(&catalog(), "foreign"), None);
    }

    #[test]
    fn only_https_addresses_on_epic_hosts_are_trusted() {
        assert!(is_epic_image_url("https://cdn1.epicgames.com/a.jpg"));
        assert!(is_epic_image_url("https://CDN1.EpicGames.com:443/a.jpg"));
        for bad in [
            "http://cdn1.epicgames.com/a.jpg",
            "https://evil.test/a.jpg",
            "https://cdn1.epicgames.com.evil.test/a.jpg",
            "https://cdn1.epicgames.com@evil.test/a.jpg",
            "https://evil.test/cdn1.epicgames.com",
            "https://.epicgames.com/a.jpg",
            "https://epicgames.com/a.jpg",
            "https://evil.test\\wide.epicgames.com/a.jpg",
            "https://cdn1.epicgames.com:x@evil.test/a.jpg",
            "https://cdn1.epicgames.com:/a.jpg",
            "https://evil.test\t.epicgames.com/a.jpg",
            "https://evil.test%2F.epicgames.com/a.jpg",
            "https:///a.jpg",
            "https://:443/a.jpg",
        ] {
            assert!(!is_epic_image_url(bad), "{bad}");
        }
    }

    #[test]
    fn a_catalog_must_be_base64_of_a_json_array() {
        // «W10=» — это «[]», «e30=» — это «{}».
        assert_eq!(parse_epic_catalog(b"W10="), Some(serde_json::json!([])));
        assert_eq!(parse_epic_catalog(b"e30="), None);
        assert_eq!(parse_epic_catalog(b""), None);
        assert_eq!(parse_epic_catalog(b"!!!!"), None);
    }

    #[test]
    fn odd_catalog_shapes_do_not_panic() {
        assert_eq!(epic_key_image_url(&serde_json::json!({ "id": "x" }), "x"), None);
        assert_eq!(epic_key_image_url(&serde_json::json!([{ "id": "x", "keyImages": "nope" }]), "x"), None);
        assert_eq!(
            epic_key_image_url(&serde_json::json!([{ "id": "x", "keyImages": [{ "type": "DieselGameBox" }] }]), "x"),
            None
        );
    }

    #[test]
    fn an_oversized_catalog_file_is_not_read() {
        let dir = tempdir("oversized");
        let path = dir.join("catcache.bin");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_CATALOG_BYTES + 1).unwrap();
        assert_eq!(read_epic_catalog(&path), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}
