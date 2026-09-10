import { convertFileSrc } from "@tauri-apps/api/core";
import { gradientFor } from "../lib/gradient";
import type { GameView } from "../types";

export default function GameArt({ game }: { game: GameView | null }) {
  if (!game) {
    return <div className="art" style={{ background: gradientFor("") }} />;
  }

  if (!game.artPath) {
    // Заливки выводятся из названия устойчиво, поэтому надпись поверх больше
    // не нужна: имя игры и так есть в правом верхнем углу экрана.
    return <div className="art" style={{ background: gradientFor(game.title) }} />;
  }

  return (
    <div className="art">
      <img
        key={game.artPath}
        className="art-photo"
        src={convertFileSrc(game.artPath)}
        alt=""
      />
    </div>
  );
}
