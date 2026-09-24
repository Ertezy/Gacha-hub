import { describe, expect, it } from "vitest";

// Сторож кириллицы (спека этапа 6 §7.1): русские буквы в коде страницы живут
// только в словаре ru.ts и в native.ts. Комментарии и файлы тестов не в счёт.
//
// PENDING — файлы, которые ещё не переведены. Задачи перевода убирают из него
// свои файлы; последняя удаляет список целиком. Второй тест не даёт держать в
// списке уже чистый файл — иначе он бы тихо выпал из-под надзора.

const FILES = import.meta.glob<string>(["/src/**/*.{ts,tsx}", "!/src/**/*.test.{ts,tsx}", "!/src/**/*.d.ts"], {
  query: "?raw",
  import: "default",
  eager: true,
});

const ALLOWED = new Set(["/src/i18n/ru.ts", "/src/i18n/native.ts"]);

const PENDING = new Set<string>([
  "/src/App.tsx",
  "/src/components/GameArt.tsx",
  "/src/components/GameDock.tsx",
  "/src/components/PlayButton.tsx",
  "/src/lib/api.ts",
  "/src/lib/background.ts",
  "/src/lib/gradient.ts",
  "/src/lib/launch.ts",
  "/src/lib/motion.ts",
  "/src/screens/Settings.tsx",
  "/src/screens/settings/FirstRunView.tsx",
  "/src/screens/settings/GameEditor.tsx",
  "/src/screens/settings/GameRow.tsx",
  "/src/screens/settings/GamesSection.tsx",
  "/src/screens/settings/LookSection.tsx",
  "/src/screens/settings/SmallSections.tsx",
  "/src/types.ts",
]);

const CYRILLIC = /[Ѐ-ӿ]/;

/** Код без комментариев. Строчный комментарий — от `//`, перед которым не
 *  двоеточие: иначе резались бы адреса вида https://. */
function withoutComments(text: string): string {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => line.replace(/(^|[^:])\/\/.*$/, "$1"))
    .join("\n");
}

const hasCyrillic = (path: string) => CYRILLIC.test(withoutComments(FILES[path] ?? ""));

describe("сторож кириллицы", () => {
  it("русские буквы — только в словаре и в ещё не переведённых файлах", () => {
    const offenders = Object.keys(FILES).filter((p) => !ALLOWED.has(p) && !PENDING.has(p) && hasCyrillic(p));
    expect(offenders).toEqual([]);
  });

  it("в списке ожидающих нет уже чистых файлов", () => {
    const clean = [...PENDING].filter((p) => !hasCyrillic(p));
    expect(clean).toEqual([]);
  });
});
