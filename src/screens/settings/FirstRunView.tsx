import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import { currentLang, dictionary, setLanguage, useLang, useT, type Lang } from "../../i18n";
import { errorText } from "../../i18n/errors";
import { LANGS, LANG_SHORT, LANGUAGE_BLOCK_TITLE } from "../../i18n/native";
import Segmented from "./Segmented";
import type { FoundGame } from "../../types";

interface Props {
  onDone: () => void;
}

export default function FirstRunView({ onDone }: Props) {
  const [found, setFound] = useState<FoundGame[]>([]);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const t = useT();
  const lang = useLang();

  // Выбор применяется сразу — страница перерисовывается на новом языке, — а
  // сохраняется следом. Не сохранился — язык возвращается, ошибка на прежнем.
  async function chooseLanguage(next: Lang) {
    const before = currentLang();
    setLanguage(next);
    // Тот же busy, что блокирует кнопки ниже: переключатель не должен принять
    // второй клик, пока этот запрос или другое сохранение ещё выполняется.
    setBusy(true);
    try {
      await api.setLanguage(next);
      setError("");
    } catch (e) {
      setLanguage(before);
      setError(errorText(dictionary(before), e));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void (async () => {
      const list = await api.scanInstalled();
      setFound(list);
      // По умолчанию отмечены только игры, про которые у нас есть контент,
      // и только те, которых ещё нет в списке (§6.3 общей спеки).
      setChecked(
        new Set(
          list.filter((g) => g.contentId !== null && !g.alreadyAdded).map((g) => g.title),
        ),
      );
    })();
  }, []);

  const known = found.filter((g) => g.contentId !== null);
  const rest = found.filter((g) => g.contentId === null);

  const toggle = (title: string) =>
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(title)) next.delete(title);
      else next.add(title);
      return next;
    });

  // Первый запуск считается состоявшимся после любого решения — и когда
  // добавили отмеченные, и когда нажали «Пропустить». Пропуск это тоже
  // осознанный выбор, и повторно спрашивать нельзя.
  const markSeededAndClose = async () => {
    await api.markSeeded();
    onDone();
  };

  // Отметка пишется в конфиг, а на этом экране нет ни кнопки «назад», ни
  // вкладок — если запись не удастся, об этом обязательно нужно сказать,
  // иначе «Пропустить» молча перестанет что-либо делать.
  const skip = () =>
    void (async () => {
      setBusy(true);
      setError("");
      try {
        await markSeededAndClose();
      } catch (e) {
        setError(errorText(t, e));
      } finally {
        setBusy(false);
      }
    })();

  const rows = (list: FoundGame[]) =>
    list.map((g) => (
      <label key={g.title} className="found-row">
        <input
          type="checkbox"
          checked={checked.has(g.title)}
          disabled={g.alreadyAdded}
          onChange={() => toggle(g.title)}
        />
        <span className="found-title">{g.title}</span>
        <span className="found-source">{t.main.launch.from[g.sourceKind]}</span>
        {g.alreadyAdded && <span className="found-note">{t.settings.firstRun.alreadyAdded}</span>}
      </label>
    ));

  return (
    <div>
      <div className="first-run-head">
        <h2 className="settings-section-title">{t.settings.firstRun.title}</h2>
        <Segmented
          options={LANGS.map((l) => ({ value: l, label: LANG_SHORT[l] }))}
          value={lang}
          label={LANGUAGE_BLOCK_TITLE}
          disabled={busy}
          onChange={(next) => void chooseLanguage(next)}
        />
      </div>

      {error && <div className="settings-error">{error}</div>}

      <p className="settings-hint">{t.settings.firstRun.knownHint}</p>
      {rows(known)}

      {rest.length > 0 && (
        <>
          <p className="settings-hint">{t.settings.firstRun.restHint}</p>
          {rows(rest)}
        </>
      )}

      <div className="settings-actions">
        <button
          type="button"
          className="button accent"
          disabled={busy}
          onClick={() =>
            void (async () => {
              setBusy(true);
              setError("");
              try {
                for (const g of found) {
                  if (!checked.has(g.title) || g.alreadyAdded) continue;
                  // Путь берётся из того же поиска, что заполнил список.
                  await api.addGameFromScan(g.title);
                }
                await markSeededAndClose();
              } catch (e) {
                setError(errorText(t, e));
              } finally {
                // Разблокировать кнопки при любом исходе: иначе провал
                // на середине списка запирает экран первого запуска
                // без кнопки «назад» и без иного способа выйти.
                setBusy(false);
              }
            })()
          }
        >
          {t.settings.firstRun.addChecked}
        </button>
        <button type="button" className="button" disabled={busy} onClick={skip}>
          {t.settings.firstRun.skip}
        </button>
      </div>
    </div>
  );
}
