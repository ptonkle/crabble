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

pub mod api;
pub mod app;
pub mod glossary;
pub mod online;
pub mod patch_notes;
pub mod rng;
pub mod standin;
pub mod strings;
pub mod tuning;
pub mod view;
