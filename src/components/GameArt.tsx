import { useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { gradientFor } from "../lib/gradient";
import { dominantHue } from "../lib/hue";
import type { GameView } from "../types";

interface Props {
  game: GameView | null;
  onHue: (hue: number | null) => void;
}

export default function GameArt({ game, onHue }: Props) {
  const art = game?.artPath ?? null;

  // Арта нет — сказать об этом сразу, иначе остался бы оттенок прошлой игры.
  useEffect(() => {
    if (!art) onHue(null);
  }, [art, onHue]);

  if (!game) {
    return <div className="art" style={{ background: gradientFor("") }} />;
  }

  if (!art) {
    // Заливки выводятся из названия устойчиво, поэтому надпись поверх больше
    // не нужна: имя игры и так есть в правом верхнем углу экрана.
    return <div className="art" style={{ background: gradientFor(game.title) }} />;
  }

  return (
    <div className="art">
      <img
        key={art}
        className="art-photo"
        crossOrigin="anonymous"
        src={convertFileSrc(art)}
        alt=""
        onLoad={(e) => onHue(dominantHue(e.currentTarget))}
        onError={() => onHue(null)}
      />
    </div>
  );
}
