// Какой слой фона показывать на главном экране. Отдельно от компонента, чтобы
// порядок проверялся тестами без окна (спека этапа 5, §2.1 и §6.4).

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
