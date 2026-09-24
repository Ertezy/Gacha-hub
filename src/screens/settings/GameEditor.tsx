import { useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "../../lib/api";
import { useT } from "../../i18n";
import { errorText } from "../../i18n/errors";
import type { GameView, HubGame } from "../../types";

interface Props {
  game: GameView;
  /** Игры из каталога — для привязки контента. */
  hubGames: HubGame[];
  onChanged: () => void;
}

export default function GameEditor({ game, hubGames, onChanged }: Props) {
  const [title, setTitle] = useState(game.title);
  const [args, setArgs] = useState(game.args);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const t = useT();

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
      onChanged();
    } catch (e) {
      setError(errorText(t, e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      {error && <div className="settings-error">{error}</div>}

      <div className="settings-row">
        <div className="settings-row-text">
          <span className="settings-row-title">Название</span>
        </div>
        <div className="settings-row-control">
          <input
            className="input editor-input"
            aria-label="Название"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <button
            type="button"
            className="button"
            disabled={busy || title.trim() === "" || title === game.title}
            onClick={() => void run(() => api.updateGame({ gameId: game.id, title }))}
          >
            Сохранить
          </button>
        </div>
      </div>

      <div className="settings-row">
        <div className="settings-row-text">
          <span className="settings-row-title">Файл игры</span>
          <span className="settings-row-hint">
            Указанный вручную файл запускается напрямую, а не через магазин.
          </span>
        </div>
        <div className="settings-row-control">
          <button
            type="button"
            className="button"
            disabled={busy}
            onClick={() =>
              void run(async () => {
                const exe = await api.pickExe();
                if (exe) await api.updateGame({ gameId: game.id, exe });
              })
            }
          >
            Выбрать…
          </button>
        </div>
      </div>

      <div className="settings-row">
        <div className="settings-row-text">
          <span className="settings-row-title">Иконка</span>
          <span className="settings-row-hint">Пусто — иконка берётся из файла игры.</span>
        </div>
        <div className="settings-row-control">
          <button
            type="button"
            className="button"
            disabled={busy}
            onClick={() =>
              void (async () => {
                const picked = await api.pickImage();
                if (!picked) return;
                await run(() => api.updateGame({ gameId: game.id, icon: picked }));
              })()
            }
          >
            Выбрать…
          </button>
          <button
            type="button"
            className="button"
            disabled={busy}
            onClick={() => void run(() => api.updateGame({ gameId: game.id, icon: "" }))}
          >
            Сбросить
          </button>
        </div>
      </div>

      <div className="settings-row">
        <div className="settings-row-text">
          <span className="settings-row-title">Показывать контент как</span>
          <span className="settings-row-hint">
            Коды, баннеры и видео на панели слева берутся для этой игры.
          </span>
        </div>
        <div className="settings-row-control">
          <select
            className="input"
            aria-label="Показывать контент как"
            value={game.contentId ?? ""}
            disabled={busy}
            onChange={(e) =>
              void run(() => api.updateGame({ gameId: game.id, contentId: e.target.value }))
            }
          >
            <option value="">нет — коды и баннеры не показывать</option>
            {hubGames.map((h) => (
              <option key={h.id} value={h.id}>
                {h.title}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="settings-row">
        <div className="settings-row-text" />
        <div className="settings-row-control">
          {game.missing && (
            <button
              type="button"
              className="button"
              disabled={busy}
              onClick={() => void run(() => api.relocateGame(game.id))}
            >
              Найти заново
            </button>
          )}
          <button
            type="button"
            className="button danger"
            disabled={busy}
            onClick={() => {
              void (async () => {
                const yes = await confirm(`Удалить «${game.title}» из списка?`, {
                  title: "Удаление игры",
                  kind: "warning",
                  okLabel: "Удалить",
                  cancelLabel: "Отмена",
                });
                if (yes) await run(() => api.removeGame(game.id));
              })();
            }}
          >
            Удалить игру
          </button>
        </div>
      </div>

      <details className="settings-extra">
        <summary>Дополнительно</summary>
        {game.sourceLabel === "напрямую" ? (
          <div className="settings-row">
            <div className="settings-row-text">
              <span className="settings-row-title">Аргументы запуска</span>
              <span className="settings-row-hint">
                Передаются игре как есть. Неверное значение может помешать ей запуститься.
              </span>
            </div>
            <div className="settings-row-control">
              <input
                className="input editor-input"
                aria-label="Аргументы запуска"
                value={args}
                onChange={(e) => setArgs(e.target.value)}
                placeholder="например, -window"
              />
              <button
                type="button"
                className="button"
                disabled={busy || args === game.args}
                onClick={() => void run(() => api.updateGame({ gameId: game.id, args }))}
              >
                Сохранить
              </button>
            </div>
          </div>
        ) : (
          <p className="settings-row-hint">
            Аргументы запуска здесь недоступны: игру открывает {game.sourceLabel} по
            собственной ссылке, и передать ей что-то дополнительное нельзя.
          </p>
        )}
      </details>
    </>
  );
}
