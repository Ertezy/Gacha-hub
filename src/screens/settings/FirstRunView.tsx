import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import { useT } from "../../i18n";
import { errorText } from "../../i18n/errors";
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
        {g.alreadyAdded && <span className="found-note">уже добавлена</span>}
      </label>
    ));

  return (
    <div>
      <h2 className="settings-section-title">Что нашлось на компьютере</h2>

      {error && <div className="settings-error">{error}</div>}

      <p className="settings-hint">По этим играм есть коды, баннеры и видео.</p>
      {rows(known)}

      {rest.length > 0 && (
        <>
          <p className="settings-hint">
            Остальное установленное. Эти игры запустятся, но новостей и кодов
            по ним не будет.
          </p>
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
          Добавить отмеченные
        </button>
        <button type="button" className="button" disabled={busy} onClick={skip}>
          Пропустить
        </button>
      </div>
    </div>
  );
}
