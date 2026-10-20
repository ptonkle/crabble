//! The door into the core. Every function takes strings and returns a JSON
//! string: `{"ok": ...}` on success or `{"error": "..."}` with a message the
//! page can show as it is. The shim forwards these calls and nothing else.
//!
//! The core describes the screen as a view model (`Ui`): the screen's name,
//! its panels and their actions, and the glossary's definitions. The page
//! builds the DOM from that description without knowing what a panel means.
//! Settings, screens and flags all come back through one function, `act`,
//! with a dotted name such as `screen.show`, so a new option needs a new
//! `match` arm here and no new shim function. (After Peter's Mandible.)

use serde::{Deserialize, Serialize};

use crate::{glossary, page, sound};

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// What the page hands back on the next call. The page stores it and does
/// not read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub screen: String,
    pub strings: page::Strings,
}

/// One button: the action name and value the page sends back through `act`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Action {
    pub name: String,
    pub value: String,
    pub label: String,
    pub enabled: bool,
}

/// One block of the screen. Empty fields are left out of the JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Panel {
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub heading: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

/// A term and its definition, for the page's hover hints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hint {
    pub term: String,
    pub definition: String,
}

/// Everything the page draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ui {
    pub screen: String,
    pub title: String,
    pub panels: Vec<Panel>,
    pub hints: Vec<Hint>,
    pub state: State,
}

fn ui(state: State, hints: Vec<Hint>) -> Ui {
    let panels = vec![Panel {
        id: "intro".into(),
        heading: String::new(),
        text: state.strings.intro.clone(),
        // the Gallery's one entry on the Title (`crate::gallery`)
        actions: vec![Action {
            name: "gallery.open".into(),
            value: String::new(),
            label: state.strings.gallery.clone(),
            enabled: true,
        }],
    }];
    Ui {
        screen: state.screen.clone(),
        title: state.strings.title.clone(),
        panels,
        hints,
        state,
    }
}

fn hints_from(glossary_md: &str) -> Result<Vec<Hint>, String> {
    let entries = glossary::parse(glossary_md).map_err(|e| e.message())?;
    Ok(entries
        .into_iter()
        .map(|e| Hint {
            term: e.term,
            definition: e.definition,
        })
        .collect())
}

/// Checks the strings file and the glossary and returns the first screen.
pub fn start(strings_json: &str, glossary_md: &str) -> String {
    let run = || -> Result<Ui, String> {
        let strings = page::parse_strings(strings_json).map_err(|e| e.message())?;
        let hints = hints_from(glossary_md)?;
        let state = State {
            screen: "start".into(),
            strings,
        };
        Ok(ui(state, hints))
    };
    match run() {
        Ok(u) => ok(&u),
        Err(e) => error(e),
    }
}

