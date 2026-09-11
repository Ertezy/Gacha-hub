import { useCallback, useEffect, useState } from "react";
import { api } from "../../lib/api";

export default function LookSection() {
  const [hue, setHue] = useState<number | null>(null);
  const [adapt, setAdapt] = useState(true);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void api.getLook().then((look) => {
      setHue(look.accentHue);
      setAdapt(look.adaptFromArt);
    });
  }, []);

  const save = useCallback(
    (nextHue: number | null, nextAdapt: boolean) => {
      setBusy(true);
      setError("");
      api
        .setLook(nextHue, nextAdapt)
        .then(() => {
          setHue(nextHue);
          setAdapt(nextAdapt);
        })
        .catch((e) => setError(String(e)))
        .finally(() => setBusy(false));
    },
    [],
  );

  return (
    <div>
      <h2 className="settings-section-title">Вид</h2>

      <label className="field-row">
        <input
          type="checkbox"
          checked={adapt}
          disabled={busy}
          onChange={(e) => save(hue, e.target.checked)}
        />
        <span>Подбирать цвет из фона игры</span>
      </label>
      <p className="settings-hint">
        Берётся только оттенок — насыщенность и яркость остаются прежними,
        поэтому испортить вид нельзя.
      </p>

      <label className="field">
        <span>Оттенок вручную</span>
        <input
          type="range"
          min={0}
          max={359}
          value={hue ?? 45}
          disabled={busy}
          onChange={(e) => save(Number(e.target.value), adapt)}
        />
      </label>
      <p className="settings-hint">
        Заданный вручную оттенок побеждает подобранный из фона.
      </p>

      <div className="field-row">
        <button type="button" disabled={busy || hue === null} onClick={() => save(null, adapt)}>
          Сбросить оттенок
        </button>
      </div>

      {error && <div className="settings-error">{error}</div>}
    </div>
  );
}
