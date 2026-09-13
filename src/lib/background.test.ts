import { describe, it, expect } from "vitest";
import { layerFor, layerKey } from "./background";

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
