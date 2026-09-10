import { useCallback, useEffect, useState } from "react";
import { api } from "./lib/api";
import SidePanel from "./components/SidePanel";
import GameArt from "./components/GameArt";
import GameHeader from "./components/GameHeader";
import PlayButton from "./components/PlayButton";
import GameDock from "./components/GameDock";
import Settings from "./screens/Settings";
import FirstRunView from "./screens/settings/FirstRunView";
import type { GameView, HubData, LaunchResult, Look } from "./types";

type Screen = "loading" | "firstRun" | "main" | "settings";

export default function App() {
  const [games, setGames] = useState<GameView[]>([]);
  const [hub, setHub] = useState<HubData | null>(null);
  const [error, setError] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // "loading" — пока не узнали, был ли уже первый запуск: не показывать
  // главный экран, чтобы не мигнуть им перед экраном первого запуска.
  const [screen, setScreen] = useState<Screen>("loading");
  const [look, setLook] = useState<Look | null>(null);

  useEffect(() => {
    void api.getLook().then(setLook);
  }, []);

  /** Оттенок ставится переопределением переменной на корне документа. */
  const applyAccent = useCallback((hue: number | null) => {
    const root = document.documentElement;
    if (hue === null) {
      root.style.removeProperty("--accent");
      root.style.removeProperty("--accent-ink");
      return;
    }
    // Насыщенность и светлота взяты из объявленного в палитре --accent
    // (#ffc531 = hsl(43 100% 60%)) и --accent-ink (#20180a = hsl(38 52% 8%))
    // и НЕ меняются: подбирается только оттенок. Это и делает подбор
    // безопасным — цвет можно лишь сдвинуть по кругу, а не испортить.
    root.style.setProperty("--accent", `hsl(${hue} 100% 60%)`);
    root.style.setProperty("--accent-ink", `hsl(${hue} 52% 8%)`);
  }, []);

  /** Заданный вручную оттенок побеждает подобранный из арта. */
  const onHue = useCallback(
    (artHue: number | null) => {
      if (!look) return;
      if (look.accentHue !== null) {
        applyAccent(look.accentHue);
        return;
      }
      applyAccent(look.adaptFromArt ? artHue : null);
    },
    [look, applyAccent],
  );

  const load = useCallback(async () => {
    try {
      const [list, last] = await Promise.all([api.getGames(), api.getLastPlayed()]);
      setGames(list);
      setSelectedId(last);
      setError("");
    } catch (e) {
      setError(String(e));
    }
    // Хаб грузится отдельно: он может ходить в сеть, и его задержка
    // не должна откладывать появление главного экрана.
    api.getHub().then(setHub).catch(() => setHub(null));
  }, []);

  useEffect(() => {
    void (async () => {
      if (await api.needsFirstRun()) {
        setScreen("firstRun");
        return;
      }
      await load();
      setScreen("main");
    })();
  }, [load]);

  // Экран первого запуска закрывается — список игр мог измениться.
  const finishFirstRun = useCallback(() => {
    setScreen("main");
    void load();
  }, [load]);

  const contentIds = games
    .map((g) => g.contentId)
    .filter((id): id is string => id !== null);

  const selected = games.find((g) => g.id === selectedId) ?? games[0] ?? null;

  const select = useCallback((id: string) => {
    setSelectedId(id);
  }, []);

  const launch = useCallback(async (): Promise<LaunchResult> => {
    if (!selected) return { ok: false, msg: "игра не выбрана" };
    try {
      await api.launchGame(selected.id);
      return { ok: true, msg: "" };
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
  }, [selected]);

  // Возврат из настроек обязан перечитать список игр: человек мог там всё
  // поменять (добавить, убрать, переименовать, отвязать от хаба).
  const closeSettings = useCallback(() => {
    setScreen("main");
    void load();
  }, [load]);

  if (screen === "loading") {
    return <div className="screen" />;
  }

  if (screen === "firstRun") {
    // Тот же визуальный каркас, что и у настроек, но без навигации по
    // вкладкам: настраивать пока нечего, кроме списка игр.
    return (
      <div className="settings">
        <div className="settings-body">
          <FirstRunView onDone={finishFirstRun} />
        </div>
      </div>
    );
  }

  if (screen === "settings") {
    return <Settings onClose={closeSettings} />;
  }

  return (
    <div className="screen">
      <SidePanel
        hub={hub}
        contentIds={contentIds}
        selectedContentId={selected?.contentId ?? null}
        onOpenSettings={() => setScreen("settings")}
      />
      <main className="stage">
        <GameArt game={selected} onHue={onHue} />
        {selected && <GameHeader game={selected} />}
        {selected && <PlayButton gameId={selected.id} onLaunch={launch} onFixed={load} />}
        {error && <div className="banner">Не удалось загрузить: {error}</div>}
        <GameDock games={games} selectedId={selected?.id ?? null} onSelect={select} />
      </main>
    </div>
  );
}
