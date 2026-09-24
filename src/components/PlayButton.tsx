import { useState } from "react";
import { api } from "../lib/api";
import { useT } from "../i18n";
import { errorText } from "../i18n/errors";
import type { LaunchResult } from "../types";

interface Props {
  gameId: string;
  /** Подпись над кнопкой: откуда запустится игра (`launchNote`). */
  note: string;
  onLaunch: () => Promise<LaunchResult>;
  /** Путь починили — родителю нужно перечитать список игр, чтобы ошибка
   *  и значок пропажи исчезли (спека §3.4). */
  onFixed: () => void;
  disabled?: boolean;
}

export default function PlayButton({ gameId, note, onLaunch, onFixed, disabled }: Props) {
  const [busy, setBusy] = useState(false);
  const [fixing, setFixing] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const t = useT();

  const click = async () => {
    setBusy(true);
    setFailure(null);
    const result = await onLaunch();
    if (!result.ok) setFailure(result.msg);
    setBusy(false);
  };

  // Оба способа починки — те же, что уже есть в правке игры в настройках
  // (GameEditor): при успехе список должен перечитаться, чтобы ошибка и
  // значок пропажи исчезли; при провале — показать, что пошло не так.
  const runFix = async (fn: () => Promise<void>) => {
    setFixing(true);
    try {
      await fn();
      setFailure(null);
      onFixed();
    } catch (e) {
      setFailure(errorText(t, e));
    } finally {
      setFixing(false);
    }
  };

  const relocate = () => runFix(() => api.relocateGame(gameId));

  const pickManually = () =>
    runFix(async () => {
      const exe = await api.pickExe();
      if (exe) await api.updateGame({ gameId, exe });
    });

  return (
    <div className="play-area">
      {failure && (
        <>
          <div className="play-error">{failure}</div>
          <div className="play-fix">
            <button type="button" disabled={fixing} onClick={() => void relocate()}>
              Найти заново
            </button>
            <button type="button" disabled={fixing} onClick={() => void pickManually()}>
              Указать вручную
            </button>
          </div>
        </>
      )}
      <div className="play-note">{note}</div>
      <button
        className="play"
        disabled={busy || fixing || disabled}
        onClick={() => void click()}
      >
        {busy ? "Запускаю…" : "▶ Играть"}
      </button>
    </div>
  );
}
