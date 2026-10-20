//! The screens and their flow, through `act` as the page drives them.
//! Expected values come from the brief and Ape Race's menus.

use super::*;
use crate::strings::tests::complete;

const NOTES: &str = include_str!("../../../../data/patch-notes.json");
const TUNING: &str = include_str!("../../../../data/tuning.json");
const LOOK: &str = include_str!("../../../../data/standin.json");

pub(crate) fn app() -> App {
    let mut a = App::new(&complete(), NOTES).unwrap();
    a.act("data.tuning", TUNING).unwrap();
    a.act("data.standin", LOOK).unwrap();
    a
}

fn ids(panel: &Panel) -> Vec<String> {
    panel
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Button { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// Advances the page's clock by `ms`.
pub(crate) fn wait(a: &mut App, ms: i64) {
    // in frames of 16 ms, as a page would
    let mut left = ms;
    while left > 0 {
        let step = left.min(16);
        left -= step;
        let now = a.clock + step * 1000;
        a.advance(now);
    }
}

#[test]
fn the_title_lists_its_entries_in_order() {
    let a = app();
    let ui = a.ui();
    assert_eq!(ui.screen, Screen::Title);
    assert_eq!(ui.panels[0].heading, "TITLE");
    assert_eq!(
        ids(&ui.panels[1]),
        vec![
            "menu.local",
            "menu.online",
            "menu.how",
            "menu.settings",
            "menu.notes",
            "menu.gallery"
        ]
    );
    assert_eq!(ui.version, "v0.1.0");
    assert!(ui.version_opens);
}

#[test]
fn each_entry_opens_its_screen_and_back_or_escape_returns() {
    let mut a = app();
    for (entry, screen) in [
        ("menu.local", Screen::Local),
        ("menu.online", Screen::Online),
        ("menu.how", Screen::How),
        ("menu.settings", Screen::Settings),
        ("menu.notes", Screen::Notes),
    ] {
        a.act(entry, "").unwrap();
        assert_eq!(a.screen, screen);
        a.act("key", "Escape").unwrap();
        assert_eq!(a.screen, Screen::Title, "Escape off {entry}");
        a.act(entry, "").unwrap();
        a.act("menu.back", "").unwrap();
        assert_eq!(a.screen, Screen::Title, "Back off {entry}");
    }
    a.act("version", "").unwrap();
    assert_eq!(a.screen, Screen::Notes);
}

#[test]
fn how_to_play_shows_five_lines_and_five_controls() {
    let mut a = app();
    a.act("menu.how", "").unwrap();
    let p = &a.ui().panels[0];
    assert_eq!(p.heading, "MENU.HOW");
    assert!(matches!(&p.items[0], Item::Lines { lines } if lines.len() == 5 && lines[0] == "HOW.GOAL"));
    assert!(
        matches!(&p.items[1], Item::List { rows } if rows.len() == 5 && rows[4][1] == "HOW.OVERHEAD_KEYS")
    );
}

#[test]
fn settings_keep_their_steps_and_ranges() {
    let mut a = app();
    a.act("settings.volume", "83").unwrap();
    assert_eq!(a.settings.volume, 80);
    a.act("settings.volume", "500").unwrap();
    assert_eq!(a.settings.volume, 100);
    a.act("settings.mouse", "5").unwrap();
    assert_eq!(a.settings.mouse_pct, 25);
    a.act("settings.mouse", "152").unwrap();
    assert_eq!(a.settings.mouse_pct, 150);
    a.act("settings.invert_y", "on").unwrap();
    assert!(a.settings.invert_y);
    assert!(a.act("settings.volume", "loud").is_err());
}

#[test]
fn a_local_game_takes_one_to_three_ai_crabs() {
    let mut a = app();
    a.act("menu.local", "").unwrap();
    assert!(a.act("local.ai", "0").is_err());
    assert!(a.act("local.ai", "4").is_err());
    a.act("local.ai", "3").unwrap();
    a.act("local.start", "").unwrap();
    assert_eq!(a.screen, Screen::Play);
    let g = &a.play.as_ref().unwrap().game;
    assert_eq!(g.crabs.len(), 4);
    assert_eq!(g.local, 0);
}

#[test]
fn play_counts_down_then_shows_four_minutes() {
    let mut a = app();
    a.act("local.start", "").unwrap();
    let ui = a.ui();
    assert_eq!(ui.phase, "countdown");
    assert_eq!(ui.panels[0].heading, "4:00");
    assert_eq!(ui.panels[1].kind, "countdown");
    assert_eq!(ui.panels[1].heading, "3");
    assert!(ui.capture && ui.live && ui.scene);
    wait(&mut a, 3000);
    let ui = a.ui();
    assert_eq!(ui.phase, "running");
    assert_eq!(ui.panels.len(), 1);
    wait(&mut a, 1500);
    assert_eq!(a.ui().panels[0].heading, "3:59");
}

#[test]
fn fast_mode_skips_the_countdown() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    assert_eq!(a.ui().phase, "running");
}

#[test]
fn w_walks_the_crab_and_the_mouse_turns_only_while_captured() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    a.act("key", "KeyW").unwrap();
    wait(&mut a, 500);
    assert!(a.ui().moved);
    a.act("key.up", "KeyW").unwrap();
    let yaw = a.play.as_ref().unwrap().game.yaw;
    a.act("mouse.move", "10,0").unwrap();
    assert_eq!(a.play.as_ref().unwrap().game.yaw, yaw + 270);
    a.act("key", "Escape").unwrap();
    a.act("mouse.move", "10,0").unwrap();
    assert_eq!(
        a.play.as_ref().unwrap().game.yaw,
        yaw + 270,
        "no turning under the pause menu"
    );
}

