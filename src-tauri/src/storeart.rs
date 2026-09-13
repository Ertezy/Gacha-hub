//! Картинки игр, которые магазины уже держат у человека (спека этапа 5, §3).
//!
//! Кеши Steam и Epic не рассчитаны на чужие программы и могут поменять
//! устройство после обновления магазина. Отсюда правило всего модуля: любое
//! расхождение с ожидаемым видом даёт `None`, а не панику и не ошибку наружу.

use std::path::PathBuf;
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
}
