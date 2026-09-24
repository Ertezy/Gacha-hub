//! Тексты, которые Rust показывает сам, — только меню трея (спека этапа 6
//! §6.1). Всё остальное переводит страница: ошибки уходят к ней кодами.

use crate::config::Language;

pub struct TrayTexts {
    pub show: &'static str,
    pub quit: &'static str,
}

pub fn tray(lang: Language) -> TrayTexts {
    match lang {
        Language::En => TrayTexts { show: "Show window", quit: "Quit" },
        Language::Ru => TrayTexts { show: "Показать окно", quit: "Выход" },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_name_both_items() {
        let en = tray(Language::En);
        let ru = tray(Language::Ru);
        assert_eq!((en.show, en.quit), ("Show window", "Quit"));
        assert_eq!((ru.show, ru.quit), ("Показать окно", "Выход"));
    }
}
