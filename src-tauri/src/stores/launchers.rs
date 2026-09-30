//! Игры, поставленные через собственные лаунчеры: HoYoPlay, лаунчер Kuro
//! (Wuthering Waves) и GRYPHLINK (Arknights: Endfield).
//!
//! У этих лаунчеров нет манифестов, как у Steam и Epic, зато они пишут путь
//! установки в реестр Windows (а Kuro и GRYPHLINK ещё и кладут игру в папку по
//! умолчанию). Реестр читается через маленький трейт, чтобы искатели проверялись
//! на таблице в памяти, а не на чужом компьютере: настоящая реализация — winreg
//! под `#[cfg(windows)]`, как в `steam.rs`.
//!
//! Найденная игра запускается напрямую: `Launch::Exe` плюс `exe_path`, ровно как
//! игра, добавленная вручную, — новых полей в конфиге не нужно.

use std::path::{Path, PathBuf};

use super::{InstalledGame, Launch, Source};

/// Куст реестра.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    CurrentUser,
    LocalMachine,
}

/// То, что нужно искателям от реестра. Настоящая реализация — winreg, в тестах —
/// таблица в памяти.
pub trait Registry {
    /// Имена подразделов; пусто, если раздела нет.
    fn subkeys(&self, root: Root, path: &str) -> Vec<String>;
    /// Строковое значение; `None`, если его нет.
    fn value(&self, root: Root, path: &str, name: &str) -> Option<String>;
}

