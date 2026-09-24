//! Запуск игры. Три способа, но для пользователя их два: через магазин
//! и напрямую.
//!
//! 1. Steam — `steam://rungameid/<appid>`. Клиент Steam сам разбирается
//!    с обновлениями и авторизацией.
//! 2. Epic — `com.epicgames.launcher://apps/<ns>%3A<cid>%3A<app>?action=launch&silent=true`.
//!    Три идентификатора берутся из манифеста Epic, пользователь ничего
//!    не вводит.
//! 3. Свой .exe — `std::process::Command` (CreateProcessW):
//!      * аргументы передаются вектором, оболочка не участвует, поэтому
//!        проблемы экранирования невозможны по построению;
//!      * строка аргументов разбирается по правилам Windows
//!        `CommandLineToArgvW` (см. `split_args`), а НЕ по POSIX: последний
//!        съедает `\` в путях;
//!      * рабочий каталог — папка самого exe: клиенты игр ищут свои данные
//!        относительно себя.

use std::path::PathBuf;
use std::process::Command;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::config::{Game, Launch};
use crate::error::{code, AppError};

/// Разбор строки аргументов по правилам Windows `CommandLineToArgvW`
/// (НЕ по POSIX):
///   * двойные кавычки ограничивают аргумент;
///   * `2n` обратных слэшей перед кавычкой  -> `n` слэшей, кавычка ограничивает;
///   * `2n+1` слэшей перед кавычкой -> `n` слэшей и буквальная `"`.
///
/// Возвращает `None`, если кавычка осталась незакрытой: вызывающий обязан
/// показать это ошибкой, а не молча запустить игру без аргументов.
fn split_args(input: &str) -> Option<Vec<String>> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut have_arg = false;
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                have_arg = true;
            }
            c if c.is_whitespace() => {
                if in_quotes {
                    current.push(c);
                } else if have_arg || !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                    have_arg = false;
                }
            }
            '\\' => {
                let mut backslashes = 1;
                while let Some(&'\\') = chars.peek() {
                    chars.next();
                    backslashes += 1;
                }
                let half = backslashes / 2;
                match chars.peek() {
                    Some(&'"') => {
                        chars.next();
                        current.push_str(&"\\".repeat(half));
                        if backslashes % 2 == 1 {
                            // Нечётное: последний слэш экранирует кавычку —
                            // она становится буквальной внутри аргумента.
                            current.push('"');
                        } else {
                            // Чётное: кавычка ограничивает аргумент.
                            in_quotes = !in_quotes;
                            have_arg = true;
                        }
                    }
                    _ => {
                        // Дальше не кавычка: все слэши буквальные.
                        current.push_str(&"\\".repeat(backslashes));
                    }
                }
            }
            c => current.push(c),
        }
    }

    if in_quotes {
        return None;
    }
    if have_arg || !current.is_empty() {
        args.push(current);
    }
    Some(args)
}

/// Официальная ссылка запуска Epic Games Launcher.
/// Разделитель между тремя идентификаторами — двоеточие в URL-кодировке.
pub fn epic_uri(namespace: &str, catalog_item_id: &str, app_name: &str) -> String {
    format!(
        "com.epicgames.launcher://apps/{namespace}%3A{catalog_item_id}%3A{app_name}?action=launch&silent=true"
    )
}

/// Ошибка — код с подробностью: фразу на языке интерфейса собирает страница
/// (спека этапа 6 §6.2).
pub fn launch(app: &AppHandle, game: &Game) -> Result<(), AppError> {
    match &game.launch {
        Launch::Steam { appid } => {
            let uri = format!("steam://rungameid/{appid}");
            app.opener()
                .open_url(&uri, None::<String>)
                .map_err(|e| AppError::with(code::STEAM_OPEN_FAILED, format!("{uri}: {e}")))
        }
        Launch::Epic {
            namespace,
            catalog_item_id,
            app_name,
        } => {
            let uri = epic_uri(namespace, catalog_item_id, app_name);
            app.opener()
                .open_url(&uri, None::<String>)
                .map_err(|e| AppError::with(code::EPIC_OPEN_FAILED, e.to_string()))
        }
        Launch::Exe => {
            let exe = game
                .exe_path
                .clone()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or_else(|| AppError::new(code::EXE_PATH_MISSING))?;

            if !exe.is_file() {
                return Err(AppError::with(code::FILE_MISSING, exe.display().to_string()));
            }

            let parsed = split_args(&game.args)
                .ok_or_else(|| AppError::with(code::ARGS_UNCLOSED_QUOTE, game.args.clone()))?;

            let cwd = exe
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));

            Command::new(&exe)
                .args(&parsed)
                .current_dir(&cwd)
                .spawn()
                .map(|_| ())
                .map_err(|e| {
                    AppError::with(code::EXE_START_FAILED, format!("{}: {e}", exe.display()))
                })
        }
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn builds_the_official_epic_launch_uri() {
        assert_eq!(
            epic_uri("879b0d87", "7d690c12", "41869934"),
            "com.epicgames.launcher://apps/879b0d87%3A7d690c12%3A41869934?action=launch&silent=true"
        );
    }

    #[test]
    fn windows_argument_rules_keep_backslashes() {
        // Ради этого мы не берём POSIX-разбор: shlex съел бы обратные слэши.
        assert_eq!(
            split_args(r"-config C:\Games\Genshin -dx12").unwrap(),
            vec![r"-config", r"C:\Games\Genshin", "-dx12"]
        );
    }

    #[test]
    fn unterminated_quote_is_an_error_not_a_silent_drop() {
        assert!(split_args(r#"-name "unterminated"#).is_none());
    }
}
