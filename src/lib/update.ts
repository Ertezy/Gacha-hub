import type { AppRelease, HubData } from "../types";

/**
 * «1.2.3» → [1, 2, 3]; любой другой вид — null. То же правило — в приложении на
 * Rust (`src-tauri/src/hub/schema.rs`, `lenient_app`) и у сборщика.
 */
function parts(version: string): [number, number, number] | null {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** Новее ли a, чем b, по трём числам. Номер, который не читается, новее не бывает. */
export function isNewer(a: string, b: string): boolean {
  const x = parts(a);
  const y = parts(b);
  if (x === null || y === null) return false;
  for (let i = 0; i < 3; i++) {
    if (x[i] !== y[i]) return x[i]! > y[i]!;
  }
  return false;
}

/**
 * Вышедшая версия, о которой стоит сказать (спека 2026-10-01 §2.2): из файла
 * хаба и строго новее работающей. Иначе null — строки «Вышла версия» нет.
 */
export function availableUpdate(hub: HubData | null, current: string | null): AppRelease | null {
  const app = hub?.app;
  if (!app || current === null) return null;
  return isNewer(app.version, current) ? app : null;
}
