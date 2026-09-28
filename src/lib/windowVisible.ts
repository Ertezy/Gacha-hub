import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Видно ли окно. Уход в трей сообщает Rust (`tray::hide_main_window`), а
 * сворачивание страница видит сама через `visibilitychange` (спека §6.3).
 */
export function useWindowVisible(): boolean {
  const [shown, setShown] = useState(true);
  const [pageVisible, setPageVisible] = useState(() => document.visibilityState === "visible");

  useEffect(() => {
    let active = true;
    let eventArrived = false;
    // Подписка регистрируется первой: событие может прийти раньше, чем
    // разрешится чтение ниже, и тогда результат чтения — устаревший, его
    // нельзя применять поверх события (правка финальной ревизии этапа 7).
    const unlisten = listen<boolean>("window-visibility", (e) => {
      eventArrived = true;
      setShown(e.payload);
    });
    // Автозапуск создаёт окно сразу спрятанным (спека этапа 7 §4.2), а
    // событие о видимости при этом не приходит — без начального чтения
    // карусель и видео фона крутились бы вхолостую в невидимом окне, пока
    // его не открыли вручную ни разу.
    void getCurrentWindow()
      .isVisible()
      .then((visible) => {
        if (active && !eventArrived) setShown(visible);
      })
      .catch(() => {});
    const onVisibility = () => setPageVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  return shown && pageVisible;
}
