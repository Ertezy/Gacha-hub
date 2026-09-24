import { afterEach, describe, expect, it } from "vitest";
import { asVideoLang, setVideoLanguage, subscribeVideoLanguage, videoLanguage } from "./videoLanguage";

afterEach(() => setVideoLanguage("en"));

describe("язык видео", () => {
  it("по умолчанию английский, незнакомое значение — английский", () => {
    expect(videoLanguage()).toBe("en");
    expect(asVideoLang("ja")).toBe("ja");
    expect(asVideoLang("ru")).toBe("en");
  });

  it("смена оповещает подписчиков", () => {
    const seen: string[] = [];
    const stop = subscribeVideoLanguage((l) => seen.push(l));
    setVideoLanguage("ja");
    stop();
    expect(seen).toEqual(["ja"]);
  });
});
