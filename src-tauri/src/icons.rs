//! Откуда взять иконку игры и где её хранить.
//!
//! Знает про игры и про файловую систему, но не знает, как устроен
//! исполняемый файл: этим занимается `pe`.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::config::Game;

/// Исполняемый файл, из которого берём иконку.
///
/// Источник картинки и способ запуска — вещи независимые, и это важно: игры из
/// Steam запускаются по номеру приложения и пути к файлу не имеют вовсе. Для
/// них картинка ищется в папке установки.
pub fn source_exe(game: &Game) -> Option<PathBuf> {
    if let Some(exe) = &game.exe_path {
        return Some(exe.clone());
    }
    let dir = game.install_path.as_deref()?;
    exe_in_root(dir, &game.title)
}

/// Файл в корне папки установки. **Только в корне, не вглубь** (спека §4):
/// поиск вглубь находит игровой бинарник на сотни мегабайт, установщик античита
/// и служебные программы стороннего набора.
fn exe_in_root(dir: &Path, title: &str) -> Option<PathBuf> {
    let wanted = crate::catalog::normalize(title);
    let mut biggest: Option<(u64, PathBuf)> = None;

    for entry in std::fs::read_dir(dir).ok()? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase)
            != Some("exe".to_string())
        {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if crate::catalog::normalize(stem) == wanted {
            return Some(path);
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if biggest.as_ref().is_none_or(|(b, _)| size > *b) {
            biggest = Some((size, path));
        }
    }
    biggest.map(|(_, p)| p)
}

/// Папка кеша иконок.
///
/// **Отдельно от кеша картинок хаба намеренно.** Тот чистится по возрасту при
/// каждом запуске (`images::evict`), и иконки оттуда постоянно вылетали бы и
/// добывались заново. Иконка не устаревает сама по себе — она устаревает
/// только вместе с файлом игры.
pub fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("нет папки кеша: {e}"))?
        .join("icons");
    std::fs::create_dir_all(&dir).map_err(|e| format!("не создать папку кеша иконок: {e}"))?;
    Ok(dir)
}

/// Имя файла в кеше: устойчивая свёртка пути. Пути содержат двоеточия и
/// обратные косые, именем файла они быть не могут.
fn file_name_for(exe: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in exe.to_string_lossy().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}.png")
}

/// Путь к иконке игры, добывая её при необходимости.
///
/// Порядок источников — спека §4. Своя картинка человека выигрывает у всего
/// остального: это осознанный выбор, и автоматика не имеет права его
/// переписать. Только если её нет — берём картинку из исполняемого файла.
///
/// **Возвращаемый путь всегда лежит в папке кеша, и это обязательно.** Окну
/// разрешено читать файлы только из перечисленных папок (`assetProtocol.scope`
/// в `tauri.conf.json`). Своя картинка человека лежит где угодно на диске, и
/// сослаться на неё напрямую нельзя — она просто не отобразилась бы, причём
/// молча. Поэтому её копия кладётся в кеш, а в конфиге хранится **исходный
/// путь**: он говорит, что человек выбрал, и переживает очистку кеша.
pub fn ensure(app: &AppHandle, game: &Game) -> Option<PathBuf> {
    let dir = cache_dir(app).ok()?;

    if let Some(path) = own_icon(&dir, game) {
        return Some(path);
    }

    let exe = source_exe(game)?;
    let cached = dir.join(file_name_for(&exe));

    if is_fresh(&cached, &exe) {
        return Some(cached);
    }

    let bytes = std::fs::read(&exe).ok()?;
    let png = crate::pe::icon_png_256(&bytes)?;
    if let Err(e) = std::fs::write(&cached, &png) {
        log::warn!("[icons] не записать иконку в кеш: {e}");
        return None;
    }
    Some(cached)
}

/// Своя картинка человека, скопированная в кеш — или `None`, если её нет,
/// файл на диске пропал, или копирование не удалось.
///
/// Принимает уже готовую папку кеша, а не `AppHandle`, по той же причине,
/// что и `is_fresh` ниже: `AppHandle` неоткуда взять в модульных тестах.
///
/// Имя копии в кеше зависит от **исходного пути** (`file_name_for`), а не от
/// идентификатора игры. Иначе смена картинки на файл старше уже лежащей
/// копии заставила бы `is_fresh` посчитать копию свежей и молча отдать
/// прежнюю картинку — то самое, что этот выбор человека переписывать не
/// имеет права. Ключ по пути даёт новой картинке новое имя в кеше, и подмены
/// произойти не может — так же, как у иконки из exe.
fn own_icon(dir: &Path, game: &Game) -> Option<PathBuf> {
    let own = game.icon.as_ref()?;
    if !own.is_file() {
        log::warn!("[icons] своя картинка не найдена: {}", own.display());
        return None;
    }

    let copy = dir.join(format!("custom-{}", file_name_for(own)));
    if is_fresh(&copy, own) {
        return Some(copy);
    }
    match std::fs::copy(own, &copy) {
        Ok(_) => Some(copy),
        Err(e) => {
            log::warn!("[icons] не скопировать свою картинку в кеш: {e}");
            None
        }
    }
}

