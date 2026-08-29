import { useState } from "react";
import type { AppConfig, GameLaunchConfig, GameView } from "../types";
import { api } from "../lib/api";

interface Props {
  games: GameView[];
  config: AppConfig;
  configDir: string;
  onPatchGame: (id: string, patch: Partial<GameLaunchConfig>) => void;
  onPatchApp: (patch: Partial<AppConfig>) => void;
}

export function SettingsPage({ games, config, configDir, onPatchGame, onPatchApp }: Props) {
  const [scanMsg, setScanMsg] = useState<Record<string, string>>({});
  const [scanning, setScanning] = useState<string | null>(null);

  /** Full rescan (bypasses the session cache) and fill the first hit for
   * this game; the detected path wins over whatever was there. */
  const scan = async (id: string) => {
    setScanning(id);
    try {
      const found = (await api.rescanInstalls()).filter((f) => f.gameId === id);
      if (found.length === 0) {
        setScanMsg((m) => ({ ...m, [id]: "установка не найдена — укажите путь вручную" }));
      } else {
        const hit = found[0];
        onPatchGame(id, { launchMode: "exe", exePath: hit.path });
        setScanMsg((m) => ({ ...m, [id]: `найдено: ${hit.path} (${hit.source})` }));
      }
    } catch (e) {
      setScanMsg((m) => ({ ...m, [id]: `ошибка сканирования: ${e}` }));
    } finally {
      setScanning(null);
    }
  };

  const pickExe = async (id: string) => {
    try {
      const path = await api.pickExe();
      if (path) onPatchGame(id, { exePath: path });
    } catch {
      /* cancelled */
    }
  };

  return (
    <div className="stack">
      <h1>Настройки</h1>
      <div className="sub">
        Конфиг: {configDir || "—"}
        {" · автосохранение после изменений"}
      </div>

      <div className="field">
        <span>
          URL удалённого hub.json (необязательно). Пусто — используются
          данные из бандла; локальную копию можно положить в
          <code> {configDir || "%APPDATA%\\com.gachahub.desktop"}\hub.json</code>
        </span>
        <input
          value={config.hubUrl ?? ""}
          placeholder="https://example.com/hub.json"
          onChange={(e) => onPatchApp({ hubUrl: e.target.value || null })}
        />
      </div>

      <p className="sub">
        Для Steam-режима ничего настраивать не нужно — игра запускается по
        appid. Для Epic-режима без product id игра ищется в каталоге Epic
        Games и запускается её exe напрямую.
      </p>

      {games.map((g) => {
        // Render from the live config (never from the catalog default),
        // otherwise inputs snap back to the catalog value after every save.
        const cfg = config.games[g.id] ?? g.config;
        return (
          <div key={g.id} className="card">
            <h2>{g.name}</h2>
            <div className="row" style={{ gap: 10, flexWrap: "wrap" }}>
              <div className="field" style={{ flex: 1, minWidth: 200 }}>
                <span>Режим запуска</span>
                <select
                  value={cfg.launchMode}
                  onChange={(e) =>
                    onPatchGame(g.id, {
                      launchMode: e.target.value as GameLaunchConfig["launchMode"],
                    })
                  }
                >
                  <option value="steam">Steam (appid: {g.steamAppid ?? "—"})</option>
                  <option value="epic">Epic Games Store</option>
                  <option value="exe">Прямой .exe</option>
                </select>
              </div>

              {cfg.launchMode === "exe" && (
                <>
                  <div className="field" style={{ flex: 2, minWidth: 300 }}>
                    <span>Путь к .exe</span>
                    <input
                      value={cfg.exePath ?? ""}
                      placeholder="C:\Games\Genshin Impact\GenshinImpact.exe"
                      onChange={(e) => onPatchGame(g.id, { exePath: e.target.value })}
                    />
                  </div>
                  <button
                    onClick={() => void scan(g.id)}
                    disabled={scanning === g.id}
                    style={{ alignSelf: "flex-end" }}
                  >
                    {scanning === g.id ? "Ищем…" : "Скан"}
                  </button>
                  <button onClick={() => void pickExe(g.id)} style={{ alignSelf: "flex-end" }}>
                    Выбрать…
                  </button>
                </>
              )}

              {cfg.launchMode === "epic" && (
                <div className="field" style={{ flex: 1, minWidth: 200 }}>
                  <span>Epic product id (необязательно)</span>
                  <input
                    value={cfg.epicProductId ?? ""}
                    placeholder="например: HonkaiStarRail"
                    onChange={(e) => onPatchGame(g.id, { epicProductId: e.target.value })}
                  />
                </div>
              )}

              <div className="field" style={{ flex: 1, minWidth: 160 }}>
                <span>Доп. аргументы (попробовать)</span>
                <input
                  value={cfg.args ?? ""}
                  placeholder="-noverify"
                  onChange={(e) => onPatchGame(g.id, { args: e.target.value })}
                />
              </div>
            </div>
            {scanMsg[g.id] && <div className="sub" style={{ marginTop: 6 }}>{scanMsg[g.id]}</div>}
          </div>
        );
      })}
    </div>
  );
}
