//! tool_core: every application rule of the game.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `api.rs` is the one door the shim calls through; `app.rs` holds the
//! screens and settings, `view.rs` the panels and items the page draws,
//! `strings.rs` the words by key and `patch_notes.rs` What's New.
//! `online` is the online protocol (a pure state machine), `tuning.rs` the
//! game's numbers from `data/tuning.json`, `rng.rs` the seeded dice, and
//! `standin` the stand-in Play screen until the real game is built.
//!
//! The 3D pipeline: `gfx` (fixed point, geometry, what the renderer reads),
//! `model` (the model files), `glyph` (letters on tiles), `look` (the
//! water's light), `scene` (stills placed from data) and `gallery` (the
//! Gallery screen, with its own door in the shim). `sound` renders the
//! sounds of `data/sounds.json`.

pub mod api;
pub mod app;
pub mod gallery;
pub mod gfx;
pub mod glossary;
pub mod glyph;
pub mod letters;
pub mod look;
pub mod marks;
pub mod model;
pub mod online;
pub mod patch_notes;
pub mod rng;
pub mod scene;
pub mod sound;
pub mod standin;
pub mod strings;
#[cfg(test)]
pub mod testkit;
pub mod tuning;
pub mod view;
