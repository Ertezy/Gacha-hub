import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import { dataStatus } from "../../lib/panel";
import { useT } from "../../i18n";
import { errorText } from "../../i18n/errors";
import type { About, Behaviour, HubData } from "../../types";
import Switch from "./Switch";

interface Props {
  tab: "behaviour" | "data" | "about";
}

function Behaviours() {
  const [b, setB] = useState<Behaviour | null>(null);
  const [error, setError] = useState("");
  const t = useT();

  useEffect(() => {
    void api.getBehaviour().then(setB);
  }, []);

  if (!b) return null;

  // Галочка переключается сразу, для отзывчивости. Но если запись в конфиг
  // не прошла (папка только на чтение, файл занят), об этом нужно сказать и
  // вернуть галочку назад — иначе экран покажет состояние, которого на самом
  // деле нет, и человек, например, снимет «крестик прячет окно», закроет его
  // и получит сворачивание в трей вместо выхода.
  const save = (next: Behaviour) => {
    const prev = b;
    setB(next);
    setError("");
    void api.setBehaviour(next).catch((e) => {
      setError(errorText(t, e));
      setB(prev);
    });
  };

  return (
    <div>
      <h2 className="settings-section-title">Поведение</h2>
      {error && <div className="settings-error">{error}</div>}
      <div className="settings-card">
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Крестик прячет окно в трей</span>
            <span className="settings-row-hint">
              Приложение продолжает работать, вернуть окно можно из значка в трее.
            </span>
          </div>
          <div className="settings-row-control">
            <Switch
              checked={b.closeToTray}
              label="Крестик прячет окно в трей"
              onChange={(next) => save({ ...b, closeToTray: next })}
            />
          </div>
        </div>
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Уходить в трей после запуска игры</span>
            <span className="settings-row-hint">Окно не мешает игре.</span>
          </div>
          <div className="settings-row-control">
            <Switch
              checked={b.trayOnLaunch}
              label="Уходить в трей после запуска игры"
              onChange={(next) => save({ ...b, trayOnLaunch: next })}
            />
          </div>
        </div>
      </div>
      <p className="settings-note">Автозапуск вместе с Windows появится в следующем обновлении.</p>
    </div>
  );
}

function Data() {
  const [url, setUrl] = useState("");
  // Последнее сохранённое значение — по нему решаем, можно ли жать
  // «Сохранить». Без него нетронутое (и потому пустое до загрузки) поле
  // могло бы стереть уже настроенный адрес одним нажатием вслепую.
  const [savedUrl, setSavedUrl] = useState("");
  const [hub, setHub] = useState<HubData | null>(null);
  const [size, setSize] = useState(0);
  const [error, setError] = useState("");
  const t = useT();

  const reload = () => {
    void api.getHub().then(setHub).catch(() => setHub(null));
    void api.imageCacheSize().then(setSize);
  };

  useEffect(() => {
    reload();
    void api.getConfig().then((cfg) => {
      setUrl(cfg.hubUrl ?? "");
      setSavedUrl(cfg.hubUrl ?? "");
    });
  }, []);

  const nowSec = Math.floor(Date.now() / 1000);

  return (
    <div>
      <h2 className="settings-section-title">Данные</h2>
      {error && <div className="settings-error">{error}</div>}

      <div className="settings-card">
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">{dataStatus(hub, nowSec)}</span>
          </div>
          <div className="settings-row-control">
            <button type="button" className="button" onClick={reload}>
              Обновить сейчас
            </button>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Кеш картинок</span>
            <span className="settings-row-hint">
              Занимает {(size / 1024 / 1024).toFixed(1)} МБ: иконки, фоны и картинки панели.
            </span>
          </div>
          <div className="settings-row-control">
            <button
              type="button"
              className="button"
              onClick={() =>
                void api
                  .clearImageCache()
                  .then(reload)
                  .catch((e) => setError(errorText(t, e)))
              }
            >
              Очистить кеш картинок
            </button>
          </div>
        </div>

        {/* Поле адреса свёрнуто (спека §7.5): человеку со стороны вписывать туда
            нечего, а открытое поле на виду читается как обязательное. */}
        <details className="settings-extra">
          <summary>Дополнительно</summary>
          <div className="settings-row">
            <div className="settings-row-text">
              <span className="settings-row-title">Адрес источника данных</span>
              <span className="settings-row-hint">
                Нужен только для своего источника. Обычно поле пустое.
              </span>
            </div>
            <div className="settings-row-control">
              <input
                className="input editor-input"
                aria-label="Адрес источника данных"
                value={url}
                placeholder="https://…"
                onChange={(e) => setUrl(e.target.value)}
              />
              <button
                type="button"
                className="button accent"
                disabled={url === savedUrl}
                onClick={() =>
                  void api
                    .setHubUrl(url)
                    .then(() => {
                      setError("");
                      setSavedUrl(url);
                      reload();
                    })
                    .catch((e) => setError(errorText(t, e)))
                }
              >
                Сохранить
              </button>
            </div>
          </div>
        </details>
      </div>
    </div>
  );
}

function AboutSection() {
  const [about, setAbout] = useState<About | null>(null);
  const [error, setError] = useState("");
  const t = useT();

  useEffect(() => {
    void api.getAbout().then(setAbout);
  }, []);

  return (
    <div>
      <h2 className="settings-section-title">О программе</h2>
      {error && <div className="settings-error">{error}</div>}
      <div className="settings-card">
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Версия {about?.version ?? "…"}</span>
          </div>
          <div className="settings-row-control">
            <button
              type="button"
              className="button"
              onClick={() => void api.openSafeUrl("https://github.com/Ertezy/Gacha-hub")}
            >
              Репозиторий
            </button>
          </div>
        </div>
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-title">Журнал</span>
            <span className="settings-row-hint">
              Лежит в {about?.logPath ?? "…"}. Если что-то не работает, приложите его к
              сообщению о проблеме.
            </span>
          </div>
          <div className="settings-row-control">
            <button
              type="button"
              className="button"
              onClick={() => void api.openLogFolder().catch((e) => setError(errorText(t, e)))}
            >
              Показать журнал
            </button>
          </div>
        </div>
      </div>
      <p className="settings-note">
        Приложение не связано с разработчиками игр. Названия, изображения и другие
        материалы принадлежат правообладателям.
      </p>
    </div>
  );
}

export default function SmallSections({ tab }: Props) {
  if (tab === "behaviour") return <Behaviours />;
  if (tab === "data") return <Data />;
  return <AboutSection />;
}
