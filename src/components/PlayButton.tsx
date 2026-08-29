import { useState } from "react";
import type { LaunchResult } from "../types";

interface Props {
  onLaunch: () => Promise<LaunchResult>;
  disabled?: boolean;
}

export default function PlayButton({ onLaunch, disabled }: Props) {
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

  const click = async () => {
    setBusy(true);
    setFailure(null);
    const result = await onLaunch();
    if (!result.ok) setFailure(result.msg);
    setBusy(false);
  };

  return (
    <div className="play-area">
      {failure && <div className="play-error">{failure}</div>}
      <button
        className="play"
        disabled={busy || disabled}
        onClick={() => void click()}
      >
        {busy ? "Запускаю…" : "▶ Играть"}
      </button>
    </div>
  );
}
