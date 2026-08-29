import { describe, expect, it } from "vitest";
import { gradientFor } from "./gradient";

describe("gradientFor", () => {
  it("возвращает один и тот же градиент для одного названия", () => {
    expect(gradientFor("Limbus Company")).toBe(gradientFor("Limbus Company"));
  });

  it("разным названиям даёт разные градиенты", () => {
    expect(gradientFor("Limbus Company")).not.toBe(gradientFor("Blue Archive"));
  });

  it("выдаёт корректный CSS-градиент", () => {
    expect(gradientFor("Limbus Company")).toMatch(
      /^linear-gradient\(135deg, hsl\(\d+ \d+% \d+%\), hsl\(\d+ \d+% \d+%\)\)$/,
    );
  });

  it("не падает на пустом названии", () => {
    expect(gradientFor("")).toContain("linear-gradient");
  });

  it("похожие_по_хешу_названия_всё_равно_различимы", () => {
    const parseFirstHsl = (gradient: string): [number, number, number] => {
      const match = gradient.match(/hsl\((\d+) (\d+)% (\d+)%\)/);
      if (!match) {
        throw new Error(`не найден hsl(...) в градиенте: ${gradient}`);
      }
      return [Number(match[1]), Number(match[2]), Number(match[3])];
    };

    const isDistinct = (
      a: [number, number, number],
      b: [number, number, number],
    ): boolean => {
      const rawHueGap = Math.abs(a[0] - b[0]);
      const hueGap = Math.min(rawHueGap, 360 - rawHueGap);
      const satGap = Math.abs(a[1] - b[1]);
      const lightGap = Math.abs(a[2] - b[2]);
      return hueGap > 20 || satGap >= 6 || lightGap >= 5;
    };

    // Три тайтла HoYoverse — правдоподобный набор в одном хабе, и их хеши
    // случайно сходятся по оттенку (161°, 157°, 151° — Δ4..10°). Если
    // насыщенность и светлота не расходятся между собой, три разные игры
    // выглядят как одна и та же заливка.
    const titles = ["Genshin Impact", "Honkai: Star Rail", "Zenless Zone Zero"];
    const triples = titles.map((t) => parseFirstHsl(gradientFor(t)));

    for (let i = 0; i < triples.length; i++) {
      for (let j = i + 1; j < triples.length; j++) {
        expect(isDistinct(triples[i], triples[j])).toBe(true);
      }
    }
  });
});
