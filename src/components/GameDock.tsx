import { convertFileSrc } from "@tauri-apps/api/core";
import type { GameView } from "../types";

interface Props {
  games: GameView[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

/** Цвет заглушки выводится из названия устойчиво: одна и та же игра всегда
 *  получает один и тот же цвет, и соседние игры не сливаются. */
function tintOf(title: string): string {
  let hash = 0;
  for (const ch of title) hash = (hash * 31 + ch.charCodeAt(0)) % 360;
  return `hsl(${hash} 42% 34%)`;
}

export default function GameDock({ games, selectedId, onSelect }: Props) {
  if (games.length === 0) return null;

  return (
    <div className="dock">
      {games.map((g) => (
        <button
          key={g.id}
          className={`dock-item${g.missing ? " missing" : ""}`}
          onClick={() => onSelect(g.id)}
          title={g.missing ? `${g.title} — файл игры не найден` : g.title}
        >
          <span className="dock-name">{g.title}</span>
          {g.iconPath ? (
            <img className="dock-icon" src={convertFileSrc(g.iconPath)} alt="" />
          ) : (
            <span className="dock-icon dock-letter" style={{ background: tintOf(g.title) }}>
              {g.title.slice(0, 1).toUpperCase()}
            </span>
          )}
          {g.id === selectedId && <span className="dock-dot" />}
        </button>
      ))}
    </div>
  );
}
