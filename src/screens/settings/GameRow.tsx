import type { GameView, SourceKind } from "../../types";

// Тот же текст, что раньше лежал в GameView.sourceLabel — код перевёл на
// sourceKind (Task 5, спека §6.3), а собственно перевод этой строки ждёт
// своей задачи (Task 6).
const SOURCE_LABEL: Record<SourceKind, string> = { steam: "Steam", epic: "Epic Games", exe: "напрямую" };

interface Props {
  game: GameView;
  /** Строка развёрнута для правки. */
  expanded: boolean;
  onToggle: () => void;
  onDragStart: () => void;
  onDragOver: () => void;
  onDrop: () => void;
  children?: React.ReactNode;
}

export default function GameRow({
  game,
  expanded,
  onToggle,
  onDragStart,
  onDragOver,
  onDrop,
  children,
}: Props) {
  return (
    <div
      className={game.missing ? "game-row missing" : "game-row"}
      draggable
      onDragStart={onDragStart}
      onDragOver={(e) => {
        e.preventDefault();
        onDragOver();
      }}
      onDrop={onDrop}
    >
      <div
        className="game-row-head"
        role="button"
        tabIndex={0}
        onClick={onToggle}
        onKeyDown={(e) => {
          if (e.target !== e.currentTarget) return;
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onToggle();
          }
        }}
      >
        <span className="game-row-grip" aria-hidden="true">
          ⋮⋮
        </span>
        <span className="game-row-title">{game.title}</span>
        <span className="game-row-source">{SOURCE_LABEL[game.sourceKind]}</span>
        {game.missing && (
          <span className="game-row-warn" title="Файл игры не найден">
            файл пропал
          </span>
        )}
      </div>
      {expanded && <div className="game-row-body">{children}</div>}
    </div>
  );
}