/// Кеш годен, если он не старше исходного файла. Игра обновилась — файл
/// переписан, иконка добывается заново.
fn is_fresh(cached: &Path, exe: &Path) -> bool {
    let Ok(cached_time) = std::fs::metadata(cached).and_then(|m| m.modified()) else {
        return false;
    };
    let Ok(exe_time) = std::fs::metadata(exe).and_then(|m| m.modified()) else {
        return false;
    };
    cached_time >= exe_time
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Game, Launch};

    fn steam_game(install: Option<&str>) -> Game {
        Game {
            id: "wuthering".into(),
            title: "Wuthering Waves".into(),
            content_id: None,
            launch: Launch::Steam { appid: 3513350 },
            install_path: install.map(PathBuf::from),
            exe_path: None,
            args: String::new(),
            background: None,
            icon: None,
        }
    }

    #[test]
    fn a_game_with_its_own_exe_uses_that_exe() {
        let mut g = steam_game(None);
        g.exe_path = Some(PathBuf::from(r"C:\Games\Genshin\launcher.exe"));
        assert_eq!(source_exe(&g), Some(PathBuf::from(r"C:\Games\Genshin\launcher.exe")));
    }

    #[test]
    fn a_game_without_an_exe_and_without_a_folder_has_no_source() {
        assert_eq!(source_exe(&steam_game(None)), None);
    }

    #[test]
    fn a_matching_name_wins_over_a_bigger_file() {
        // Правило спеки §4: в корне побеждает файл, чьё имя совпадает с
        // названием игры, а не самый крупный.
        let dir = tempdir();
        write(&dir, "Wuthering Waves.exe", 1_000);
        write(&dir, "Redist.exe", 50_000);
        let g = steam_game(Some(dir.to_str().unwrap()));
        assert_eq!(source_exe(&g), Some(dir.join("Wuthering Waves.exe")));
    }

    #[test]
    fn without_a_matching_name_the_biggest_file_wins() {
        let dir = tempdir();
        write(&dir, "aaa.exe", 1_000);
        write(&dir, "bbb.exe", 50_000);
        let g = steam_game(Some(dir.to_str().unwrap()));
        assert_eq!(source_exe(&g), Some(dir.join("bbb.exe")));
    }

    #[test]
    fn the_search_does_not_go_deeper_than_the_root() {
        // Вглубь искать нельзя: в папке Wuthering Waves так находится
        // Client-Win64-Shipping.exe на 930 МБ, установщик античита и
        // служебные программы стороннего набора.
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("Binaries")).unwrap();
        write(&dir.join("Binaries"), "Client-Win64-Shipping.exe", 900_000);
        let g = steam_game(Some(dir.to_str().unwrap()));
        assert_eq!(source_exe(&g), None);
    }

    #[test]
    fn a_folder_without_executables_has_no_source() {
        let dir = tempdir();
        write(&dir, "readme.txt", 10);
        let g = steam_game(Some(dir.to_str().unwrap()));
        assert_eq!(source_exe(&g), None);
    }

    #[test]
    fn a_custom_icon_that_exists_is_copied_into_the_cache() {
        // Своя картинка человека выигрывает у всего остального (спека §4), но
        // окну разрешено читать файлы только из папки кеша — сослаться на
        // исходный файл напрямую нельзя, поэтому путь должен указывать внутрь
        // кеша, а не совпадать с исходным.
        let cache = tempdir();
        let own_dir = tempdir();
        write(&own_dir, "art.png", 42);
        let own = own_dir.join("art.png");

        let mut g = steam_game(None);
        g.icon = Some(own.clone());

        let result = own_icon(&cache, &g).expect("своя картинка должна победить");
        assert_ne!(result, own, "путь должен указывать в кеш, а не на исходный файл");
        assert!(result.starts_with(&cache));
        assert!(result.is_file());
    }

    #[test]
    fn a_missing_custom_icon_falls_back_to_none() {
        let cache = tempdir();
        let mut g = steam_game(None);
        g.icon = Some(PathBuf::from(r"C:\nope\never\icon.png"));
        assert_eq!(own_icon(&cache, &g), None);
    }

    #[test]
    fn switching_the_custom_icon_to_an_older_file_still_gets_copied() {
        // Покрывает ветку "уже закешировано" — и именно там прячется дефект:
        // если копия кешируется по идентификатору игры, а не по исходному
        // пути, смена картинки на файл старше уже лежащей копии заставляет
        // is_fresh считать копию свежей и молча отдавать прежнюю картинку.
        // Обычный случай: человек возвращается к файлу, который скачал
        // давно, — и это ровно то, что "автоматика не имеет права
        // переписать".
        let cache = tempdir();
        let own_dir = tempdir();

        // "b" старше "a": обычный случай — файл скачан раньше.
        write(&own_dir, "b.png", 2);
        let b = own_dir.join("b.png");
        let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        filetime::set_file_mtime(&b, filetime::FileTime::from_system_time(long_ago)).unwrap();

        write(&own_dir, "a.png", 1);
        let a = own_dir.join("a.png");

        let mut g = steam_game(None);
        g.icon = Some(a.clone());
        let first = own_icon(&cache, &g).expect("должно скопировать a");
        let first_len = std::fs::metadata(&first).unwrap().len();

        // Человек выбирает другой файл — b, который старше уже лежащей копии.
        g.icon = Some(b.clone());
        let second =
            own_icon(&cache, &g).expect("должно скопировать b, а не отдать старую копию a");
        let second_len = std::fs::metadata(&second).unwrap().len();

        assert_ne!(second_len, first_len, "должно вернуться содержимое b, а не кеш от a");
    }

    fn tempdir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "gh-icons-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(dir: &Path, name: &str, size: usize) {
        std::fs::write(dir.join(name), vec![0u8; size]).unwrap();
    }
}
