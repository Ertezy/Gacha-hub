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
});
