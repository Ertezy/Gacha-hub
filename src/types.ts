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

export interface HubGame {
  id: string;
  title: string;
  icon: string | null;
  /** Шаблон с подстановкой {code}. Отсутствует у игр без веб-погашения. */
  redeemUrl: string | null;
}

export interface Code {
  gameId: string;
  code: string;
  rewards: string;
  /** unix-секунды; null — бессрочный. */
  expiresAt: number | null;
  region: string;
  source: string | null;
}

export interface Banner {
  gameId: string;
  title: string;
  featured: string[];
  rarity: number | null;
  /** Уже уменьшенный адрес; null у ХСР — вики не хранит арт. */
  image: string | null;
  startsAt: number;
  endsAt: number;
  url: string | null;
}

export interface Video {
  gameId: string;
  title: string;
  url: string;
  thumb: string | null;
  publishedAt: number;
  duration: number | null;
  premiere: boolean;
}

export interface HubData {
  version: number;
  updatedAt: number;
  games: HubGame[];
  codes: Code[];
  banners: Banner[];
  videos: Video[];
  _source?: string;
}

export interface LaunchResult {
  ok: boolean;
  msg: string;
}

export interface Behaviour {
  closeToTray: boolean;
  trayOnLaunch: boolean;
}

export interface FoundGame {
  title: string;
  contentId: string | null;
  sourceLabel: string;
  alreadyAdded: boolean;
}

export interface About {
  version: string;
  logPath: string;
}

/**
 * Полный конфиг пользователя, как его отдаёт `get_config`. Список игр здесь —
 * сырые записи конфига, а не `GameView`; экрану «Данные» из всего этого нужен
 * только `hubUrl`, поэтому остальная форма намеренно не расписана подробнее.
 */
export interface AppConfig {
  version: number;
  hubUrl: string | null;
  lastPlayed: string | null;
  seeded: boolean;
  behaviour: Behaviour;
  games: unknown[];
}
