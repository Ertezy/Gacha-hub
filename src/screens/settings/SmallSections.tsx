import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { About, Behaviour, HubData } from "../../types";

interface Props {
  tab: "behaviour" | "data" | "about";
}

function Behaviours() {
  const [b, setB] = useState<Behaviour | null>(null);

  useEffect(() => {
    void api.getBehaviour().then(setB);
  }, []);

  if (!b) return null;

  const save = (next: Behaviour) => {
    setB(next);
    void api.setBehaviour(next);
  };

  return (
    <div>
      <h2 className="settings-section-title">Поведение</h2>
      <label className="field">
        <span>Крестик прячет окно в трей</span>
        <input
          type="checkbox"
          checked={b.closeToTray}
          onChange={(e) => save({ ...b, closeToTray: e.target.checked })}
        />
      </label>
      <label className="field">
        <span>Уходить в трей после запуска игры</span>
        <input
          type="checkbox"
          checked={b.trayOnLaunch}
          onChange={(e) => save({ ...b, trayOnLaunch: e.target.checked })}
        />
      </label>
      <p className="settings-hint">
        Автозапуск вместе с Windows появится в следующем обновлении.
      </p>
    </div>
  );
}

function Data() {
  const [url, setUrl] = useState("");
  const [hub, setHub] = useState<HubData | null>(null);
  const [size, setSize] = useState(0);
  const [error, setError] = useState("");

  const reload = () => {
    void api.getHub().then(setHub).catch(() => setHub(null));
    void api.imageCacheSize().then(setSize);
  };

  useEffect(reload, []);

  const source =
    { remote: "из сети", override: "из локальной подмены", cache: "из кеша", bundled: "из комплекта" }[
      hub?._source ?? ""
    ] ?? "неизвестно";

  return (
    <div>
      <h2 className="settings-section-title">Данные</h2>

      <label className="field">
        <span>Адрес источника</span>
        <input
          value={url}
          placeholder="https://…"
          onChange={(e) => setUrl(e.target.value)}
        />
        <button
          type="button"
          className="accent"
          onClick={() =>
            void api
              .setHubUrl(url)
              .then(() => {
                setError("");
                reload();
              })
              .catch((e) => setError(String(e)))
          }
        >
          Сохранить
        </button>
      </label>
      {error && <div className="settings-error">{error}</div>}

      <p className="settings-hint">
        Данные приехали {source}. Кеш картинок занимает{" "}
        {(size / 1024 / 1024).toFixed(1)} МБ.
      </p>

      <div className="field-row">
        <button type="button" onClick={reload}>
          Обновить сейчас
        </button>
        <button
          type="button"
          onClick={() => void api.clearImageCache().then(reload)}
        >
          Очистить кеш картинок
        </button>
      </div>
    </div>
  );
}

function AboutSection() {
  const [about, setAbout] = useState<About | null>(null);

  useEffect(() => {
    void api.getAbout().then(setAbout);
  }, []);

  return (
    <div>
      <h2 className="settings-section-title">О программе</h2>
      <p className="settings-hint">Версия {about?.version ?? "…"}</p>
      <div className="field-row">
        <button
          type="button"
          onClick={() => void api.openSafeUrl("https://github.com/Ertezy/Gacha-hub")}
        >
          Репозиторий
        </button>
        <button
          type="button"
          onClick={() => void api.openLogFolder()}
        >
          Показать журнал
        </button>
      </div>
      <p className="settings-hint">
        Журнал лежит в {about?.logPath ?? "…"}. Если что-то не работает,
        приложите его к сообщению о проблеме.
      </p>
      <p className="settings-hint">
        Приложение не связано с разработчиками игр. Названия, изображения и
        другие материалы принадлежат правообладателям.
      </p>
    </div>
  );
}

export default function SmallSections({ tab }: Props) {
  if (tab === "behaviour") return <Behaviours />;
  if (tab === "data") return <Data />;
  return <AboutSection />;
}
