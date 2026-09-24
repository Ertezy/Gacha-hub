// Английский словарь — основной (спека этапа 6 §4.1). Русский обязан повторять
// его форму: иначе проект не соберётся. Термины — по глоссарию спеки §4.5.
// Разделы добавляют задачи перевода: panel, main, settings, errors.

import { plural } from "./format";

export const en = {
  language: {
    interface: "Interface language",
    video: "Video language",
  },
  time: {
    expired: "expired",
    minutes: (m: number) => `${m} min`,
    hoursMinutes: (h: number, m: number) => `${h} h ${m} min`,
    days: (n: number) => `${n} ${plural("en", n, { one: "day", other: "days" })}`,
    fresh: "data is fresh",
    dataFrom: (date: string) => `data from ${date}`,
  },
  panel: {
    noDataYet: "Codes, banners and videos will appear once the data service is up.",
    panelEmpty: "No codes, banners or videos yet. They will appear once the data service is up.",
    sources: { remote: "online", override: "local override", cache: "from cache", bundled: "bundled" },
    unknownSource: "unknown",
    dataStatus: (source: string, freshness: string) => `Source: ${source}, ${freshness}.`,
    settingsAndGames: "Settings and games",
    disclaimer: "Not affiliated with the game developers. Content belongs to its rights holders.",
    codes: "Codes",
    noExpiry: "no expiry",
    expiresIn: (t: string) => `expires in ${t}`,
    timeRemaining: (t: string) => `${t} left`,
    regionOnly: (region: string) => `${region} only`,
    copyCode: "Copy code",
    redeem: "Redeem on the website",
    banners: "Banners",
    previousBanner: "Previous banner",
    nextBanner: "Next banner",
    bannerDot: (i: number, n: number) => `Banner ${i} of ${n}`,
    videos: "Videos",
    premiere: "premiere",
  },
  // Главный экран, док игр и общие обёртки страницы (спека этапа 6, Task 5).
  main: {
    launch: {
      note: { steam: "Launches via Steam", epic: "Launches via Epic Games", exe: "Launches directly" },
      fileMissing: "game file not found",
      from: { steam: "from Steam", epic: "from Epic Games", exe: "from a folder" },
    },
    play: "▶ Play",
    launching: "Launching…",
    settings: "Settings and games",
    gameFileMissing: "Game file not found",
    dockFileMissing: (title: string) => `${title} — game file not found`,
    noGameSelected: "No game selected",
    loadFailed: (detail: string) => `Failed to load: ${detail}`,
    relocate: "Find again",
    pickManually: "Specify manually",
    background: {
      source: { video: "custom video", picture: "custom image", fill: "gradient" },
      videoMissing: (source: string) => `${source}, video file not found`,
      videoUnreadable: "Couldn't read the video — pick an mp4 (H.264) or webm file.",
      videoTooBig: "Video is larger than 2560×1440 — pick a smaller clip.",
    },
    filters: {
      program: "Program",
      image: "Picture",
      video: "Video",
    },
    refuseNonHttps: (url: string) => `refusing to open a non-https link: ${url}`,
  },
  // Ошибки из Rust: код → фраза, подробность подставляется (спека §6.2,
  // src/i18n/errors.ts). Имена полей — коды из src-tauri/src/error.rs.
  errors: {
    unknown: "Something went wrong",
    internal: (d?: string) => (d ? `Something went wrong: ${d}` : "Something went wrong"),
    gameNotFound: (d?: string) => (d ? `Game not found: ${d}` : "Game not found"),
    fileMissing: (d?: string) => (d ? `File not found: ${d}` : "File not found"),
    exePathMissing: (_d?: string) => "The path to the game is not set",
    argsUnclosedQuote: (d?: string) => `Can't read the launch arguments${d ? ` "${d}"` : ""}: a quote is not closed.`,
    exeStartFailed: (d?: string) => (d ? `Couldn't start the game: ${d}` : "Couldn't start the game"),
    steamOpenFailed: (d?: string) => `Couldn't open Steam${d ? ` (${d})` : ""}. Check that Steam is installed and has been launched at least once.`,
    epicOpenFailed: (d?: string) => `Couldn't open the Epic Games Launcher${d ? ` (${d})` : ""}. Check that it is installed.`,
    emptyTitle: (_d?: string) => "The name can't be empty",
    hubUrlNotHttps: (_d?: string) => "The address must start with https://",
    videoPickFailed: (d?: string) => (d ? `Couldn't open the video: ${d}` : "Couldn't open the video"),
    videoWrongFormat: (_d?: string) => "The video must be mp4 or webm.",
    videoFileMissing: (_d?: string) => "Video file not found.",
    videoTooLarge: (d?: string) =>
      d ? `The video is larger than ${d} MB — pick a smaller file.` : "The video is too large — pick a smaller file.",
    cacheReadFailed: (d?: string) => (d ? `Can't read the cache: ${d}` : "Can't read the cache"),
    logFolderMissing: (d?: string) => (d ? `The log folder was not found: ${d}` : "The log folder was not found"),
    openFolderFailed: (d?: string) => (d ? `Couldn't open the folder: ${d}` : "Couldn't open the folder"),
    configSaveFailed: (d?: string) => (d ? `Couldn't save the settings: ${d}` : "Couldn't save the settings"),
  },
};

export type Dictionary = typeof en;
