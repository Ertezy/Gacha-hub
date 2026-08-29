import { invoke } from "@tauri-apps/api/core";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppConfig, DetectedInstall, GameView, HubData } from "../types";

/**
 * Typed wrappers over the Tauri invoke bridge.
 * All system work (launching, config IO, hub data) happens in Rust.
 */
export const api = {
  getGames: () => invoke<GameView[]>("get_games"),
  getConfig: () => invoke<AppConfig>("get_config"),
  getConfigDir: () => invoke<string>("get_config_dir"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),
  launchGame: (gameId: string) => invoke<void>("launch_game", { gameId }),
  getHub: () => invoke<HubData>("get_hub"),

  /** Scan standard install locations (Epic/Steam/manual) for game installs. */
  detectInstalls: () => invoke<DetectedInstall[]>("detect_installs"),
  rescanInstalls: () => invoke<DetectedInstall[]>("rescan_installs"),

  /**
   * Open an external URL in the default browser.
   * https-only by design: hub data may later come from a remote JSON and
   * must never be able to trigger file:// or arbitrary protocol handlers.
   */
  openSafeUrl: async (url: string) => {
    if (!url.startsWith("https://")) {
      throw new Error(`refusing to open non-https url: ${url}`);
    }
    await openUrl(url);
  },

  /** OS file picker for an .exe; returns the chosen path or null. */
  pickExe: async (): Promise<string | null> => {
    const res = await pickFile({
      multiple: false,
      directory: false,
      filters: [{ name: "Executable", extensions: ["exe"] }],
    });
    return typeof res === "string" ? res : null;
  },
};
