import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "../../lib/api";
import { gradientFor } from "../../lib/gradient";

interface Props {
  url: string | null;
  /** Показывается на градиенте, когда картинки нет. */
  fallbackText: string;
  className?: string;
}

/**
 * Картинка из локального кеша.
 *
 * Отсутствие картинки — обычное дело, а не поломка: у баннеров ХСР арта нет
 * вовсе, а любая загрузка может не удаться. В этом случае рисуется градиент
 * из названия — узнаваемый и стабильный, а не серая дырка.
 */
export default function CachedImage({ url, fallbackText, className }: Props) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let alive = true;
    setSrc(null);
    setFailed(false);
    if (!url) {
      setFailed(true);
      return;
    }
    api
      .cacheImage(url)
      .then((path) => {
        if (alive) setSrc(convertFileSrc(path));
      })
      .catch(() => {
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
    };
  }, [url]);

  if (failed || !src) {
    return (
      <div
        className={className}
        style={{ background: gradientFor(fallbackText) }}
        role="img"
        aria-label={fallbackText}
      >
        {failed && <span className="img-fallback-text">{fallbackText}</span>}
      </div>
    );
  }

  return <img className={className} src={src} alt={fallbackText} loading="lazy" />;
}
