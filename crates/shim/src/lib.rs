//! tool_shim: forwards strings between the page and `tool_core::api`.
//!
//! Builder requirement: the shim decides nothing. Each method is one call
//! into the core and a conversion of its arguments. The page's float clock
//! (milliseconds) becomes whole microseconds, and the pixel ratio whole
//! hundredths. `crates/core/tests/boundary.rs` fails if a branch, a loop or
//! the `?` operator appears here; if a feature seems to need one, the rule
//! belongs in the core.

use wasm_bindgen::prelude::*;

/// The page's one session with the core.
#[wasm_bindgen]
pub struct Game {
    s: tool_core::api::Session,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(strings_json: &str, notes_json: &str) -> Game {
        Game {
            s: tool_core::api::Session::new(strings_json, notes_json),
        }
    }

    pub fn ui(&self) -> String {
        self.s.ui()
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.s.act(name, value)
    }

    /// CSS size and device pixel ratio in; drawing-buffer size out.
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Vec<i32> {
        self.s.resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
    }

    /// The stand-in Play screen's picture, as JSON (`web/standin.js`).
    pub fn scene(&self) -> String {
        self.s.scene()
    }

    /// A transport event (`web/online.js` through `web/net.js`).
    pub fn net_event(&mut self, kind: &str, peer: &str, text: &str, host: bool) {
        self.s.net_event(kind, peer, text, host)
    }

    pub fn net_code(&mut self, which: &str, code: &str) {
        self.s.net_code(which, code)
    }

    /// The page's clock in (ms); the text to send and the transport's
    /// requests out.
    pub fn net_pump(&mut self, now_ms: f64) -> String {
        self.s.net_pump((now_ms * 1000.0) as i64)
    }
}
