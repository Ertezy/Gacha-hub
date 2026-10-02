import { describe, it, expect } from "vitest";
import { hasOwnBackground, layerFor, layerKey, sourceText, videoSizeProblem } from "./background";
import { en } from "../i18n/en";
import { ru } from "../i18n/ru";

describe("layerFor", () => {
  it("своё видео идёт первым, картинка остаётся его неподвижным кадром", () => {
    expect(layerFor({ videoPath: "v.mp4", artPath: "a.img" }, false)).toEqual({
      kind: "video",
      path: "v.mp4",
      still: "a.img",
    });
  });

  it("при «уменьшить движение» вместо видео показывается картинка", () => {
    expect(layerFor({ videoPath: "v.mp4", artPath: "a.img" }, true)).toEqual({
      kind: "image",
      path: "a.img",
    });
  });

  it("при «уменьшить движение» без картинки остаётся заливка, а не видео", () => {
    expect(layerFor({ videoPath: "v.mp4", artPath: null }, true)).toEqual({ kind: "fill" });
  });

  it("без видео показывается картинка", () => {
    expect(layerFor({ videoPath: null, artPath: "a.img" }, false)).toEqual({
      kind: "image",
      path: "a.img",
    });
  });

  it("ничего нет — заливка", () => {
    expect(layerFor({ videoPath: null, artPath: null }, false)).toEqual({ kind: "fill" });
  });
});

describe("layerKey", () => {
  it("видео и картинка с одинаковым путём — разные слои", () => {
    expect(layerKey({ kind: "video", path: "x", still: null })).not.toBe(
      layerKey({ kind: "image", path: "x" }),
    );
  });

  it("у заливки один ключ", () => {
    expect(layerKey({ kind: "fill" })).toBe("fill");
  });
});

describe("sourceText", () => {
  const ruCases: [Parameters<typeof sourceText>[1]["artSource"], string][] = [
    ["video", "своё видео"],
    ["picture", "своя картинка"],
    ["steam", "из Steam"],
    ["epic", "из Epic Games"],
    ["fill", "заливка"],
  ];
  for (const [artSource, expected] of ruCases) {
    it(`русский: ${artSource} -> ${expected}`, () => {
      expect(sourceText(ru, { artSource, videoMissing: false })).toBe(expected);
    });
  }

  const enCases: [Parameters<typeof sourceText>[1]["artSource"], string][] = [
    ["video", "custom video"],
    ["picture", "custom image"],
    ["steam", "from Steam"],
    ["epic", "from Epic Games"],
    ["fill", "gradient"],
  ];
  for (const [artSource, expected] of enCases) {
    it(`английский: ${artSource} -> ${expected}`, () => {
      expect(sourceText(en, { artSource, videoMissing: false })).toBe(expected);
    });
  }

  it("пропавшее видео называется прямо — русский", () => {
    expect(sourceText(ru, { artSource: "epic", videoMissing: true })).toBe(
      "из Epic Games, файл видео не найден",
    );
  });

  it("пропавшее видео называется прямо — английский", () => {
    expect(sourceText(en, { artSource: "epic", videoMissing: true })).toBe(
      "from Epic Games, video file not found",
    );
  });

  it("labels the official launcher background like the launch button", () => {
    expect(sourceText(en, { artSource: "hoyoplay", videoMissing: false })).toBe(en.main.launch.from.hoyoplay);
    expect(sourceText(ru, { artSource: "hoyoplay", videoMissing: false })).toBe(ru.main.launch.from.hoyoplay);
  });
});

describe("hasOwnBackground", () => {
  it("своё видео и своя картинка — есть что убрать", () => {
    expect(hasOwnBackground({ artSource: "video", videoMissing: false })).toBe(true);
    expect(hasOwnBackground({ artSource: "picture", videoMissing: false })).toBe(true);
  });
  it("пропавшее видео тоже можно убрать", () => {
    expect(hasOwnBackground({ artSource: "fill", videoMissing: true })).toBe(true);
  });
  it("магазин и заливка — убирать нечего", () => {
    expect(hasOwnBackground({ artSource: "steam", videoMissing: false })).toBe(false);
    expect(hasOwnBackground({ artSource: "fill", videoMissing: false })).toBe(false);
  });
});

describe("videoSizeProblem", () => {
  it("1920×1080 годится", () => {
    expect(videoSizeProblem(ru, 1920, 1080)).toBeNull();
  });

  it("2560×1440 — ровно на потолке, годится", () => {
    expect(videoSizeProblem(ru, 2560, 1440)).toBeNull();
  });

  it("1440×2560 — тот же потолок повёрнутым боком, годится", () => {
    expect(videoSizeProblem(ru, 1440, 2560)).toBeNull();
  });

  it("3840×2160 отклоняется — русский", () => {
    expect(videoSizeProblem(ru, 3840, 2160)).toBe("Видео больше 2560×1440 — выберите ролик поменьше.");
  });

  it("3840×2160 отклоняется — английский", () => {
    expect(videoSizeProblem(en, 3840, 2160)).toBe("Video is larger than 2560×1440 — pick a smaller clip.");
  });

  it("2561×1000 отклоняется — превышена только большая сторона", () => {
    expect(videoSizeProblem(ru, 2561, 1000)).toBe("Видео больше 2560×1440 — выберите ролик поменьше.");
  });

  it("0×0 нечитаемо — русский", () => {
    expect(videoSizeProblem(ru, 0, 0)).toBe(
      "Не удалось прочитать видео — выберите mp4 (H.264) или webm.",
    );
  });

  it("0×0 нечитаемо — английский", () => {
    expect(videoSizeProblem(en, 0, 0)).toBe(
      "Couldn't read the video — pick an mp4 (H.264) or webm file.",
    );
  });

  it("NaN нечитаемо", () => {
    expect(videoSizeProblem(ru, NaN, NaN)).toBe(
      "Не удалось прочитать видео — выберите mp4 (H.264) или webm.",
    );
  });
});
