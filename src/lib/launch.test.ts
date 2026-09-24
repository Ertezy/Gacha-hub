import { describe, expect, it } from "vitest";
import { launchNote } from "./launch";
import { en } from "../i18n/en";
import { ru } from "../i18n/ru";

describe("подпись над кнопкой запуска", () => {
  it("английский", () => {
    expect(launchNote(en, { sourceKind: "steam", missing: false })).toBe("Launches via Steam");
    expect(launchNote(en, { sourceKind: "epic", missing: false })).toBe("Launches via Epic Games");
    expect(launchNote(en, { sourceKind: "exe", missing: true })).toBe("Launches directly · game file not found");
  });

  it("русский — «напрямую» без «через»", () => {
    expect(launchNote(ru, { sourceKind: "steam", missing: false })).toBe("Запустится через Steam");
    expect(launchNote(ru, { sourceKind: "exe", missing: false })).toBe("Запустится напрямую");
    expect(launchNote(ru, { sourceKind: "epic", missing: true })).toBe("Запустится через Epic Games · файл не найден");
  });
});
