// TS mirrors of the Rust schemas (src-tauri/src). Field names are camelCase
// on the wire (serde rename_all = "camelCase").

export type LaunchMode = "steam" | "epic" | "exe";

export interface GameLaunchConfig {
  launchMode: LaunchMode;
  exePath?: string | null;
  args: string;
  epicProductId?: string | null;
}

export interface GameView {
  id: string;
  name: string;
  publisher: string;
  steamAppid: number | null;
  epicSupported: boolean;
  hasOfficialLauncher: boolean;
  storeUrl: string;
  config: GameLaunchConfig;
}

export interface AppConfig {
  version: number;
  games: Record<string, GameLaunchConfig>;
  hubUrl?: string | null;
}

export interface PromoCode {
  gameId: string;
  code: string;
  rewards: string;
  /** ISO date YYYY-MM-DD: valid through this day inclusive. */
  expired: string;
  source?: string;
}

export interface NewsItem {
  gameId: string;
  title: string;
  date: string;
  summary: string;
  url: string;
}

export interface Guide {
  gameId: string;
  title: string;
  character?: string;
  date?: string;
  url: string;
}

export interface HubData {
  version?: number;
  updatedAt?: string;
  promoCodes: PromoCode[];
  news: NewsItem[];
  guides: Guide[];
  _note?: string;
  _source?: string;
}

export interface DetectedInstall {
  gameId: string;
  /** Path to the launcher .exe. */
  path: string;
  /** "epic" | "steam" | "official" */
  source: string;
}

export type Page = "games" | "news" | "promo" | "guides" | "settings";

export interface LaunchResult {
  ok: boolean;
  msg: string;
}
