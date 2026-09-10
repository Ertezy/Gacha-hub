//! Перенос картинки, выбранной человеком, в папку кеша.
//!
//! Зачем копия, а не ссылка на файл по месту: окну разрешено читать файлы
//! только из папок, перечисленных в `assetProtocol.scope` (`tauri.conf.json`).
//! Картинка человека лежит где угодно на диске, и сослаться на неё напрямую
//! нельзя — она просто не отобразилась бы, причём молча. В конфиге при этом
//! хранится **исходный путь**: он говорит, что человек выбрал, и переживает
//! очистку кеша.
//!
//! Модуль общий для иконок и фонов намеренно. Два похожих переноса неизбежно
//! разошлись бы, и один из них унаследовал бы ошибку, которую в другом уже
//! починили.

use std::path::{Path, PathBuf};

/// Имя файла в кеше: устойчивая свёртка **исходного пути**.
///
/// Ключ обязан зависеть от источника. Если ключевать по чему-то другому — по
/// игре, скажем, — то смена картинки на файл с более старой датой молча не
/// применится: копия окажется «свежее» нового источника.
pub fn file_name_for(path: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}.img")
}

/// Копия годна, если она не старше источника.
pub fn is_fresh(cached: &Path, source: &Path) -> bool {
    let Ok(cached_time) = std::fs::metadata(cached).and_then(|m| m.modified()) else {
        return false;
    };
    let Ok(source_time) = std::fs::metadata(source).and_then(|m| m.modified()) else {
        return false;
    };
    cached_time >= source_time
}

/// Кладёт копию `source` в `dir` и отдаёт путь к ней.
///
/// `prefix` разделяет назначения внутри одной папки. Отказ тихий: в журнал
/// уходит строка, наружу — `None`, приложение работает дальше.
pub fn copy_into(dir: &Path, source: &Path, prefix: &str) -> Option<PathBuf> {
    if !source.is_file() {
        log::warn!("[localcopy] файл не найден: {}", source.display());
        return None;
    }
    let copy = dir.join(format!("{prefix}{}", file_name_for(source)));
    if is_fresh(&copy, source) {
        return Some(copy);
    }
    match std::fs::copy(source, &copy) {
        Ok(_) => Some(copy),
        Err(e) => {
            log::warn!("[localcopy] не скопировать в кеш: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_follows_the_source_path() {
        // Имя в кеше выводится из пути, а не из чего-то, что от источника не
        // зависит. Иначе смена картинки на файл постарше молча не применится —
        // ровно это уже ловилось на иконках.
        let a = file_name_for(Path::new(r"C:\one\art.png"));
        let b = file_name_for(Path::new(r"C:\two\art.png"));
        assert_ne!(a, b);
        assert_eq!(a, file_name_for(Path::new(r"C:\one\art.png")));
    }

    #[test]
    fn the_name_has_no_path_characters() {
        let name = file_name_for(Path::new(r"C:\Games\My Game\art.png"));
        assert!(!name.contains('\\') && !name.contains('/') && !name.contains(':'));
    }

    #[test]
    fn a_missing_source_is_not_copied() {
        let dir = tempdir("missing");
        assert_eq!(copy_into(&dir, Path::new(r"C:\nope\never.png"), "x-"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_copy_lands_inside_the_given_folder() {
        let dir = tempdir("lands");
        let src = dir.join("src.png");
        std::fs::write(&src, b"first").unwrap();
        let copy = copy_into(&dir, &src, "p-").expect("копия должна получиться");
        assert!(copy.starts_with(&dir));
        assert!(copy.file_name().unwrap().to_string_lossy().starts_with("p-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn switching_to_an_older_source_still_copies() {
        // Тот же случай, что чинился на иконках: файл, лежащий на диске давно,
        // не должен проиграть более свежей копии предыдущего выбора.
        let dir = tempdir("older");
        let a = dir.join("a.png");
        let b = dir.join("b.png");
        std::fs::write(&a, b"aaaa").unwrap();
        std::fs::write(&b, b"bb").unwrap();
        let older = filetime::FileTime::from_unix_time(1, 0);
        filetime::set_file_mtime(&b, older).unwrap();

        let first = copy_into(&dir, &a, "p-").unwrap();
        let second = copy_into(&dir, &b, "p-").unwrap();
        assert_ne!(first, second, "разные источники — разные файлы в кеше");
        assert_eq!(std::fs::read(&second).unwrap(), b"bb");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gh-localcopy-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
