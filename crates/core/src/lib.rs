//! tool_core: every application rule of the tool.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `api.rs` is the one door the shim calls through. The 3D pipeline:
//! `gfx` (fixed point, geometry, what the renderer reads), `model` (the
//! model files), `glyph` (letters on tiles), `look` (the water's light),
//! `scene` (stills placed from data) and `gallery` (the Gallery screen).

pub mod api;
pub mod gallery;
pub mod gfx;
pub mod glossary;
pub mod glyph;
pub mod letters;
pub mod look;
pub mod model;
pub mod page;
pub mod scene;
pub mod sound;
#[cfg(test)]
pub mod testkit;
