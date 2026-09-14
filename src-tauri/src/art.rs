//! Фоновая картинка игры.
//!
//! Отдельно от иконок, потому что это другая вещь с другим жизненным циклом,
//! и класть их в одну папку значило бы путать при чистке.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::Game;
use crate::stores::{InstalledGame, StoreRef};

/// Папка кеша фонов.
///
/// Отдельно от кеша картинок хаба: тот чистится по возрасту при каждом
/// запуске, и копия фона оттуда вылетала бы, а исходник человека мог к тому
/// времени уже переехать.
pub fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("нет папки кеша: {e}"))?
        .join("art");
    std::fs::create_dir_all(&dir).map_err(|e| format!("не создать папку кеша арта: {e}"))?;
    Ok(dir)
}

/// Откуда взят фон, который показывается первым (спека этапа 5, §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtSource {
    Video,
    Picture,
    Steam,
    Epic,
    Fill,
}

/// Решение о фоне игры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    pub source: ArtSource,
    /// Картинка: своя или магазина. Когда первым идёт видео, она остаётся его
    /// неподвижным кадром при «уменьшить движение» (спека §6.4).
    pub art_path: Option<PathBuf>,
    pub video_path: Option<PathBuf>,
}

/// Порядок источников одной чистой функцией (спека §2.1): своё видео, своя
/// картинка, картинка магазина, заливка. Своё важнее магазинного по тому же
/// правилу, что у иконок.
pub fn choose(
    video: Option<PathBuf>,
    picture: Option<PathBuf>,
    store: Option<(ArtSource, PathBuf)>,
) -> Chosen {
    let (store_source, store_path) = match store {
        Some((source, path)) => (Some(source), Some(path)),
        None => (None, None),
    };
    let source = if video.is_some() {
        ArtSource::Video
    } else if picture.is_some() {
        ArtSource::Picture
    } else {
        store_source.unwrap_or(ArtSource::Fill)
    };
    Chosen {
        source,
        art_path: picture.or(store_path),
        video_path: video,
    }
}

/// Всё, что нужно для картинок магазинов, читается один раз на список игр, а не
/// на каждую игру. Манифесты и каталог Epic читаются, только если без них не
/// обойтись.
pub struct ArtContext {
    store_art: bool,
    installed: Vec<InstalledGame>,
    steam_roots: Vec<PathBuf>,
    epic_catalog: Option<serde_json::Value>,
    /// Каталог Epic нужен был, но не прочитался (испорчен, отсутствует,
    /// превысил потолок размера).
    epic_catalog_unreadable: bool,
}

impl ArtContext {
    pub fn build(store_art: bool, games: &[Game]) -> Self {
        let mut ctx = Self {
            store_art,
            installed: Vec::new(),
            steam_roots: Vec::new(),
            epic_catalog: None,
            epic_catalog_unreadable: false,
        };
        if !store_art {
            return ctx;
        }
        // Манифесты нужны только для игр с прямым запуском (спека §2.2).
        if games.iter().any(|g| matches!(g.launch, crate::config::Launch::Exe)) {
            ctx.installed = crate::stores::installed();
        }
        ctx.steam_roots = crate::stores::steam::install_roots();
        if games.iter().any(|g| matches!(ctx.store_of(g), Some(StoreRef::Epic { .. }))) {
            ctx.epic_catalog = crate::storeart::epic_catalog_path()
                .and_then(|path| crate::storeart::read_epic_catalog(&path));
            ctx.epic_catalog_unreadable = ctx.epic_catalog.is_none();
        }
        ctx
    }

    fn store_of(&self, game: &Game) -> Option<StoreRef> {
        crate::stores::store_of(game, &self.installed)
    }

    /// Каталог Epic был нужен для списка игр, но не прочитался.
    pub fn epic_catalog_unreadable(&self) -> bool {
        self.epic_catalog_unreadable
    }

    #[cfg(test)]
    pub fn for_test(
        store_art: bool,
        installed: Vec<InstalledGame>,
        epic_catalog: Option<serde_json::Value>,
    ) -> Self {
        Self {
            store_art,
            installed,
            steam_roots: Vec::new(),
            epic_catalog,
            epic_catalog_unreadable: false,
        }
    }

