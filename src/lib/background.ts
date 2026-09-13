// Какой слой фона показывать на главном экране. Отдельно от компонента, чтобы
// порядок проверялся тестами без окна (спека этапа 5, §2.1 и §6.4).

import type { GameView } from "../types";

export type Layer =
  | { kind: "video"; path: string; still: string | null }
  | { kind: "image"; path: string }
  | { kind: "fill" };

export function layerFor(
  game: { videoPath: string | null; artPath: string | null },
  reducedMotion: boolean,
): Layer {
  // При «уменьшить движение» ролик не показывается вовсе: вместо него следующий
  // источник, а не остановленное видео.
  if (game.videoPath && !reducedMotion) {
    return { kind: "video", path: game.videoPath, still: game.artPath };
  }
  if (game.artPath) return { kind: "image", path: game.artPath };
  return { kind: "fill" };
}

/** Устойчивый ключ слоя: по нему перетекание понимает, что фон сменился. */
export function layerKey(layer: Layer): string {
  return layer.kind === "fill" ? "fill" : `${layer.kind}:${layer.path}`;
}

const SOURCE_TEXT: Record<GameView["artSource"], string> = {
  video: "своё видео",
  picture: "своя картинка",
  steam: "из Steam",
  epic: "из Epic Games",
  fill: "заливка",
};

/** Откуда у игры фон — подпись на вкладке «Вид» (спека §7.3, §6.5). */
export function sourceText(game: Pick<GameView, "artSource" | "videoMissing">): string {
  const text = SOURCE_TEXT[game.artSource];
  return game.videoMissing ? `${text}, файл видео не найден` : text;
}

/** Есть ли что убирать кнопкой «Убрать»: своё видео или картинка, в том числе
 *  видео, файл которого пропал. */
export function hasOwnBackground(game: Pick<GameView, "artSource" | "videoMissing">): boolean {
  return game.artSource === "video" || game.artSource === "picture" || game.videoMissing;
}
