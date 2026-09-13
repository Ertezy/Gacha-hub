//! Фоновая картинка игры.
//!
//! Отдельно от иконок, потому что это другая вещь с другим жизненным циклом,
//! и класть их в одну папку значило бы путать при чистке.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};

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
}

impl ArtContext {
    pub fn build(store_art: bool, games: &[Game]) -> Self {
        let mut ctx = Self {
            store_art,
            installed: Vec::new(),
            steam_roots: Vec::new(),
            epic_catalog: None,
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
        }
        ctx
    }

    fn store_of(&self, game: &Game) -> Option<StoreRef> {
        crate::stores::store_of(game, &self.installed)
    }

    #[cfg(test)]
    pub fn for_test(
        store_art: bool,
        installed: Vec<InstalledGame>,
        epic_catalog: Option<serde_json::Value>,
    ) -> Self {
        Self { store_art, installed, steam_roots: Vec::new(), epic_catalog }
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
    let Ok(dir) = cache_dir(app) else {
        return choose(None, None, None);
    };
    let video = own_video(app, game);
    let picture = game
        .background
        .as_ref()
        .and_then(|own| crate::localcopy::copy_into(&dir, own, ""));
    // Своя картинка перекрывает магазин целиком, и тогда магазин не трогаем.
    let store = if picture.is_some() { None } else { store_picture(&dir, game, ctx) };
    choose(video, picture, store)
}

/// Своё видео, если файл на месте.
///
/// Ролик не копируется: окну разрешается читать ровно этот файл там, где он
/// лежит (спека §6.2). Разрешение живёт до закрытия приложения, повторная
/// выдача того же разрешения безвредна.
fn own_video(app: &AppHandle, game: &Game) -> Option<PathBuf> {
    let video = game.video.as_ref()?;
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

/// Адреса картинок Epic, которых ещё нет в кеше фонов. Без сети и без окна —
/// ради тестов.
pub fn pending_epic_downloads(games: &[Game], ctx: &ArtContext, dir: &Path) -> Vec<String> {
    let mut urls: Vec<String> = games
        .iter()
        .filter_map(|g| epic_image_url(g, ctx))
        .filter(|url| crate::images::cached(dir, url).is_none())
        .collect();
    urls.sort();
    urls.dedup();
    urls
}

/// Докачивает недостающие картинки Epic (спека §3.3). Вызывается из отдельного
/// потока при запуске: окно её не ждёт. Возвращает, сколько картинок появилось.
///
/// Отказ по одной картинке пишется в журнал и не мешает остальным: игра просто
/// останется с заливкой до следующего запуска (спека §3.4).
pub fn download_missing_store_art(app: &AppHandle) -> usize {
    let cfg = crate::config::load(app);
    if !cfg.store_art {
        return 0;
    }
    let Ok(dir) = cache_dir(app) else {
        return 0;
    };
    let ctx = ArtContext::build(cfg.store_art, &cfg.games);
    let mut done = 0;
    for url in pending_epic_downloads(&cfg.games, &ctx, &dir) {
        match crate::images::fetch_into(&dir, &url, MAX_STORE_ART_BYTES) {
            Ok(_) => done += 1,
            Err(e) => log::warn!("[art] картинка Epic не скачалась: {e}"),
        }
    }
    done
}

/// Картинка магазина, уже лежащая в кеше фонов.
///
/// Картинка Steam копируется сюда сразу: это чтение с диска. Картинка Epic здесь
/// только ищется в кеше — скачивает её фоновый поток после запуска (спека §3.3),
/// и окно список игр не ждёт.
fn store_picture(dir: &Path, game: &Game, ctx: &ArtContext) -> Option<(ArtSource, PathBuf)> {
    if !ctx.store_art {
        return None;
    }
    match ctx.store_of(game)? {
        StoreRef::Steam { appid } => {
            let hero = crate::storeart::steam_hero(&ctx.steam_roots, appid)?;
            crate::localcopy::copy_into(dir, &hero, "steam-").map(|p| (ArtSource::Steam, p))
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
            pending_epic_downloads(&games, &ctx, &dir),
            vec!["https://cdn1.epicgames.com/b.jpg".to_string()]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nothing_is_queued_with_the_switch_off() {
        let dir = scratch("off");
        let ctx = ArtContext::for_test(false, Vec::new(), Some(two_game_catalog()));
        assert!(pending_epic_downloads(&[epic_game("a")], &ctx, &dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_same_picture_is_queued_once() {
        let dir = scratch("dedup");
        let ctx = ArtContext::for_test(true, Vec::new(), Some(two_game_catalog()));
        let games = [epic_game("a"), epic_game("a")];
        assert_eq!(pending_epic_downloads(&games, &ctx, &dir).len(), 1);
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
}
