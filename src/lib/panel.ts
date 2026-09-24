import { hubFreshness } from "./time";
import { dictionary, type Dictionary, type Lang, type VideoLang } from "../i18n";
import type { Banner, Code, HubData, Video } from "../types";

/** Коды, которые ещё не сгорели, ближайшие к сгоранию первыми. */
export function activeCodes(codes: Code[], nowSec: number): Code[] {
  return codes
    .filter((c) => c.expiresAt === null || c.expiresAt > nowSec)
    .sort((a, b) => (a.expiresAt ?? Infinity) - (b.expiresAt ?? Infinity));
}

/** Баннеры, которые идут прямо сейчас. */
export function runningBanners(banners: Banner[], nowSec: number): Banner[] {
  return banners.filter((b) => b.startsAt <= nowSec && b.endsAt > nowSec);
}

/**
 * Панели нечего показать (спека этапа 5, §8). Считается теми же функциями,
 * которыми секции решают, рисоваться ли: отдельная проверка разошлась бы с ними,
 * и панель то молчала бы при пустых секциях, то писала «пусто» под кодами.
 */
export function panelIsEmpty(
  codes: Code[],
  banners: Banner[],
  videos: Video[],
  nowSec: number,
): boolean {
  return (
    activeCodes(codes, nowSec).length === 0 &&
    runningBanners(banners, nowSec).length === 0 &&
    videos.length === 0
  );
}

/**
 * Видео игры на выбранном языке; если их нет — английские этой игры (спека
 * этапа 6 §3.3); если и английских нет (лента для этого языка ни разу не
 * сработала) — любые видео игры, лишь бы секция не пропадала совсем. Видео
 * без `lang` — английское: так записаны файлы до этапа 6.
 */
export function videosFor(videos: Video[], gameId: string, lang: VideoLang): Video[] {
  const ofGame = videos.filter((v) => v.gameId === gameId);
  const langOf = (v: Video) => v.lang ?? "en";
  const wanted = ofGame.filter((v) => langOf(v) === lang);
  if (wanted.length > 0) return wanted;
  const english = ofGame.filter((v) => langOf(v) === "en");
  return english.length > 0 ? english : ofGame;
}

/** Откуда пришли данные — словами, одинаковыми на панели и во «Данных». */
export function sourceLabel(source: string | undefined, t: Dictionary): string {
  if (source === undefined) return "";
  const labels = t.panel.sources as unknown as Record<string, string | undefined>;
  return labels[source] ?? t.panel.unknownSource;
}

/** Строка состояния на вкладке «Данные» (спека этапа 5, §7.5). */
export function dataStatus(hub: HubData | null, nowSec: number, lang: Lang): string {
  const t = dictionary(lang);
  if (!hub || hub._source === "bundled") return t.panel.noDataYet;
  if (hub.codes.length === 0 && hub.banners.length === 0 && hub.videos.length === 0) {
    return t.panel.noDataYet;
  }
  return t.panel.dataStatus(sourceLabel(hub._source, t), hubFreshness(hub.updatedAt, nowSec, lang));
}
