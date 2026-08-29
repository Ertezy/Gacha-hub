import { useState } from "react";
import { api } from "../lib/api";
import type { AppConfig, GameLaunchConfig, GameView, LaunchResult } from "../types";

interface CardProps {
  game: GameView;
  /** Effective config: user-saved value, or the catalog default. */
  cfg: GameLaunchConfig;
  onLaunch: (id: string) => Promise<LaunchResult>;
  onPatchGame: (id: string, patch: Partial<GameLaunchConfig>) => void;
}

function modeLabel(g: GameView, cfg: GameLaunchConfig): string {
  switch (cfg.launchMode) {
    case "steam":
      return `Steam (appid ${g.steamAppid ?? "?"})`;
    case "epic":
      return "Epic Games Store";
    case "exe":
      return "Прямой .exe";
  }
}

function GameCard({ game, cfg, onLaunch, onPatchGame }: CardProps) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<LaunchResult | null>(null);

  const play = async () => {
    setBusy(true);
    setStatus(null);
    setStatus(await onLaunch(game.id));
    setBusy(false);
  };

  return (
    <div className="game-card">
      <div className="game-card-head">
        <div>
          <h3>{game.name}</h3>
          <div className="muted small">{game.publisher}</div>
        </div>
        <div className="badges">
          {game.steamAppid !== null && <span className="badge">Steam</span>}
          {game.epicSupported && <span className="badge">Epic</span>}
          {game.hasOfficialLauncher && <span className="badge">Official</span>}
        </div>
      </div>

      <div className="muted small">Способ запуска: {modeLabel(game, cfg)}</div>

      {cfg.launchMode === "exe" && (
        <label className="field">
          <span>Аргументы запуска</span>
          <input
            value={cfg.args}
            placeholder="-dx12"
            onChange={(e) => onPatchGame(game.id, { args: e.target.value })}
          />
        </label>
      )}

      <div className="row">
        <button className="btn primary" disabled={busy} onClick={() => void play()}>
          {busy ? "Запуск…" : "▶ Играть"}
        </button>
        <button
          className="btn ghost"
          onClick={() =>
            void api.openSafeUrl(game.storeUrl).catch((e) => console.error("open url failed:", e))
          }
        >
          Store / сайт
        </button>
      </div>

      {status && <div className={`status ${status.ok ? "ok" : "err"}`}>{status.msg}</div>}
    </div>
  );
}

export default function GamesPage({
  games,
  config,
  onLaunch,
  onPatchGame,
}: {
  games: GameView[];
  config: AppConfig;
  onLaunch: (id: string) => Promise<LaunchResult>;
  onPatchGame: (id: string, patch: Partial<GameLaunchConfig>) => void;
}) {
  return (
    <>
      <h2 className="page-title">Игры</h2>
      <p className="page-sub">
        Способ запуска и путь к .exe настраиваются на странице «Настройки». Аргументы
        (например, -dx12) применяются к прямому запуску .exe.
      </p>
      <div className="grid">
        {games.map((g) => (
          <GameCard
            key={g.id}
            game={g}
            cfg={config.games[g.id] ?? g.config}
            onLaunch={onLaunch}
            onPatchGame={onPatchGame}
          />
        ))}
      </div>
    </>
  );
}
