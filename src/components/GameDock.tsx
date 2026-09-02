import { useCallback, useRef, useState } from "react";
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
  const dockRef = useRef<HTMLDivElement>(null);
  const [scales, setScales] = useState<number[]>([]);

  /* Только на стилях это не делается, и это ограничение, а не выбор: CSS умеет
     дотянуться до следующего соседа, но не до предыдущего, и увеличение
     раздувало бы иконки только справа от курсора. Поэтому считаем расстояние
     от курсора до центра каждой иконки сами. */
  const onMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const dock = dockRef.current;
    if (!dock) return;
    const x = e.clientX;
    const next = Array.from(dock.children).map((child) => {
      const r = (child as HTMLElement).getBoundingClientRect();
      const distance = Math.abs(x - (r.left + r.width / 2));
      const reach = 110;
      if (distance > reach) return 1;
      return 1 + 0.64 * (1 - distance / reach);
    });
    setScales(next);
  }, []);

  const onLeave = useCallback(() => setScales([]), []);

  if (games.length === 0) return null;

  return (
    <div className="dock" ref={dockRef} onMouseMove={onMove} onMouseLeave={onLeave}>
      {games.map((g, i) => {
        const zoom = { transform: `scale(${scales[i] ?? 1})` };
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
  );
}
