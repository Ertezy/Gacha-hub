import type { Dictionary } from "../i18n";
import type { GameView } from "../types";

/** Подпись над кнопкой запуска: откуда запустится игра, и нет ли её файла. */
export function launchNote(t: Dictionary, game: Pick<GameView, "sourceKind" | "missing">): string {
  const note = t.main.launch.note[game.sourceKind];
  return game.missing ? `${note} · ${t.main.launch.fileMissing}` : note;
}
