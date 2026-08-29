import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./lib/api";
import Sidebar from "./components/Sidebar";
import GamesPage from "./components/GamesPage";
import NewsPage from "./components/NewsPage";
import PromoCodesPage from "./components/PromoCodesPage";
import GuidesPage from "./components/GuidesPage";
import { SettingsPage } from "./components/SettingsPage";
import type { AppConfig, GameLaunchConfig, GameView, HubData, Page } from "./types";

const EMPTY_CONFIG: AppConfig = { version: 1, games: {} };

export default function App() {
  const [page, setPage] = useState<Page>("games");
  const [games, setGames] = useState<GameView[]>([]);
  const [config, setConfig] = useState<AppConfig>(EMPTY_CONFIG);
  const [hub, setHub] = useState<HubData | null>(null);
  const [configDir, setConfigDir] = useState("");
  const [loadError, setLoadError] = useState("");

  // Never persist config before the first successful load has landed:
  // otherwise a failed get_config (fallback EMPTY_CONFIG) would be written
  // back to disk and wipe the user's settings.
  const configLoaded = useRef(false);

  const refresh = useCallback(async () => {
    // The hub is optional data: a broken hub.json must not blank the whole
    // app, so we settle each promise independently instead of Promise.all.
    const [g, c, h, dir] = await Promise.allSettled([
      api.getGames(),
      api.getConfig(),
      api.getHub(),
      api.getConfigDir(),
    ]);
    const ok = <T,>(r: PromiseSettledResult<T>, fallback: T): T =>
      r.status === "fulfilled" ? r.value : fallback;
    setGames(ok(g, []));
    setConfig(ok(c, EMPTY_CONFIG));
    if (c.status === "fulfilled") configLoaded.current = true;
    setHub(ok(h, null));
    setConfigDir(ok(dir, ""));
    setLoadError(
      [g, c, h, dir]
        .filter((r): r is PromiseRejectedResult => r.status === "rejected")
        .map((r) => String(r.reason))
        .join(" | "),
    );
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  /** Update the config in memory only. Persistence happens in the
   *  debounced effect below — the setState updater stays side-effect free
   *  (safe under React StrictMode, which double-invokes updaters). */
  const updateConfig = useCallback((updater: (prev: AppConfig) => AppConfig) => {
    setConfig((prev) => updater(prev));
  }, []);

  // Debounced persistence: one config.json write per 300 ms pause in
  // typing, not per keystroke.
  const lastSaved = useRef<string | null>(null);
  useEffect(() => {
    if (!configLoaded.current) return;
    const json = JSON.stringify(config);
    if (json === lastSaved.current) return;
    lastSaved.current = json;
    const t = window.setTimeout(() => {
      api.saveConfig(config).catch((err) => console.error("config save failed:", err));
    }, 300);
    return () => window.clearTimeout(t);
  }, [config]);

  // One-time auto-configure: the scan results are the source of truth, not
  // the static catalog. Found Steam install → steam mode; found
  // Epic/official install → exe mode + path; nothing found → catalog
  // default. Games with a saved config are never touched.
  const autoDetectDone = useRef(false);
  useEffect(() => {
    if (autoDetectDone.current || games.length === 0) return;
    autoDetectDone.current = true;
    api
      .detectInstalls()
      .then((found) => {
        updateConfig((prev) => {
          let changed = false;
          const nextGames = { ...prev.games };
          for (const g of games) {
            if (prev.games[g.id]) continue; // already configured — don't touch
            const candidates = found.filter((f) => f.gameId === g.id);
            if (candidates.length === 0) continue; // keep the catalog default
            const base: GameLaunchConfig = {
              launchMode: g.steamAppid !== null ? "steam" : "exe",
              args: "",
            };
            const steamFound =
              g.steamAppid !== null && candidates.some((f) => f.source === "steam");
            nextGames[g.id] = steamFound
              ? base
              : { ...base, launchMode: "exe", exePath: candidates[0].path };
            changed = true;
          }
          return changed ? { ...prev, games: nextGames } : prev;
        });
      })
      .catch((e) => console.error("install auto-detect failed:", e));
  }, [games, updateConfig]);

  const updateGame = useCallback(
    (gameId: string, patch: Partial<GameLaunchConfig>) => {
      updateConfig((prev) => {
        // Default mirrors the backend (GameLaunchConfig::default_for):
        // Steam when the game has a Steam appid, otherwise direct .exe.
        const meta = games.find((g) => g.id === gameId);
        const cur: GameLaunchConfig =
          prev.games[gameId] ??
          (meta
            ? { launchMode: meta.steamAppid !== null ? "steam" : "exe", args: "" }
            : { launchMode: "exe", args: "" });
        return { ...prev, games: { ...prev.games, [gameId]: { ...cur, ...patch } } };
      });
    },
    [updateConfig, games],
  );

  const launch = useCallback(
    async (id: string): Promise<{ ok: boolean; msg: string }> => {
      try {
        await api.launchGame(id);
        return { ok: true, msg: "Игра запущена." };
      } catch (e) {
        return { ok: false, msg: String(e) };
      }
    },
    [],
  );

  const patchApp = useCallback(
    (patch: Partial<AppConfig>) => updateConfig((prev) => ({ ...prev, ...patch })),
    [updateConfig],
  );

  return (
    <div className="app">
      <Sidebar page={page} onNavigate={setPage} />
      <main className="main">
        {loadError && <div className="banner error">Не удалось загрузить данные: {loadError}</div>}
        {page === "games" && (
          <GamesPage games={games} config={config} onLaunch={launch} onPatchGame={updateGame} />
        )}
        {page === "news" && <NewsPage hub={hub} games={games} />}
        {page === "promo" && <PromoCodesPage hub={hub} games={games} />}
        {page === "guides" && <GuidesPage hub={hub} games={games} />}
        {page === "settings" && (
          <SettingsPage
            games={games}
            config={config}
            configDir={configDir}
            onPatchGame={updateGame}
            onPatchApp={patchApp}
          />
        )}
      </main>
    </div>
  );
}
