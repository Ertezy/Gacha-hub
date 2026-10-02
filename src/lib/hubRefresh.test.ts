import { describe, it, expect } from "vitest";
import { REFRESH_EVERY_MS, REFRESH_ON_SHOW_AFTER_MS, ONLINE_REFRESH_AFTER_MS, backgroundsKey, sameHub, shouldRefreshOnShow, shouldRefreshOnOnline } from "./hubRefresh";
import type { HubData, HubGame } from "../types";

const hub = (updatedAt: number): HubData => ({
  version: 2,
  updatedAt,
  games: [],
  codes: [],
  banners: [],
  videos: [],
  _source: "remote",
});

const game = (id: string, background?: HubGame["background"]): HubGame => ({
  id,
  title: id,
  redeemUrl: null,
  ...(background ? { background } : {}),
});

describe("hubRefresh", () => {
  it("проверяет раз в три часа и при возвращении окна через час", () => {
    expect(REFRESH_EVERY_MS).toBe(10_800_000);
    expect(shouldRefreshOnShow(0, REFRESH_ON_SHOW_AFTER_MS - 1)).toBe(false);
    expect(shouldRefreshOnShow(0, REFRESH_ON_SHOW_AFTER_MS)).toBe(true);
  });

  it("при появлении сети проверяет, если с прошлой проверки прошло больше минуты", () => {
    expect(ONLINE_REFRESH_AFTER_MS).toBe(60_000);
    expect(shouldRefreshOnOnline(0, 60_000)).toBe(false);
    expect(shouldRefreshOnOnline(0, 60_001)).toBe(true);
  });

  it("одинаковые данные не считаются новыми", () => {
    expect(sameHub(hub(1), hub(1))).toBe(true);
    expect(sameHub(hub(1), hub(2))).toBe(false);
    expect(sameHub(null, hub(1))).toBe(false);
    expect(sameHub(null, null)).toBe(true);
  });
});

describe("backgroundsKey", () => {
  const withGames = (games: HubGame[], rest: Partial<HubData> = {}): HubData => ({ ...hub(1), games, ...rest });

  it("меняется, когда сменилась картинка фона игры", () => {
    const a = withGames([game("genshin", { image: "a.png" })]);
    const b = withGames([game("genshin", { image: "b.png" })]);
    expect(backgroundsKey(a)).not.toBe(backgroundsKey(b));
  });

  it("меняется, когда сменилось или появилось видео фона", () => {
    const still = withGames([game("genshin", { image: "a.png" })]);
    const video = withGames([game("genshin", { image: "a.png", video: "a.mp4" })]);
    const other = withGames([game("genshin", { image: "a.png", video: "b.mp4" })]);
    expect(backgroundsKey(still)).not.toBe(backgroundsKey(video));
    expect(backgroundsKey(video)).not.toBe(backgroundsKey(other));
  });

  it("меняется, когда у игры появился фон", () => {
    const without = withGames([game("genshin")]);
    const withBg = withGames([game("genshin", { image: "a.png" })]);
    expect(backgroundsKey(without)).not.toBe(backgroundsKey(withBg));
  });

  it("не меняется от кодов, баннеров, видео и времени обновления", () => {
    const base = withGames([game("genshin", { image: "a.png" })]);
    const noisy = withGames([game("genshin", { image: "a.png" })], {
      updatedAt: 999,
      codes: [{ gameId: "genshin", code: "ABC", rewards: "", expiresAt: null, region: "", source: null }],
      banners: [{ gameId: "genshin", title: "b", featured: [], rarity: null, image: null, startsAt: 1, endsAt: 2, url: null }],
      videos: [{ gameId: "genshin", title: "t", url: "u", thumb: null, publishedAt: 1, duration: null, premiere: false }],
    });
    expect(backgroundsKey(noisy)).toBe(backgroundsKey(base));
  });

  it("пустой хаб и его отсутствие дают один и тот же ключ", () => {
    expect(backgroundsKey(null)).toBe(backgroundsKey(withGames([])));
  });
});
