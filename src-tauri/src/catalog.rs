//! Static catalog of supported games.
//!
//! Research results (checked 2026-07-21 via the Steam store API,
//! `store.steampowered.com/api/appdetails` and the local Steam library):
//!   - Wuthering Waves  -> Steam appid 3513350 (verified; also installed
//!     locally in Steam/steamapps/common/Wuthering Waves,
//!     steam_appid.txt matches)
//!   - Zenless Zone Zero -> Steam appid 4162040 (verified via appdetails)
//!   - Arknights: Endfield -> Steam appid 4732690 (verified via appdetails)
//!   - Genshin Impact  -> NOT on Steam
//!   - Honkai: Star Rail -> not found in the Steam search on the dev machine
//!     (appdetails for 2436370 returns success:false here); treat as
//!     "verify before shipping" — the catalog is data, just fix the field.
//!
//! Epic Games Store blocks automated access (Cloudflare), so Epic presence
//! is recorded from user reports and marked accordingly.

use crate::config::LaunchMode;

pub struct GameMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub publisher: &'static str,
    /// Steam appid for `steam://rungameid/<appid>` (verified values above).
    pub steam_appid: Option<u32>,
    /// User-reported presence in the Epic Games Store (verify before release).
    pub epic_supported: bool,
    /// Whether the game ships a standalone official launcher (HoYoverse /
    /// Hypergritype clients downloaded from the official website).
    pub has_official_launcher: bool,
    /// Store / official download page.
    pub store_url: &'static str,
}

impl GameMeta {
    /// Sensible default launch method for a game without user config:
    /// Steam when an appid is known, otherwise direct .exe (the user picks
    /// the official launcher or game binary).
    pub fn default_mode(&self) -> LaunchMode {
        if self.steam_appid.is_some() {
            LaunchMode::Steam
        } else {
            LaunchMode::Exe
        }
    }
}

pub const GAMES: &[GameMeta] = &[
    GameMeta {
        id: "genshin",
        name: "Genshin Impact",
        publisher: "HoYoverse",
        steam_appid: None,
        // Not on Steam; on Epic per user report (unverified — CF blocks checks).
        epic_supported: true,
        has_official_launcher: true,
        store_url: "https://genshin.hoyoverse.com/download",
    },
    GameMeta {
        id: "hsr",
        name: "Honkai: Star Rail",
        publisher: "HoYoverse",
        // Community lists HSR on Steam, but the Steam API on the dev machine
        // returned success:false for the remembered appid — verify and fill.
        steam_appid: None,
        epic_supported: true,
        has_official_launcher: true,
        store_url: "https://hsr.hoyoverse.com/download",
    },
    GameMeta {
        id: "zzz",
        name: "Zenless Zone Zero",
        publisher: "HoYoverse",
        steam_appid: Some(4162040),
        epic_supported: true,
        has_official_launcher: true,
        store_url: "https://zzz.hoyoverse.com/download",
    },
    GameMeta {
        id: "wuthering",
        name: "Wuthering Waves",
        publisher: "Kuro Games",
        steam_appid: Some(3513350),
        // User believes it is on EGS; not verifiable from this machine and
        // not in the Steam-free Kuro distribution — flip to true if you
        // confirm it and then fill the Epic product id in Settings.
        epic_supported: false,
        has_official_launcher: false,
        store_url: "https://store.steampowered.com/app/3513350",
    },
    GameMeta {
        id: "endfield",
        name: "Arknights: Endfield",
        publisher: "Hypergritype",
        steam_appid: Some(4732690),
        epic_supported: true,
        has_official_launcher: true,
        store_url: "https://endfield.hypergritype.com",
    },
];
