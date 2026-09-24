//! Значок в трее и перехват закрытия окна.
//!
//! Меню намеренно состоит из двух пунктов. Запуск игр отсюда — отложенная
//! возможность (§12 общей спеки, «удобство поверх работающего лаунчера»), и
//! соблазн добавить её велик именно потому, что меню уже создано. Не добавлять.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::config::{self, Language};
use crate::i18n;

/// Событие для страницы: окно показано (`true`) или спрятано (`false`).
pub const VISIBILITY_EVENT: &str = "window-visibility";

/// Прячет главное окно. Единственное место в приложении, где это делается.
///
/// Раньше окно пряталось в двух местах, у крестика и после запуска игры, и
/// проверка «есть ли значок в трее» жила у каждого отдельно — во втором её
/// однажды забыли. Теперь и проверка, и сообщение странице (видео фона встаёт на
/// паузу, спека этапа 5 §6.3) живут здесь, и забыть их в новом месте нельзя:
/// другого способа спрятать окно в коде нет.
///
/// `false` — окно не спрятано: без значка в трее его стало бы нечем вернуть.
pub fn hide_main_window(app: &AppHandle) -> bool {
    if app.tray_by_id("main").is_none() {
        return false;
    }
    let Some(window) = app.get_webview_window("main") else {
        return false;
    };
    if window.hide().is_err() {
        return false;
    }
    let _ = app.emit(VISIBILITY_EVENT, false);
    true
}

/// Пункты меню трея — чтобы переименовать их при смене языка без перезапуска.
pub struct TrayMenu {
    show: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

/// Переименовывает пункты меню на новый язык. Без трея — ничего не делает.
pub fn apply_language(app: &AppHandle, lang: Language) {
    let Some(menu) = app.try_state::<TrayMenu>() else {
        return;
    };
    let texts = i18n::tray(lang);
    let _ = menu.show.set_text(texts.show);
    let _ = menu.quit.set_text(texts.quit);
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let texts = i18n::tray(config::load(app).language);
    let show = MenuItem::with_id(app, "show", texts.show, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    app.manage(TrayMenu { show: show.clone(), quit: quit.clone() });

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
        let _ = app.emit(VISIBILITY_EVENT, true);
    }
}
