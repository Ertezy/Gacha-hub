import { useEffect, useRef, useState } from "react";
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

  // Два слоя вместо одной картинки по ключу: раньше при смене игры React в
  // одном проходе убирал старую картинку и вставлял новую, и между ними на
  // мгновение проглядывал фон окна вместо настоящего перетекания (спека
  // §2.5). Нижний слой держит прежнюю картинку неподвижно и непрозрачно,
  // верхний проявляется поверх неё — фону окна попасть в кадр негде.
  const [top, setTop] = useState<string | null>(art);
  const [bottom, setBottom] = useState<string | null>(null);
  // Путь верхнего слоя вне состояния — чтобы решить, что именно уехало вниз,
  // до того как состояние переприменится следующим рендером.
  const topRef = useRef(art);

  // Арта нет — сказать об этом сразу, иначе остался бы оттенок прошлой игры.
  useEffect(() => {
    if (!art) {
      onHue(null);
      setTop(null);
      setBottom(null);
      topRef.current = null;
      return;
    }
    if (topRef.current !== art) {
      setBottom(topRef.current);
      setTop(art);
      topRef.current = art;
    }
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
      {bottom && (
        <img
          key={`bottom-${bottom}`}
          className="art-photo art-photo-under"
          src={convertFileSrc(bottom)}
          alt=""
        />
      )}
      {top && (
        <img
          key={`top-${top}`}
          className="art-photo"
          crossOrigin="anonymous"
          src={convertFileSrc(top)}
          alt=""
          onLoad={(e) => onHue(dominantHue(e.currentTarget))}
          onError={() => onHue(null)}
        />
      )}
    </div>
  );
}
