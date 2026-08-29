//! Launch Engine: the three ways to start a game on Windows.
//!
//! 1. Steam — the `steam://rungameid/<appid>` URI scheme.
//!    The Steam client registers a `steam` protocol handler in the Windows
//!    registry (HKCU\Software\Classes\steam — verified on the dev machine).
//!    The URI is handed to the OS through the Tauri opener plugin; Steam
//!    picks it up and launches the game by appid, handling
//!    updates/auth itself.
//!
//! 2. Epic — the `egstore://launch/<product-id>` deep link.
//!    The Epic Games Launcher registers the `egstore` protocol handler
//!    (HKCU\Software\Classes\egstore). Epic has not published an official
//!    launch URI scheme, so the `launch/<product-id>` form is
//!    community-documented; the product id comes from user config. If it
//!    doesn't work, the fallback is a direct .exe launch.
//!
//! 3. Exe — direct process creation via `std::process::Command`
//!    (CreateProcessW on Windows). Key Windows-specific points:
//!      * arguments are passed as a *vector* of strings, never one shell
//!        string — no shell is involved, so quoting/injection problems
//!        are impossible by construction;
//!      * the user's flag string ("-dx12 -high") is tokenized with
//!        `split_args` below, which follows the Windows
//!        CommandLineToArgvW rules — NOT POSIX `shlex`: shlex treats `\`
//!        as an escape character and would mangle Windows paths like
//!        `C:\Games\Genshin`;
//!      * `current_dir` is set to the exe's own directory — game clients
//!        (HoYoverse/Kuro/Hypergryph) resolve their `Client/`, `Engine/`
//!        folders relative to the executable.

use std::path::PathBuf;
use std::process::Command;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::catalog::GAMES;
use crate::detect;

pub enum LaunchTarget {
    Steam(u32),
    Epic(String),
    /// (path to the .exe, raw args string)
    Exe(PathBuf, String),
}

/// Tokenize a raw args string using the Windows `CommandLineToArgvW`
/// rules (NOT POSIX shlex):
///   * double quotes delimit an argument;
///   * `2n` backslashes before a quote   -> `n` backslashes, the quote delimits;
///   * `2n+1` backslashes before a quote -> `n` backslashes + a literal `"`.
///
/// Returns `None` if a double quote is left unterminated; callers must
/// surface that as an error instead of silently launching with no args.
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
                            // Odd: the last backslash escapes the quote —
                            // it becomes a literal '"' inside the argument.
                            current.push('"');
                        } else {
                            // Even: the quote is a delimiter.
                            in_quotes = !in_quotes;
                            have_arg = true;
                        }
                    }
                    _ => {
                        // Not followed by a quote: all backslashes are literal.
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

/// Launch the game. URIs are opened through the Tauri opener plugin (the
/// same plugin behind the frontend `openUrl` IPC command), so the app has
/// a single "open URI" code path instead of a second `open` crate.
pub fn launch(app: &AppHandle, game_id: &str, target: LaunchTarget) -> Result<(), String> {
    match target {
        LaunchTarget::Steam(appid) => {
            let uri = format!("steam://rungameid/{appid}");
            app.opener()
                .open_url(&uri, None::<String>)
                .map_err(|e| {
                    format!(
                        "failed to open {uri}: {e}. Check that Steam is installed (and start it once)."
                    )
                })
        }
        LaunchTarget::Epic(product_id) => {
            let uri = format!("egstore://launch/{product_id}");
            app.opener()
                .open_url(&uri, None::<String>)
                .map_err(|e| {
                    format!(
                        "failed to open {uri}: {e}. Either the Epic Games Store is not installed \
                         or the product id is wrong. You can switch the launch method to \
                         \"direct .exe\" in the settings."
                    )
                })
        }
        LaunchTarget::Exe(exe, args) => {
            if !exe.is_file() {
                return Err(format!(
                    "file not found: {}. Check the path in the settings (or press \"Scan\").",
                    exe.display()
                ));
            }
            // Sanity check: the path points at another game's install.
            // The folder name is the primary signal (Epic HoYoverse installs
            // share a generic launcher_epic.exe between games).
            if let Some(parent) = exe.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
                if let Some(other) = detect::game_for_folder(parent) {
                    if other != game_id {
                        let other_name = GAMES
                            .iter()
                            .find(|g| g.id == other)
                            .map(|g| g.name)
                            .unwrap_or(other);
                        return Err(format!(
                            "looks like a different game: the folder \u{ab}{parent}\u{bb} belongs to {other_name}. \
                             Check the launch path in the settings."
                        ));
                    }
                }
            }
            // Strong check on the exe name (GenshinImpact.exe is unambiguously Genshin).
            if let Some(stem) = exe.file_stem().and_then(|s| s.to_str()) {
                if let Some(other) = detect::strong_game_for_exe(stem) {
                    if other != game_id {
                        let other_name = GAMES
                            .iter()
                            .find(|g| g.id == other)
                            .map(|g| g.name)
                            .unwrap_or(other);
                        return Err(format!(
                            "looks like a different game: the exe name matches {other_name}. \
                             Check the launch path in the settings."
                        ));
                    }
                }
            }
            // Windows-rules tokenization; an unterminated quote is a
            // visible error, not a silent no-args launch.
            let parsed: Vec<String> = split_args(&args).ok_or_else(|| {
                format!(
                    "could not parse launch args \"{args}\": unterminated double quote. \
                     Fix the args in the settings."
                )
            })?;
            // Working directory = the exe's folder: game clients load their
            // game data relative to the executable location.
            let cwd = exe
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            Command::new(&exe)
                .args(&parsed)
                .current_dir(&cwd)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("failed to launch {}: {e}", exe.display()))
        }
    }
}
