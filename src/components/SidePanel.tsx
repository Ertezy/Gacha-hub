import { useEffect, useState } from "react";
import PromoCodes from "./panel/PromoCodes";
import Banners from "./panel/Banners";
import Videos from "./panel/Videos";
import { codesFor, panelIsEmpty, sourceLabel, videosFor } from "../lib/panel";
import { hubFreshness } from "../lib/time";
import { useLang, useT } from "../i18n";
import { useVideoLanguage } from "../lib/videoLanguage";
import type { HubData } from "../types";

interface Props {
  hub: HubData | null;
  /** Игра, выбранная в доке. Коды, баннеры и видео — только по ней. */
  selectedContentId: string | null;
  /** Открыть экран настроек — вызывается по клику на шестерёнку. */
  onOpenSettings: () => void;
}

const DAY = 86400;

/** Как часто пересчитывается «сейчас». */
const TICK_MS = 30_000;

export default function SidePanel({ hub, selectedContentId, onOpenSettings }: Props) {
  const lang = useLang();
  const t = useT();
  const videoLang = useVideoLanguage();

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

  const selected = (gameId: string) => gameId === selectedContentId;

  const codes = !hub || tooOld ? [] : codesFor(hub.codes, selectedContentId);
  const banners = !hub || tooOld ? [] : hub.banners.filter((b) => selected(b.gameId));
  // Видео остаются даже на протухших данных: устаревший список роликов
  // просто устаревший, он никого не обманывает.
  const videos = !hub || selectedContentId === null ? [] : videosFor(hub.videos, selectedContentId, videoLang);
  // Хаб ещё не пришёл (null) — это не то же самое, что «пришёл и пуст»:
  // тихая пауза до первого ответа, а не сообщение «кодов нет», которое
  // тут же сменится настоящими данными и будет выглядеть как ложная тревога.
  const empty = hub !== null && panelIsEmpty(codes, banners, videos, nowSec);

  // Источник данных виден в интерфейсе — требование общей спеки §9.4.
  // Без него невозможно отличить «сеть отвалилась, показываю прошлогодний
  // комплект» от «всё свежее», а это первое, что спросят при разборе жалобы.
  const origin = sourceLabel(hub?._source, t);

  return (
    <aside className="panel">
      {/* Секции прокручиваются внутри себя, шестерёнка остаётся прибитой
          к низу (спека §2.4). */}
      <div className="panel-scroll">
        {hub && <PromoCodes codes={codes} games={hub.games} nowSec={nowSec} />}
        <Banners banners={banners} nowSec={nowSec} />
        <Videos videos={videos} nowSec={nowSec} />
        {empty && <p className="panel-empty">{t.panel.panelEmpty}</p>}
      </div>

      <div className="panel-foot">
        {/* Шестерёнка без подписи: значок узнаётся сам, а название есть во
            всплывающей подсказке и для экранного диктора. Зубцы — пунктир
            толстой обводки круга, без чужих иконок и зависимостей. */}
        <button
          type="button"
          className="panel-gear"
          onClick={onOpenSettings}
          aria-label={t.panel.settingsAndGames}
          title={t.panel.settingsAndGames}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle
              cx="12"
              cy="12"
              r="8.2"
              fill="none"
              stroke="currentColor"
              strokeWidth="3.2"
              strokeDasharray="2.6 3.84"
            />
            <circle cx="12" cy="12" r="6" fill="none" stroke="currentColor" strokeWidth="2" />
            <circle cx="12" cy="12" r="2.3" fill="none" stroke="currentColor" strokeWidth="2" />
          </svg>
        </button>
        {hub && (
          <div className="panel-stale">
            {hubFreshness(hub.updatedAt, nowSec, lang)}
            {origin && ` · ${origin}`}
          </div>
        )}
        {/* Материалы принадлежат издателям игр — так помечают себя все
            фанатские источники, из которых мы берём данные. */}
        <div className="panel-disclaimer">{t.panel.disclaimer}</div>
      </div>
    </aside>
  );
}
