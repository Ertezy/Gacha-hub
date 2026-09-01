import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { About, AppConfig, Behaviour, FoundGame, GameView, HubData } from "../types";

/**
 * Типизированные обёртки над мостом Tauri.
 * Вся системная работа — запуск, конфиг, данные хаба — происходит в Rust.
 */
export const api = {
  getGames: () => invoke<GameView[]>("get_games"),
  /** Весь конфиг целиком. Использовать только ради конкретного поля —
   *  показывать его содержимое на экране нельзя. */
  getConfig: () => invoke<AppConfig>("get_config"),
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

  addGame: (title: string, exe: string) => invoke<string>("add_game", { title, exe }),
  updateGame: (args: {
    gameId: string;
    title?: string;
    exe?: string;
    background?: string;
    // Пустая строка стирает привязку; отсутствие поля её не трогает.
    // `null` здесь не годится — при переходе через JSON он неотличим от
    // отсутствующего поля, и стереть привязку было бы невозможно.
    contentId?: string;
  }) => invoke<void>("update_game", args),
  removeGame: (gameId: string) => invoke<void>("remove_game", { gameId }),
  reorderGames: (ids: string[]) => invoke<void>("reorder_games", { ids }),
  relocateGame: (gameId: string) => invoke<void>("relocate_game", { gameId }),
  scanInstalled: () => invoke<FoundGame[]>("scan_installed"),
  addGameFromScan: (title: string) => invoke<string>("add_game_from_scan", { title }),

  /** Первый запуск ещё не состоялся — вместо главного экрана нужен экран с галочками. */
  needsFirstRun: () => invoke<boolean>("needs_first_run"),
  /** Пометить, что первый запуск состоялся — не важно, что человек на нём выбрал. */
  markSeeded: () => invoke<void>("mark_seeded"),

  getBehaviour: () => invoke<Behaviour>("get_behaviour"),
  setBehaviour: (b: Behaviour) =>
    invoke<void>("set_behaviour", { closeToTray: b.closeToTray, trayOnLaunch: b.trayOnLaunch }),

  setHubUrl: (url: string | null) => invoke<void>("set_hub_url", { url }),
  imageCacheSize: () => invoke<number>("image_cache_size"),
  clearImageCache: () => invoke<number>("clear_image_cache"),
  getAbout: () => invoke<About>("get_about"),
  openLogFolder: () => invoke<void>("open_log_folder"),

  /** Выбрать исполняемый файл игры. `null` — человек отменил. */
  pickExe: async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Программа", extensions: ["exe"] }],
    });
    return typeof picked === "string" ? picked : null;
  },

  /** Выбрать картинку фона. `null` — человек отменил. */
  pickImage: async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Картинка", extensions: ["png", "jpg", "jpeg", "webp"] }],
    });
    return typeof picked === "string" ? picked : null;
  },
};
