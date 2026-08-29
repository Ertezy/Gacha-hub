/**
 * Фон для игры, про которую мы не знаем арта.
 *
 * Градиент выводится из названия детерминированно: одно и то же название
 * всегда даёт один и тот же фон, иначе игра меняла бы вид при каждом запуске.
 */

/** FNV-1a — короткий, стабильный, без зависимостей. */
function hash(text: string): number {
  let h = 2166136261;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function gradientFor(title: string): string {
  const h = hash(title);
  const from = h % 360;
  const to = (from + 35 + ((h >>> 8) % 70)) % 360;
  return `linear-gradient(135deg, hsl(${from} 48% 24%), hsl(${to} 55% 48%))`;
}
