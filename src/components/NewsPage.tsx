import { api } from "../lib/api";
import type { GameView, HubData } from "../types";

export default function NewsPage({ hub, games }: { hub: HubData | null; games: GameView[] }) {
  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;
  const items = [...(hub?.news ?? [])].sort((a, b) => b.date.localeCompare(a.date));

  return (
    <>
      <h2 className="page-title">Новости</h2>
      <p className="page-sub">
        Обновлено: {hub?.updatedAt ?? "—"}
        {hub?._source ? ` · источник: ${hub._source}` : ""}
        {hub?._note ? ` · ${hub._note}` : ""}
      </p>
      <div className="list">
        {items.map((n) => (
          <article key={`${n.gameId}:${n.date}:${n.url}`} className="list-item">
            <div className="list-item-head">
              <span className="badge">{nameOf(n.gameId)}</span>
              <span className="muted small">{n.date}</span>
            </div>
            <h3>{n.title}</h3>
            <p className="muted">{n.summary}</p>
            <button
              className="btn ghost small-btn"
              onClick={() =>
                void api.openSafeUrl(n.url).catch((e) => console.error("open url failed:", e))
              }
            >
              Читать →
            </button>
          </article>
        ))}
        {items.length === 0 && <div className="muted">Пока нет новостей.</div>}
      </div>
    </>
  );
}
