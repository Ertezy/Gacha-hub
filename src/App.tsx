import { useCallback, useEffect, useState } from "react";
import { api } from "./lib/api";
import SidePanel from "./components/SidePanel";
import type { GameView, HubData } from "./types";

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

  return (
    <div className="screen">
      <SidePanel hub={hub} contentIds={contentIds} />
      <main className="stage">
        {error && <div className="banner">Не удалось загрузить: {error}</div>}
        <div className="stage-placeholder">
          {games.length} игр найдено
        </div>
      </main>
    </div>
  );
}
