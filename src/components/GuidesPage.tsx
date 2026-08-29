import { api } from "../lib/api";
import type { GameView, HubData } from "../types";

export default function GuidesPage({
  hub,
  games,
}: {
  hub: HubData | null;
  games: GameView[];
}) {
  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;
  const items = [...(hub?.guides ?? [])].sort((a, b) => (b.date ?? "").localeCompare(a.date ?? ""));

  return (
    <>
      <h2 className="page-title">Гайды</h2>
      <p className="page-sub">Гайды по новым персонажам и контенту. Данные из bundled-файла hub.json.</p>
      <div className="list">
        {items.map((g) => (
          <article key={`${g.gameId}:${g.title}`} className="list-item">
            <div className="list-item-head">
              <span className="badge">{nameOf(g.gameId)}</span>
              {g.character && <span className="badge accent">{g.character}</span>}
              {g.date && <span className="muted small">{g.date}</span>}
            </div>
            <h3>{g.title}</h3>
            <button
              className="btn ghost small-btn"
              onClick={() =>
                void api.openSafeUrl(g.url).catch((e) => console.error("open url failed:", e))
              }
            >
              Открыть гайд →
            </button>
          </article>
        ))}
        {items.length === 0 && <div className="muted">Пока нет гайдов.</div>}
      </div>
    </>
  );
}
