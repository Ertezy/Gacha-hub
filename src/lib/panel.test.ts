import { describe, it, expect } from "vitest";
import { activeCodes, dataStatus, NO_DATA_YET, panelIsEmpty, runningBanners } from "./panel";
import type { Banner, Code, HubData, Video } from "../types";

const NOW = 1788091200;

const code = (expiresAt: number | null, name = "C"): Code => ({
  gameId: "g",
  code: name,
  rewards: "",
  expiresAt,
  region: "",
  source: null,
});

const banner = (startsAt: number, endsAt: number): Banner => ({
  gameId: "g",
  title: "B",
  featured: [],
  rarity: null,
  image: null,
  startsAt,
  endsAt,
  url: null,
});

const video: Video = {
  gameId: "g",
  title: "V",
  url: "https://example.test",
  thumb: null,
  publishedAt: NOW - 100,
  duration: null,
  premiere: false,
};

const hub = (source: string, codes: Code[] = []): HubData => ({
  version: 1,
  updatedAt: NOW - 60,
  games: [],
  codes,
  banners: [],
  videos: [],
  _source: source,
});

describe("activeCodes", () => {
  it("убирает сгоревшие и ставит бессрочные в конец", () => {
    const list = [code(null, "forever"), code(NOW - 1, "burnt"), code(NOW + 50, "soon")];
    expect(activeCodes(list, NOW).map((c) => c.code)).toEqual(["soon", "forever"]);
  });
});

describe("runningBanners", () => {
  it("оставляет только идущие сейчас", () => {
    const list = [banner(NOW - 10, NOW + 10), banner(NOW + 5, NOW + 10), banner(NOW - 10, NOW)];
    expect(runningBanners(list, NOW)).toHaveLength(1);
  });
});

describe("panelIsEmpty", () => {
  it("пусто, когда коды сгорели, баннеры кончились и видео нет", () => {
    expect(panelIsEmpty([code(NOW - 1)], [banner(NOW - 10, NOW)], [], NOW)).toBe(true);
  });
  it("не пусто, когда есть видео", () => {
    expect(panelIsEmpty([], [], [video], NOW)).toBe(false);
  });
  it("не пусто, когда идёт баннер", () => {
    expect(panelIsEmpty([], [banner(NOW - 10, NOW + 10)], [], NOW)).toBe(false);
  });
});

describe("dataStatus", () => {
  it("без данных говорит, что они появятся с сервисом", () => {
    expect(dataStatus(null, NOW)).toBe(NO_DATA_YET);
  });
  it("данные из комплекта — то же самое", () => {
    expect(dataStatus(hub("bundled", [code(null)]), NOW)).toBe(NO_DATA_YET);
  });
  it("пустые данные из сети — то же самое", () => {
    expect(dataStatus(hub("remote"), NOW)).toBe(NO_DATA_YET);
  });
  it("живые данные называют источник и свежесть", () => {
    expect(dataStatus(hub("remote", [code(null)]), NOW)).toBe("Источник — из сети, данные свежие.");
  });
});
