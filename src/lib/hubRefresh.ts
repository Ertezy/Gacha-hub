import type { HubData } from "../types";

/** Пока приложение открыто или лежит в трее, данные панели проверяются раз в 3 часа (спека сборщика §9). */
export const REFRESH_EVERY_MS = 3 * 60 * 60 * 1000;

/** При возвращении окна данные проверяются, если с прошлой проверки прошёл час. */
export const REFRESH_ON_SHOW_AFTER_MS = 60 * 60 * 1000;

export function shouldRefreshOnShow(lastCheckMs: number, nowMs: number): boolean {
  return nowMs - lastCheckMs >= REFRESH_ON_SHOW_AFTER_MS;
}

/** Те же ли данные. Сервер ответил «не изменилось» — панель перерисовывать незачем. */
export function sameHub(a: HubData | null, b: HubData | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
