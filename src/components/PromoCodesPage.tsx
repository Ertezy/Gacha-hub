import { useState } from "react";
import type { GameView, HubData, PromoCode } from "../types";

function isExpired(c: PromoCode): boolean {
  // No expiry date -> treat as still valid (and avoid building an
  // `Invalid Date` from "T23:59:59").
  if (!c.expired) return false;
  return new Date(`${c.expired}T23:59:59`) < new Date();
}

export default function PromoCodesPage({
  hub,
  games,
}: {
  hub: HubData | null;
  games: GameView[];
}) {
  const [filter, setFilter] = useState("all");
  const [copied, setCopied] = useState<string | null>(null);

  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;
  const codes = (hub?.promoCodes ?? []).filter((c) => filter === "all" || c.gameId === filter);

  const copy = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(code);
      window.setTimeout(() => setCopied((cur) => (cur === code ? null : cur)), 1500);
    } catch (e) {
      console.error("clipboard:", e);
    }
  };

  return (
    <>
      <h2 className="page-title">Промокоды</h2>
      <p className="page-sub">
        Из bundled-файла hub.json. Поля: code / rewards / expired (ISO-дата, инклюзивно).
        {hub?._note ? ` · ${hub._note}` : ""}
      </p>

      <select className="filter" value={filter} onChange={(e) => setFilter(e.target.value)}>
        <option value="all">Все игры</option>
        {games.map((g) => (
          <option key={g.id} value={g.id}>
            {g.name}
          </option>
        ))}
      </select>

      <div className="list">
        {codes.map((c) => {
          const expired = isExpired(c);
          return (
            <article key={`${c.gameId}:${c.code}`} className={`promo${expired ? " expired" : ""}`}>
              <div className="promo-head">
                <span className="badge">{nameOf(c.gameId)}</span>
                <span className="muted small">
                  {expired ? `истёк ${c.expired}` : `действует до ${c.expired}`}
                </span>
                {expired && <span className="badge warn">истёк</span>}
              </div>
              <div className="promo-code-row">
                <code>{c.code}</code>
                <button
                  className="btn ghost small-btn"
                  disabled={expired}
                  onClick={() => void copy(c.code)}
                >
                  {copied === c.code ? "Скопировано ✓" : "Копировать"}
                </button>
              </div>
              <div className="muted small">{c.rewards}</div>
              {c.source && <div className="muted tiny">источник: {c.source}</div>}
            </article>
          );
        })}
        {codes.length === 0 && <div className="muted">Нет промокодов для этого фильтра.</div>}
      </div>
    </>
  );
}
