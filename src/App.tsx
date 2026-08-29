import { useCallback, useEffect, useState } from "react";
import { api } from "./lib/api";
import SidePanel from "./components/SidePanel";
import GameArt from "./components/GameArt";
import GameHeader from "./components/GameHeader";
import PlayButton from "./components/PlayButton";
import type { GameView, HubData, LaunchResult } from "./types";

export default function App() {
  const [games, setGames] = useState<GameView[]>([]);
  const [hub, setHub] = useState<HubData | null>(null);
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    try {
      setGames(await api.getGames());
      setError("");
    } catch (e) {
      setError(String(e));
    }
    // Хаб грузится отдельно: он может ходить в сеть, и его задержка
    // не должна откладывать появление главного экрана.
    api.getHub().then(setHub).catch(() => setHub(null));
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const contentIds = games
    .map((g) => g.contentId)
    .filter((id): id is string => id !== null);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = games.find((g) => g.id === selectedId) ?? games[0] ?? null;
  // setSelectedId пока не вызывается — его подключит Task 12.
  void setSelectedId;

  const launch = useCallback(async (): Promise<LaunchResult> => {
    if (!selected) return { ok: false, msg: "игра не выбрана" };
    try {
      await api.launchGame(selected.id);
      return { ok: true, msg: "" };
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
  }, [selected]);

  return (
    <div className="screen">
      <SidePanel hub={hub} contentIds={contentIds} />
      <main className="stage">
        <GameArt game={selected} />
        {selected && <GameHeader game={selected} />}
        {selected && <PlayButton onLaunch={launch} />}
        {error && <div className="banner">Не удалось загрузить: {error}</div>}
      </main>
    </div>
  );
}
