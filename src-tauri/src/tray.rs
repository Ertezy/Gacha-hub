//! Значок в трее и перехват закрытия окна.
//!
//! Меню намеренно состоит из двух пунктов. Запуск игр отсюда — отложенная
//! возможность (§12 общей спеки, «удобство поверх работающего лаунчера»), и
//! соблазн добавить её велик именно потому, что меню уже создано. Не добавлять.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Показать окно", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    // Мягкий отказ вместо паники: без значка не собрать окно, но приложение
    // не обязано падать — оно просто останется без трея (см. lib.rs).
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("значок окна не задан в tauri.conf.json".into()))?;

    // with_id — конструктор, а не звено в цепочке: без своего id значок нельзя
    // будет найти через tray_by_id при перехвате закрытия окна.
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => restore(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Возврат окна — обычный левый клик по значку (§8 общей спеки).
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                restore(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn restore(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
