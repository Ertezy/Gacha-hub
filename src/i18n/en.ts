// Английский словарь — основной (спека этапа 6 §4.1). Русский обязан повторять
// его форму: иначе проект не соберётся. Термины — по глоссарию спеки §4.5.
// Разделы добавляют задачи перевода: panel, main, settings, errors.

export const en = {
  language: {
    interface: "Interface language",
    video: "Video language",
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
    steamOpenFailed: (d?: string) => `Couldn't open Steam${d ? ` (${d})` : ""}. Check that Steam is installed and you are signed in.`,
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
