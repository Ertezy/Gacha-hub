//! Схема файла хаба, версия 2.
//!
//! Все моменты времени — unix-секунды числом, а не строкой с датой. Сроки
//! кодов и баннеров привязаны к серверам игр в разных поясах, а показываются
//! человеку в местном времени; строка «до 30 сентября» соврала бы на часы.
//!
//! Каждое поле — недоверенный ввод (спека §8.1). Разбор обязан пережить
//! неполный, лишний и неожиданный набор полей: файл приходит извне, и падение
//! на нём означало бы пустую панель у человека.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

/// Разбирает массив поэлементно, пропуская то, что не разобралось.
///
/// Без этого одна битая запись роняет разбор всего файла, и человек получает
/// пустую панель вместо одной пропавшей карточки. Файл приходит извне (§8.1),
/// и устойчивость тут важнее строгости: пропущенную карточку видно, а пустую
/// панель невозможно отличить от «сегодня ничего не раздают».
fn lenient_vec<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    // Разбираем в `Value`, а не сразу в `Vec<Value>`: при `"codes": null`
    // или `"codes": 42` разбор вектора вернул бы ошибку наружу и уронил
    // разбор ВСЕГО файла — ровно та пустая панель, которую эта функция и
    // должна предотвращать. `#[serde(default)]` тут не помогает: он
    // срабатывает на отсутствующее поле, а не на присутствующее с чужим типом.
    let raw = match serde_json::Value::deserialize(d)? {
        serde_json::Value::Array(items) => items,
        _ => return Ok(Vec::new()),
    };
    Ok(raw
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubData {
    #[serde(default)]
    pub version: u32,
    /// Когда сборщик собрал файл. Основа правила протухания (спека §6).
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub games: Vec<HubGame>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub codes: Vec<Code>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub banners: Vec<Banner>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub videos: Vec<Video>,
    /// Откуда приехали данные: "remote" | "override" | "cache" | "bundled".
    /// Проставляется при загрузке, в файле не лежит.
    #[serde(rename = "_source", default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubGame {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    /// Шаблон адреса погашения с подстановкой `{code}`. Отсутствует у игр
    /// без веб-погашения — у Вувы и Эндфилда его нет, коды вводятся в игре.
    #[serde(default)]
    pub redeem_url: Option<String>,
    /// `match` — ключевое слово Rust, поэтому поле названо иначе.
    #[serde(rename = "match", default)]
    pub matching: Match,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    #[serde(default)]
    pub steam_app_ids: Vec<u32>,
    #[serde(default)]
    pub epic_app_names: Vec<String>,
    #[serde(default)]
    pub folder_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Code {
    pub game_id: String,
    pub code: String,
    #[serde(default)]
    pub rewards: String,
    /// `None` — срок неизвестен или код бессрочный. Показывается «бессрочный».
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// "all" или регион сервера. Носим, но не фильтруем до этапа 3.
    #[serde(default = "region_all")]
    pub region: String,
    #[serde(default)]
    pub source: Option<String>,
}

fn region_all() -> String {
    "all".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Banner {
    pub game_id: String,
    #[serde(default)]
    pub title: String,
    /// Имена персонажей на баннере.
    #[serde(default)]
    pub featured: Vec<String>,
    #[serde(default)]
    pub rarity: Option<u8>,
    /// Уже уменьшенный адрес — уменьшает сборщик, не приложение.
    /// `None` у ХСР: вики называет файл, но не хранит его (спека §4.2).
    #[serde(default)]
    pub image: Option<String>,
    pub starts_at: i64,
    pub ends_at: i64,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub game_id: String,
    #[serde(default)]
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub thumb: Option<String>,
    pub published_at: i64,
    /// Секунды. Лента RSS ютуба длительности не даёт, поэтому обычно `None`.
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub premiere: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "version": 2,
      "updatedAt": 1788091200,
      "games": [{
        "id": "zzz",
        "title": "Zenless Zone Zero",
        "icon": "https://example.test/zzz.png",
        "redeemUrl": "https://zenless.hoyoverse.com/redemption?code={code}",
        "match": { "steamAppIds": [], "epicAppNames": ["nap"], "folderNames": ["ZenlessZoneZero"] }
      }],
      "codes": [{
        "gameId": "zzz", "code": "FLINTWORKS", "rewards": "300 полихромов",
        "expiresAt": 1788105599, "region": "all", "source": "https://example.test"
      }],
      "banners": [{
        "gameId": "genshin", "title": "Ледяная тень лебедя",
        "featured": ["Одетт"], "rarity": 5,
        "image": "https://example.test/art.png",
        "startsAt": 1786489200, "endsAt": 1788256740,
        "url": "https://example.test/news"
      }],
      "videos": [{
        "gameId": "genshin", "title": "Local Legend",
        "url": "https://www.youtube.com/watch?v=abc",
        "thumb": "https://i3.ytimg.com/vi/abc/hqdefault.jpg",
        "publishedAt": 1787738439, "duration": null, "premiere": false
      }]
    }"#;

    #[test]
    fn parses_a_complete_file() {
        let d: HubData = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(d.version, 2);
        assert_eq!(d.updated_at, 1788091200);
        assert_eq!(d.games[0].id, "zzz");
        assert_eq!(d.games[0].matching.epic_app_names, vec!["nap"]);
        assert_eq!(d.codes[0].expires_at, Some(1788105599));
        assert_eq!(d.banners[0].starts_at, 1786489200);
        assert_eq!(d.videos[0].duration, None);
    }

    #[test]
    fn a_code_without_an_expiry_is_open_ended() {
        let json = r#"{"gameId":"zzz","code":"X","rewards":"","expiresAt":null,"region":"all"}"#;
        let c: Code = serde_json::from_str(json).unwrap();
        assert_eq!(c.expires_at, None);
    }

    #[test]
    fn missing_optional_fields_do_not_fail_the_parse() {
        // Сборщик может не заполнить необязательное. Разбор обязан выжить:
        // файл приходит извне, и падение на нём означало бы пустую панель.
        let json = r#"{"gameId":"gi","code":"Y","rewards":"","region":"all"}"#;
        let c: Code = serde_json::from_str(json).unwrap();
        assert_eq!(c.expires_at, None);
        assert_eq!(c.source, None);
    }

    #[test]
    fn an_empty_object_parses_into_empty_lists() {
        let d: HubData = serde_json::from_str("{}").unwrap();
        assert_eq!(d.version, 0);
        assert!(d.games.is_empty());
        assert!(d.codes.is_empty());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        // Сборщик впереди приложения: он добавит поле раньше, чем мы его
        // научимся читать. Старая версия обязана пережить новый файл.
        let d: HubData = serde_json::from_str(r#"{"version":2,"guides":[{"x":1}]}"#).unwrap();
        assert_eq!(d.version, 2);
    }

    #[test]
    fn one_broken_entry_does_not_take_the_whole_file_down() {
        // Самое важное свойство разбора. Без него одна битая запись у
        // сборщика оставляет каждого пользователя с пустой панелью вместо
        // одной пропавшей карточки.
        let json = r#"{
          "version": 2,
          "codes": [
            {"gameId":"zzz","code":"GOOD","rewards":"","region":"all"},
            {"code":"НЕТ ИГРЫ"},
            {"gameId":"gi","code":"ALSOGOOD","rewards":"","region":"all"}
          ]
        }"#;
        let d: HubData = serde_json::from_str(json).unwrap();
        assert_eq!(d.codes.len(), 2, "уцелеть должны обе целые записи");
        assert_eq!(d.codes[0].code, "GOOD");
        assert_eq!(d.codes[1].code, "ALSOGOOD");
    }

    #[test]
    fn entries_of_the_wrong_shape_are_skipped_not_fatal() {
        let d: HubData = serde_json::from_str(r#"{"version":2,"codes":[42,"строка",null]}"#).unwrap();
        assert!(d.codes.is_empty());
    }

    #[test]
    fn a_list_that_is_not_a_list_yields_nothing_rather_than_failing() {
        for json in [
            r#"{"version":2,"codes":null}"#,
            r#"{"version":2,"codes":42}"#,
            r#"{"version":2,"codes":{"a":1}}"#,
        ] {
            let d: HubData = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("разбор упал на {json}: {e}"));
            assert_eq!(d.version, 2);
            assert!(d.codes.is_empty());
        }
    }

    #[test]
    fn region_falls_back_to_all_when_absent() {
        // Прежний тест «отсутствующих полей» проставлял region явно,
        // поэтому ветка умолчания не исполнялась ни разу.
        let json = r#"{"gameId":"gi","code":"Z","rewards":""}"#;
        let c: Code = serde_json::from_str(json).unwrap();
        assert_eq!(c.region, "all");
    }
}
