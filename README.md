# Gacha Hub

A free desktop launcher for gacha games on Windows: Genshin Impact, Honkai: Star Rail,
Zenless Zone Zero, Wuthering Waves and Arknights: Endfield — one window instead of
several launchers.

Not affiliated with the games' developers. Game names and artwork belong to their owners.

## Download

Get the installer from [Releases](https://github.com/Ertezy/Gacha-hub/releases/latest) — the file ending in `_x64-setup.exe`. Windows 10 or 11, 64-bit.

## Install

Run the installer. It installs for your user only, no administrator rights needed, and
downloads Microsoft WebView2 if the system doesn't have it yet.

The installer isn't code-signed, so Windows SmartScreen may show "Windows protected your
PC". Click **More info → Run anyway**.

## Update

Download the newer installer and run it over the old version. Games, order and settings
are kept.

## Uninstall

Windows Settings → Apps → Installed apps → Gacha Hub → Uninstall. To remove your settings
and cached pictures too, tick "Delete the application data" in the uninstaller, or delete
`%APPDATA%\com.gachahub.desktop` and `%LOCALAPPDATA%\com.gachahub.desktop` yourself.

## Privacy

No account, no telemetry, nothing is sent about you. The app downloads the data file
from GitHub Pages, and pictures for the panel and backgrounds from the game wikis
(Fandom, wiki.gg), YouTube, Steam and Epic Games.

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by
[SignPath Foundation](https://signpath.org). *Applied for, not active yet — until it is,
installers are unsigned (see [Install](#install)).*

- Only installers built by GitHub Actions from a tagged commit of this repository are
  signed, and every release is approved by hand before signing.
- Committers and reviewers: [Ertezy](https://github.com/Ertezy)
- Approvers: [Ertezy](https://github.com/Ertezy)
- Privacy policy: see [Privacy](#privacy). The app sends nothing about you; it only
  downloads the public data and pictures listed there.

## Features

- **Your games in one dock.** Games are found through Steam, the Epic Games Launcher, HoYoPlay,
  the Kuro launcher and GRYPHLINK, or added by pointing at the game's `.exe`.
  Reorder them by drag in Settings → Games.
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

Requirements: Node.js 20 or newer, Rust (stable, MSVC toolchain) and the Tauri 2 prerequisites
for Windows (https://tauri.app/start/prerequisites/). On a clean machine:

    winget install Rustlang.Rustup
    winget install Microsoft.VisualStudio.2022.BuildTools --override "add --wait --passive --modify Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

Then:

    npm install
    npm run tauri dev      # run with hot reload
    npm run tauri build    # installer in src-tauri/target/release/bundle/nsis

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

`src-tauri/icons/` holds the app icon, drawn from one kitsune artwork:

- `icon.ico` — 16–48 px show a close-up of the face, 64–256 px the whole badge; Windows
  picks the size for the taskbar and Explorer by display scaling;
- `icon.png` — the whole badge, 512 px;
- `tray-32.rgba` — the tray icon: the same close-up as raw 32×32 RGBA pixels, embedded by
  `src-tauri/src/tray.rs`.

Replace all three together when the artwork changes.

## License

MIT — see [LICENSE](LICENSE). Game names, art and trademarks belong to their owners.
