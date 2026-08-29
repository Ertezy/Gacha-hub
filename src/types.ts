// Зеркала Rust-схем. На проводе camelCase (serde rename_all = "camelCase").

export type LaunchKind = "steam" | "epic" | "exe";

export interface GameView {
  id: string;
  title: string;
  contentId: string | null;
  /** Подпись «запустится через …». */
  sourceLabel: string;
  /** Файл или папка игры пропали с диска. */
  missing: boolean;
}

export interface PromoCode {
  gameId: string;
  code: string;
  rewards: string;
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

export interface LaunchResult {
  ok: boolean;
  msg: string;
}
