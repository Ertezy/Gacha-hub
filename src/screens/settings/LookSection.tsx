import { useCallback, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { api } from "../../lib/api";
import { gradientFor } from "../../lib/gradient";
import { hasOwnBackground, sourceText, videoSizeProblem } from "../../lib/background";
import { setMotion } from "../../lib/motion";
import Switch from "./Switch";
import type { GameView } from "../../types";

/** Вкладка «Вид»: всё про фоны в одном месте (спека этапа 5, §7.3). */
export default function LookSection() {
  const [games, setGames] = useState<GameView[]>([]);
  const [storeArt, setStoreArt] = useState<boolean | null>(null);
  const [animation, setAnimation] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const reload = useCallback(async () => {
    try {
      const [list, storeArtEnabled, animationEnabled] = await Promise.all([
        api.getGames(),
        api.getStoreArt(),
        api.getAnimation(),
      ]);
      setGames(list);
      setStoreArt(storeArtEnabled);
      setAnimation(animationEnabled);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    let active = true;
    void reload();
    // Картинки Epic докачиваются в фоне и могут появиться, пока вкладка открыта.
    // Подписка регистрируется асинхронно, а Tauri не копит события для тех, кто
    // подпишется позже. Поэтому, как только подписка точно действует, список
    // перечитывается ещё раз: всё, что успело докачаться, уже лежит в кеше.
    const unlisten = listen("games-changed", () => void reload());
    void unlisten.then(() => {
      if (active) void reload();
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, [reload]);

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const pick = (gameId: string, kind: "background" | "video") =>
    void (async () => {
      if (kind === "background") {
        const picked = await api.pickImage();
        if (picked) await run(() => api.updateGame({ gameId, background: picked }));
        return;
      }
      const picked = await api.pickVideo();
      if (!picked) return;
      // Вся цепочка — проверка формата и веса в Rust, затем размер в точках —
      // идёт внутри `run`: кнопки блокируются на время проверки, а отказ
      // `checkVideo` попадает в баннер ошибки той же дорогой, что и остальные.
      await run(async () => {
        await api.checkVideo(picked);
        const { width, height } = await videoPixelSize(picked);
        const problem = videoSizeProblem(width, height);
        if (problem) throw problem;
        await api.updateGame({ gameId, video: picked });
      });
    })();

  return (
    <div>
      <h2 className="settings-section-title">Вид</h2>
      {error && <div className="settings-error">{error}</div>}

      <div className="settings-card">
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Брать фоны из магазинов</span>
            <span className="settings-row-hint">
              Для игр из Steam и Epic Games берётся картинка из самого магазина. Своя
              картинка или видео всегда важнее.
            </span>
          </div>
          <div className="settings-row-control">
            {storeArt !== null && (
              <Switch
                checked={storeArt}
                label="Брать фоны из магазинов"
                disabled={busy}
                onChange={(next) => void run(() => api.setStoreArt(next))}
              />
            )}
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Анимация</span>
            <span className="settings-row-hint">
              Видео фона, медленное движение картинки, увеличение иконок в доке и
              поворот шестерёнки. Настройка Windows на приложение не влияет.
            </span>
          </div>
          <div className="settings-row-control">
            {animation !== null && (
              <Switch
                checked={animation}
                label="Анимация"
                disabled={busy}
                onChange={(next) =>
                  void run(async () => {
                    await api.setAnimation(next);
                    // Применяется сразу, не дожидаясь закрытия настроек.
                    setMotion(next);
                  })
                }
              />
            )}
          </div>
        </div>
      </div>

      {games.length === 0 ? (
        <p className="settings-note">Игр пока нет — добавьте их в разделе «Игры».</p>
      ) : (
        <div className="settings-card">
          {games.map((g) => (
            <div className="settings-row" key={g.id}>
              <Preview game={g} />
              <div className="settings-row-text">
                <span className="settings-row-title">{g.title}</span>
                <span className={g.videoMissing ? "settings-row-hint look-warn" : "settings-row-hint"}>
                  {sourceText(g)}
                </span>
              </div>
              <div className="settings-row-control">
                <button
                  type="button"
                  className="button"
                  disabled={busy}
                  onClick={() => pick(g.id, "background")}
                >
                  Картинка…
                </button>
                <button
                  type="button"
                  className="button"
                  disabled={busy}
                  onClick={() => pick(g.id, "video")}
                >
                  Видео…
                </button>
                <button
                  type="button"
                  className="button"
                  disabled={busy || !hasOwnBackground(g)}
                  onClick={() =>
                    void run(() => api.updateGame({ gameId: g.id, background: "", video: "" }))
                  }
                >
                  Убрать
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * Превью фона. Видео — неподвижным первым кадром, а не проигрыванием: пять
 * роликов сразу на одном экране грузили бы компьютер (спека §7.3).
 */
function Preview({ game }: { game: GameView }) {
  if (game.videoPath) {
    return (
      <video
        className="look-preview"
        src={convertFileSrc(game.videoPath)}
        preload="metadata"
        muted
        playsInline
      />
    );
  }
  if (game.artPath) {
    return <img className="look-preview" src={convertFileSrc(game.artPath)} alt="" />;
  }
  return <div className="look-preview" style={{ background: gradientFor(game.title) }} />;
}

/**
 * Размер видео в точках. Измеряется здесь, на странице, а не в Rust: Rust
 * умеет проверить формат и вес файла, но разбор самого mp4 или webm ради
 * одних только размеров кадра пришлось бы писать вручную — готового парсера
 * в зависимостях нет, а детачнутый `<video>` браузера делает это бесплатно.
 *
 * Файл к этому моменту уже разрешён окну командой `checkVideo`. `error` и
 * десятисекундный таймер дают `0, 0` — тот же сигнал «не удалось прочитать»,
 * что и у испорченного файла.
 */
function videoPixelSize(path: string): Promise<{ width: number; height: number }> {
  return new Promise((resolve) => {
    const video = document.createElement("video");
    video.preload = "metadata";
    video.muted = true;

    let done = false;
    const finish = (width: number, height: number) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      // Файл держится открытым, пока у элемента есть источник — освобождаем
      // его сразу, не дожидаясь сборки мусора.
      video.src = "";
      video.load();
      resolve({ width, height });
    };

    const timer = setTimeout(() => finish(0, 0), 10_000);
    video.addEventListener("loadedmetadata", () => finish(video.videoWidth, video.videoHeight));
    video.addEventListener("error", () => finish(0, 0));
    video.src = convertFileSrc(path);
  });
}
