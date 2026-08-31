import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../lib/api";
import GameRow from "./GameRow";
import type { GameView } from "../../types";

export default function GamesSection() {
  const [games, setGames] = useState<GameView[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [error, setError] = useState("");
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
          {/* Правку добавляет задача 10. */}
        </GameRow>
      ))}
    </div>
  );
}
