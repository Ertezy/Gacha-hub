//! Разбор текстового формата Valve KeyValues (файлы `.vdf` и `.acf`).
//!
//! Формат вложенный, но нам нужны только листья вида `"ключ" "значение"`,
//! поэтому разбираем в плоский *список* пар: в `libraryfolders.vdf` ключ
//! `path` встречается по разу на каждую библиотеку, и словарь их потерял бы.

/// Все пары `"ключ" "значение"` в порядке появления.
/// Строки-заголовки блоков (одна строка — одна кавычка) и скобки пропускаются.
pub fn pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let segments = quoted_segments(line);
        if segments.len() == 2 {
            let mut it = segments.into_iter();
            let key = it.next().unwrap();
            let value = it.next().unwrap();
            out.push((key, value));
        }
    }
    out
}

/// Первое значение для ключа.
pub fn first<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Все значения для ключа, в порядке появления.
pub fn all<'a>(pairs: &'a [(String, String)], key: &str) -> Vec<&'a str> {
    pairs
        .iter()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .collect()
}

/// Куски строки, заключённые в двойные кавычки. `\\` внутри кавычек — это
/// экранированный обратный слэш: в путях Valve пишет `C:\\Program Files`.
fn quoted_segments(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut inside = false;
    let mut escaped = false;

    for c in line.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' if inside => escaped = true,
            '"' => {
                if inside {
                    out.push(std::mem::take(&mut current));
                }
                inside = !inside;
            }
            _ if inside => current.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARY_FOLDERS: &str = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}"#;

    const APP_MANIFEST: &str = r#""AppState"
{
	"appid"		"3513350"
	"name"		"Wuthering Waves"
	"installdir"		"Wuthering Waves"
}"#;

    #[test]
    fn collects_every_repeated_key() {
        let p = pairs(LIBRARY_FOLDERS);
        assert_eq!(
            all(&p, "path"),
            vec![r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]
        );
    }

    #[test]
    fn unescapes_double_backslashes() {
        let p = pairs(LIBRARY_FOLDERS);
        assert_eq!(first(&p, "path"), Some(r"C:\Program Files (x86)\Steam"));
    }

    #[test]
    fn reads_app_manifest_fields() {
        let p = pairs(APP_MANIFEST);
        assert_eq!(first(&p, "appid"), Some("3513350"));
        assert_eq!(first(&p, "name"), Some("Wuthering Waves"));
        assert_eq!(first(&p, "installdir"), Some("Wuthering Waves"));
    }

    #[test]
    fn ignores_block_headers_and_braces() {
        let p = pairs(LIBRARY_FOLDERS);
        assert_eq!(first(&p, "libraryfolders"), None);
        assert_eq!(first(&p, "0"), None);
    }

    #[test]
    fn keeps_empty_values() {
        let p = pairs(LIBRARY_FOLDERS);
        assert_eq!(first(&p, "label"), Some(""));
    }
}
