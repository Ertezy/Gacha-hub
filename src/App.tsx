import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./lib/api";
import { launchNote } from "./lib/launch";
import { setMotion } from "./lib/motion";
import { REFRESH_EVERY_MS, sameHub, shouldRefreshOnShow } from "./lib/hubRefresh";
import SidePanel from "./components/SidePanel";
import GameArt from "./components/GameArt";
import PlayButton from "./components/PlayButton";
import GameDock from "./components/GameDock";
import Settings from "./screens/Settings";
import FirstRunView from "./screens/settings/FirstRunView";
import { currentT, useT } from "./i18n";
import { errorText } from "./i18n/errors";
import type { GameView, HubData, LaunchResult } from "./types";

type Screen = "loading" | "firstRun" | "main" | "settings";

export default function App() {
  const [games, setGames] = useState<GameView[]>([]);
  const [hub, setHub] = useState<HubData | null>(null);
  const [error, setError] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // "loading" — пока не узнали, был ли уже первый запуск: не показывать
  // главный экран, чтобы не мигнуть им перед экраном первого запуска.
  const [screen, setScreen] = useState<Screen>("loading");
  const t = useT();

  // Когда данные панели проверялись последний раз — для проверки при возвращении окна.
  const lastHubCheck = useRef(0);

  // Счётчик запросов к хабу: запросы могут идти внахлёст (таймер, возврат
  // окна, load() после настроек) и ответить не в том порядке, в каком ушли.
  // Побеждает последний запущенный запрос, а не последний ответивший —
  // иначе смена адреса хаба в настройках могла бы откатиться назад, если
  // медленный ответ со старого адреса придёт позже нового.
  const hubRequestId = useRef(0);

  // Хаб грузится отдельно: он может ходить в сеть, и его задержка не должна
  // откладывать появление главного экрана. Те же данные панель не
  // перерисовывают; отказ сети оставляет на экране то, что уже показано.
  const refreshHub = useCallback(async () => {
    lastHubCheck.current = Date.now();
    const requestId = ++hubRequestId.current;
    try {
      const next = await api.getHub();
      if (hubRequestId.current !== requestId) return;
      setHub((current) => (sameHub(current, next) ? current : next));
    } catch {
      // Показанные данные остаются; следующая проверка попробует снова.
    }
  }, []);

  const load = useCallback(async () => {
    try {
      const [list, last] = await Promise.all([api.getGames(), api.getLastPlayed()]);
      setGames(list);
      setSelectedId(last);
      setError("");
    } catch (e) {
      // `currentT()`, а не `t`: с `t` в зависимостях `load` менялась бы при
      // смене языка, и эффект первого запуска ниже срабатывал бы заново —
      // уводил бы из настроек на главный экран.
      setError(errorText(currentT(), e));
    }
    void refreshHub();
  }, [refreshHub]);

  // Только список игр, без выбора и хаба: событие о новых фонах не должно
  // сбрасывать игру, которую человек уже выбрал в доке.
  const reloadGames = useCallback(async () => {
    try {
      setGames(await api.getGames());
      setError("");
    } catch (e) {
      // `currentT()` по той же причине, что и в `load`: подписка ниже не
      // должна пересоздаваться при смене языка.
      setError(errorText(currentT(), e));
    }
  }, []);

  // Фоны Epic докачиваются после запуска (спека этапа 5, §3.3), и Rust сообщает
  // об этом событием. Подписка регистрируется асинхронно, а Tauri не копит
  // события для тех, кто подпишется позже: докачка, закончившаяся до
  // регистрации, прошла бы мимо. Поэтому, как только подписка точно действует,
  // список перечитывается один раз — картинка, успевшая скачаться, уже лежит в
  // кеше. Всё, что докачается позже, придёт событием.
  useEffect(() => {
    let active = true;
    const unlisten = listen("games-changed", () => void reloadGames());
    void unlisten.then(() => {
      if (active) void reloadGames();
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, [reloadGames]);

  // Приложение живёт в трее днями: данные панели проверяются раз в 3 часа и
  // при возвращении окна, если с прошлой проверки прошёл час (спека сборщика §9).
  useEffect(() => {
    const timer = window.setInterval(() => void refreshHub(), REFRESH_EVERY_MS);
    const onShown = () => {
      if (shouldRefreshOnShow(lastHubCheck.current, Date.now())) void refreshHub();
    };
    const unlisten = listen<boolean>("window-visibility", (e) => {
      if (e.payload) onShown();
    });
    const onVisibility = () => {
      if (document.visibilityState === "visible") onShown();
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.clearInterval(timer);
      void unlisten.then((stop) => stop());
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [refreshHub]);

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

  // Тумблер «Анимация» читается один раз при запуске: дальше `motion.ts` сам
  // держит актуальное значение и рассылает его подписчикам при переключении
  // на вкладке «Вид» (спека этапа 5, §12).
  useEffect(() => {
    api.getAnimation().then(setMotion).catch(() => {});
  }, []);

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
    if (!selected) return { ok: false, msg: t.main.noGameSelected };
    try {
      await api.launchGame(selected.id);
      return { ok: true, msg: "" };
    } catch (e) {
      return { ok: false, msg: errorText(t, e) };
    }
  }, [selected, t]);

  // Возврат из настроек обязан перечитать список игр: человек мог там всё
  // поменять (добавить, убрать, переименовать, отвязать от хаба, сменить фон).
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
        <GameArt game={selected} />
        {selected && (
          <PlayButton
            gameId={selected.id}
            note={launchNote(t, selected)}
            onLaunch={launch}
            onFixed={load}
          />
        )}
        {error && <div className="banner">{t.main.loadFailed(error)}</div>}
        <GameDock games={games} selectedId={selected?.id ?? null} onSelect={select} />
      </main>
    </div>
  );
}