    #[cfg(test)]
    pub fn with_steam_roots(mut self, steam_roots: Vec<PathBuf>) -> Self {
        self.steam_roots = steam_roots;
        self
    }
}

/// Адрес картинки Epic для игры — или `None`, если галочка выключена, игра не из
/// Epic или каталог её не знает.
pub fn epic_image_url(game: &Game, ctx: &ArtContext) -> Option<String> {
    if !ctx.store_art {
        return None;
    }
    let Some(StoreRef::Epic { catalog_item_id }) = ctx.store_of(game) else {
        return None;
    };
    crate::storeart::epic_key_image_url(ctx.epic_catalog.as_ref()?, &catalog_item_id)
}

/// Фон игры, готовый к показу в окне.
pub fn resolve(app: &AppHandle, game: &Game, ctx: &ArtContext) -> Chosen {
    // Видео кеш не нужен: без папки кеша пропадают только картинки.
    let video = own_video(app, game);
    let dir = match cache_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("[art] фон без кеша: {e}");
            return choose(video, None, None);
        }
    };
    let picture = game
        .background
        .as_ref()
        .and_then(|own| crate::localcopy::copy_into(&dir, own, ""));
    // Своя картинка перекрывает магазин целиком, и тогда магазин не трогаем.
    let store = if picture.is_some() { None } else { store_picture(&dir, game, ctx) };
    choose(video, picture, store)
}

/// Ролик ли это по расширению (спека §6.1). Окну выдаётся только mp4 и webm.
pub fn is_video_file_name(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4") || e.eq_ignore_ascii_case("webm"))
}

/// Ролик подходящего формата, и файл на месте.
pub fn is_usable_video(path: &Path) -> bool {
    is_video_file_name(path) && path.is_file()
}

/// Потолок веса своего видео фона — решение владельца после живой проверки
/// (спека этапа 5, §12). Длина ролика не ограничена: он читается с диска
/// кусками и крутится по кругу, поэтому нагрузку даёт вес и разрешение кадра,
/// а не то, сколько секунд он длится.
pub const MAX_VIDEO_BYTES: u64 = 300 * 1024 * 1024;

/// Правда, если файл тяжелее потолка. Вынесена отдельно от `video_file_problem`
/// ради границы: тест проверяет её напрямую, без файла на диске — тот же
/// приём, что у `exceeds_ceiling` в `images.rs`.
fn exceeds_video_limit(len: u64) -> bool {
    len > MAX_VIDEO_BYTES
}

/// Первая найденная проблема с файлом своего видео фона — фраза для показа
/// человеку, или `None`, если файл годится. Проверяется и до выбора (окно
/// зовёт `check_video`), и при записи в конфиг (`update_game`): страница не
/// единственная защита (спека §12).
pub fn video_file_problem(path: &Path) -> Option<String> {
    if !is_video_file_name(path) {
        return Some("Видео должно быть в формате mp4 или webm.".to_string());
    }
    let Ok(meta) = std::fs::metadata(path) else {
        return Some("Файл видео не найден.".to_string());
    };
    if !meta.is_file() {
        return Some("Файл видео не найден.".to_string());
    }
    if exceeds_video_limit(meta.len()) {
        return Some("Видео тяжелее 300 МБ — выберите файл поменьше.".to_string());
    }
    None
}

/// Своё видео, если файл на месте.
///
/// Ролик не копируется: окну разрешается читать ровно этот файл там, где он
/// лежит (спека §6.2). Разрешение живёт до закрытия приложения, повторная
/// выдача того же разрешения безвредна.
fn own_video(app: &AppHandle, game: &Game) -> Option<PathBuf> {
    let video = game.video.as_ref()?;
    if !is_video_file_name(video) {
        log::warn!("[art] видео не mp4 и не webm, окну не выдаётся: {}", video.display());
        return None;
    }
    if !video.is_file() {
        return None;
    }
    if let Err(e) = app.asset_protocol_scope().allow_file(video) {
        log::warn!("[art] окну не разрешить видео: {e}");
        return None;
    }
    Some(video.clone())
}

/// Потолок размера картинки магазина. Тот же, что у картинок хаба, пока обмер
/// картинок владельца не покажет, что его мало (спека этапа 5, §3.2).
pub const MAX_STORE_ART_BYTES: u64 = crate::images::MAX_IMAGE_BYTES;

