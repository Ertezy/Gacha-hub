import { useEffect, useState } from "react";
import PromoCodes from "./panel/PromoCodes";
import Banners from "./panel/Banners";
import Videos from "./panel/Videos";
import type { HubData } from "../types";

interface Props {
  hub: HubData | null;
  /** Идентификаторы контента для всех добавленных игр. */
  contentIds: string[];
  /** Игра, выбранная на полке. Баннеры и видео — только по ней. */
  selectedContentId: string | null;
}

const DAY = 86400;

/** Как часто пересчитывается «сейчас». */
const TICK_MS = 30_000;

export default function SidePanel({ hub, contentIds, selectedContentId }: Props) {
  // Часы держатся в состоянии, а не вычисляются при отрисовке.
  //
  // Иначе таймеры замерзают: «сгорит через 3 ч 52 мин» висит неизменным, пока
  // человек не кликнет что-нибудь ещё, и код, сгоревший час назад, продолжает
  // предлагать себя. Полминуты достаточно: самая мелкая величина, которую мы
  // показываем, — минуты.
  const [nowSec, setNowSec] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    const id = window.setInterval(
      () => setNowSec(Math.floor(Date.now() / 1000)),
      TICK_MS,
    );
    return () => window.clearInterval(id);
  }, []);

  const age = hub ? nowSec - hub.updatedAt : 0;

  // Старше недели — коды и баннеры прячутся целиком. Код, который наверняка
  // сгорел, и баннер, который наверняка кончился, хуже, чем их отсутствие:
  // таймер на протухших данных — это враньё в лицо (спека §6).
  const tooOld = hub !== null && age > 7 * DAY;
  const stale = hub !== null && age > DAY;

  const mine = (gameId: string) => contentIds.includes(gameId);
  const selected = (gameId: string) => gameId === selectedContentId;

  const codes = !hub || tooOld ? [] : hub.codes.filter((c) => mine(c.gameId));
  const banners = !hub || tooOld ? [] : hub.banners.filter((b) => selected(b.gameId));
  // Видео остаются даже на протухших данных: устаревший список роликов
  // просто устаревший, он никого не обманывает.
  const videos = !hub ? [] : hub.videos.filter((v) => selected(v.gameId));

  const updated = hub
    ? new Date(hub.updatedAt * 1000).toLocaleDateString("ru-RU", {
        day: "numeric",
        month: "long",
      })
    : "";

  // Источник данных виден в интерфейсе — требование общей спеки §9.4.
  // Без него невозможно отличить «сеть отвалилась, показываю прошлогодний
  // комплект» от «всё свежее», а это первое, что спросят при разборе жалобы.
  const sourceLabel: Record<string, string> = {
    remote: "из сети",
    override: "из локальной подмены",
    cache: "из кеша",
    bundled: "из комплекта",
  };
  const origin = hub?._source ? sourceLabel[hub._source] ?? hub._source : "";

  return (
    <aside className="panel">
      {/* Секции прокручиваются внутри себя, шестерёнка остаётся прибитой
          к низу (спека §2.4). */}
      <div className="panel-scroll">
        {hub && <PromoCodes codes={codes} games={hub.games} nowSec={nowSec} />}
        <Banners banners={banners} nowSec={nowSec} />
        <Videos videos={videos} nowSec={nowSec} />
      </div>

      <div className="panel-foot">
        <div className="panel-foot-row">⚙ Настройки и игры</div>
        {hub && (
          <div className="panel-stale">
            {stale ? `данные от ${updated}` : "данные свежие"}
            {origin && ` · ${origin}`}
          </div>
        )}
        {/* Материалы принадлежат издателям игр — так помечают себя все
            фанатские источники, из которых мы берём данные. */}
        <div className="panel-disclaimer">
          Не связано с разработчиками игр. Материалы принадлежат правообладателям.
        </div>
      </div>
    </aside>
  );
}
