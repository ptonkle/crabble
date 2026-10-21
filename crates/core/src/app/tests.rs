//! The screens and their flow, through `act` as the page drives them.
//! Expected values come from the brief and Ape Race's menus.

use super::*;
use crate::strings::tests::complete;

const NOTES: &str = include_str!("../../../../data/patch-notes.json");
const TUNING: &str = include_str!("../../../../data/tuning.json");

pub(crate) fn app() -> App {
    let mut a = App::new(&complete(), NOTES).unwrap();
    a.act("data.tuning", TUNING).unwrap();

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
    assert_eq!(ui.version, format!("v{VERSION}"));
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
fn how_to_play_shows_five_lines_and_six_controls() {
    let mut a = app();
    a.act("menu.how", "").unwrap();
    let p = &a.ui().panels[0];
    assert_eq!(p.heading, "MENU.HOW");
    assert!(matches!(&p.items[0], Item::Lines { lines } if lines.len() == 5 && lines[0] == "HOW.GOAL"));
    assert!(
        matches!(&p.items[1], Item::List { rows } if rows.len() == 6 && rows[4][1] == "HOW.OVERHEAD_KEYS" && rows[5][1] == "HOW.TOUCH_KEYS")
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
    assert_eq!(a.play.as_ref().unwrap().local, 0);
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
    let yaw = a.play.as_ref().unwrap().yaw;
    a.act("mouse.move", "10,0").unwrap();
    assert_eq!(a.play.as_ref().unwrap().yaw, yaw - 270);
    a.act("key", "Escape").unwrap();
    a.act("mouse.move", "10,0").unwrap();
    assert_eq!(
        a.play.as_ref().unwrap().yaw,
        yaw - 270,
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
    assert_eq!(ids(pause), vec!["menu.resume", "menu.settings", "menu.quit"]);
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
fn the_buffer_is_the_css_size_at_the_pixel_ratio_and_no_frame_draws_off_play() {
    let mut a = app();
    a.resize(400, 300, 200);
    assert_eq!(a.size, [800, 600]);
    a.build_frame();
    assert!(a.view.frame.is_empty() && a.view.info.passes.is_empty());
    assert!(!a.ui().crosshair);
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    assert!(
        a.ui().crosshair,
        "the crosshair shows while the game has the mouse"
    );
    a.act("key", "Escape").unwrap();
    assert!(!a.ui().crosshair, "and not under the pause menu");
}

#[test]
fn space_jumps_the_crab_and_its_sound_plays() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    a.act("key", "Space").unwrap();
    wait(&mut a, 100);
    let g = &a.play.as_ref().unwrap().game;
    assert!(!g.crabs[0].on_ground && g.crabs[0].pos.y > 0, "in the air");
    assert!(a.cues.iter().any(|c| c.key == "jump"));
    wait(&mut a, 1500);
    assert!(a.play.as_ref().unwrap().game.crabs[0].on_ground);
    assert!(a.cues.iter().any(|c| c.key == "land"));
}

#[test]
fn the_clock_runs_whole_ticks_and_keeps_the_rest() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    // 16 ms frames: three of them are two 20 ms ticks and 8 ms over
    wait(&mut a, 48);
    assert_eq!(a.play.as_ref().unwrap().game.elapsed_ms, 40);
    wait(&mut a, 16);
    assert_eq!(a.play.as_ref().unwrap().game.elapsed_ms, 60);
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
    // 3, 2, 1, then the clock, then 0:00, then the tally's winner (no
    // words, no look file: at once)
    assert_eq!(
        keys,
        vec!["countdown", "countdown", "countdown", "go", "end", "fanfare"]
    );
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

#[test]
fn the_mouse_turns_the_camera_by_the_sensitivity_and_tilt_can_invert() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    let p = a.play.as_mut().unwrap();
    p.yaw = 10_000;
    let p0 = p.pitch;
    p.look(100, 10, 100, false);
    // 27 units a pixel at 100%; to the right is a lower yaw
    assert_eq!(p.yaw, 10_000 - 2700);
    assert_eq!(p.pitch, p0 + 270);
    p.look(100, 10, 200, true);
    assert_eq!(p.yaw, 10_000 - 2700 - 5400);
    assert_eq!(p.pitch, p0 + 270 - 540);
    p.look(0, 100_000, 100, false);
    assert_eq!(p.pitch, 13_000, "the tilt stops at pitch_max");
}

#[test]
fn the_idle_call_shows_its_countdown_under_the_clock_then_the_minus_twos() {
    let mut a = app();
    a.act("data.look", &crate::look::tests::shipped()).unwrap();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    let kinds = |a: &App| a.ui().panels.iter().map(|p| p.kind.clone()).collect::<Vec<_>>();
    wait(&mut a, 29_000);
    assert!(
        !kinds(&a).contains(&"idle".to_string()),
        "nothing before the last ten seconds"
    );
    wait(&mut a, 1_500);
    let ui = a.ui();
    let call = ui.panels.iter().find(|p| p.kind == "idle").unwrap();
    assert_eq!(
        (call.heading.clone(), call.sub.as_str()),
        (a.t("idle.call"), "10")
    );
    assert!(a.cues.iter().any(|c| c.key == "idle_tick"));
    // past 40 s: the "-2" shows, and only this player sees it
    wait(&mut a, 10_800);
    let ui = a.ui();
    let pen = ui.panels.iter().find(|p| p.kind == "penalty").unwrap();
    assert_eq!(pen.heading, "-2");
    assert!(
        ui.panels.iter().all(|p| !p.heading.contains("score")),
        "no scores in play"
    );
}

#[test]
fn the_tally_takes_each_seats_idle_penalty_as_points_lost() {
    let mut a = app();
    a.act("flag.fast", "on").unwrap();
    a.act("local.ai", "2").unwrap();
    a.act("local.start", "").unwrap();
    // 43 s without placing: three seconds past the grace, 2 points each
    wait(&mut a, 43_000);
    let g = &a.play.as_ref().unwrap().game;
    let (_, idle) = crate::app::tally::final_state(g);
    assert_eq!(idle, vec![6, 6, 6]);
}

/// A local game with seat 0 on the board two squares from the centre,
/// facing it, the crosshair on (7, 8), the centre an A, and `held` in its
/// claws: each a tile's letter.
fn aimed_at_the_centre(held: [Option<char>; 2]) -> App {
    let mut a = app();
    a.act("data.look", &crate::look::tests::shipped()).unwrap();
    a.act("flag.fast", "on").unwrap();
    a.act("local.start", "").unwrap();
    wait(&mut a, 100);
    let p = a.play.as_mut().unwrap();
    // a plain sand map of two tiles, and the centre's
    let mut m = crate::sim::tests::flat();
    for _ in 0..3 {
        m.slots.push((crate::gfx::geom::Affine::IDENTITY, false));
    }
    p.game = crate::sim::tests::game_on(&m, vec![crate::sim::Driver::Local]);
    let g = &mut p.game;
    let centre = g.tiles.all.len() - 1;
    g.tiles.all[centre].letter = 'A';
    for (claw, letter) in held.iter().enumerate() {
        g.crabs[0].claws[claw] = letter.map(|l| {
            g.tiles.all[claw].letter = l;
            claw as crate::sim::TileId
        });
    }
    let top = g.rules.board_top;
    let c = &mut g.crabs[0];
    c.pos = crate::gfx::geom::v3(crate::model::mm1(520), top, 0);
    c.yaw = -crate::gfx::fx::QUARTER;
    p.yaw = -crate::gfx::fx::QUARTER;
    a
}

fn preview_rows(a: &App) -> Vec<Vec<String>> {
    let ui = a.ui();
    let Some(p) = ui.panels.iter().find(|p| p.kind == "preview") else {
        return Vec::new();
    };
    match &p.items[0] {
        Item::Table { rows, .. } => rows.iter().map(|r| r.cells.clone()).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn the_preview_shows_each_claws_words_marked_with_its_claw_when_both_are_full() {
    // one tile held: its words, no claw named
    let a = aimed_at_the_centre([None, Some('T')]);
    let g = &a.play.as_ref().unwrap().game;
    assert_eq!(g.ghost(0).map(|gh| gh.2), Some((7, 8)));
    let rows = preview_rows(&a);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "AT");
    assert_eq!(rows[0].len(), 2);
    // a tile in each claw: the left claw's words, then the right's, each
    // row starting with its claw
    let a = aimed_at_the_centre([Some('S'), Some('T')]);
    let rows = preview_rows(&a);
    let l = a.t("preview.left");
    let r = a.t("preview.right");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][..2], [l, "AS".to_string()]);
    assert_eq!(rows[1][..2], [r, "AT".to_string()]);
}

/// The triangles of the frame's pass `k`.
fn pass_tris(a: &App, k: usize) -> usize {
    a.view.frame[k].solid.len() / (3 * crate::gfx::render::VERTEX_VALUES)
}

#[test]
fn colourblind_mode_switched_from_the_pause_menu_shows_on_the_game_at_once() {
    // the person, R19: "colorblind mode doesn't seem to do literally
    // anything"; it could only be switched on the Title's Settings
    let mut a = app();
    let models = serde_json::json!({ "tile": crate::testkit::tile(), "crab": crate::testkit::crab() });
    a.act("data.models", &models.to_string()).unwrap();
    a.act("data.glyphs", &crate::testkit::font_json().to_string())
        .unwrap();
    a.act("data.look", &crate::look::tests::shipped()).unwrap();
    a.act("flag.webgl", "on").unwrap();
    a.act("flag.fast", "on").unwrap();
    a.act("local.ai", "1").unwrap();
    a.act("local.start", "").unwrap();
    a.build_frame();
    let (main, overhead) = (pass_tris(&a, 0), pass_tris(&a, 1));
    assert!(!a.view.colourblind());
    // the pause menu's Settings opens over the game, still drawn behind
    a.act("key", "Escape").unwrap();
    a.act("menu.settings", "").unwrap();
    let ui = a.ui();
    assert_eq!(ui.screen, Screen::Play);
    assert!(ui.scene, "the game stays drawn behind Settings");
    assert!(ui.panels.iter().any(|p| p.kind == "settings"));
    assert!(!ui.panels.iter().any(|p| p.kind == "pause"));
    a.act("settings.colourblind", "on").unwrap();
    assert!(a.ui().colourblind, "the page is told");
    a.build_frame();
    assert!(a.view.colourblind(), "the next frame draws it");
    // two crabs: a symbol on each shell, rimmed (seat 0's triangle, 2 x 3
    // triangles; seat 1's square, 2 x 4), and the other crab's over it
    // (2 x 4); on the overhead circles a triangle and a square (3 + 4)
    assert_eq!(pass_tris(&a, 0), main + 6 + 8 + 8);
    assert_eq!(pass_tris(&a, 1), overhead + 3 + 4);
    // Back, and Escape, go from Settings to the pause menu, not the Title
    a.act("menu.back", "").unwrap();
    assert!(a.ui().panels.iter().any(|p| p.kind == "pause"));
    a.act("menu.settings", "").unwrap();
    a.act("key", "Escape").unwrap();
    assert_eq!(a.screen, Screen::Play);
    assert!(a.ui().panels.iter().any(|p| p.kind == "pause"));
    // and off again, at once
    a.act("menu.settings", "").unwrap();
    a.act("settings.colourblind", "off").unwrap();
    a.build_frame();
    assert!(!a.view.colourblind());
    assert_eq!((pass_tris(&a, 0), pass_tris(&a, 1)), (main, overhead));
}
