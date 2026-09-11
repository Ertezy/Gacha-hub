import { useCallback, useEffect, useState } from "react";
import { api } from "../../lib/api";

/** Положение ползунка, пока оттенок вручную не задан: середина круга. */
const UNSET_SLIDER_HUE = 180;

export default function LookSection() {
  // `hue` — то, что реально записано на диск (совпадает с настройками из
  // конфига). `null` означает «не задан вручную».
  const [hue, setHue] = useState<number | null>(null);
  const [adapt, setAdapt] = useState(true);
  // Локальное положение ползунка. Двигается сразу, на каждое событие
  // перетаскивания — запись на диск слушает не его, а конец взаимодействия.
  const [sliderHue, setSliderHue] = useState(UNSET_SLIDER_HUE);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void api.getLook().then((look) => {
      setHue(look.accentHue);
      setAdapt(look.adaptFromArt);
      setSliderHue(look.accentHue ?? UNSET_SLIDER_HUE);
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

  // Запись на диск — только по завершении взаимодействия с ползунком: по
  // отпусканию мыши или клавиши. Само перетаскивание диск не трогает,
  // иначе быстрое перетаскивание — это десятки записей файла настроек в
  // секунду, да ещё и с чтением всего конфига перед каждой.
  const commitSlider = useCallback(() => {
    save(sliderHue, adapt);
  }, [save, sliderHue, adapt]);

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
          value={sliderHue}
          disabled={busy}
          className={hue === null ? "range-unset" : undefined}
          onChange={(e) => setSliderHue(Number(e.target.value))}
          onMouseUp={commitSlider}
          onTouchEnd={commitSlider}
          onKeyUp={commitSlider}
        />
      </label>
      <p className="settings-hint">
        {hue === null
          ? "Сейчас не задан — цвет берётся из фона игры или из палитры."
          : `Сейчас задан: ${hue}°.`}{" "}
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
