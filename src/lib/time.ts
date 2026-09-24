/**
 * Форматирование времени для панели.
 *
 * Все функции принимают `nowSec` явным параметром, а не читают часы сами:
 * иначе их поведение зависело бы от момента запуска тестов.
 * Время везде — unix-секунды, как в файле хаба.
 */

import { dayMonth, relativeDays } from "../i18n/format";
import { dictionary, type Lang } from "../i18n";

const HOUR = 3600;
const DAY = 86400;

/** «3 h 52 min», «9 days», «expired» — и то же по-русски. */
export function timeLeft(untilSec: number, nowSec: number, lang: Lang): string {
  const t = dictionary(lang).time;
  const left = untilSec - nowSec;
  if (left <= 0) return t.expired;
  if (left >= DAY) return t.days(Math.floor(left / DAY));
  const hours = Math.floor(left / HOUR);
  const minutes = Math.floor((left % HOUR) / 60);
  if (hours === 0) return t.minutes(minutes);
  return t.hoursMinutes(hours, minutes);
}

/** «today», «yesterday», «4 days ago», «in 2 days» — и то же по-русски. */
export function timeAgo(sinceSec: number, nowSec: number, lang: Lang): string {
  const diff = nowSec - sinceSec;
  if (diff < 0) return relativeDays(lang, Math.max(1, Math.round(-diff / DAY)));
  return relativeDays(lang, -Math.floor(diff / DAY));
}

/** Сгорает ли код в ближайшие сутки. Уже истёкшие — нет. */
export function burnsToday(untilSec: number, nowSec: number): boolean {
  const left = untilSec - nowSec;
  return left > 0 && left < DAY;
}

/** Доля пройденного срока, 0..1. Полоса заполнения баннера. */
export function progress(startSec: number, endSec: number, nowSec: number): number {
  const total = endSec - startSec;
  if (total <= 0) return 1;
  const passed = (nowSec - startSec) / total;
  return Math.min(1, Math.max(0, passed));
}

/**
 * «data is fresh» или «data from Sep 1» — единая формулировка свежести
 * данных хаба. Используется и в подвале панели, и в разделе «Данные»
 * настроек: два места не должны разойтись в словах. И то же по-русски.
 */
export function hubFreshness(updatedAt: number, nowSec: number, lang: Lang): string {
  const t = dictionary(lang).time;
  if (nowSec - updatedAt <= DAY) return t.fresh;
  return t.dataFrom(dayMonth(lang, updatedAt));
}
