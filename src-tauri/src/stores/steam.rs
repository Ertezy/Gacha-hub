//! Steam: корень установки из реестра, список библиотек из
//! `steamapps/libraryfolders.vdf`, сами игры из `steamapps/appmanifest_*.acf`.

use std::path::{Path, PathBuf};

use super::{InstalledGame, Launch, Source};
use crate::vdf;

/// Корни установки самого Steam: сначала реестр (пользователь мог поставить
/// куда угодно), затем стандартные места.
pub fn install_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\Valve\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                if !path.trim().is_empty() {
                    roots.push(PathBuf::from(path));
                }
            }
        }

        // Пользовательская запись: не подвержена перенаправлению WOW64 и
        // остаётся на месте, когда машинной записи нет. Steam пишет сюда путь
        // через прямые слэши — PathBuf на Windows их понимает. Если оба
        // источника дали один и тот же каталог в разном написании, дубль стоит
        // одного лишнего чтения каталога: installed() всё равно отсеивает игры
        // по appid.
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam") {
            if let Ok(path) = key.get_value::<String, _>("SteamPath") {
                if !path.trim().is_empty() {
                    roots.push(PathBuf::from(path));
                }
            }
        }
    }

    roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    roots.push(PathBuf::from(r"C:\Program Files\Steam"));

    let mut seen = std::collections::HashSet::new();
    roots.retain(|p| seen.insert(p.clone()));
    roots
}

/// Библиотеки Steam: сам корень плюс всё, что перечислено в
/// `libraryfolders.vdf`. Пустой или нечитаемый файл — остаётся один корень.
pub fn library_roots_from_vdf(vdf_text: &str, steam_root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![steam_root.to_path_buf()];
    for path in vdf::all(&vdf::pairs(vdf_text), "path") {
        roots.push(PathBuf::from(path));
    }
    let mut seen = std::collections::HashSet::new();
    roots.retain(|p| seen.insert(p.clone()));
    roots
}

/// Одна игра из содержимого `appmanifest_*.acf`.
/// `library` — корень библиотеки, то есть папка, содержащая `steamapps`.
pub fn game_from_manifest(acf_text: &str, library: &Path) -> Option<InstalledGame> {
    let pairs = vdf::pairs(acf_text);
    let appid: u32 = vdf::first(&pairs, "appid")?.parse().ok()?;
    let title = vdf::first(&pairs, "name")?.to_string();
    let install_dir = vdf::first(&pairs, "installdir")?;

    Some(InstalledGame {
        title,
        install_path: library.join("steamapps").join("common").join(install_dir),
        exe_path: None,
        launch: Launch::Steam { appid },
        source: Source::Steam,
    })
}

/// Всё, что Steam считает установленным, во всех библиотеках.
pub fn installed() -> Vec<InstalledGame> {
    let mut games = Vec::new();
    let mut seen_appids = std::collections::HashSet::new();

    for steam_root in install_roots() {
        let steamapps = steam_root.join("steamapps");
        if !steamapps.is_dir() {
            continue;
        }
        let vdf_text = std::fs::read_to_string(steamapps.join("libraryfolders.vdf"))
            .unwrap_or_default();

        for library in library_roots_from_vdf(&vdf_text, &steam_root) {
            let dir = library.join("steamapps");
            let Ok(entries) = std::fs::read_dir(&dir) else {
                eprintln!("[steam] не могу прочитать библиотеку {:?}", dir);
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_manifest = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"))
                    .unwrap_or(false);
                if !is_manifest {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    eprintln!("[steam] не могу прочитать манифест {:?}", path);
                    continue;
                };
                if let Some(game) = game_from_manifest(&text, &library) {
                    if let Launch::Steam { appid } = game.launch {
                        if seen_appids.insert(appid) {
                            games.push(game);
                        }
                    }
                } else {
                    eprintln!("[steam] не могу разобрать манифест {:?}", path);
                }
            }
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_app_manifest() {
        let acf = r#""AppState"
{
	"appid"		"3513350"
	"name"		"Wuthering Waves"
	"installdir"		"Wuthering Waves"
}"#;
        let library = Path::new(r"C:\Program Files (x86)\Steam");
        let game = game_from_manifest(acf, library).expect("должен разобраться");

        assert_eq!(game.title, "Wuthering Waves");
        assert_eq!(
            game.install_path,
            Path::new(r"C:\Program Files (x86)\Steam\steamapps\common\Wuthering Waves")
        );
        assert!(matches!(game.launch, Launch::Steam { appid: 3513350 }));
        assert!(matches!(game.source, Source::Steam));
    }

    #[test]
    fn rejects_manifest_without_appid() {
        let acf = "\"AppState\"\n{\n\t\"name\"\t\t\"Broken\"\n}";
        assert!(game_from_manifest(acf, Path::new(r"C:\Steam")).is_none());
    }

    #[test]
    fn rejects_manifest_with_non_numeric_appid() {
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"abc\"\n\t\"name\"\t\t\"X\"\n\t\"installdir\"\t\t\"X\"\n}";
        assert!(game_from_manifest(acf, Path::new(r"C:\Steam")).is_none());
    }

    #[test]
    fn finds_every_library_including_other_drives() {
        let vdf_text = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}"#;
        let roots = library_roots_from_vdf(vdf_text, Path::new(r"C:\Program Files (x86)\Steam"));
        assert_eq!(
            roots,
            vec![
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"D:\SteamLibrary"),
            ]
        );
    }

    #[test]
    fn falls_back_to_the_steam_root_when_vdf_is_unreadable() {
        let roots = library_roots_from_vdf("", Path::new(r"C:\Steam"));
        assert_eq!(roots, vec![PathBuf::from(r"C:\Steam")]);
    }
}
