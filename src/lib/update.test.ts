import { describe, it, expect } from "vitest";
import { availableUpdate, isNewer } from "./update";
import type { HubData } from "../types";

const hub = (app?: { version: string; url: string }): HubData => ({
  version: 2,
  updatedAt: 1,
  games: [],
  codes: [],
  banners: [],
  videos: [],
  ...(app ? { app } : {}),
});

const RELEASE = { version: "0.1.1", url: "https://github.com/Ertezy/Kitsudock/releases/tag/v0.1.1" };

describe("isNewer", () => {
  it("compares the three numbers as numbers", () => {
    expect(isNewer("0.1.1", "0.1.0")).toBe(true);
    expect(isNewer("0.2.0", "0.1.9")).toBe(true);
    expect(isNewer("1.0.0", "0.9.9")).toBe(true);
    expect(isNewer("0.10.0", "0.9.0")).toBe(true);
  });

  it("the same or an older version is not newer", () => {
    expect(isNewer("0.1.0", "0.1.0")).toBe(false);
    expect(isNewer("0.1.0", "0.1.1")).toBe(false);
    expect(isNewer("0.9.0", "0.10.0")).toBe(false);
  });

  it("a version it cannot read is never newer", () => {
    expect(isNewer("0.2", "0.1.0")).toBe(false);
    expect(isNewer("0.2.0-beta", "0.1.0")).toBe(false);
    expect(isNewer("0.2.0", "dev")).toBe(false);
  });
});

describe("availableUpdate", () => {
  it("returns the release only when it is newer than the running app", () => {
    expect(availableUpdate(hub(RELEASE), "0.1.0")).toEqual(RELEASE);
    expect(availableUpdate(hub(RELEASE), "0.1.1")).toBeNull();
    expect(availableUpdate(hub(RELEASE), "0.2.0")).toBeNull();
  });

  it("nothing to say without data, without the field or before the version is known", () => {
    expect(availableUpdate(null, "0.1.0")).toBeNull();
    expect(availableUpdate(hub(), "0.1.0")).toBeNull();
    expect(availableUpdate(hub(RELEASE), null)).toBeNull();
  });
});
