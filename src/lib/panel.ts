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
 * Баннеры для карусели (спека этапа 7 §3.1): сначала идущие — ближайшие к
 * окончанию первыми, затем будущие — ближайшие к началу первыми. Кончившиеся
 * не показываются.
 */
export function shownBanners(banners: Banner[], nowSec: number): Banner[] {
  const running = runningBanners(banners, nowSec).sort((a, b) => a.endsAt - b.endsAt);
  const upcoming = banners.filter((b) => b.startsAt > nowSec).sort((a, b) => a.startsAt - b.startsAt);
  return [...running, ...upcoming];
}

/**
 * Ключи для карусели баннеров (правка финальной ревизии этапа 7): игры и
 * момента начала одних не хватает — у владельца в данных есть пары баннеров
 * одной игры с общим стартом, и `${gameId}:${startsAt}` у них совпадает.
 * Добавляем заголовок, а точные повторы (тот же заголовок тоже) разводит
 * счётчик встречи. Считается один раз за отрисовку и используется и для
 * слайдов, и для точек, и для `shownKey`, чтобы все три места не разошлись.
 */
export function bannerKeys(shown: Banner[]): string[] {
  const seen = new Map<string, number>();
  return shown.map((b) => {
    const base = `${b.gameId}:${b.startsAt}:${b.title}`;
    const n = seen.get(base) ?? 0;
    seen.set(base, n + 1);
    return n === 0 ? base : `${base}#${n}`;
  });
}

/**
 * Панели нечего показать (спека этапа 5, §8). Считается теми же функциями,
 * которыми секции решают, рисоваться ли: отдельная проверка разошлась бы с ними,
 * и панель то молчала бы при пустых секциях, то писала «пусто» под кодами.
 * Будущие баннеры тоже считаются содержимым (этап 7).
 */
export function panelIsEmpty(
  codes: Code[],
  banners: Banner[],
  videos: Video[],
  nowSec: number,
): boolean {
  return (
    activeCodes(codes, nowSec).length === 0 &&
    shownBanners(banners, nowSec).length === 0 &&
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
