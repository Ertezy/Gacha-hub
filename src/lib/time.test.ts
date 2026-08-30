import { describe, it, expect } from "vitest";
import { plural, timeLeft, timeAgo, burnsToday, progress } from "./time";

const HOUR = 3600;
const DAY = 86400;
const NOW = 1788091200; // 30 августа 2026, 12:00 UTC

describe("plural", () => {
  // Правило зависит от последней цифры, кроме 11–14, где не зависит.
  const cases: [number, string][] = [
    [1, "день"], [2, "дня"], [3, "дня"], [4, "дня"],
    [5, "дней"], [9, "дней"], [10, "дней"],
    [11, "дней"], [12, "дней"], [13, "дней"], [14, "дней"],
    [21, "день"], [22, "дня"], [25, "дней"],
    [101, "день"], [102, "дня"], [105, "дней"],
    [111, "дней"], [112, "дней"], [114, "дней"],
    [121, "день"],
    [0, "дней"],
  ];
  for (const [n, expected] of cases) {
    it(`${n} -> ${expected}`, () => {
      expect(plural(n, "день", "дня", "дней")).toBe(expected);
    });
  }
});

describe("timeLeft", () => {
  it("часы и минуты, когда меньше суток", () => {
    expect(timeLeft(NOW + 3 * HOUR + 52 * 60, NOW)).toBe("3 ч 52 мин");
  });
  it("только минуты, когда меньше часа", () => {
    expect(timeLeft(NOW + 7 * 60, NOW)).toBe("7 мин");
  });
  it("дни, когда больше суток", () => {
    expect(timeLeft(NOW + 9 * DAY, NOW)).toBe("9 дней");
  });
  it("один день склоняется верно", () => {
    expect(timeLeft(NOW + 1 * DAY + HOUR, NOW)).toBe("1 день");
  });
  it("двадцать один день склоняется верно", () => {
    expect(timeLeft(NOW + 21 * DAY, NOW)).toBe("21 день");
  });
  it("истёкшее время", () => {
    expect(timeLeft(NOW - HOUR, NOW)).toBe("истёк");
  });
});

describe("timeAgo", () => {
  it("сегодня", () => expect(timeAgo(NOW - HOUR, NOW)).toBe("сегодня"));
  it("вчера", () => expect(timeAgo(NOW - 1 * DAY, NOW)).toBe("вчера"));
  it("несколько дней", () => expect(timeAgo(NOW - 4 * DAY, NOW)).toBe("4 дня назад"));
  it("одиннадцать дней", () => expect(timeAgo(NOW - 11 * DAY, NOW)).toBe("11 дней назад"));
  it("будущее — премьера", () => expect(timeAgo(NOW + 2 * DAY, NOW)).toBe("через 2 дня"));
});

describe("burnsToday", () => {
  it("сгорает через три часа", () => expect(burnsToday(NOW + 3 * HOUR, NOW)).toBe(true));
  it("сгорает через три дня", () => expect(burnsToday(NOW + 3 * DAY, NOW)).toBe(false));
  it("уже сгорел", () => expect(burnsToday(NOW - HOUR, NOW)).toBe(false));
});

describe("progress", () => {
  it("половина срока", () => {
    expect(progress(NOW - 5 * DAY, NOW + 5 * DAY, NOW)).toBeCloseTo(0.5, 3);
  });
  it("не выходит за единицу", () => {
    expect(progress(NOW - 10 * DAY, NOW - 1 * DAY, NOW)).toBe(1);
  });
  it("не уходит ниже нуля", () => {
    expect(progress(NOW + 1 * DAY, NOW + 5 * DAY, NOW)).toBe(0);
  });
  it("нулевая длительность не даёт деления на ноль", () => {
    expect(progress(NOW, NOW, NOW)).toBe(1);
  });
});