#[test]
fn escape_pauses_a_local_game_and_its_clock() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    a.act("key", "Escape").unwrap();
    let ui = a.ui();
    assert!(!ui.capture);
    let pause = ui.panels.iter().find(|p| p.kind == "pause").unwrap();
    assert_eq!(ids(pause), vec!["menu.resume", "menu.quit"]);
    let before = a.play.as_ref().unwrap().game.elapsed_ms;
    wait(&mut a, 2000);
    assert_eq!(a.play.as_ref().unwrap().game.elapsed_ms, before);
    a.act("menu.resume", "").unwrap();
    assert!(a.ui().capture);
    a.act("pointer.released", "").unwrap();
    assert!(
        !a.ui().capture,
        "the browser letting the pointer go opens the menu"
    );
    a.act("menu.quit", "").unwrap();
    assert_eq!(a.screen, Screen::Title);
    assert!(a.play.is_none());
}

#[test]
fn a_short_game_ends_on_the_end_screen_with_back_to_title() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("flag.short", "on").unwrap();
    a.act("local.start", "").unwrap();
    wait(&mut a, 3100);
    let ui = a.ui();
    assert_eq!(ui.phase, "ended");
    assert!(!ui.capture);
    assert_eq!(ui.panels[0].kind, "end");
    assert_eq!(
        ids(&ui.panels[0]),
        vec!["end.title"],
        "no Rematch in a Local game"
    );
    a.act("end.title", "").unwrap();
    assert_eq!(a.screen, Screen::Title);
}

#[test]
fn the_scene_draws_on_play_only() {
    let mut a = app();
    a.resize(400, 300, 200);
    assert_eq!(a.size, [800, 600]);
    assert!(a.scene().shapes.is_empty());
    a.act("local.start", "").unwrap();
    let s = a.scene();
    assert!(s.shapes.len() > 30, "sand, board, lines and crabs");
}

#[test]
fn unknown_actions_and_flags_are_refused_by_name() {
    let mut a = app();
    assert_eq!(
        a.act("menu.jump", "").unwrap_err(),
        "Unknown action \"menu.jump\"."
    );
    assert_eq!(
        a.act("flag.loud", "").unwrap_err(),
        "Unknown action \"flag.loud\"."
    );
    assert!(a.act("flag.seed", "x").is_err());
}

#[test]
fn the_gallery_entry_names_its_screen_and_back_returns_but_escape_does_not() {
    let mut a = app();
    a.act("menu.gallery", "").unwrap();
    assert_eq!(a.screen, Screen::Gallery);
    assert!(a.ui().panels.is_empty(), "the Gallery draws itself");
    // its own Back closes it; a stray Escape would leave it drawn over the Title
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Gallery);
    a.act("menu.back", "").unwrap();
    assert_eq!(a.screen, Screen::Title);
}

#[test]
fn the_countdown_ticks_go_starts_the_clock_and_end_ends_it_at_the_volume() {
    let mut a = app();
    a.act("settings.volume", "60").unwrap();
    a.act("flag.short", "1").unwrap();
    a.act("local.start", "").unwrap();
    wait(&mut a, 5000);
    let keys: Vec<&str> = a.cues.iter().map(|c| c.key.as_str()).collect();
    // 3, 2, 1, then the clock, then 0:00
    assert_eq!(keys, vec!["countdown", "countdown", "countdown", "go", "end"]);
    assert!(a.cues.iter().all(|c| c.volume == 60));
    assert_ne!(a.cues[0].seed, a.cues[1].seed, "each tick varies");
}

#[test]
fn fast_mode_starts_with_go_and_the_pump_hands_the_sounds_over_once() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    let p = a.net_pump(16_000);
    let keys: Vec<String> = p.sounds.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys, vec!["go"]);
    assert!(a.net_pump(32_000).sounds.is_empty());
}

#[test]
fn a_sounds_file_without_a_named_sound_is_refused_by_name() {
    let mut a = app();
    let none = r#"{"sample_rate": 8000, "master": 10000, "cues": [
      {"id": "go", "layers": [{"wave": "sine", "freq": 1000, "release_ms": 8, "gain": 0.5}]}]}"#;
    assert_eq!(
        a.act("data.sounds", none).unwrap_err(),
        "The sounds file has no \"countdown\"."
    );
}
