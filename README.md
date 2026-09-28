# Gacha Hub

A free desktop launcher for gacha games on Windows: Genshin Impact, Honkai: Star Rail,
Zenless Zone Zero, Wuthering Waves and Arknights: Endfield — one window instead of
several launchers.

Not affiliated with the games' developers. Game names and artwork belong to their owners.

## Features

- **Your games in one dock.** Games are found through Steam and the Epic Games Launcher,
  or added by pointing at the game's `.exe`. Reorder them by drag in Settings → Games.
- **Play** launches through the store the game was installed from, or directly.
- **Side panel** for the selected game: promo codes with expiry, current and upcoming
  banners (the carousel slides on its own), and the latest official videos.
- **Languages:** interface in English or Russian; videos in English or Japanese.
- **Backgrounds:** store artwork, your own image or your own video.
- **Tray:** closing the window can hide it to the tray; games can be launched from the
  tray menu.
- **Launch with Windows:** starts hidden in the tray and keeps the panel's data fresh.

## Where the data comes from

The panel's codes, banners and videos come from a separate collector that runs on
GitHub Actions and publishes one file:

- repository: https://github.com/Ertezy/Gacha-hub-info
- file: https://ertezy.github.io/Gacha-hub-info/hub.json

The collector reads community wikis (Fandom, wiki.gg — CC BY-SA), an open API mirror
and the games' official YouTube channels. Every code and banner links back to its
source. A different address can be set in Settings → Data → Data source URL.

## Building from source

Requirements: Node.js 24, Rust (stable, MSVC toolchain) and the Tauri 2 prerequisites
for Windows (https://tauri.app/start/prerequisites/). On a clean machine:

    winget install Rustlang.Rustup
    winget install Microsoft.VisualStudio.2022.BuildTools --override "add --wait --passive --modify Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

Then:

    npm install
    npm run tauri dev      # run with hot reload
    npm run tauri build    # installer in src-tauri/target/release/bundle

Tests:

    npm test
    cd src-tauri && cargo test --lib

## Release: fresh data in the bundle

The installer carries a copy of the data file for the very first launch without internet.
Before building a release, refresh it and commit it with the release:

    npm run update-hub

## Where settings live

`%APPDATA%\com.gachahub.desktop\`:

- `config.json` — games, order and settings;
- `hub_cache.json`, `hub_cache.meta.json` — the last downloaded data file and its version tag;
- `hub.json` — optional manual override, used only when the collector cannot be reached.

Logs are kept separately, under `%LOCALAPPDATA%\com.gachahub.desktop\logs\`; open the
folder from Settings → About → Log → Show log.

## Icons

Regenerate the app icons from one source image:

    npx tauri icon src-tauri/icons/icon.png
