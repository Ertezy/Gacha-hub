import { describe, it, expect } from "vitest";
import { motionOn, setMotion, subscribeMotion } from "./motion";

describe("motionOn", () => {
  it("включена по умолчанию", () => {
    expect(motionOn()).toBe(true);
  });
});

describe("setMotion", () => {
  it("меняет то, что дальше отдаёт motionOn", () => {
    setMotion(false);
    expect(motionOn()).toBe(false);
    setMotion(true);
    expect(motionOn()).toBe(true);
  });

  it("не падает без DOM — окружение тестов его не даёт", () => {
    // Настоящая проверка защитного `typeof document !== "undefined"`:
    // окружение тестов — vitest без jsdom, и `document` здесь не существует.
    expect(() => setMotion(false)).not.toThrow();
    setMotion(true);
  });
});

describe("subscribeMotion", () => {
  it("сообщает подписчику каждое новое значение", () => {
    const seen: boolean[] = [];
    const unsubscribe = subscribeMotion((on) => seen.push(on));
    setMotion(false);
    setMotion(true);
    unsubscribe();
    expect(seen).toEqual([false, true]);
  });

  it("отписавшийся слушатель больше не получает значений", () => {
    const seen: boolean[] = [];
    const unsubscribe = subscribeMotion((on) => seen.push(on));
    unsubscribe();
    setMotion(false);
    expect(seen).toEqual([]);
    setMotion(true);
  });

  it("два независимых слушателя получают одно и то же значение", () => {
    const a: boolean[] = [];
    const b: boolean[] = [];
    const stopA = subscribeMotion((on) => a.push(on));
    const stopB = subscribeMotion((on) => b.push(on));
    setMotion(false);
    stopA();
    stopB();
    expect(a).toEqual([false]);
    expect(b).toEqual([false]);
    setMotion(true);
  });
});
