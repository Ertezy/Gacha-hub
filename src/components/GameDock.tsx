import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { motionOn, subscribeMotion } from "../lib/motion";
import type { GameView } from "../types";

interface Props {
  games: GameView[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

/** Цвет заглушки выводится из названия устойчиво: одна и та же игра всегда
 *  получает один и тот же цвет, и соседние игры не сливаются. */
/** Докуда достаёт увеличение, в пикселях от курсора до центра иконки. */
const REACH = 110;

function tintOf(title: string): string {
  let hash = 0;
  for (const ch of title) hash = (hash * 31 + ch.charCodeAt(0)) % 360;
  return `hsl(${hash} 42% 34%)`;
}

export default function GameDock({ games, selectedId, onSelect }: Props) {
  const dockRef = useRef<HTMLDivElement>(null);
  const [near, setNear] = useState<number[]>([]);
  const restingCentres = useRef<number[] | null>(null);

  const onLeave = useCallback(() => {
    restingCentres.current = null;
    setNear([]);
  }, []);

  // Тумблер «Анимация» могли выключить, пока курсор стоит над доком без
  // движения: `onMove` в таком случае больше не позовут, и увеличенные иконки
  // застряли бы такими до следующего шевеления мыши. Подписка возвращает их в
  // покой сразу же, а не ждёт следующего движения (спека этапа 5, §12).
  useEffect(() => {
    return subscribeMotion((on) => {
      if (!on) onLeave();
    });
  }, [onLeave]);

  /* Только на стилях это не делается, и это ограничение, а не выбор: CSS умеет
     дотянуться до следующего соседа, но не до предыдущего, и увеличение
     раздувало бы иконки только справа от курсора. Поэтому считаем расстояние
     от курсора до центра каждой иконки сами.

     Центры меряются один раз, пока ряд в покое, и дальше служат опорой.
     **Мерить их на каждом движении нельзя.** Увеличенные иконки раздвигают
     ряд, ряд стоит по центру экрана и от этого едет вбок — значит на
     следующем движении центры окажутся уже другими, от них другие
     расстояния, от тех другие размеры. Вышла бы погоня за собственным
     хвостом. От опорных центров близость — простая функция от положения
     курсора, и дрожать ей не с чего. */
  const onMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!motionOn()) return;
    const dock = dockRef.current;
    if (!dock) return;
    if (restingCentres.current === null) {
      restingCentres.current = Array.from(dock.children).map((child) => {
        const r = (child as HTMLElement).getBoundingClientRect();
        return r.left + r.width / 2;
      });
    }
    const x = e.clientX;
    setNear(restingCentres.current.map((c) => Math.max(0, 1 - Math.abs(x - c) / REACH)));
  }, []);

  if (games.length === 0) return null;

  return (
    <div className="dock-wrap">
      <div className="dock-bg" aria-hidden="true" />
      <div className="dock" ref={dockRef} onMouseMove={onMove} onMouseLeave={onLeave}>
        {games.map((g, i) => {
          // Насколько иконка вырастет, решают стили: сюда уходит только
          // близость к курсору, размер и кратность увеличения живут в
          // таблице стилей рядом с теми правилами, которым они тоже нужны.
          const zoom = { "--near": near[i] ?? 0 } as React.CSSProperties;
          return (
            <button
              key={g.id}
              className={`dock-item${g.missing ? " missing" : ""}`}
              onClick={() => onSelect(g.id)}
              title={g.missing ? `${g.title} — файл игры не найден` : g.title}
            >
              <span className="dock-name">{g.title}</span>
              {g.iconPath ? (
                <img className="dock-icon" style={zoom} src={convertFileSrc(g.iconPath)} alt="" />
              ) : (
                <span
                  className="dock-icon dock-letter"
                  style={{ ...zoom, background: tintOf(g.title) }}
                >
                  {g.title.slice(0, 1).toUpperCase()}
                </span>
              )}
              {g.id === selectedId && <span className="dock-dot" />}
            </button>
          );
        })}
      </div>
    </div>
  );
}
