import { describe, it, expect } from "vitest";
import { REFRESH_EVERY_MS, REFRESH_ON_SHOW_AFTER_MS, sameHub, shouldRefreshOnShow } from "./hubRefresh";
import type { HubData } from "../types";

const hub = (updatedAt: number): HubData => ({
  version: 2,
  updatedAt,
  games: [],
  codes: [],
  banners: [],
  videos: [],
  _source: "remote",
});

describe("hubRefresh", () => {
  it("проверяет раз в три часа и при возвращении окна через час", () => {
    expect(REFRESH_EVERY_MS).toBe(10_800_000);
    expect(shouldRefreshOnShow(0, REFRESH_ON_SHOW_AFTER_MS - 1)).toBe(false);
    expect(shouldRefreshOnShow(0, REFRESH_ON_SHOW_AFTER_MS)).toBe(true);
  });

  it("одинаковые данные не считаются новыми", () => {
    expect(sameHub(hub(1), hub(1))).toBe(true);
    expect(sameHub(hub(1), hub(2))).toBe(false);
    expect(sameHub(null, hub(1))).toBe(false);
    expect(sameHub(null, null)).toBe(true);
  });
});
