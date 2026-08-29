import { gradientFor } from "../lib/gradient";
import type { GameView } from "../types";

export default function GameArt({ game }: { game: GameView | null }) {
  if (!game) {
    return <div className="art" style={{ background: gradientFor("") }} />;
  }
  return (
    <div className="art" style={{ background: gradientFor(game.title) }}>
      <div className="art-caption">{game.title}</div>
    </div>
  );
}
