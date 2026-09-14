import { describe, it, expect } from "vitest";
import { launchNote } from "./launch";

describe("launchNote", () => {
  it("говорит, откуда запустится игра", () => {
    expect(launchNote({ sourceLabel: "Epic Games", missing: false })).toBe(
      "Запустится через Epic Games",
    );
  });

  it("добавляет пометку, когда файла игры нет", () => {
    expect(launchNote({ sourceLabel: "напрямую", missing: true })).toBe(
      "Запустится через напрямую · файл не найден",
    );
  });
});