/// Applies one named action to the state the page holds. The template knows
/// one action, `screen.show`, as the pattern for a project's own.
pub fn act(state_json: &str, glossary_md: &str, name: &str, value: &str) -> String {
    let run = || -> Result<Ui, String> {
        let mut state: State =
            serde_json::from_str(state_json).map_err(|e| format!("The state could not be read: {e}"))?;
        let hints = hints_from(glossary_md)?;
        match name {
            "screen.show" => match value {
                "start" => state.screen = value.to_string(),
                other => return Err(format!("There is no screen \"{other}\".")),
            },
            // the Gallery keeps its own state (`crate::gallery::Gallery`);
            // this only names its screen for the page
            "gallery.open" => state.screen = "gallery".into(),
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(ui(state, hints))
    };
    match run() {
        Ok(u) => ok(&u),
        Err(e) => error(e),
    }
}

/// The sounds file checked: the sounds it holds, which of them loop, and
/// the volume (percent) to play them at until a setting gives another.
pub fn sounds(sounds_json: &str) -> String {
    match sound::Bank::parse(sounds_json) {
        Ok(b) => ok(&serde_json::json!({
            "sounds": b.cues.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            "loops": b.cues.iter().filter(|c| c.loop_ms.is_some()).map(|c| c.id.as_str()).collect::<Vec<_>>(),
            "volume": sound::DEFAULT_VOLUME,
        })),
        Err(e) => error(e),
    }
}

/// The sound `key` as a complete WAV file at `volume` percent, its pitch
/// moved by `seed` (a sound heard again and again gets a new seed each
/// time). Empty when the file or the key is refused; `sounds` says why.
pub fn sound(sounds_json: &str, key: &str, volume: u32, seed: u32) -> Vec<u8> {
    sound::Bank::parse(sounds_json)
        .and_then(|b| b.wav(key, volume, seed as u64))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRINGS: &str = r#"{"title": "T", "intro": "I", "gallery": "G"}"#;
    const GLOSSARY: &str = "### Spout\n\nWhere slush comes out.\n";

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn start_returns_the_start_screen_with_its_panel_and_hints() {
        let v = json(&start(STRINGS, GLOSSARY));
        assert_eq!(v["ok"]["screen"], "start");
        assert_eq!(v["ok"]["title"], "T");
        assert_eq!(v["ok"]["panels"][0]["id"], "intro");
        assert_eq!(v["ok"]["panels"][0]["text"], "I");
        assert_eq!(v["ok"]["hints"][0]["term"], "Spout");
        assert_eq!(v["ok"]["hints"][0]["definition"], "Where slush comes out.");
    }

    #[test]
    fn start_wraps_a_bad_strings_file_in_error() {
        assert_eq!(
            start(r#"{"title": "", "intro": "I", "gallery": "G"}"#, ""),
            r#"{"error":"The strings file has an empty \"title\"."}"#
        );
    }

    #[test]
    fn start_wraps_a_bad_glossary_in_error() {
        let v = json(&start(STRINGS, "### Spout\n"));
        assert_eq!(v["error"], "The glossary entry \"Spout\" has no definition.");
    }

    #[test]
    fn act_goes_through_the_one_door() {
        let first = json(&start(STRINGS, ""));
        let state = first["ok"]["state"].to_string();
        let v = json(&act(&state, "", "screen.show", "start"));
        assert_eq!(v["ok"]["screen"], "start");
    }

    const SOUNDS: &str = r#"{"sample_rate": 8000, "master": 10000, "cues": [
      {"id": "tick", "layers": [{"wave": "sine", "freq": 1000, "release_ms": 8, "gain": 0.5}]},
      {"id": "bed", "loop_ms": 100, "layers": [{"wave": "noise", "whole": true, "gain": 0.1}]}]}"#;

    #[test]
    fn sounds_lists_the_sounds_and_the_loops() {
        let v = json(&sounds(SOUNDS));
        assert_eq!(v["ok"]["sounds"], serde_json::json!(["tick", "bed"]));
        assert_eq!(v["ok"]["loops"], serde_json::json!(["bed"]));
        assert_eq!(v["ok"]["volume"], 50);
        let v = json(&sounds(r#"{"sample_rate": 8000, "master": 10000}"#));
        assert_eq!(v["error"], "The sounds file needs a list of cues.");
    }

    #[test]
    fn sound_hands_back_a_wav_file_or_nothing() {
        let bytes = sound(SOUNDS, "tick", 100, 1);
        // 1 + 8 ms and a 10 ms tail at 8 samples a ms: 152 samples
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 2 * 152);
        assert!(sound(SOUNDS, "nothing", 100, 1).is_empty());
        assert!(sound("{}", "tick", 100, 1).is_empty());
    }

    #[test]
    fn the_title_opens_the_gallery() {
        let first = json(&start(STRINGS, ""));
        assert_eq!(first["ok"]["panels"][0]["actions"][0]["name"], "gallery.open");
        assert_eq!(first["ok"]["panels"][0]["actions"][0]["label"], "G");
        let state = first["ok"]["state"].to_string();
        let v = json(&act(&state, "", "gallery.open", ""));
        assert_eq!(v["ok"]["screen"], "gallery");
        let back = json(&act(&v["ok"]["state"].to_string(), "", "screen.show", "start"));
        assert_eq!(back["ok"]["screen"], "start");
    }

    #[test]
    fn act_refuses_an_unknown_action_and_screen_by_name() {
        let first = json(&start(STRINGS, ""));
        let state = first["ok"]["state"].to_string();
        let v = json(&act(&state, "", "screen.jump", "start"));
        assert_eq!(v["error"], "Unknown action \"screen.jump\".");
        let v = json(&act(&state, "", "screen.show", "nowhere"));
        assert_eq!(v["error"], "There is no screen \"nowhere\".");
    }
}
