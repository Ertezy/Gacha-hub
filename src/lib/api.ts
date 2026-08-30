import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { GameView, HubData } from "../types";

/**
 * Типизированные обёртки над мостом Tauri.
 * Вся системная работа — запуск, конфиг, данные хаба — происходит в Rust.
 */
export const api = {
  getGames: () => invoke<GameView[]>("get_games"),
  getHub: () => invoke<HubData>("get_hub"),
  getLastPlayed: () => invoke<string | null>("get_last_played"),
  launchGame: (gameId: string) => invoke<string>("launch_game", { gameId }),

  /** Путь к картинке в локальном кеше; Rust качает её, если надо. */
  cacheImage: (url: string) => invoke<string>("cache_image", { url }),

  /** Положить текст в буфер обмена. */
  copyText: (text: string) => writeText(text),

  /**
   * Открыть внешнюю ссылку в браузере пользователя.
   * Только https: данные хаба приходят из сети и не должны иметь возможности
   * дёрнуть file:// или произвольный обработчик протокола.
   */
  openSafeUrl: async (url: string) => {
    if (!url.startsWith("https://")) {
      throw new Error(`отказываюсь открывать не-https ссылку: ${url}`);
    }
    await openUrl(url);
  },
};
