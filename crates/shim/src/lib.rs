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

    /// Builds the game's 3D frame; read it with the calls below.
    pub fn build_frame(&mut self) {
        self.s.build_frame()
    }

    /// Each static layer's signature and each pass's viewport and layers.
    pub fn frame_info(&self) -> String {
        self.s.frame_info()
    }

    pub fn pass_view(&self, k: usize) -> Vec<f32> {
        floats(&self.s.pass_view(k))
    }

    pub fn pass_solid(&self, k: usize) -> Vec<f32> {
        floats(&self.s.pass_solid(k))
    }

    pub fn pass_solid_count(&self, k: usize) -> u32 {
        tool_core::gfx::render::vertex_count(&self.s.pass_solid(k))
    }

    /// The see-through triangles, far to near.
    pub fn pass_clear(&self, k: usize) -> Vec<f32> {
        floats(&self.s.pass_clear(k))
    }

    pub fn pass_clear_count(&self, k: usize) -> u32 {
        tool_core::gfx::render::vertex_count(&self.s.pass_clear(k))
    }

    /// Static layer `k`'s triangles (a layer's vertex count is its length
    /// over `numbers().vertex.values`).
    pub fn layer(&mut self, k: usize) -> Vec<f32> {
        floats(&self.s.layer(k))
    }

    /// Every number the renderer needs (`gfx::render::numbers_json`).
    pub fn numbers(&self) -> String {
        tool_core::gfx::render::numbers_json()
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

#[wasm_bindgen]
pub fn sounds(sounds_json: &str) -> String {
    tool_core::api::sounds(sounds_json)
}

#[wasm_bindgen]
pub fn sound(sounds_json: &str, key: &str, volume: u32, seed: u32) -> Vec<u8> {
    tool_core::api::sound(sounds_json, key, volume, seed)
}

// ---- the Gallery (`tool_core::gallery`), a door of its own ----
//
// The core speaks Q16.16 integers; WebGL wants `f32`, so the frame's values
// are divided by the core's own `ONE` on the way out. The page's float
// clock (milliseconds) becomes whole microseconds, and the pixel ratio
// whole hundredths.

const ONE: f32 = tool_core::gfx::fx::ONE as f32;

fn floats(v: &[i32]) -> Vec<f32> {
    v.iter().map(|x| *x as f32 / ONE).collect()
}

/// The page's Gallery.
#[wasm_bindgen]
pub struct Gallery {
    g: tool_core::gallery::Gallery,
    frame: tool_core::gfx::render::Frame,
}

#[wasm_bindgen]
#[allow(clippy::new_without_default)]
impl Gallery {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Gallery {
        Gallery {
            g: tool_core::gallery::Gallery::new(),
            frame: Default::default(),
        }
    }

    /// The model files (one `{id: model}` object), the glyphs, the scenes,
    /// the look and the Gallery's words in; its controls or why not out.
    pub fn load(&mut self, models: &str, glyphs: &str, scenes: &str, look: &str, text: &str) -> String {
        self.g.load_json(models, glyphs, scenes, look, text)
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.g.act_json(name, value)
    }

    pub fn ui(&self) -> String {
        self.g.ui_json()
    }

    pub fn advance(&mut self, now_ms: f64) {
        self.g.advance((now_ms * 1000.0) as i64)
    }

    /// CSS size and device pixel ratio in; drawing-buffer size out.
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Vec<i32> {
        self.g
            .resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
            .to_vec()
    }

    /// Every number the renderer needs (`gfx::render::numbers_json`).
    pub fn numbers(&self) -> String {
        tool_core::gfx::render::numbers_json()
    }

    /// Builds this frame; read it with `view`, `solid`, `solid_count`,
    /// `clear` and `clear_count`.
    pub fn build_frame(&mut self) {
        self.frame = self.g.frame_or_empty();
    }

    pub fn view(&self) -> Vec<f32> {
        floats(&self.frame.view)
    }

    pub fn solid(&self) -> Vec<f32> {
        floats(&self.frame.tris)
    }

    pub fn solid_count(&self) -> u32 {
        tool_core::gfx::render::vertex_count(&self.frame.tris)
    }

    /// The see-through triangles, far to near.
    pub fn clear(&self) -> Vec<f32> {
        floats(&self.frame.clear)
    }

    pub fn clear_count(&self) -> u32 {
        tool_core::gfx::render::vertex_count(&self.frame.clear)
    }
}
