import { useEffect, useState } from "react";
import CachedImage from "./CachedImage";
import { runningBanners } from "../../lib/panel";
import { progress, timeLeft } from "../../lib/time";
import { useLang, useT } from "../../i18n";
import type { Banner } from "../../types";

interface Props {
  /** Уже отфильтрованы по выбранной игре. */
  banners: Banner[];
  nowSec: number;
}

/**
 * Баннеры выбранной игры, каруселью.
 *
 * Стрелки в заголовке секции, а не поверх арта: поверх они закрывают ровно то,
 * ради чего арт и показывают.
 */
export default function Banners({ banners, nowSec }: Props) {
  const lang = useLang();
  const t = useT();
  const running = runningBanners(banners, nowSec);
  const [index, setIndex] = useState(0);

  // Сброс навешен на СОДЕРЖИМОЕ списка, а не на ссылку на массив.
  //
  // Родитель пересчитывает `banners` фильтром на каждой отрисовке, а часы в
  // панели тикают раз в полминуты — значит ссылка меняется постоянно, даже
  // когда список тот же. Навесив сброс на неё, мы возвращали бы карусель на
  // первый баннер каждые тридцать секунд, и стрелки выглядели бы сломанными.
  const runningKey = running.map((b) => `${b.gameId}:${b.startsAt}`).join("|");
  useEffect(() => setIndex(0), [runningKey]);

  if (running.length === 0) return null;

  const safeIndex = Math.min(index, running.length - 1);
  const b = running[safeIndex];
  const filled = progress(b.startsAt, b.endsAt, nowSec);
  const step = (d: number) =>
    setIndex((i) => (i + d + running.length) % running.length);

  return (
    <section className="panel-section">
      <div className="panel-label">
        <span>{t.panel.banners}</span>
        {running.length > 1 && (
          <span className="carousel-arrows">
            <button type="button" aria-label={t.panel.previousBanner} onClick={() => step(-1)}>
              ‹
            </button>
            <button type="button" aria-label={t.panel.nextBanner} onClick={() => step(1)}>
              ›
            </button>
          </span>
        )}
      </div>

      <div className="banner-card">
        <div className="banner-art-wrap">
          <CachedImage className="banner-art" url={b.image} fallbackText={b.title} />
          {b.featured.length > 0 && (
            <div className="banner-names">
              <div className="banner-who">{b.featured.join(" · ")}</div>
              {b.rarity !== null && <div className="banner-rarity">{b.rarity}★</div>}
            </div>
          )}
        </div>

        <div className="banner-foot">
          <div className="banner-bar">
            <div className="banner-bar-fill" style={{ width: `${Math.round(filled * 100)}%` }} />
          </div>
          <div className="banner-line">
            <span className="banner-title">{b.title}</span>
            <span className="banner-left">{timeLeft(b.endsAt, nowSec, lang)}</span>
          </div>
        </div>
      </div>

      {running.length > 1 && (
        <div className="carousel-dots">
          {running.map((r, i) => (
            <button
              key={`${r.gameId}:${r.startsAt}`}
              type="button"
              className={i === safeIndex ? "dot active" : "dot"}
              aria-label={t.panel.bannerDot(i + 1, running.length)}
              onClick={() => setIndex(i)}
            />
          ))}
        </div>
      )}
    </section>
  );
}
