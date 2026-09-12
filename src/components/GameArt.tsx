import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { gradientFor } from "../lib/gradient";
import type { GameView } from "../types";

interface Props {
  game: GameView | null;
}

export default function GameArt({ game }: Props) {
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

  // Арта нет — оба слоя очищаются сразу. Иначе при переходе к следующей игре
  // с фоном нижним слоем стала бы картинка, которая давно ушла с экрана.
  useEffect(() => {
    if (!art) {
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
  }, [art]);

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
        <img key={`top-${top}`} className="art-photo" src={convertFileSrc(top)} alt="" />
      )}
    </div>
  );
}
