/**
 * Форматирование времени для панели.
 *
 * Все функции принимают `nowSec` явным параметром, а не читают часы сами:
 * иначе их поведение зависело бы от момента запуска тестов.
 * Время везде — unix-секунды, как в файле хаба.
 */

const HOUR = 3600;
const DAY = 86400;

/**
 * Русское склонение по числу.
 * Форма зависит от последней цифры — кроме чисел, оканчивающихся на 11–14,
 * где она всегда «много»: «11 дней», но «21 день».
 */
export function plural(n: number, one: string, few: string, many: string): string {
  const abs = Math.abs(n) % 100;
  if (abs >= 11 && abs <= 14) return many;
  switch (abs % 10) {
    case 1:
      return one;
    case 2:
    case 3:
    case 4:
      return few;
    default:
      return many;
  }
}

/** «3 ч 52 мин», «9 дней», «истёк». */
export function timeLeft(untilSec: number, nowSec: number): string {
  const left = untilSec - nowSec;
  if (left <= 0) return "истёк";

  if (left >= DAY) {
    const days = Math.floor(left / DAY);
    return `${days} ${plural(days, "день", "дня", "дней")}`;
  }
  const hours = Math.floor(left / HOUR);
  const minutes = Math.floor((left % HOUR) / 60);
  if (hours === 0) return `${minutes} мин`;
  return `${hours} ч ${minutes} мин`;
}

/** «сегодня», «вчера», «4 дня назад», «через 2 дня». */
export function timeAgo(sinceSec: number, nowSec: number): string {
  const diff = nowSec - sinceSec;
  if (diff < 0) {
    const days = Math.max(1, Math.round(-diff / DAY));
    return `через ${days} ${plural(days, "день", "дня", "дней")}`;
  }
  const days = Math.floor(diff / DAY);
  if (days === 0) return "сегодня";
  if (days === 1) return "вчера";
  return `${days} ${plural(days, "день", "дня", "дней")} назад`;
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
 * «данные свежие» или «данные от 1 сентября» — единая формулировка
 * свежести данных хаба. Используется и в подвале панели, и в разделе
 * «Данные» настроек: два места не должны разойтись в словах.
 */
export function hubFreshness(updatedAt: number, nowSec: number): string {
  const age = nowSec - updatedAt;
  if (age <= DAY) return "данные свежие";
  const date = new Date(updatedAt * 1000).toLocaleDateString("ru-RU", {
    day: "numeric",
    month: "long",
  });
  return `данные от ${date}`;
}
