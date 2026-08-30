//! Кеш картинок: арт баннеров и превью роликов.
//!
//! Качает Rust, интерфейсу отдаётся путь к локальному файлу. Иначе пришлось бы
//! разрешить странице ходить в интернет, а политика содержимого это запрещает.
//!
//! Адреса приходят из файла хаба, то есть извне (спека §8.1). Отсюда три
//! правила, каждое проверено тестом: имя файла выводится хешем, а не из
//! адреса; схема проверяется перед запросом; потолок размера соблюдается при
//! чтении, а не по заголовку.
//!
//! Модуль подключается к остальному коду в задаче 6 этапа (команда кеша
//! картинок и вызов уборки при старте). До этого ни одна публичная функция
//! не вызывается изнутри крейта, и анализ мёртвого кода принял бы за
//! неиспользуемый весь файл, кроме проверенного тестами.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Manager};

use crate::hub::is_safe_https;

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Пять мегабайт: уменьшенный арт весит десятки килобайт, всё крупнее —
/// либо ошибка сборщика, либо попытка занять нам диск.
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

/// Правда, если прочитанное тело не влезло в потолок размера.
///
/// Вынесена отдельно от `fetch`, потому что именно в этой границе легко
/// ошибиться на один байт: `fetch` читает `max + 1` байт и проверяет
/// результат этой функции, а не наоборот. Сетевой части `fetch` тест не
/// нужен — граница проверяется здесь напрямую, без сервера.
fn exceeds_ceiling(len: usize, max: u64) -> bool {
    len as u64 > max
}

/// Картинка, к которой не обращались столько, удаляется при следующем запуске.
/// Баннеры меняются каждые три недели, ролики каждый день; без уборки папка
/// за год превращается в свалку.
pub const MAX_AGE: Duration = Duration::from_secs(30 * 86400);

/// Расширение по типу содержимого, а не по адресу: адрес недоверенный, и
/// `.png` в конце ничего не доказывает. Неизвестный тип получает `bin`,
/// чтобы в кеше не оказалось ничего, что оболочка сочтёт исполняемым.
fn ext_for(content_type: &str) -> &'static str {
    let base = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match base.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

/// Имя файла в кеше: шестнадцатеричный хеш адреса плюс расширение.
///
/// Из адреса не берётся ни один символ. Это единственное, что защищает от
/// адреса вроде `https://ho.st/../../../evil.png`: собери мы имя из пути,
/// такой адрес записал бы файл за пределы папки кеша.
fn file_name_for(url: &str, ext: &str) -> String {
    // FNV-1a, 64 бита. Криптостойкость тут не нужна: задача не в защите от
    // подбора, а в том, чтобы имя не содержало ничего из адреса.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in url.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{h:016x}.{ext}")
}

pub fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("cache dir: {e}"))?
        .join("images");
    fs::create_dir_all(&dir).map_err(|e| format!("не удалось создать папку кеша: {e}"))?;
    Ok(dir)
}

/// Путь к картинке в кеше; скачивает, если её там нет.
pub fn fetch(app: &AppHandle, url: &str) -> Result<PathBuf, String> {
    if !is_safe_https(url) {
        return Err(format!("отказываюсь качать не-https адрес: {url}"));
    }
    let dir = cache_dir(app)?;

    // Один адрес может прийти под разными типами содержимого (перенастройка
    // CDN, смена картинки на той стороне), и тогда в кеше окажутся файлы с
    // одним хешем и разными расширениями. Берём самый свежий, лишние удаляем:
    // фиксированный порядок списка отдавал бы устаревший файл до самой уборки.
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    for ext in ["png", "jpg", "webp", "gif", "bin"] {
        let candidate = dir.join(file_name_for(url, ext));
        if let Ok(meta) = fs::metadata(&candidate) {
            if meta.is_file() {
                found.push((meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), candidate));
            }
        }
    }
    if !found.is_empty() {
        found.sort_by_key(|(t, _)| *t);
        let (_, newest) = found.pop().expect("проверено на непустоту");
        for (_, stale) in found {
            let _ = fs::remove_file(stale);
        }
        // Отодвигает файл в конец очереди на уборку.
        let _ = touch(&newest);
        return Ok(newest);
    }

    // Редиректов не больше двух и только внутри https.
    //
    // `.redirects(N)` пропускает N-1 переходов (ureq считает
    // `history.len() + 1 >= redirects`), поэтому для двух ставим 3.
    // `https_only` обязателен отдельно: без него сервер, до которого мы дошли
    // по https, уводит редиректом на http, и ureq послушно идёт — адрес
    // приходит из недоверенного файла, проверять его один раз мало.
    let resp = ureq::builder()
        .redirects(3)
        .https_only(true)
        .build()
        .get(url)
        .timeout(FETCH_TIMEOUT)
        .call()
        .map_err(|e| format!("не удалось скачать {url}: {e}"))?;

    let ext = ext_for(resp.header("Content-Type").unwrap_or(""));

    // Читаем на байт больше потолка: если он прочитался, ответ не влез,
    // и записывать обрезок нельзя — он не раскодируется никогда, а цикл
    // повторного использования выше проверяет только наличие файла, так что
    // испорченный кеш сам не вылечится.
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("не удалось прочитать {url}: {e}"))?;
    if exceeds_ceiling(bytes.len(), MAX_IMAGE_BYTES) {
        return Err(format!("картинка больше {MAX_IMAGE_BYTES} байт: {url}"));
    }

    let path = dir.join(file_name_for(url, ext));
    // Проверка после сборки пути, а не до: убеждаемся в свойстве результата.
    if !path.starts_with(&dir) {
        return Err("путь вышел за пределы кеша".to_string());
    }
    fs::write(&path, &bytes).map_err(|e| format!("не удалось записать {}: {e}", path.display()))?;
    Ok(path)
}

