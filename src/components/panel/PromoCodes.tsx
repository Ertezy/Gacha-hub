import { useState } from "react";
import { api } from "../../lib/api";
import { burnsToday, timeLeft } from "../../lib/time";
import type { Code, HubGame } from "../../types";

interface Props {
  codes: Code[];
  games: HubGame[];
  nowSec: number;
}

/**
 * Промокоды — по всем играм сразу, отсортированные по сгоранию.
 *
 * Не по выбранной игре, в отличие от баннеров и видео: код — это список
 * бесплатного, которое пропадает. Показывай мы только выбранную игру,
 * человек прозевал бы код, потому что кликает то, во что играет.
 */
export default function PromoCodes({ codes, games, nowSec }: Props) {
  const [copied, setCopied] = useState<string | null>(null);

  const active = codes
    .filter((c) => c.expiresAt === null || c.expiresAt > nowSec)
    .sort((a, b) => (a.expiresAt ?? Infinity) - (b.expiresAt ?? Infinity));

  if (active.length === 0) return null;

  const gameOf = (id: string) => games.find((g) => g.id === id) ?? null;

  const copy = async (code: string) => {
    await api.copyText(code);
    setCopied(code);
    window.setTimeout(() => setCopied((c) => (c === code ? null : c)), 1500);
  };

  const redeem = async (code: Code) => {
    const template = gameOf(code.gameId)?.redeemUrl;
    if (!template) return;
    // Код приходит из недоверенного файла: символ & или # в нём поменял бы
    // смысл ссылки. Подставляем через процентное кодирование (спека §8.1).
    await api.openSafeUrl(template.replace("{code}", encodeURIComponent(code.code)));
  };

  return (
    <section className="panel-section">
      <div className="panel-label">
        <span>Промокоды</span>
        <span>{active.length}</span>
      </div>

      <div className="codes-box">
        {active.map((c) => {
          const game = gameOf(c.gameId);
          const soon = c.expiresAt !== null && burnsToday(c.expiresAt, nowSec);
          return (
            <div
              key={`${c.gameId}:${c.code}`}
              className={soon ? "code-card burning" : "code-card"}
              onClick={() => void copy(c.code)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  void copy(c.code);
                }
              }}
            >
              <div className="code-main">
                <code>{c.code}</code>
                <div className={soon ? "code-timer soon" : "code-timer"}>
                  {c.expiresAt === null
                    ? "бессрочный"
                    : soon
                      ? `сгорит через ${timeLeft(c.expiresAt, nowSec)}`
                      : `осталось ${timeLeft(c.expiresAt, nowSec)}`}
                </div>
                {c.rewards && <div className="code-meta">{c.rewards}</div>}
                {c.region !== "all" && (
                  <div className="code-region">только {c.region}</div>
                )}
              </div>

              <div className="code-side">
                <span className="game-chip" title={game?.title ?? c.gameId}>
                  {(game?.title ?? c.gameId).slice(0, 3).toUpperCase()}
                </span>
                <div className="code-buttons">
                  <button
                    type="button"
                    aria-label="Копировать код"
                    onClick={(e) => {
                      e.stopPropagation();
                      void copy(c.code);
                    }}
                  >
                    {copied === c.code ? "✓" : "⧉"}
                  </button>
                  {game?.redeemUrl && (
                    <button
                      type="button"
                      className="primary"
                      aria-label="Забрать на сайте"
                      onClick={(e) => {
                        e.stopPropagation();
                        void redeem(c);
                      }}
                    >
                      ↗
                    </button>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
