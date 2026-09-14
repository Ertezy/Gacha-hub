import type { GameView } from "../types";

/**
 * Подпись над кнопкой «Играть»: откуда запустится игра и не пропал ли её файл.
 * Человек не помнит, где что лежит, и лаунчер говорит это за него.
 */
export function launchNote(game: Pick<GameView, "sourceLabel" | "missing">): string {
  const note = `Запустится через ${game.sourceLabel}`;
  return game.missing ? `${note} · файл не найден` : note;
}
