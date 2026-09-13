import { useCallback, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { api } from "../../lib/api";
import { gradientFor } from "../../lib/gradient";
import { hasOwnBackground, sourceText } from "../../lib/background";
import Switch from "./Switch";
import type { GameView } from "../../types";

/** Вкладка «Вид»: всё про фоны в одном месте (спека этапа 5, §7.3). */
export default function LookSection() {
  const [games, setGames] = useState<GameView[]>([]);
  const [storeArt, setStoreArt] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const reload = useCallback(async () => {
    try {
      const [list, enabled] = await Promise.all([api.getGames(), api.getStoreArt()]);
      setGames(list);
      setStoreArt(enabled);
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
      const picked = kind === "video" ? await api.pickVideo() : await api.pickImage();
      if (picked) await run(() => api.updateGame({ gameId, [kind]: picked }));
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
