import type { GameView } from "../types";

export default function GameHeader({ game }: { game: GameView }) {
  return (
    <div className="game-header">
      <b>{game.title}</b>
      <span>
        запустится через {game.sourceLabel}
        {game.missing && " · файл не найден"}
      </span>
    </div>
  );
}