/// Отодвигает метку изменения файла на «сейчас».
///
/// Используется именно метка изменения, а не доступа: на Windows обновление
/// времени доступа отключено по умолчанию (`NtfsDisableLastAccessUpdate`),
/// поэтому чтение файла его метку не двигает, и уборка удаляла бы картинки,
/// которые человек видит каждый день. Метку изменения мы ставим сами при
/// каждом попадании в кеш, и она надёжна на любой машине.
fn touch(path: &Path) -> std::io::Result<()> {
    let f = fs::OpenOptions::new().write(true).open(path)?;
    f.set_times(fs::FileTimes::new().set_modified(SystemTime::now()))?;
    Ok(())
}

/// Удаляет из папки файлы, к которым не обращались дольше `max_age`.
/// Возвращает количество удалённых.
pub fn evict(dir: &Path, max_age: Duration) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let now = SystemTime::now();
    let mut removed = 0;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let Ok(stamp) = meta.modified() else { continue };
        if now.duration_since(stamp).unwrap_or_default() > max_age
            && fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_url_always_gets_the_same_name() {
        let a = file_name_for("https://example.test/art.png", "png");
        let b = file_name_for("https://example.test/art.png", "png");
        assert_eq!(a, b);
    }

    #[test]
    fn different_urls_get_different_names() {
        let a = file_name_for("https://example.test/a.png", "png");
        let b = file_name_for("https://example.test/b.png", "png");
        assert_ne!(a, b);
    }

    #[test]
    fn the_name_never_carries_anything_from_the_url_path() {
        // Имя выводится хешем, а не из адреса: адрес приходит из недоверенного
        // файла, и подготовленная строка записала бы файл за пределы кеша.
        let name = file_name_for("https://example.test/../../../../Windows/evil.png", "png");
        assert!(!name.contains(".."), "имя не должно содержать переход вверх: {name}");
        assert!(!name.contains('/'), "имя не должно содержать разделитель: {name}");
        assert!(!name.contains('\\'), "имя не должно содержать разделитель: {name}");
        assert!(!name.contains("Windows"), "имя не должно нести путь: {name}");
        assert!(name.ends_with(".png"));
    }

    #[test]
    fn the_name_is_hex_plus_extension() {
        let name = file_name_for("https://example.test/a.png", "png");
        let stem = name.strip_suffix(".png").unwrap();
        assert!(stem.chars().all(|c| c.is_ascii_hexdigit()), "не шестнадцатеричное: {stem}");
        assert!(!stem.is_empty());
    }

    #[test]
    fn extension_comes_from_the_content_type_not_the_url() {
        assert_eq!(ext_for("image/png"), "png");
        assert_eq!(ext_for("image/jpeg"), "jpg");
        assert_eq!(ext_for("image/webp"), "webp");
        assert_eq!(ext_for("image/png; charset=binary"), "png");
        assert_eq!(ext_for("IMAGE/PNG"), "png");
    }

    #[test]
    fn an_unknown_content_type_is_not_cached_as_something_executable() {
        assert_eq!(ext_for("text/html"), "bin");
        assert_eq!(ext_for("application/octet-stream"), "bin");
        assert_eq!(ext_for(""), "bin");
    }

    #[test]
    fn the_built_path_stays_inside_the_cache_directory() {
        let dir = std::path::Path::new(r"C:\cache");
        let p = dir.join(file_name_for("https://example.test/../../evil.png", "png"));
        assert!(p.starts_with(dir), "путь ушёл из кеша: {}", p.display());
    }

    #[test]
    fn a_body_exactly_at_the_ceiling_is_accepted() {
        // Проверяет ровно ту границу, на которой легко ошибиться на один
        // байт: `fetch` читает `MAX_IMAGE_BYTES + 1` байт и передаёт длину
        // сюда, а не наоборот. Тело точно в потолок обязано пройти.
        assert!(!exceeds_ceiling(MAX_IMAGE_BYTES as usize, MAX_IMAGE_BYTES));
    }

    #[test]
    fn a_body_one_byte_over_the_ceiling_is_rejected() {
        // Ответ длиннее потолка ровно на один байт — то самое значение,
        // которое получилось бы прочитать `.take(MAX_IMAGE_BYTES + 1)`,
        // если сервер прислал больше положенного.
        assert!(exceeds_ceiling(MAX_IMAGE_BYTES as usize + 1, MAX_IMAGE_BYTES));
    }

    #[test]
    fn eviction_removes_only_what_is_older_than_the_limit() {
        let dir = std::env::temp_dir().join(format!("gh-evict-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("old.png");
        let fresh = dir.join("fresh.png");
        std::fs::write(&old, b"x").unwrap();
        std::fs::write(&fresh, b"y").unwrap();

        // Метка у старого отодвигается на 40 дней назад.
        let long_ago = std::time::SystemTime::now() - Duration::from_secs(40 * 86400);
        filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(long_ago)).unwrap();

        let removed = evict(&dir, Duration::from_secs(30 * 86400));
        assert_eq!(removed, 1);
        assert!(!old.exists());
        assert!(fresh.exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
