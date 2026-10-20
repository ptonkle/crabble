//! tool_shim: forwards strings between the page and `tool_core::api`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core. `crates/core/tests/boundary.rs` fails if a branch, a loop or
//! the `?` operator appears here; if a feature seems to need one, the rule
//! belongs in the core.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start(strings_json: &str, glossary_md: &str) -> String {
    tool_core::api::start(strings_json, glossary_md)
}

#[wasm_bindgen]
pub fn act(state_json: &str, glossary_md: &str, name: &str, value: &str) -> String {
    tool_core::api::act(state_json, glossary_md, name, value)
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
