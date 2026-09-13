import { useEffect, useRef, useState, type Ref } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { gradientFor } from "../lib/gradient";
import { layerFor, layerKey, type Layer } from "../lib/background";
import type { GameView } from "../types";

interface Props {
  game: GameView | null;
}

const REDUCED_MOTION = "(prefers-reduced-motion: reduce)";

/** Системная настройка «уменьшить движение», с подпиской на её смену. */
function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => window.matchMedia(REDUCED_MOTION).matches);
  useEffect(() => {
    const query = window.matchMedia(REDUCED_MOTION);
    const onChange = () => setReduced(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return reduced;
}

/**
 * Видно ли окно. Уход в трей сообщает Rust (`tray::hide_main_window`), а
 * сворачивание страница видит сама через `visibilitychange` (спека §6.3).
 */
function useWindowVisible(): boolean {
  const [shown, setShown] = useState(true);
  const [pageVisible, setPageVisible] = useState(() => document.visibilityState === "visible");
  useEffect(() => {
    const unlisten = listen<boolean>("window-visibility", (e) => setShown(e.payload));
    const onVisibility = () => setPageVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      void unlisten.then((stop) => stop());
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);
  return shown && pageVisible;
}

export default function GameArt({ game }: Props) {
  const reduced = useReducedMotion();
  const visible = useWindowVisible();
  const layer: Layer = game ? layerFor(game, reduced) : { kind: "fill" };
  const key = layerKey(layer);

  // Два слоя вместо одной картинки по ключу: раньше при смене игры React в
  // одном проходе убирал старую картинку и вставлял новую, и между ними на
  // мгновение проглядывал фон окна вместо настоящего перетекания (спека этапа
  // 4, §2.5). Нижний слой держит прежний фон неподвижно и непрозрачно, верхний
  // проявляется поверх него. Для видео так же (спека этапа 5, §6.6).
  const [top, setTop] = useState<Layer>(layer);
  const [bottom, setBottom] = useState<Layer | null>(null);
  const topKey = useRef(key);
  const videoRef = useRef<HTMLVideoElement>(null);

  // Зависимость — только ключ: `top` и `layer` берутся из того отрисовывания, в
  // котором ключ сменился, то есть это ещё прежний верхний слой и уже новый.
  useEffect(() => {
    if (topKey.current === key) return;
    topKey.current = key;
    // Фона нет — оба слоя очищаются сразу. Иначе при переходе к следующей игре
    // с фоном нижним слоем стал бы фон, который давно ушёл с экрана.
    setBottom(layer.kind === "fill" || top.kind === "fill" ? null : top);
    setTop(layer);
  }, [key]);

  // Ролик не крутится впустую, пока окно в трее или свёрнуто (спека §6.3).
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    if (visible) void video.play().catch(() => {});
    else video.pause();
  }, [visible, top]);

  if (!game || top.kind === "fill") {
    // Заливки выводятся из названия устойчиво, поэтому надпись поверх не
    // нужна: имя игры и так есть в правом верхнем углу экрана.
    return <div className="art" style={{ background: gradientFor(game?.title ?? "") }} />;
  }

  return (
    <div className="art">
      {bottom && <LayerView key={`bottom-${layerKey(bottom)}`} layer={bottom} under />}
      <LayerView key={`top-${layerKey(top)}`} layer={top} videoRef={videoRef} />
    </div>
  );
}

function LayerView({
  layer,
  under = false,
  videoRef,
}: {
  layer: Layer;
  under?: boolean;
  videoRef?: Ref<HTMLVideoElement>;
}) {
  if (layer.kind === "fill") return null;
  if (layer.kind === "video") {
    return (
      <video
        ref={videoRef}
        className={under ? "art-video art-layer-under" : "art-video"}
        src={convertFileSrc(layer.path)}
        poster={layer.still ? convertFileSrc(layer.still) : undefined}
        muted
        loop
        playsInline
        autoPlay={!under}
      />
    );
  }
  return (
    <img
      className={under ? "art-photo art-photo-under" : "art-photo"}
      src={convertFileSrc(layer.path)}
      alt=""
    />
  );
}
