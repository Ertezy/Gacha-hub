/**
 * Преобладающий оттенок картинки.
 *
 * Берётся ТОЛЬКО оттенок: насыщенность и светлота остаются те, что заданы в
 * палитре. Это и делает подбор безопасным. Взять из картинки цвет целиком —
 * обычный способ и обычный источник уродства: попадается грязно-серый или
 * ядовитый, и интерфейс разваливается. Если меняется один оттенок, испортить
 * нельзя — цвет можно только сдвинуть по кругу.
 */

/** Оттенок в градусах, насыщенность и светлота от нуля до единицы. */
export function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rr = r / 255;
  const gg = g / 255;
  const bb = b / 255;
  const max = Math.max(rr, gg, bb);
  const min = Math.min(rr, gg, bb);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0, l];

  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h: number;
  if (max === rr) h = ((gg - bb) / d + (gg < bb ? 6 : 0)) * 60;
  else if (max === gg) h = ((bb - rr) / d + 2) * 60;
  else h = ((rr - gg) / d + 4) * 60;
  return [Math.round(h) % 360, s, l];
}

/** Пиксели слишком блёклые, чтобы о чём-то говорить. */
const MIN_SATURATION = 0.25;
/** Края по светлоте: чёрная рамка и белое небо не должны решать. */
const MIN_LIGHTNESS = 0.15;
const MAX_LIGHTNESS = 0.85;
/** Ширина корзины оттенков в градусах. */
const BIN = 10;

/**
 * Оттенок по массиву пикселей вида RGBA.
 *
 * `null` — сказать нечего: картинка почти серая или данных нет. Отказ тихий и
 * безвредный: вызывающий оставляет оттенок из палитры.
 */
export function pickHue(data: Uint8ClampedArray): number | null {
  const bins = new Array(360 / BIN).fill(0);
  let kept = 0;

  for (let i = 0; i + 3 < data.length; i += 4) {
    if (data[i + 3] < 128) continue;
    const [h, s, l] = rgbToHsl(data[i], data[i + 1], data[i + 2]);
    if (s < MIN_SATURATION) continue;
    if (l < MIN_LIGHTNESS || l > MAX_LIGHTNESS) continue;
    bins[Math.floor(h / BIN) % bins.length] += 1;
    kept += 1;
  }

  if (kept === 0) return null;

  let best = 0;
  for (let i = 1; i < bins.length; i++) {
    if (bins[i] > bins[best]) best = i;
  }
  return best * BIN + BIN / 2;
}

/**
 * Оттенок уже загруженной картинки.
 *
 * Уменьшение до 64 пикселей по большей стороне: преобладающий тон от этого не
 * меняется, а работы становится в сотни раз меньше.
 */
export function dominantHue(image: HTMLImageElement): number | null {
  const side = 64;
  const w = image.naturalWidth;
  const h = image.naturalHeight;
  if (w === 0 || h === 0) return null;

  const scale = Math.min(side / w, side / h, 1);
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(w * scale));
  canvas.height = Math.max(1, Math.round(h * scale));

  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(image, 0, 0, canvas.width, canvas.height);

  try {
    return pickHue(ctx.getImageData(0, 0, canvas.width, canvas.height).data);
  } catch {
    // Холст «испорчен» — прочитать пиксели нельзя. Оттенок остаётся из палитры.
    return null;
  }
}
