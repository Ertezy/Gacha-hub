// Русский словарь. Форма — ровно как у английского (тип Dictionary);
// формулировки — те, что были в коде до перевода (спека этапа 6 §4.1).

import type { Dictionary } from "./en";
import { plural } from "./format";

export const ru: Dictionary = {
  language: {
    interface: "Язык интерфейса",
    video: "Язык видео",
  },
  time: {
    expired: "истёк",
    minutes: (m: number) => `${m} мин`,
    hoursMinutes: (h: number, m: number) => `${h} ч ${m} мин`,
    days: (n: number) => `${n} ${plural("ru", n, { one: "день", few: "дня", many: "дней", other: "дня" })}`,
    fresh: "данные свежие",
    dataFrom: (date: string) => `данные от ${date}`,
  },
  // Строки прежней панели дословно (было в src/lib/panel.ts и компонентах
  // src/components/panel/*, src/components/SidePanel.tsx).
  panel: {
    noDataYet: "Коды, баннеры и видео появятся, когда заработает сервис данных.",
    panelEmpty: "Пока нет кодов, баннеров и видео. Они появятся, когда заработает сервис данных.",
    sources: { remote: "из сети", override: "из локальной подмены", cache: "из кеша", bundled: "из комплекта" },
    unknownSource: "неизвестно",
    dataStatus: (source: string, freshness: string) => `Источник — ${source}, ${freshness}.`,
    settingsAndGames: "Настройки и игры",
    disclaimer: "Не связано с разработчиками игр. Материалы принадлежат правообладателям.",
    codes: "Промокоды",
    noExpiry: "бессрочный",
    expiresIn: (t: string) => `сгорит через ${t}`,
    timeRemaining: (t: string) => `осталось ${t}`,
    regionOnly: (region: string) => `только ${region}`,
    copyCode: "Копировать код",
    redeem: "Забрать на сайте",
    banners: "Баннеры",
    previousBanner: "Предыдущий баннер",
    nextBanner: "Следующий баннер",
    bannerDot: (i: number, n: number) => `Баннер ${i} из ${n}`,
    videos: "Видео",
    premiere: "премьера",
  },
  // Прежние тексты ошибок Rust дословно, переменная часть — подробность.
  errors: {
    unknown: "Что-то пошло не так",
    internal: (d?: string) => (d ? `Что-то пошло не так: ${d}` : "Что-то пошло не так"),
    // Без подробности — отказ правки (`with_config`), с подробностью — игры
    // нет среди найденных или в списке.
    gameNotFound: (d?: string) => (d ? `игра больше не найдена: ${d}` : "игра не найдена или правка невозможна"),
    fileMissing: (d?: string) => (d ? `файл не найден: ${d}` : "файл не найден"),
    exePathMissing: (_d?: string) => "путь к игре не задан",
    argsUnclosedQuote: (d?: string) => `не удалось разобрать аргументы${d ? ` «${d}»` : ""}: незакрытая кавычка.`,
    exeStartFailed: (d?: string) => (d ? `не удалось запустить ${d}` : "не удалось запустить игру"),
    steamOpenFailed: (d?: string) =>
      `не удалось открыть ${d ?? "Steam"}. Проверь, что Steam установлен и хотя бы раз запускался.`,
    epicOpenFailed: (d?: string) => `не удалось открыть Epic Games Launcher${d ? `: ${d}` : ""}. Проверь, что он установлен.`,
    emptyTitle: (_d?: string) => "название не может быть пустым",
    hubUrlNotHttps: (_d?: string) => "адрес должен начинаться с https://",
    videoPickFailed: (d?: string) => (d ? `Окну не открыть видео: ${d}` : "Окну не открыть видео"),
    videoWrongFormat: (_d?: string) => "Видео должно быть в формате mp4 или webm.",
    videoFileMissing: (_d?: string) => "Файл видео не найден.",
    videoTooLarge: (d?: string) =>
      d ? `Видео тяжелее ${d} МБ — выберите файл поменьше.` : "Видео слишком тяжёлое — выберите файл поменьше.",
    cacheReadFailed: (d?: string) => (d ? `не читается кеш: ${d}` : "не читается кеш"),
    logFolderMissing: (d?: string) => (d ? `не найдена папка журнала: ${d}` : "не найдена папка журнала"),
    openFolderFailed: (d?: string) => (d ? `не удалось открыть папку: ${d}` : "не удалось открыть папку"),
    // Раньше человек видел сам текст ошибки сохранения — он и остаётся.
    configSaveFailed: (d?: string) => d || "не удалось сохранить настройки",
  },
};
