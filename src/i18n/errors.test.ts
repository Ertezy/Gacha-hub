import { describe, expect, it } from "vitest";
import { errorText } from "./errors";
import { en } from "./en";
import { ru } from "./ru";

describe("текст ошибки", () => {
  it("код с подробностью — фраза выбранного языка", () => {
    expect(errorText(en, { code: "fileMissing", detail: "C:\\g.exe" })).toBe("File not found: C:\\g.exe");
    expect(errorText(ru, { code: "fileMissing", detail: "C:\\g.exe" })).toContain("C:\\g.exe");
  });

  it("код без подробности", () => {
    expect(errorText(en, { code: "emptyTitle" })).toBe("The name can't be empty");
  });

  it("незнакомый код — общая фраза с подробностью", () => {
    expect(errorText(en, { code: "somethingNew", detail: "x" })).toBe("Something went wrong: x");
    expect(errorText(en, { code: "somethingNew" })).toBe("Something went wrong");
  });

  it("обычная ошибка JavaScript и строка", () => {
    expect(errorText(en, new Error("boom"))).toBe("boom");
    expect(errorText(en, "plain")).toBe("plain");
  });

  it("у каждого кода Rust есть фраза на обоих языках", () => {
    // Список повторяет src-tauri/src/error.rs, модуль code.
    const codes = [
      "gameNotFound", "fileMissing", "exePathMissing", "argsUnclosedQuote", "exeStartFailed",
      "steamOpenFailed", "epicOpenFailed", "emptyTitle", "hubUrlNotHttps", "videoPickFailed",
      "videoWrongFormat", "videoFileMissing", "videoTooLarge",
      "cacheReadFailed", "logFolderMissing", "openFolderFailed", "configSaveFailed", "internal",
    ];
    for (const code of codes) {
      const english = errorText(en, { code, detail: "d" });
      if (code === "internal") expect(english).toBe("Something went wrong: d");
      else expect(english).not.toBe("Something went wrong: d");
      const russian = errorText(ru, { code, detail: "d" });
      if (code === "internal") expect(russian).toBe("Что-то пошло не так: d");
      else expect(russian).not.toBe("Что-то пошло не так: d");
    }
  });

  it("русский — прежние тексты Rust дословно, с подробностью на месте переменной части", () => {
    const ruText = (code: string, detail?: string) => errorText(ru, { code, detail });
    expect(ruText("gameNotFound", "Genshin Impact")).toBe("игра больше не найдена: Genshin Impact");
    expect(ruText("gameNotFound")).toBe("игра не найдена или правка невозможна");
    expect(ruText("fileMissing", "C:\\g.exe")).toBe("файл не найден: C:\\g.exe");
    expect(ruText("exePathMissing")).toBe("путь к игре не задан");
    expect(ruText("argsUnclosedQuote", '-name "x')).toBe('не удалось разобрать аргументы «-name "x»: незакрытая кавычка.');
    expect(ruText("exeStartFailed", "C:\\g.exe: отказано")).toBe("не удалось запустить C:\\g.exe: отказано");
    expect(ruText("steamOpenFailed", "steam://rungameid/1: нет")).toBe(
      "не удалось открыть steam://rungameid/1: нет. Проверь, что Steam установлен и хотя бы раз запускался.",
    );
    expect(ruText("epicOpenFailed", "нет")).toBe(
      "не удалось открыть Epic Games Launcher: нет. Проверь, что он установлен.",
    );
    expect(ruText("emptyTitle")).toBe("название не может быть пустым");
    expect(ruText("hubUrlNotHttps")).toBe("адрес должен начинаться с https://");
    expect(ruText("videoPickFailed", "x")).toBe("Окну не открыть видео: x");
    expect(ruText("videoWrongFormat")).toBe("Видео должно быть в формате mp4 или webm.");
    expect(ruText("videoFileMissing")).toBe("Файл видео не найден.");
    expect(ruText("videoTooLarge", "300")).toBe("Видео тяжелее 300 МБ — выберите файл поменьше.");
    expect(ruText("cacheReadFailed", "x")).toBe("не читается кеш: x");
    expect(ruText("logFolderMissing", "x")).toBe("не найдена папка журнала: x");
    expect(ruText("openFolderFailed", "x")).toBe("не удалось открыть папку: x");
    expect(ruText("configSaveFailed", "запись C:\\config.json.tmp: x")).toBe("запись C:\\config.json.tmp: x");
  });

  it("английский — подробность на месте", () => {
    expect(errorText(en, { code: "videoTooLarge", detail: "300" })).toBe(
      "The video is larger than 300 MB — pick a smaller file.",
    );
    expect(errorText(en, { code: "gameNotFound" })).toBe("Game not found");
    expect(errorText(en, { code: "configSaveFailed", detail: "x" })).toBe("Couldn't save the settings: x");
  });
});
