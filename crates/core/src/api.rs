//! The door into the core. The shim holds one `Session` and forwards its
//! calls; each returns a JSON string, `{"ok": ...}` on success or
//! `{"error": "..."}` with a message the page can show as it is.
//!
//! The core describes the screen as a view model (`app::Ui`): the screen's
//! name, its panels and their items. The page builds the DOM from that
//! description without knowing what a panel means. Buttons, settings, keys,
//! flags and screens all come back through one function, `act`, with a
//! dotted name such as `menu.settings`, so a new option needs a new `match`
//! arm in `app.rs` and no new shim function. (After Peter's Mandible; the
//! session held here rather than a state round-trip through the page is
//! Honey Hole's, SECOND-ORDER R1.20.)

use serde::Serialize;

use crate::app::App;
use crate::sound;

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// The app for one page, or why it could not start.
pub struct Session {
    app: Result<App, String>,
}

impl Session {
    /// Checks the strings file and the patch notes and opens on the Title.
    pub fn new(strings_json: &str, notes_json: &str) -> Session {
        Session {
            app: App::new(strings_json, notes_json),
        }
    }

    /// What the page draws now.
    pub fn ui(&self) -> String {
        match &self.app {
            Ok(app) => ok(&app.ui()),
            Err(e) => error(e),
        }
    }

    /// Applies one named action and returns what the page draws next. A
    /// refused action leaves the app as it was.
    pub fn act(&mut self, name: &str, value: &str) -> String {
        match &mut self.app {
            Ok(app) => match app.act(name, value) {
                Ok(()) => ok(&app.ui()),
                Err(e) => error(&e),
            },
            Err(e) => error(e),
        }
    }

    /// The canvas's CSS size and pixel ratio (hundredths) in; the drawing
    /// buffer's size out.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> Vec<i32> {
        match &mut self.app {
            Ok(app) => app.resize(css_w, css_h, dpr_pct).to_vec(),
            Err(_) => vec![1, 1],
        }
    }

    /// Builds the game's 3D frame (`app::view3d`); read it with
    /// `frame_info`, `pass_*` and `layer`.
    pub fn build_frame(&mut self) {
        if let Ok(app) = &mut self.app {
            app.build_frame();
        }
    }

    /// Each static layer's signature, and each pass's viewport and layers,
    /// as JSON.
    pub fn frame_info(&self) -> String {
        match &self.app {
            Ok(app) => serde_json::to_string(&app.view.info).unwrap_or_default(),
            Err(_) => "{\"layers\":[],\"passes\":[]}".into(),
        }
    }

    fn pass(&self, k: usize) -> Option<&crate::app::view3d::Pass> {
        self.app.as_ref().ok()?.view.frame.get(k)
    }

    /// Pass `k`'s view values, solid and see-through triangles; empty for
    /// no such pass.
    pub fn pass_view(&self, k: usize) -> Vec<i32> {
        self.pass(k).map(|p| p.view.clone()).unwrap_or_default()
    }

    pub fn pass_solid(&self, k: usize) -> Vec<i32> {
        self.pass(k).map(|p| p.solid.clone()).unwrap_or_default()
    }

    pub fn pass_clear(&self, k: usize) -> Vec<i32> {
        self.pass(k).map(|p| p.clear.clone()).unwrap_or_default()
    }

    /// Static layer `k`'s triangles, as the game now is.
    pub fn layer(&mut self, k: usize) -> Vec<i32> {
        match &mut self.app {
            Ok(app) => app.layer(k),
            Err(_) => Vec::new(),
        }
    }

    /// A transport event from `web/online.js` (`app/online_play.rs`).
    pub fn net_event(&mut self, kind: &str, peer: &str, text: &str, host: bool) {
        if let Ok(app) = &mut self.app {
            app.net_event(kind, peer, text, host);
        }
    }

    /// A pasted code the transport made.
    pub fn net_code(&mut self, which: &str, code: &str) {
        if let Ok(app) = &mut self.app {
            app.net_code(which, code);
        }
    }

    /// Every frame while the page runs: the clock (microseconds) in, and
    /// the text to send and the transport's requests out
    /// (`app::online_play::Pump`).
    pub fn net_pump(&mut self, now: i64) -> String {
        match &mut self.app {
            Ok(app) => ok(&app.net_pump(now)),
            Err(e) => error(e),
        }
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
        Err(e) => error(&e),
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

    const NOTES: &str = r#"[{"version": "0.1.0", "date": "2026-10-19", "notes": ["A."]}]"#;

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn a_session_opens_on_the_title_and_acts_through_the_one_door() {
        let mut s = Session::new(&crate::strings::tests::complete(), NOTES);
        assert_eq!(json(&s.ui())["ok"]["screen"], "title");
        let v = json(&s.act("menu.settings", ""));
        assert_eq!(v["ok"]["screen"], "settings");
    }

    #[test]
    fn a_refused_action_comes_back_as_an_error_and_changes_nothing() {
        let mut s = Session::new(&crate::strings::tests::complete(), NOTES);
        assert_eq!(
            s.act("screen.jump", ""),
            r#"{"error":"Unknown action \"screen.jump\"."}"#
        );
        assert_eq!(json(&s.ui())["ok"]["screen"], "title");
    }

    #[test]
    fn a_bad_strings_file_or_bad_notes_is_wrapped_in_error() {
        let s = Session::new(r#"{"title": "T"}"#, NOTES);
        assert_eq!(s.ui(), r#"{"error":"The strings file has no \"menu.local\"."}"#);
        let s = Session::new(&crate::strings::tests::complete(), "[]");
        assert_eq!(s.ui(), r#"{"error":"The patch notes list no version."}"#);
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
}
