import { describe, it, expect } from "vitest";
import { rgbToHsl, pickHue } from "./hue";

describe("rgbToHsl", () => {
  it("переводит чистый красный", () => {
    const [h, s, l] = rgbToHsl(255, 0, 0);
    expect(h).toBe(0);
    expect(s).toBeCloseTo(1, 2);
    expect(l).toBeCloseTo(0.5, 2);
  });

  it("переводит чистый синий", () => {
    expect(rgbToHsl(0, 0, 255)[0]).toBe(240);
  });

  it("у серого насыщенность нулевая", () => {
    expect(rgbToHsl(128, 128, 128)[1]).toBeCloseTo(0, 3);
  });
});

describe("pickHue", () => {
  it("берёт самый частый оттенок", () => {
    // Три насыщенных красных пикселя против одного синего.
    const px = new Uint8ClampedArray([
      220, 30, 30, 255,
      220, 30, 30, 255,
      220, 30, 30, 255,
      30, 30, 220, 255,
    ]);
    const hue = pickHue(px);
    expect(hue).not.toBeNull();
    expect(hue! < 20 || hue! > 340).toBe(true);
  });

  it("отбрасывает блёклые пиксели", () => {
    // Много почти-серого и мало насыщенного синего: победить должен синий.
    const px = new Uint8ClampedArray([
      130, 128, 128, 255,
      128, 130, 128, 255,
      128, 128, 130, 255,
      30, 30, 220, 255,
    ]);
    const hue = pickHue(px);
    expect(hue).not.toBeNull();
    expect(hue!).toBeGreaterThan(200);
    expect(hue!).toBeLessThan(280);
  });

  it("отбрасывает слишком тёмное и слишком светлое", () => {
    // Насыщенные, но чёрный и белый края не должны решать.
    const px = new Uint8ClampedArray([
      8, 0, 0, 255,
      250, 248, 248, 255,
      30, 30, 220, 255,
    ]);
    const hue = pickHue(px);
    expect(hue).toBeGreaterThan(200);
    expect(hue!).toBeLessThan(280);
  });

  it("на почти сером отказывается молча", () => {
    // Отказ обязан быть тихим: оттенок остаётся заданным в палитре, а не
    // становится случайным.
    const px = new Uint8ClampedArray([
      128, 128, 128, 255,
      130, 130, 130, 255,
    ]);
    expect(pickHue(px)).toBeNull();
  });

  it("на пустых данных отказывается молча", () => {
    expect(pickHue(new Uint8ClampedArray([]))).toBeNull();
  });
});
