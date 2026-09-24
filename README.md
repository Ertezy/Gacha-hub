# Gacha Hub — MVP

Кастомный лаунчер для gacha-игр (Genshin Impact, Honkai: Star Rail,
Zenless Zone Zero, Wuthering Waves, Arknights: Endfield) с мини-хабом
(новости, промокоды, гайды). Стек: **Tauri 2 (Rust) + React 18 + TypeScript**.

## Архитектура

```
src-tauri/src/
  catalog.rs    — статический каталог игр (appid Steam, поддержка Epic,
                  ссылки на официальные загрузки)
  config.rs     — Config Manager: %APPDATA%\com.gachahub.desktop\config.json,
                  serde-валидация, версия схемы, атомарная запись (tmp + rename),
                  ленивая парсинг (битая per-game запись отбрасывается, не роняя конфиг)
  launch.rs     — Launch Engine: steam:// | egstore://launch/ | прямой .exe
  detect.rs     — Install Detector: скан Epic/Steam (все библиотеки из
                  libraryfolders.vdf)/всех дисков, кэш на сессию, unit-тесты
  hub.rs        — данные мини-хаба: удалённый URL → локальный override →
                  кеш → бандл-файл (схема в hub.rs и src/types.ts)
  commands.rs   — #[tauri::command] — единственный мост UI <-> OS
src/
  components/   — Sidebar, GamesPage, NewsPage, PromoCodesPage, GuidesPage,
                  SettingsPage
  lib/api.ts    — типизированные обёртки invoke()
  types.ts      — TS-зеркала Rust-схем
src-tauri/resources/hub.json — данные мини-хаба (бандлится через "bundle.resources")
```

UI только вызывает команды; все системные вызовы — в Rust.

## Автоопределение установок (detect.rs)

- **Корни скана:** Epic-каталоги на всех дисках (`<drive>\Epic Games`,
  `Program Files[Epic Games]`), Steam-библиотеки — из реестра + дефолтов +
  **`steamapps\libraryfolders.vdf`** (вторая библиотека на D: видна),
  `Program Files` + один уровень вложенности на каждом диске
  (`D:\Games\GenshinImpact` найдётся; `Windows/Users/ProgramData` пропущены).
- **Матчинг:** имя папки — case-insensitive подстрока (пользователи
  переименовывают папки: `Genshin Impact game`, `HonkaiStarRail`,
  `ArknightsEndfieldgowoU`); имя exe — точный stem из приоритетного списка
  (у Epic-клиентов HoYoverse общие имена `launcher_epic.exe`, поэтому папка
  — основной сигнал).
- **Автоконфиг на первом старте:** для игр без сохранённой настройки детект
  решает режим: нашлась Steam-установка и есть appid → `steam`; нашлась
  Epic/официальная → `exe` + найденный путь; ничего → каталожный дефолт.
  Игры с уже сохранённой настройкой никогда не трогаются.
- **Кэш на сессию:** полный скан (все диски) выполняется не чаще одного раза
  за запуск; кнопка «Скан» в Настройках делает принудительный рескан.
- **Защита от чужой игры:** запуск exe из папки, которая однозначно belongs
  другой игре каталога (папка `ZenlessZoneZero` + exe `HYP.exe` → это ZZZ),
  блокируется с ошибкой. Точный список сильных exe-имён — в `detect.rs`.
- Проверено `cargo test` (unit: vdf-парсер, case-insensitive .exe; smoke:
  все 5 игр локальной машины находятся, дублей путей нет).

## Механики запуска (Windows)

| Способ | Как работает |
|---|---|
| **Steam** | `steam://rungameid/<appid>` — Steam регистрирует протокол-хендлер в реестре (`HKCU\Software\Classes\steam`, проверено на dev-машине). Открывается через tauri-plugin-opener (общий «open URI»-путь с `openUrl` на фронте). |
| **Epic** | `egstore://launch/<product-id>`. Публичного официального launch-URI у Epic нет — формат `launch/<id>` задокументирован сообществом; product id вводится в Настройках (из JSON страницы товара EGS). **Если product id не задан — игра ищется в каталоге Epic Games (автоскан) и запускается её exe напрямую** (то, что реально делает EGS). |
| **Прямой .exe** | `std::process::Command` (CreateProcessW): аргументы — массив (токенизация по правилам Windows `CommandLineToArgvW` — не POSIX shlex, который жрёт бэкслеши в путях; no shell, no injection), `current_dir` = каталог exe (клиенты HoYoverse/Kuro ищут Client/ относительно себя). Работает и для официальных лаунчеров (Genshin/HSR/ZZZ — их клиент из официального сайта, Endfield — от Hypergritype), и для бинарника Steam-игры (например `...\Wuthering Waves.exe`). |

