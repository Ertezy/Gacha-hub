import type { GameView } from "../types";

interface Props {
  games: GameView[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export default function GameShelf({ games, selectedId, onSelect }: Props) {
  if (games.length === 0) return null;

  return (
    <div className="shelf-zone">
      <div className="shelf-hint">игры</div>
      <div className="shelf">
        {games.map((g) => (
          <button
            key={g.id}
            className={`chip${g.id === selectedId ? " active" : ""}${g.missing ? " missing" : ""}`}
            onClick={() => onSelect(g.id)}
            title={g.missing ? "файл игры не найден" : g.title}
          >
            {g.title}
          </button>
        ))}
      </div>
    </div>
  );
}
