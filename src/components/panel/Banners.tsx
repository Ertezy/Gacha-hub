import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import CachedImage from "./CachedImage";
import { shownBanners } from "../../lib/panel";
import { progress, timeLeft } from "../../lib/time";
import { useMotionOn } from "../../lib/motion";
import { useLang, useT, type Lang } from "../../i18n";
import type { Banner } from "../../types";

/** Как часто карусель листает сама (спека этапа 7 §3.3). */
export const AUTO_ADVANCE_MS = 7000;

interface Props {
  /** Уже отфильтрованы по выбранной игре. */
  banners: Banner[];
  nowSec: number;
}

/**
 * Баннеры выбранной игры каруселью: сначала идущие, потом будущие.
 *
 * Стрелки — поверх картинки, по бокам и полупрозрачные: так решил владелец на
 * этапе 7. Раньше они стояли в заголовке секции, чтобы не закрывать арт.
 */
export default function Banners({ banners, nowSec }: Props) {
  const lang = useLang();
  const t = useT();
  const motion = useMotionOn();
  const shown = shownBanners(banners, nowSec);
  const [index, setIndex] = useState(0);
  // Ручное листание перезапускает отсчёт: иначе автолистание перебило бы клик.
  const [restart, setRestart] = useState(0);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [windowShown, setWindowShown] = useState(true);

  // Сброс навешен на СОДЕРЖИМОЕ списка, а не на ссылку на массив.
  //
  // Родитель пересчитывает `banners` фильтром на каждой отрисовке, а часы в
  // панели тикают раз в полминуты — значит ссылка меняется постоянно, даже
  // когда список тот же. Навесив сброс на неё, мы возвращали бы карусель на
  // первый баннер каждые тридцать секунд.
  const shownKey = shown.map((b) => `${b.gameId}:${b.startsAt}`).join("|");
  useEffect(() => setIndex(0), [shownKey]);

  // Окно спрятано в трей — карусель стоит (спека этапа 7 §3.3).
  useEffect(() => {
    const unlisten = listen<boolean>("window-visibility", (e) => setWindowShown(e.payload));
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const count = shown.length;
  const paused = hovered || focused || !windowShown || !motion;

  useEffect(() => {
    if (paused || count < 2) return;
    const id = window.setInterval(() => setIndex((i) => (i + 1) % count), AUTO_ADVANCE_MS);
    return () => window.clearInterval(id);
  }, [paused, count, restart]);

  if (count === 0) return null;

  const safeIndex = Math.min(index, count - 1);
  const go = (i: number) => {
    setIndex(((i % count) + count) % count);
    setRestart((r) => r + 1);
  };

  return (
    <section className="panel-section">
      <div className="panel-label">
        <span>{t.panel.banners}</span>
      </div>

      <div
        className="carousel"
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onFocus={() => setFocused(true)}
        onBlur={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setFocused(false);
        }}
      >
        <div className="carousel-viewport">
          <div
            className={motion ? "carousel-track" : "carousel-track instant"}
            style={{ transform: `translateX(-${safeIndex * 100}%)` }}
          >
            {shown.map((b, i) => (
              <BannerCard key={`${b.gameId}:${b.startsAt}`} banner={b} nowSec={nowSec} lang={lang} hidden={i !== safeIndex} />
            ))}
          </div>

          {count > 1 && (
            <div className="carousel-arrows">
              <button type="button" className="carousel-arrow" aria-label={t.panel.previousBanner} onClick={() => go(safeIndex - 1)}>
                ‹
              </button>
              <button type="button" className="carousel-arrow" aria-label={t.panel.nextBanner} onClick={() => go(safeIndex + 1)}>
                ›
              </button>
            </div>
          )}
        </div>

        {count > 1 && (
          <div className="carousel-dots">
            {shown.map((b, i) => (
              <button
                key={`${b.gameId}:${b.startsAt}`}
                type="button"
                className={["dot", i === safeIndex ? "active" : "", b.startsAt > nowSec ? "upcoming" : ""].filter(Boolean).join(" ")}
                aria-label={t.panel.bannerDot(i + 1, count)}
                onClick={() => go(i)}
              />
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

interface CardProps {
  banner: Banner;
  nowSec: number;
  lang: Lang;
  /** Карточка за краем ленты — для экранного диктора её нет. */
  hidden: boolean;
}

/** Одна карточка: идущий баннер — полоса и остаток, будущий — «Скоро · через …». */
function BannerCard({ banner: b, nowSec, lang, hidden }: CardProps) {
  const t = useT();
  const upcoming = b.startsAt > nowSec;
  const filled = progress(b.startsAt, b.endsAt, nowSec);

  return (
    <div className="carousel-slide" aria-hidden={hidden}>
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
          {upcoming ? (
            <div className="banner-soon">{t.panel.soonIn(timeLeft(b.startsAt, nowSec, lang))}</div>
          ) : (
            <div className="banner-bar">
              <div className="banner-bar-fill" style={{ width: `${Math.round(filled * 100)}%` }} />
            </div>
          )}
          <div className="banner-line">
            <span className="banner-title">{b.title}</span>
            {!upcoming && <span className="banner-left">{timeLeft(b.endsAt, nowSec, lang)}</span>}
          </div>
        </div>
      </div>
    </div>
  );
}
