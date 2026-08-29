import type { HubData } from "../types";

interface Props {
  hub: HubData | null;
  /** Идентификаторы контента для игр пользователя. */
  contentIds: string[];
}

export default function SidePanel({ hub, contentIds }: Props) {
  const mine = (gameId: string) => contentIds.includes(gameId);
  const codes = (hub?.promoCodes ?? []).filter((c) => mine(c.gameId));
  const news = (hub?.news ?? []).filter((n) => mine(n.gameId));

  return (
    <aside className="panel">
      <section className="panel-section">
        <div className="panel-label">
          <span>Промокоды</span>
          <span>{codes.length}</span>
        </div>
        {codes.length === 0 && <div className="panel-empty">пока пусто</div>}
        {codes.map((c) => (
          <div className="code-card" key={`${c.gameId}:${c.code}`}>
            <code>{c.code}</code>
            <div className="code-meta">{c.rewards}</div>
          </div>
        ))}
      </section>

      <section className="panel-section">
        <div className="panel-label">
          <span>Новости</span>
        </div>
        {news.length === 0 && <div className="panel-empty">пока пусто</div>}
        {news.map((n, i) => (
          <div className="news-card" key={i}>
            <div className="news-title">{n.title}</div>
            <div className="code-meta">{n.date}</div>
          </div>
        ))}
      </section>

      <div className="panel-foot">⚙ Настройки и игры</div>
    </aside>
  );
}