/// Разделы «Установка и удаление программ»: 64-битный и 32-битный вид машины и
/// раздел пользователя. Kuro и GRYPHLINK записываются в любой из них — смотря
/// как их поставили.
const UNINSTALL_ROOTS: [(Root, &str); 3] = [
    (Root::LocalMachine, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    (Root::LocalMachine, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    (Root::CurrentUser, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
];

/// Где HoYoPlay хранит установленные игры: глобальная и китайская редакции.
const HOYOPLAY_ROOTS: [&str; 2] = [r"Software\Cognosphere\HYP", r"Software\miHoYo\HYP"];

/// Ключ игры в HoYoPlay → название и исполняемый файл. Глобальная и китайская
/// редакции называют файл по-разному только у Genshin Impact.
const HOYOPLAY_GAMES: [(&str, &str, &str); 6] = [
    ("hk4e_global", "Genshin Impact", "GenshinImpact.exe"),
    ("hk4e_cn", "Genshin Impact", "YuanShen.exe"),
    ("hkrpg_global", "Honkai: Star Rail", "StarRail.exe"),
    ("hkrpg_cn", "Honkai: Star Rail", "StarRail.exe"),
    ("nap_global", "Zenless Zone Zero", "ZenlessZoneZero.exe"),
    ("nap_cn", "Zenless Zone Zero", "ZenlessZoneZero.exe"),
];

const KURO_TITLE: &str = "Wuthering Waves";
/// Имя раздела удаления у Kuro начинается так: за ним идёт редакция игры
/// («Overseas», «China»).
const KURO_KEY_PREFIX: &str = "KRInstall Wuthering Waves";

const GRYPHLINK_TITLE: &str = "Arknights: Endfield";
const GRYPHLINK_NAME: &str = "GRYPHLINK";

/// Подраздел вида `1_0`, `1_2`: две группы цифр через подчёркивание. Так HoYoPlay
/// называет версию своих настроек; `standalone` и прочее под это не подходит.
fn is_version_key(name: &str) -> bool {
    let mut parts = name.split('_');
    let (Some(a), Some(b), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    digits(a) && digits(b)
}

/// Значение без пробелов и кавычек по краям; пустое — как отсутствующее.
fn text(value: Option<String>) -> Option<String> {
    let value = value?;
    let value = value.trim().trim_matches('"').trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Папка исполняемого файла из командной строки вроде
/// `"C:\Games\Kuro\unins000.exe" /SILENT` или `C:\Games\Kuro\unins000.exe /S`.
fn folder_of_command(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    let exe = if let Some(rest) = command.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        // Без кавычек аргументы отделены пробелом, а сам путь может его
        // содержать («C:\Program Files\…»), поэтому режем по «.exe», а не по
        // первому пробелу.
        match command.to_ascii_lowercase().find(".exe") {
            Some(at) => &command[..at + ".exe".len()],
            None => command,
        }
    };
    Path::new(exe.trim())
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// Записи из разделов удаления: куст, полный путь к записи и её имя.
fn uninstall_entries(reg: &dyn Registry) -> Vec<(Root, String, String)> {
    let mut out = Vec::new();
    for (root, path) in UNINSTALL_ROOTS {
        for name in reg.subkeys(root, path) {
            out.push((root, format!(r"{path}\{name}"), name));
        }
    }
    out
}

/// Папка установки из записи удаления: первое непустое из перечисленных
/// значений, а если их нет — папка программы удаления.
fn install_dir(reg: &dyn Registry, root: Root, key: &str, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .find_map(|name| text(reg.value(root, key, name)).map(PathBuf::from))
        .or_else(|| {
            text(reg.value(root, key, "UninstallString")).and_then(|c| folder_of_command(&c))
        })
}

fn same_path(a: &Path, b: &Path) -> bool {
    super::is_inside(a, b) && super::is_inside(b, a)
}

/// Добавляет находку, если такого exe в списке ещё нет: одну папку могут
/// назвать и реестр, и список папок по умолчанию.
fn push_find(
    found: &mut Vec<InstalledGame>,
    title: &str,
    source: Source,
    launcher: &str,
    exe: PathBuf,
) {
    if found
        .iter()
        .any(|g| g.exe_path.as_deref().is_some_and(|e| same_path(e, &exe)))
    {
        return;
    }
    log::info!("[launchers] {title}: найдена через {launcher}, {}", exe.display());
    found.push(InstalledGame {
        title: title.to_string(),
        install_path: exe.parent().map(Path::to_path_buf).unwrap_or_default(),
        exe_path: Some(exe),
        launch: Launch::Exe,
        source,
    });
}

/// Первый существующий файл из кандидатов; промахи пишутся в отладочный лог.
fn first_existing(launcher: &str, candidates: Vec<PathBuf>) -> Option<PathBuf> {
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
        log::debug!("[launchers] {launcher}: файла нет, {}", candidate.display());
    }
    None
}

/// Папки без повторов, порядок сохраняется.
fn unique_dirs(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        if !out.iter().any(|d| same_path(d, &dir)) {
            out.push(dir);
        }
    }
    out
}

/// HoYoPlay: `<HYP>\<версия>\<игра>\GameInstallPath`. Пустой путь или нет
/// файла игры в папке — игры нет (её удалили, а ключ остался).
pub fn hoyoplay(reg: &dyn Registry) -> Vec<InstalledGame> {
    let mut found = Vec::new();
    for hyp in HOYOPLAY_ROOTS {
        for version in reg.subkeys(Root::CurrentUser, hyp) {
            if !is_version_key(&version) {
                continue;
            }
            let version_key = format!(r"{hyp}\{version}");
            for biz in reg.subkeys(Root::CurrentUser, &version_key) {
                let Some(&(_, title, exe_name)) = HOYOPLAY_GAMES
                    .iter()
                    .find(|(known, _, _)| known.eq_ignore_ascii_case(&biz))
                else {
                    log::debug!("[launchers] HoYoPlay: неизвестная игра {biz} ({version_key})");
                    continue;
                };
                let key = format!(r"{version_key}\{biz}");
                let Some(dir) = text(reg.value(Root::CurrentUser, &key, "GameInstallPath")) else {
                    log::debug!("[launchers] HoYoPlay: {title} — путь установки пуст ({key})");
                    continue;
                };
                if let Some(exe) = first_existing("HoYoPlay", vec![Path::new(&dir).join(exe_name)]) {
                    push_find(&mut found, title, Source::HoYoPlay, "HoYoPlay", exe);
                }
            }
        }
    }
    found
}

/// Лаунчер Kuro: записи удаления `KRInstall Wuthering Waves…` и папки по
/// умолчанию. В каждой папке игра лежит либо во вложенной `Wuthering Waves Game`,
/// либо рядом с лаунчером — берётся первое из существующих.
pub fn kuro(reg: &dyn Registry, default_dirs: &[PathBuf]) -> Vec<InstalledGame> {
    let mut dirs = Vec::new();
    for (root, key, name) in uninstall_entries(reg) {
        if !name.to_lowercase().starts_with(&KURO_KEY_PREFIX.to_lowercase()) {
            continue;
        }
        match install_dir(reg, root, &key, &["InstallPath", "InstallLocation"]) {
            Some(dir) => dirs.push(dir),
            None => log::debug!("[launchers] Kuro: в записи {key} нет папки установки"),
        }
    }
    dirs.extend_from_slice(default_dirs);

    let mut found = Vec::new();
    for dir in unique_dirs(dirs) {
        let candidates = vec![
            dir.join("Wuthering Waves Game").join("Wuthering Waves.exe"),
            dir.join("Wuthering Waves.exe"),
        ];
        if let Some(exe) = first_existing("лаунчер Kuro", candidates) {
            push_find(&mut found, KURO_TITLE, Source::Kuro, "лаунчер Kuro", exe);
        }
    }
    found
}

/// GRYPHLINK: записи удаления с этим именем и папка по умолчанию. Запускается
/// сам лаунчер, где он хранит игру — публично не описано.
pub fn gryphlink(reg: &dyn Registry, default_dirs: &[PathBuf]) -> Vec<InstalledGame> {
    let mut dirs = Vec::new();
    for (root, key, _) in uninstall_entries(reg) {
        let is_gryphlink = text(reg.value(root, &key, "DisplayName"))
            .is_some_and(|n| n.to_lowercase().contains(&GRYPHLINK_NAME.to_lowercase()));
        if !is_gryphlink {
            continue;
        }
        match install_dir(reg, root, &key, &["InstallLocation"]) {
            Some(dir) => dirs.push(dir),
            None => log::debug!("[launchers] GRYPHLINK: в записи {key} нет папки установки"),
        }
    }
    dirs.extend_from_slice(default_dirs);

    let mut found = Vec::new();
    for dir in unique_dirs(dirs) {
        if let Some(exe) = first_existing(GRYPHLINK_NAME, vec![dir.join("Launcher.exe")]) {
            push_find(&mut found, GRYPHLINK_TITLE, Source::Gryphlink, GRYPHLINK_NAME, exe);
        }
    }
    found
}

/// Настоящий реестр Windows. Как и в `steam.rs`, ошибки открытия и чтения —
/// это «раздела или значения нет», а не сбой.
#[cfg(windows)]
struct WinRegistry;

#[cfg(windows)]
impl WinRegistry {
    fn open(root: Root, path: &str) -> Option<winreg::RegKey> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let hive = match root {
            Root::CurrentUser => HKEY_CURRENT_USER,
            Root::LocalMachine => HKEY_LOCAL_MACHINE,
        };
        winreg::RegKey::predef(hive).open_subkey(path).ok()
    }
}

#[cfg(windows)]
impl Registry for WinRegistry {
    fn subkeys(&self, root: Root, path: &str) -> Vec<String> {
        Self::open(root, path)
            .map(|key| key.enum_keys().flatten().collect())
            .unwrap_or_default()
    }

    fn value(&self, root: Root, path: &str, name: &str) -> Option<String> {
        Self::open(root, path)?.get_value::<String, _>(name).ok()
    }
}

/// Все три с настоящим реестром и папками по умолчанию. Вне Windows лаунчеров
/// нет — список пуст.
pub fn installed() -> Vec<InstalledGame> {
    #[cfg(windows)]
    {
        let reg = WinRegistry;
        let mut all = hoyoplay(&reg);
        all.extend(kuro(&reg, &[PathBuf::from(r"C:\Program Files\Wuthering Waves")]));
        all.extend(gryphlink(&reg, &[PathBuf::from(r"C:\Program Files\GRYPHLINK")]));
        all
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Реестр в памяти: ключ — корень и путь, значения — пары «имя → строка».
    /// Подразделы выводятся из путей, как в настоящем реестре, где промежуточный
    /// ключ существует, пока под ним что-то есть.
    type FakeKey = (Root, String, Vec<(String, String)>);

    #[derive(Default)]
    struct FakeRegistry {
        keys: Vec<FakeKey>,
    }

    impl FakeRegistry {
        fn with(mut self, root: Root, path: &str, values: &[(&str, &str)]) -> Self {
            let values = values
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect();
            self.keys.push((root, path.to_string(), values));
            self
        }
    }

    impl Registry for FakeRegistry {
        fn subkeys(&self, root: Root, path: &str) -> Vec<String> {
            let prefix = format!("{}\\", path.to_lowercase());
            let mut out: Vec<String> = Vec::new();
            for (r, p, _) in &self.keys {
                if *r != root || !p.to_lowercase().starts_with(&prefix) {
                    continue;
                }
                let name = p[prefix.len()..].split('\\').next().unwrap().to_string();
                if !out.iter().any(|o| o.eq_ignore_ascii_case(&name)) {
                    out.push(name);
                }
            }
            out
        }

        fn value(&self, root: Root, path: &str, name: &str) -> Option<String> {
            self.keys
                .iter()
                .find(|(r, p, _)| *r == root && p.eq_ignore_ascii_case(path))
                .and_then(|(_, _, values)| {
                    values
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(name))
                        .map(|(_, v)| v.clone())
                })
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gh-launch-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Создаёт пустой файл (с промежуточными папками) и возвращает его путь.
    fn touch(dir: &Path, rel: &str) -> PathBuf {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, b"MZ").unwrap();
        p
    }

    fn s(p: &Path) -> &str {
        p.to_str().unwrap()
    }

    const UNINSTALL_HKLM: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_WOW: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_HKCU: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    // ---------------------------------------------------------------- HoYoPlay

    #[test]
    fn hoyoplay_finds_global_genshin_impact() {
        let dir = scratch("hyp-genshin");
        let exe = touch(&dir, r"Genshin Impact game\GenshinImpact.exe");
        let reg = FakeRegistry::default().with(
            Root::CurrentUser,
            r"Software\Cognosphere\HYP\1_0\hk4e_global",
            &[("GameInstallPath", s(&dir.join("Genshin Impact game")))],
        );

        let found = hoyoplay(&reg);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Genshin Impact");
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        assert_eq!(found[0].install_path, dir.join("Genshin Impact game"));
        assert_eq!(found[0].launch, Launch::Exe);
        assert_eq!(found[0].source, Source::HoYoPlay);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hoyoplay_knows_the_cn_and_the_other_games() {
        let dir = scratch("hyp-all");
        let yuanshen = touch(&dir, r"ys\YuanShen.exe");
        let rail_cn = touch(&dir, r"rail-cn\StarRail.exe");
        let rail_os = touch(&dir, r"rail-os\StarRail.exe");
        let zzz_cn = touch(&dir, r"zzz-cn\ZenlessZoneZero.exe");
        let zzz_os = touch(&dir, r"zzz-os\ZenlessZoneZero.exe");
        let cn = r"Software\miHoYo\HYP\1_2";
        let os = r"Software\Cognosphere\HYP\1_0";
        let path = |sub: &str| dir.join(sub).to_str().unwrap().to_string();
        let reg = FakeRegistry::default()
            .with(Root::CurrentUser, &format!(r"{cn}\hk4e_cn"), &[("GameInstallPath", &path("ys"))])
            .with(Root::CurrentUser, &format!(r"{cn}\hkrpg_cn"), &[("GameInstallPath", &path("rail-cn"))])
            .with(Root::CurrentUser, &format!(r"{os}\hkrpg_global"), &[("GameInstallPath", &path("rail-os"))])
            .with(Root::CurrentUser, &format!(r"{cn}\nap_cn"), &[("GameInstallPath", &path("zzz-cn"))])
            .with(Root::CurrentUser, &format!(r"{os}\nap_global"), &[("GameInstallPath", &path("zzz-os"))]);

        let mut found: Vec<(String, PathBuf)> = hoyoplay(&reg)
            .into_iter()
            .map(|g| {
                assert_eq!(g.source, Source::HoYoPlay);
                (g.title, g.exe_path.unwrap())
            })
            .collect();
        found.sort();

        assert_eq!(
            found,
            vec![
                ("Genshin Impact".to_string(), yuanshen),
                ("Honkai: Star Rail".to_string(), rail_cn),
                ("Honkai: Star Rail".to_string(), rail_os),
                ("Zenless Zone Zero".to_string(), zzz_cn),
                ("Zenless Zone Zero".to_string(), zzz_os),
            ]
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hoyoplay_skips_an_empty_install_path() {
        let reg = FakeRegistry::default()
            .with(
                Root::CurrentUser,
                r"Software\Cognosphere\HYP\1_0\hk4e_global",
                &[("GameInstallPath", "")],
            )
            .with(
                Root::CurrentUser,
                r"Software\Cognosphere\HYP\1_0\hkrpg_global",
                &[("GameInstallPath", "   ")],
            )
            .with(Root::CurrentUser, r"Software\Cognosphere\HYP\1_0\nap_global", &[]);
        assert!(hoyoplay(&reg).is_empty());
    }

    #[test]
    fn hoyoplay_skips_a_folder_without_the_game_exe() {
        let dir = scratch("hyp-noexe");
        touch(&dir, r"game\readme.txt");
        // Чужой exe в папке не в счёт: нужен именно файл этой игры.
        touch(&dir, r"game\StarRail.exe");
        let reg = FakeRegistry::default().with(
            Root::CurrentUser,
            r"Software\Cognosphere\HYP\1_0\hk4e_global",
            &[("GameInstallPath", s(&dir.join("game")))],
        );
        assert!(hoyoplay(&reg).is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hoyoplay_ignores_the_standalone_subtree() {
        // Встроенные в магазины лаунчеры уже находятся через Epic.
        let dir = scratch("hyp-standalone");
        touch(&dir, "GenshinImpact.exe");
        let reg = FakeRegistry::default().with(
            Root::CurrentUser,
            r"Software\Cognosphere\HYP\standalone\1_3\hk4e_global",
            &[("GameInstallPath", s(&dir))],
        );
        assert!(hoyoplay(&reg).is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hoyoplay_ignores_unknown_games_and_keys_that_are_not_versions() {
        let dir = scratch("hyp-unknown");
        touch(&dir, "BH3.exe");
        touch(&dir, "GenshinImpact.exe");
        let reg = FakeRegistry::default()
            .with(
                Root::CurrentUser,
                r"Software\Cognosphere\HYP\1_0\bh3_global",
                &[("GameInstallPath", s(&dir))],
            )
            .with(
                Root::CurrentUser,
                r"Software\Cognosphere\HYP\beta\hk4e_global",
                &[("GameInstallPath", s(&dir))],
            );
        assert!(hoyoplay(&reg).is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hoyoplay_with_no_registry_keys_finds_nothing() {
        assert!(hoyoplay(&FakeRegistry::default()).is_empty());
    }

    // -------------------------------------------------------------------- Kuro

    fn kuro_key(root_path: &str, name: &str) -> String {
        format!(r"{root_path}\{name}")
    }

    #[test]
    fn kuro_finds_the_game_in_the_nested_folder() {
        let dir = scratch("kuro-nested");
        let exe = touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "KRInstall Wuthering Waves Overseas"),
            &[("InstallPath", s(&dir))],
        );

        let found = kuro(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Wuthering Waves");
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        assert_eq!(found[0].install_path, dir.join("Wuthering Waves Game"));
        assert_eq!(found[0].launch, Launch::Exe);
        assert_eq!(found[0].source, Source::Kuro);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_falls_back_to_the_exe_right_in_the_folder() {
        let dir = scratch("kuro-flat");
        let exe = touch(&dir, "Wuthering Waves.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "KRInstall Wuthering Waves Overseas"),
            &[("InstallPath", s(&dir))],
        );

        let found = kuro(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        assert_eq!(found[0].install_path, dir);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_prefers_the_nested_exe_when_both_exist() {
        let dir = scratch("kuro-both");
        let nested = touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");
        touch(&dir, "Wuthering Waves.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "KRInstall Wuthering Waves Overseas"),
            &[("InstallPath", s(&dir))],
        );

        let found = kuro(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(nested.as_path()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_reads_install_location_when_install_path_is_missing_or_blank() {
        let dir = scratch("kuro-location");
        let exe = touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_WOW, "KRInstall Wuthering Waves China"),
            &[("InstallPath", "  "), ("InstallLocation", s(&dir))],
        );

        let found = kuro(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_takes_the_folder_of_a_quoted_uninstall_string() {
        let dir = scratch("kuro-quoted");
        let exe = touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");
        let uninstall = format!("\"{}\" /SILENT", dir.join("unins000.exe").display());
        let reg = FakeRegistry::default().with(
            Root::CurrentUser,
            &kuro_key(UNINSTALL_HKCU, "KRInstall Wuthering Waves Overseas"),
            &[("UninstallString", &uninstall)],
        );

        let found = kuro(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_takes_the_folder_of_an_unquoted_uninstall_string_with_arguments() {
        let dir = scratch("kuro-unquoted");
        let exe = touch(&dir, "Wuthering Waves.exe");
        let uninstall = format!("{} /SILENT", dir.join("unins000.exe").display());
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "KRInstall Wuthering Waves Overseas"),
            &[("UninstallString", &uninstall)],
        );

        let found = kuro(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_uses_a_default_folder_when_the_registry_has_nothing() {
        let dir = scratch("kuro-default");
        let exe = touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");

        let found = kuro(&FakeRegistry::default(), std::slice::from_ref(&dir));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        assert_eq!(found[0].source, Source::Kuro);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_reports_a_folder_named_twice_only_once() {
        let dir = scratch("kuro-twice");
        touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "KRInstall Wuthering Waves Overseas"),
            &[("InstallPath", s(&dir))],
        );

        assert_eq!(kuro(&reg, std::slice::from_ref(&dir)).len(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_finds_nothing_when_no_candidate_exists() {
        let dir = scratch("kuro-none");
        let empty_default = dir.join("no-such-default");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "KRInstall Wuthering Waves Overseas"),
            &[("InstallPath", s(&dir))],
        );

        assert!(kuro(&reg, std::slice::from_ref(&empty_default)).is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kuro_ignores_other_uninstall_entries() {
        let dir = scratch("kuro-other");
        touch(&dir, r"Wuthering Waves Game\Wuthering Waves.exe");
        let reg = FakeRegistry::default()
            .with(
                Root::LocalMachine,
                &kuro_key(UNINSTALL_HKLM, "KRInstall Punishing Gray Raven"),
                &[("InstallPath", s(&dir))],
            )
            .with(
                Root::LocalMachine,
                &kuro_key(UNINSTALL_HKLM, "Wuthering Waves"),
                &[("InstallPath", s(&dir))],
            );

        assert!(kuro(&reg, &[]).is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    // --------------------------------------------------------------- GRYPHLINK

    #[test]
    fn gryphlink_finds_the_launcher_by_display_name() {
        let dir = scratch("gryph-name");
        let exe = touch(&dir, "Launcher.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "{6F1A3A52-0000-0000-0000-000000000001}"),
            &[("DisplayName", "GRYPHLINK"), ("InstallLocation", s(&dir))],
        );

        let found = gryphlink(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Arknights: Endfield");
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        assert_eq!(found[0].install_path, dir);
        assert_eq!(found[0].launch, Launch::Exe);
        assert_eq!(found[0].source, Source::Gryphlink);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gryphlink_matches_the_display_name_in_any_letter_case_and_root() {
        let dir = scratch("gryph-case");
        let exe = touch(&dir, "Launcher.exe");
        let reg = FakeRegistry::default().with(
            Root::CurrentUser,
            &kuro_key(UNINSTALL_HKCU, "gryphlink"),
            &[("DisplayName", "Gryphlink 1.0"), ("InstallLocation", s(&dir))],
        );

        let found = gryphlink(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gryphlink_takes_the_folder_of_the_uninstall_string() {
        let dir = scratch("gryph-uninstall");
        let exe = touch(&dir, "Launcher.exe");
        let uninstall = format!("\"{}\"", dir.join("uninst.exe").display());
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_WOW, "GRYPHLINK"),
            &[("DisplayName", "GRYPHLINK"), ("UninstallString", &uninstall)],
        );

        let found = gryphlink(&reg, &[]);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gryphlink_uses_a_default_folder_when_the_registry_has_nothing() {
        let dir = scratch("gryph-default");
        let exe = touch(&dir, "Launcher.exe");

        let found = gryphlink(&FakeRegistry::default(), std::slice::from_ref(&dir));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].exe_path.as_deref(), Some(exe.as_path()));
        assert_eq!(found[0].source, Source::Gryphlink);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gryphlink_ignores_entries_with_another_display_name() {
        let dir = scratch("gryph-other");
        touch(&dir, "Launcher.exe");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "Some Launcher"),
            &[("DisplayName", "Some Launcher"), ("InstallLocation", s(&dir))],
        );

        assert!(gryphlink(&reg, &[]).is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn gryphlink_finds_nothing_without_a_launcher_exe() {
        let dir = scratch("gryph-none");
        let reg = FakeRegistry::default().with(
            Root::LocalMachine,
            &kuro_key(UNINSTALL_HKLM, "GRYPHLINK"),
            &[("DisplayName", "GRYPHLINK"), ("InstallLocation", s(&dir))],
        );

        assert!(gryphlink(&reg, &[dir.join("no-such-default")]).is_empty());
        fs::remove_dir_all(&dir).ok();
    }
}
