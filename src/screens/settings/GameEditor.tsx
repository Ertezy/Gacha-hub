import { useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "../../lib/api";
import type { GameView, HubGame } from "../../types";

interface Props {
  game: GameView;
  /** Игры из каталога — для привязки контента. */
  hubGames: HubGame[];
  onChanged: () => void;
}

export default function GameEditor({ game, hubGames, onChanged }: Props) {
  const [title, setTitle] = useState(game.title);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      {error && <div className="settings-error">{error}</div>}

      <label className="field">
        <span>Название</span>
        <input value={title} onChange={(e) => setTitle(e.target.value)} />
        <button
          type="button"
          className="accent"
          disabled={busy || title === game.title}
          onClick={() => void run(() => api.updateGame({ gameId: game.id, title }))}
        >
          Сохранить
        </button>
      </label>

      <label className="field">
        <span>Файл игры</span>
        <button
          type="button"
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
      </label>

      <label className="field">
        <span>Своя картинка фона</span>
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            void run(async () => {
              const background = await api.pickImage();
              if (background) await api.updateGame({ gameId: game.id, background });
            })
          }
        >
          Выбрать…
        </button>
      </label>

      <label className="field">
        <span>Показывать контент как</span>
        <select
          value={game.contentId ?? ""}
          disabled={busy}
          onChange={(e) =>
            void run(() =>
              api.updateGame({
                gameId: game.id,
                contentId: e.target.value,
              }),
            )
          }
        >
          <option value="">нет — коды и баннеры не показывать</option>
          {hubGames.map((h) => (
            <option key={h.id} value={h.id}>
              {h.title}
            </option>
          ))}
        </select>
      </label>

      <div className="field-row">
        {game.missing && (
          <button
            type="button"
            disabled={busy}
            onClick={() => void run(() => api.relocateGame(game.id))}
          >
            Найти заново
          </button>
        )}
        <button
          type="button"
          className="danger"
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
    </>
  );
}