/// Адрес большой картинки Steam для игры — или `None`, если галочка выключена,
/// игра не из Steam или картинка библиотеки лежит в старой раскладке кеша.
fn steam_big_hero_url(game: &Game, ctx: &ArtContext) -> Option<String> {
    if !ctx.store_art {
        return None;
    }
    let Some(StoreRef::Steam { appid }) = ctx.store_of(game) else {
        return None;
    };
    let hero = crate::storeart::steam_hero(&ctx.steam_roots, appid)?;
    crate::storeart::steam_hero_2x_url(&hero, appid)
}

/// Адреса картинок магазинов, которых ещё нет в кеше фонов: картинки Epic и
/// большие картинки Steam. Без сети и без окна — ради тестов.
pub fn pending_store_downloads(games: &[Game], ctx: &ArtContext, dir: &Path) -> Vec<String> {
    let mut urls: Vec<String> = games
        .iter()
        .filter_map(|g| epic_image_url(g, ctx).or_else(|| steam_big_hero_url(g, ctx)))
        .filter(|url| crate::images::cached(dir, url).is_none())
        .collect();
    urls.sort();
    urls.dedup();
    urls
}

/// Докачка картинок магазинов за время работы приложения.
#[derive(Debug, Default)]
pub struct DownloadState {
    /// Поток докачки уже работает — второй не запускается.
    running: bool,
    /// Адреса, которые уже пробовали скачать. Неудачный до следующего запуска
    /// не повторяется, иначе без сети каждое построение списка шло бы в сеть.
    tried: HashSet<String>,
    /// О нечитаемом каталоге Epic журнал уже знает.
    catalog_reported: bool,
}

/// Состояние докачки, которое хранит Tauri (`manage`).
#[derive(Debug, Default)]
pub struct StoreArtDownloads(pub Mutex<DownloadState>);

/// Отдаёт под докачку новые адреса, если поток свободен, и сразу запоминает их
/// как опробованные. `None` — докачка уже идёт, либо добавить нечего.
pub fn claim(state: &mut DownloadState, pending: Vec<String>) -> Option<Vec<String>> {
    if state.running {
        return None;
    }
    let fresh: Vec<String> =
        pending.into_iter().filter(|url| !state.tried.contains(url)).collect();
    if fresh.is_empty() {
        return None;
    }
    state.tried.extend(fresh.iter().cloned());
    state.running = true;
    Some(fresh)
}

/// Докачка в потоке закончилась — следующий вызов `claim` может начать новую.
pub fn finish(state: &mut DownloadState) {
    state.running = false;
}

/// Забывает опробованные адреса: после очистки кеша неудачный раньше адрес
/// можно попробовать снова.
pub fn forget_tried(state: &mut DownloadState) {
    state.tried.clear();
}

/// Правда только в первый раз за время работы приложения — дальше о
/// нечитаемом каталоге Epic в журнал больше не пишем.
pub fn report_catalog_once(state: &mut DownloadState) -> bool {
    if state.catalog_reported {
        return false;
    }
    state.catalog_reported = true;
    true
}

/// Докачивает недостающие картинки магазинов в отдельном потоке: картинки Epic и
/// большие картинки Steam (спека §3.1, §3.3).
///
/// Вызывается при каждом построении списка игр, поэтому одна дорога покрывает
/// запуск, первый запуск, поиск установленных игр, ручное добавление, включение
/// галочки и очистку кеша. Окно докачку не ждёт. Отказ по одной картинке пишется
/// в журнал и не мешает остальным (спека §3.4).
pub fn start_missing_downloads(app: &AppHandle, games: &[Game], ctx: &ArtContext) {
    if ctx.epic_catalog_unreadable() {
        let should_log = {
            let state = app.state::<StoreArtDownloads>();
            let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
            report_catalog_once(&mut state)
        };
        if should_log {
            log::warn!("[art] каталог Epic не прочитан, картинок Epic не будет");
        }
    }
    // Отказ уже разобран в `resolve`: здесь просто нечего качать без папки.
    let Ok(dir) = cache_dir(app) else { return };
    let pending = pending_store_downloads(games, ctx, &dir);
    let claimed = {
        let state = app.state::<StoreArtDownloads>();
        let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
        claim(&mut state, pending)
    };
    let Some(urls) = claimed else { return };

    let app = app.clone();
    std::thread::spawn(move || {
        let mut done = 0;
        for url in urls {
            match crate::images::fetch_into(&dir, &url, MAX_STORE_ART_BYTES) {
                Ok(_) => done += 1,
                Err(e) => log::warn!("[art] картинка магазина не скачалась: {e}"),
            }
        }
        {
            let state = app.state::<StoreArtDownloads>();
            let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
            finish(&mut state);
        }
        if done > 0 {
            if let Err(e) = app.emit("games-changed", ()) {
                log::warn!("[art] окно не узнало о новых фонах: {e}");
            }
        }
    });
}

