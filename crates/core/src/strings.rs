//! The reader-visible strings, by key, from `data/strings.json`. The core
//! lists every key it uses in `KEYS`, and the file must hold exactly those:
//! a missing key would draw as nothing, and a key no screen uses is a string
//! nobody reads. Keys starting with `_` are notes for people and are skipped.

use std::collections::BTreeMap;

/// Every key a screen reads, in the order the screens use them.
pub const KEYS: &[&str] = &[
    "title",
    "menu.local",
    "menu.online",
    "menu.how",
    "menu.settings",
    "menu.notes",
    "menu.gallery",
    "how.goal",
    "how.claws",
    "how.claim",
    "how.fake",
    "how.idle",
    "how.move",
    "how.move_keys",
    "how.look",
    "how.look_keys",
    "how.claw",
    "how.claw_keys",
    "how.jump",
    "how.jump_keys",
    "how.overhead",
    "how.overhead_keys",
    "how.touch",
    "how.touch_keys",
    "touch.look",
    "touch.stick",
    "touch.jump",
    "touch.left",
    "touch.right",
    "touch.pause",
    "touch.overhead",
    "touch.turn",
    "settings.look",
    "menu.back",
    "setup.ai",
    "local.start",
    "pause.heading",
    "menu.resume",
    "menu.quit",
    "end.heading",
    "end.skip",
    "tally.idle",
    "tally.wins",
    "tally.tie",
    "end.rematch",
    "end.title",
    "end.host_left",
    "hud.stalled",
    "idle.call",
    "preview.real",
    "preview.fake",
    "settings.heading",
    "settings.volume",
    "settings.mouse",
    "settings.invert_y",
    "notes.heading",
    "notes.minor",
    "online.name",
    "name.default",
    "online.host",
    "online.code",
    "online.join",
    "online.code_shape",
    "online.host_code",
    "online.invite",
    "online.join_code",
    "online.reply",
    "online.connect",
    "online.next_invite",
    "online.copy",
    "online.copy_link",
    "online.copied",
    "online.ready",
    "online.unready",
    "online.leave",
    "online.joining",
    "lobby.heading",
    "lobby.colour",
    "lobby.state",
    "lobby.ping_head",
    "lobby.you",
    "lobby.host",
    "lobby.ready",
    "lobby.not_ready",
    "lobby.ai_crab",
    "lobby.ping",
    "colour.red",
    "colour.yellow",
    "colour.blue",
    "colour.purple",
    "online.refused.full",
    "online.refused.underway",
    "online.refused.version",
    "online.host_left",
    "online.lost",
    // the transport's errors: web/online.js names them and the core shows them
    "online.no_bundle",
    "online.no_relay",
    "online.no_path",
    "online.signal_failed",
    "online.wrong_code",
    "online.bad_code",
    "online.not_hosting",
    "online.no_offer",
];

/// The strings file, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strings(BTreeMap<String, String>);

/// Why a strings file cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsError {
    NotJson(String),
    Missing(String),
    Empty(String),
    Unused(String),
}

impl StringsError {
    /// The message the page shows.
    pub fn message(&self) -> String {
        match self {
            StringsError::NotJson(detail) => format!("The strings file could not be read: {detail}"),
            StringsError::Missing(key) => format!("The strings file has no \"{key}\"."),
            StringsError::Empty(key) => format!("The strings file has an empty \"{key}\"."),
            StringsError::Unused(key) => {
                format!("The strings file has \"{key}\", which no screen uses.")
            }
        }
    }
}

/// Parses and checks the strings file. A string that is only whitespace
/// counts as empty, because the page would draw nothing.
pub fn parse(json: &str) -> Result<Strings, StringsError> {
    let all: BTreeMap<String, String> =
        serde_json::from_str(json).map_err(|e| StringsError::NotJson(e.to_string()))?;
    let map: BTreeMap<String, String> = all.into_iter().filter(|(k, _)| !k.starts_with('_')).collect();
    for key in KEYS {
        match map.get(*key) {
            None => return Err(StringsError::Missing(key.to_string())),
            Some(text) if text.trim().is_empty() => return Err(StringsError::Empty(key.to_string())),
            Some(_) => {}
        }
    }
    if let Some(key) = map.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(StringsError::Unused(key.clone()));
    }
    Ok(Strings(map))
}

impl Strings {
    /// The string for a key. A key outside `KEYS` comes back as itself, so
    /// a slip shows on screen instead of drawing nothing; `parse` has made
    /// sure every listed key is there.
    pub fn t(&self, key: &str) -> String {
        self.0.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A complete strings file: each key's text is the key in capitals.
    pub(crate) fn complete() -> String {
        let map: BTreeMap<&str, String> = KEYS.iter().map(|k| (*k, k.to_uppercase())).collect();
        serde_json::to_string(&map).unwrap()
    }

    fn without(key: &str) -> String {
        let mut v: serde_json::Value = serde_json::from_str(&complete()).unwrap();
        v.as_object_mut().unwrap().remove(key);
        v.to_string()
    }

    fn with(key: &str, text: &str) -> String {
        let mut v: serde_json::Value = serde_json::from_str(&complete()).unwrap();
        v[key] = serde_json::json!(text);
        v.to_string()
    }

    #[test]
    fn no_key_is_listed_twice() {
        let mut seen = std::collections::BTreeSet::new();
        for k in KEYS {
            assert!(seen.insert(*k), "{k} is listed twice");
        }
    }

    #[test]
    fn a_complete_file_is_accepted_and_read_by_key() {
        let s = parse(&complete()).unwrap();
        assert_eq!(s.t("menu.back"), "MENU.BACK");
    }

    #[test]
    fn a_missing_key_is_refused_by_name() {
        let e = parse(&without("menu.back")).unwrap_err();
        assert_eq!(e.message(), "The strings file has no \"menu.back\".");
    }

    #[test]
    fn an_empty_string_is_refused_by_name() {
        let e = parse(&with("title", "  ")).unwrap_err();
        assert_eq!(e, StringsError::Empty("title".into()));
        assert_eq!(e.message(), "The strings file has an empty \"title\".");
    }

    #[test]
    fn a_key_no_screen_uses_is_refused_but_a_note_is_not() {
        let e = parse(&with("menu.jump", "Jump")).unwrap_err();
        assert_eq!(e, StringsError::Unused("menu.jump".into()));
        assert!(parse(&with("_note", "for people")).is_ok());
    }

    #[test]
    fn text_that_is_not_json_is_refused() {
        assert!(matches!(parse("title: T"), Err(StringsError::NotJson(_))));
    }
}