Все внешние ссылки (store, новости, гайды) открываются только по `https://` —
`api.openSafeUrl` отклоняет прочие схемы, чтобы данные хаба (особенно когда
появятся удалённые) не могли запускать `file://` или произвольные протоколы.
CSP в `tauri.conf.json` ужесточён (default-src 'self'), удалённые данные
хаба грузит Rust, а не webview.

### Steam appid (проверены через Steam API 2026-07-21, повторная сверка 2026-07-22)

- Wuthering Waves — **3513350** (также стоит локально: `steam_appid.txt` совпадает)
- Zenless Zone Zero — **4162040**
- Arknights: Endfield — **4732690**
- Genshin Impact — нет в Steam (официальный лаунчер + Epic)
- Honkai: Star Rail — **нет в Steam** (проверено поиском по Steam-стору: страницы HSR отсутствует). Распространяется через официальный лаунчер / Epic.

> Перепроверка после код-ревью (2026-07-22): альтернативные appid из ревью
> (2064650, 1599340) через тот же API оказались *Tower of Fantasy* и *Lost Ark*
> — чужие игры. Никогда не вставляйте appid на веру: неправильный id молча
> откроет чужую игру в Steam.

## Запуск с нуля

1. **Rust** (если не установлен):
   ```
   winget install Rustlang.Rustup
   ```
2. **Visual Studio Build Tools** с C++ toolchain (нужно для компиляции Tauri):
   ```
   winget install Microsoft.VisualStudio.2022.BuildTools --override "add --wait --passive --modify Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```
3. Сборка и запуск:
   ```
   npm install
   npm run tauri dev
   ```
4. Production-сборка (инсталлятор): `npm run tauri build`.

Иконки уже сгенерированы (`src-tauri/icons/`); перегенерировать: `node scripts/gen-icons.cjs`.
Если `npm run tauri build` (NSIS/WiX) будет ругаться на иконки — проще
пересобрать полный набор стандартным инструментом:
`npx tauri icon src-tauri/icons/icon.png`.

## Как наполнять мини-хаб

Источники в приоритете (реализовано в `hub.rs`):

1. **Удалённый JSON** — поле `hubUrl` в Настройках (https только, таймаут 5 с,
   грузит Rust через ureq, а не webview). Успешный ответ кешируется в
   `%APPDATA%\com.gachahub.desktop\hub_cache.json`.
2. **Локальный override** — `%APPDATA%\com.gachahub.desktop\hub.json`:
   положите/обновите файл — коды изменятся без пересборки приложения.
3. **Кеш** последнего удачного fetch (если URL задан, но сеть не отвечает).
4. **Бандл** `src-tauri/resources/hub.json` — демо-данные на случай, если
   ничего из вышеперечисленного нет.

Схема: `promoCodes[]` — `gameId/code/rewards/expired(ISO-дата,
инклюзивно)/source`, `news[]`, `guides[]` (в `src/types.ts`). В UI
отображается источник данных (remote/override/cache/bundled) и `updatedAt`.

## Чек-лист MVP

1. `npm run tauri dev` стартует из чистого checkout
2. Карточки 5 игр, бейджи платформ
3. Wuthering Waves: «Играть» → Steam открывает игру (steam://)
4. Genshin: выбрать «Прямой .exe», указать путь, аргументы `-dx12` → старт
5. Ошибка при незаполненном пути / отсутствующем exe — видна в карточке
6. Промокоды: фильтр по игре, истёкшие серые, «Копировать»
7. Настройки переживают перезапуск приложения (config.json)

## Открытые вопросы (до релиза)

- Epic product id для каждой игры (Epic блокирует автоматическую проверку —
  Cloudflare; вводится вручную в Настройках)
- Подтвердить наличие Wuthering Waves / Endfield в EGS (сейчас `epic_supported`
  по данным пользователя)
- HSR: **проверено — в Steam отсутствует** (поиск Steam-стора, 2026-07-22); остаётся Epic/official
- Данные панели берут с сборщика (https://ertezy.github.io/Gacha-hub-info/hub.json); другой адрес можно указать в Настройках → Данные
- Официальные лаунчеры HoYoverse: точные имена exe зависят от версии
  клиента — автоскан ищет по списку приоритетов, ручной выбор пути работает
  всегда

## Выпуск: свежие данные в комплекте

Перед сборкой выпуска замените файл данных из комплекта файлом сборщика:

    npm run update-hub

Скрипт скачивает https://ertezy.github.io/Gacha-hub-info/hub.json в
`src-tauri/resources/hub.json`. Этот файл нужен только для первого запуска без
интернета; дальше приложение берёт данные у сборщика само. Закоммитьте
обновлённый файл вместе с выпуском.
