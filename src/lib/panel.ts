import { hubFreshness } from "./time";
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

export const NO_DATA_YET = "Коды, баннеры и видео появятся, когда заработает сервис данных.";

const SOURCE_LABELS: Record<string, string> = {
  remote: "из сети",
  override: "из локальной подмены",
  cache: "из кеша",
  bundled: "из комплекта",
};

/** Откуда пришли данные — словами, одинаковыми на панели и во «Данных». */
export function sourceLabel(source: string | undefined): string {
  if (source === undefined) return "";
  return SOURCE_LABELS[source] ?? "неизвестно";
}

/** Строка состояния на вкладке «Данные» (спека этапа 5, §7.5). */
export function dataStatus(hub: HubData | null, nowSec: number): string {
  if (!hub || hub._source === "bundled") return NO_DATA_YET;
  if (hub.codes.length === 0 && hub.banners.length === 0 && hub.videos.length === 0) {
    return NO_DATA_YET;
  }
  return `Источник — ${sourceLabel(hub._source)}, ${hubFreshness(hub.updatedAt, nowSec)}.`;
}