/// После очистки кеша картинки скачиваются заново, в том числе те, что не
/// скачались раньше.
pub fn forget_tried_downloads(app: &AppHandle) {
    let state = app.state::<StoreArtDownloads>();
    let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
    forget_tried(&mut state);
}

/// Картинка магазина, уже лежащая в кеше фонов.
///
/// Картинка Steam с диска копируется сюда сразу: это чтение с диска. Большая
/// картинка Steam и картинка Epic здесь только ищутся в кеше — их докачивает
/// поток, который запускает построение списка игр (спека §3.1, §3.3), и окно
/// список игр не ждёт. Скачанная большая картинка Steam заменяет картинку с диска.
fn store_picture(dir: &Path, game: &Game, ctx: &ArtContext) -> Option<(ArtSource, PathBuf)> {
    if !ctx.store_art {
        return None;
    }
    match ctx.store_of(game)? {
        StoreRef::Steam { appid } => {
            let hero = crate::storeart::steam_hero(&ctx.steam_roots, appid)?;
            let big = crate::storeart::steam_hero_2x_url(&hero, appid)
                .and_then(|url| crate::images::cached(dir, &url));
            big.or_else(|| crate::localcopy::copy_into(dir, &hero, "steam-"))
                .map(|p| (ArtSource::Steam, p))
        }
        StoreRef::Epic { .. } => {
            let url = epic_image_url(game, ctx)?;
            crate::images::cached(dir, &url).map(|p| (ArtSource::Epic, p))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn epic_game(id: &str) -> Game {
        let mut g = Game::manual(id.into(), id.into());
        g.launch = crate::config::Launch::Epic {
            namespace: "ns".into(),
            catalog_item_id: id.into(),
            app_name: "app".into(),
        };
        g
    }

    fn two_game_catalog() -> serde_json::Value {
        serde_json::json!([
            { "id": "a", "keyImages": [{ "type": "DieselGameBox", "url": "https://cdn1.epicgames.com/a.jpg" }] },
            { "id": "b", "keyImages": [{ "type": "DieselGameBox", "url": "https://cdn1.epicgames.com/b.jpg" }] }
        ])
    }

    fn scratch(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gh-art-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn only_missing_epic_pictures_are_queued() {
        let dir = scratch("pending");
        let a = "https://cdn1.epicgames.com/a.jpg";
        std::fs::write(dir.join(crate::images::file_name_for(a, "jpg")), b"jpg").unwrap();
        let ctx = ArtContext::for_test(true, Vec::new(), Some(two_game_catalog()));
        let games = [epic_game("a"), epic_game("b")];
        assert_eq!(
            pending_store_downloads(&games, &ctx, &dir),
            vec!["https://cdn1.epicgames.com/b.jpg".to_string()]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nothing_is_queued_with_the_switch_off() {
        let dir = scratch("off");
        let ctx = ArtContext::for_test(false, Vec::new(), Some(two_game_catalog()));
        assert!(pending_store_downloads(&[epic_game("a")], &ctx, &dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_same_picture_is_queued_once() {
        let dir = scratch("dedup");
        let ctx = ArtContext::for_test(true, Vec::new(), Some(two_game_catalog()));
        let games = [epic_game("a"), epic_game("a")];
        assert_eq!(pending_store_downloads(&games, &ctx, &dir).len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_own_video_comes_first_and_keeps_the_picture_as_its_still() {
        let chosen = choose(Some(p("v.mp4")), Some(p("pic.img")), Some((ArtSource::Steam, p("steam.img"))));
        assert_eq!(chosen.source, ArtSource::Video);
        assert_eq!(chosen.video_path, Some(p("v.mp4")));
        assert_eq!(chosen.art_path, Some(p("pic.img")), "картинка остаётся неподвижным кадром");
    }

    #[test]
    fn a_video_over_store_art_uses_it_as_the_still() {
        let chosen = choose(Some(p("v.mp4")), None, Some((ArtSource::Epic, p("epic.jpg"))));
        assert_eq!(chosen.source, ArtSource::Video);
        assert_eq!(chosen.art_path, Some(p("epic.jpg")));
    }

    #[test]
    fn an_own_picture_beats_the_store() {
        let chosen = choose(None, Some(p("pic.img")), Some((ArtSource::Steam, p("steam.img"))));
        assert_eq!(chosen.source, ArtSource::Picture);
        assert_eq!(chosen.art_path, Some(p("pic.img")));
    }

    #[test]
    fn store_art_is_used_when_nothing_own_is_set() {
        let chosen = choose(None, None, Some((ArtSource::Epic, p("epic.jpg"))));
        assert_eq!(chosen.source, ArtSource::Epic);
        assert_eq!(chosen.art_path, Some(p("epic.jpg")));
        assert_eq!(chosen.video_path, None);
    }

    #[test]
    fn nothing_at_all_means_the_fill() {
        assert_eq!(
            choose(None, None, None),
            Chosen { source: ArtSource::Fill, art_path: None, video_path: None }
        );
    }

    #[test]
    fn the_source_is_sent_in_lowercase() {
        assert_eq!(serde_json::to_string(&ArtSource::Epic).unwrap(), "\"epic\"");
        assert_eq!(serde_json::to_string(&ArtSource::Picture).unwrap(), "\"picture\"");
    }

    #[test]
    fn with_the_switch_off_no_epic_address_is_given() {
        let mut g = Game::manual("e".into(), "E".into());
        g.launch = crate::config::Launch::Epic {
            namespace: "ns".into(),
            catalog_item_id: "a".into(),
            app_name: "app".into(),
        };
        let catalog = serde_json::json!([
            { "id": "a", "keyImages": [{ "type": "DieselGameBox", "url": "https://cdn1.epicgames.com/a.jpg" }] }
        ]);
        let on = ArtContext::for_test(true, Vec::new(), Some(catalog.clone()));
        assert_eq!(epic_image_url(&g, &on), Some("https://cdn1.epicgames.com/a.jpg".into()));
        let off = ArtContext::for_test(false, Vec::new(), Some(catalog));
        assert_eq!(epic_image_url(&g, &off), None);
    }

    #[test]
    fn claim_hands_out_each_new_address_once() {
        let mut state = DownloadState::default();
        assert_eq!(
            claim(&mut state, vec!["a".to_string(), "b".to_string()]),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(claim(&mut state, vec!["c".to_string()]), None, "поток уже работает");
        finish(&mut state);
        assert_eq!(
            claim(&mut state, vec!["a".to_string(), "b".to_string()]),
            None,
            "оба адреса уже пробовали"
        );
        assert_eq!(
            claim(&mut state, vec!["a".to_string(), "c".to_string()]),
            Some(vec!["c".to_string()])
        );
    }

    #[test]
    fn claim_with_nothing_new_leaves_the_worker_stopped() {
        let mut state = DownloadState::default();
        assert_eq!(claim(&mut state, Vec::new()), None);
        assert_eq!(claim(&mut state, vec!["a".to_string()]), Some(vec!["a".to_string()]));
        finish(&mut state);
        assert_eq!(claim(&mut state, vec!["a".to_string()]), None);
        assert_eq!(claim(&mut state, vec!["b".to_string()]), Some(vec!["b".to_string()]));
    }

    #[test]
    fn forgetting_tried_addresses_allows_a_new_try() {
        let mut state = DownloadState::default();
        assert_eq!(claim(&mut state, vec!["a".to_string()]), Some(vec!["a".to_string()]));
        finish(&mut state);
        forget_tried(&mut state);
        assert_eq!(claim(&mut state, vec!["a".to_string()]), Some(vec!["a".to_string()]));
    }

    #[test]
    fn an_unreadable_catalog_is_reported_once() {
        let mut state = DownloadState::default();
        assert!(report_catalog_once(&mut state));
        assert!(!report_catalog_once(&mut state));
    }

    const HASH: &str = "de0f5fcf72849ff1ba2aa8603e64dd42ba7f380d";

    fn steam_game(appid: u32) -> Game {
        let mut g = Game::manual("s".into(), "S".into());
        g.launch = crate::config::Launch::Steam { appid };
        g
    }

    /// Папка Steam, где у игры лежит картинка библиотеки в новой раскладке кеша.
    fn steam_root_with_hero(tag: &str, appid: u32) -> PathBuf {
        let root = scratch(tag);
        let folder = root
            .join("appcache")
            .join("librarycache")
            .join(appid.to_string())
            .join(HASH);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("library_hero.jpg"), b"small").unwrap();
        root
    }

    fn big_hero_url(appid: u32) -> String {
        format!(
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{appid}/{HASH}/library_hero_2x.jpg"
        )
    }

    #[test]
    fn a_missing_big_steam_picture_is_queued_until_it_is_cached() {
        let root = steam_root_with_hero("steam-root-queue", 7);
        let dir = scratch("steam-dir-queue");
        let ctx = ArtContext::for_test(true, Vec::new(), None).with_steam_roots(vec![root.clone()]);
        assert_eq!(pending_store_downloads(&[steam_game(7)], &ctx, &dir), vec![big_hero_url(7)]);
        std::fs::write(dir.join(crate::images::file_name_for(&big_hero_url(7), "jpg")), b"big")
            .unwrap();
        assert!(pending_store_downloads(&[steam_game(7)], &ctx, &dir).is_empty(), "уже в кеше");
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_big_steam_picture_is_queued_with_the_switch_off() {
        let root = steam_root_with_hero("steam-root-off", 7);
        let dir = scratch("steam-dir-off");
        let ctx = ArtContext::for_test(false, Vec::new(), None).with_steam_roots(vec![root.clone()]);
        assert!(pending_store_downloads(&[steam_game(7)], &ctx, &dir).is_empty());
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_big_steam_picture_replaces_the_small_one_once_downloaded() {
        let root = steam_root_with_hero("steam-root-show", 7);
        let dir = scratch("steam-dir-show");
        let ctx = ArtContext::for_test(true, Vec::new(), None).with_steam_roots(vec![root.clone()]);
        let (source, small) = store_picture(&dir, &steam_game(7), &ctx).unwrap();
        assert_eq!(source, ArtSource::Steam);
        assert_eq!(std::fs::read(&small).unwrap(), b"small", "пока большой нет — картинка с диска");
        let big = dir.join(crate::images::file_name_for(&big_hero_url(7), "jpg"));
        std::fs::write(&big, b"big").unwrap();
        assert_eq!(store_picture(&dir, &steam_game(7), &ctx), Some((ArtSource::Steam, big)));
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn only_mp4_and_webm_are_videos() {
        assert!(is_video_file_name(Path::new("a.mp4")));
        assert!(is_video_file_name(Path::new("B.WEBM")));
        assert!(!is_video_file_name(Path::new("c.mkv")));
        assert!(!is_video_file_name(Path::new("d.mp4.exe")));
        assert!(!is_video_file_name(Path::new("noext")));
        assert!(!is_video_file_name(Path::new("e.jpg")));
    }

    #[test]
    fn video_file_problem_flags_the_wrong_extension() {
        let dir = scratch("video-ext");
        let file = dir.join("clip.txt");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(
            video_file_problem(&file),
            Some("Видео должно быть в формате mp4 или webm.".to_string())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn video_file_problem_flags_a_missing_file() {
        let dir = scratch("video-missing");
        let file = dir.join("nope.mp4");
        assert_eq!(video_file_problem(&file), Some("Файл видео не найден.".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn video_file_problem_accepts_a_small_real_file() {
        let dir = scratch("video-ok");
        let file = dir.join("clip.mp4");
        std::fs::write(&file, b"tiny").unwrap();
        assert_eq!(video_file_problem(&file), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn exceeds_video_limit_is_false_exactly_at_the_ceiling() {
        assert!(!exceeds_video_limit(MAX_VIDEO_BYTES));
    }

    #[test]
    fn exceeds_video_limit_is_true_one_byte_over() {
        assert!(exceeds_video_limit(MAX_VIDEO_BYTES + 1));
    }
}
