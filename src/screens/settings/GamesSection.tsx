import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../lib/api";
import GameRow from "./GameRow";
import GameEditor from "./GameEditor";
import FirstRunView from "./FirstRunView";
import type { GameView, HubGame } from "../../types";

export default function GamesSection() {
  const [games, setGames] = useState<GameView[]>([]);
  const [hubGames, setHubGames] = useState<HubGame[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [error, setError] = useState("");
  // Тот же экран с галочками, что и при первом запуске: список установленного
  // мог обновиться, а человеку — захотеться пересмотреть, что добавить.
  const [scanning, setScanning] = useState(false);
  const dragFrom = useRef<number | null>(null);
  const dragTo = useRef<number | null>(null);

  const reload = useCallback(async () => {
    try {
      setGames(await api.getGames());
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void reload();
    // Каталог для привязки контента — грузим один раз, он не меняется
    // действиями этого экрана.
    void (async () => {
      try {
        setHubGames((await api.getHub()).games);
      } catch {
        // Хаб мог быть недоступен — привязка контента просто не предложит
        // вариантов, это не мешает править остальные поля игры.
      }
    })();
  }, [reload]);

  const commitOrder = useCallback(async () => {
    const from = dragFrom.current;
    const to = dragTo.current;
    dragFrom.current = null;
    dragTo.current = null;
    if (from === null || to === null || from === to) return;

    const next = [...games];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    setGames(next);
    try {
      await api.reorderGames(next.map((g) => g.id));
    } catch (e) {
      setError(String(e));
      // Порядок на экране разошёлся с конфигом — перечитываем правду.
      void reload();
    }
  }, [games, reload]);

  if (scanning) {
    return (
      <FirstRunView
        onDone={() => {
          setScanning(false);
          void reload();
        }}
      />
    );
  }

  return (
    <div>
      <h2 className="settings-section-title">Игры</h2>
      <p className="settings-hint">
        Порядок в списке — это порядок на полке внизу главного экрана.
        Перетащите строку, чтобы поменять.
      </p>

      {error && <div className="settings-error">{error}</div>}

      {games.map((g, i) => (
        <GameRow
          key={g.id}
          game={g}
          expanded={expanded === g.id}
          onToggle={() => setExpanded(expanded === g.id ? null : g.id)}
          onDragStart={() => {
            dragFrom.current = i;
          }}
          onDragOver={() => {
            dragTo.current = i;
          }}
          onDrop={() => void commitOrder()}
        >
          <GameEditor game={g} hubGames={hubGames} onChanged={() => void reload()} />
        </GameRow>
      ))}

      <button
        type="button"
        className="settings-add accent"
        onClick={() =>
          void (async () => {
            const exe = await api.pickExe();
            if (!exe) return;
            const guessed = exe.split(/[\\/]/).pop()?.replace(/\.exe$/i, "") ?? "Игра";
            try {
              await api.addGame(guessed, exe);
              await reload();
            } catch (e) {
              setError(String(e));
            }
          })()
        }
      >
        + Добавить игру вручную
      </button>

      <button type="button" className="settings-add" onClick={() => setScanning(true)}>
        Найти установленные игры
      </button>
    </div>
  );
}
