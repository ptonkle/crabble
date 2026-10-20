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

    /// The stand-in Play screen's picture (`standin::draw::Scene`).
    pub fn scene(&self) -> String {
        match &self.app {
            Ok(app) => ok(&app.scene()),
            Err(e) => error(e),
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
}
