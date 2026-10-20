use super::*;
use crate::tuning::{self, tests::SHIPPED};

fn game(drivers: Vec<Driver>, fast: bool) -> Game {
    let t = tuning::parse(SHIPPED).unwrap();
    Game::new(
        &t,
        Setup {
            drivers,
            local: 0,
            seed: 9,
            fast,
            length_ms: None,
        },
    )
}

#[test]
fn a_game_counts_three_two_one_then_four_minutes_down_to_the_end() {
    let mut g = game(vec![Driver::Local, Driver::Ai], false);
    assert_eq!(g.phase(), Phase::Countdown { seconds: 3 });
    g.advance(1500);
    assert_eq!(g.phase(), Phase::Countdown { seconds: 2 });
    g.advance(1500);
    assert_eq!(g.phase(), Phase::Running { ms_left: 240_000 });
    g.advance(239_000);
    assert_eq!(g.phase(), Phase::Running { ms_left: 1000 });
    g.advance(1000);
    assert_eq!(g.phase(), Phase::Ended);
}

#[test]
fn fast_mode_skips_the_countdown_and_leaves_the_clock() {
    let g = game(vec![Driver::Local, Driver::Ai], true);
    assert_eq!(g.phase(), Phase::Running { ms_left: 240_000 });
}

#[test]
fn a_joiner_holds_at_zero_until_the_end_arrives() {
    let mut g = game(vec![Driver::Remote, Driver::Local], true);
    g.hold_at_zero = true;
    g.advance(300_000);
    assert_eq!(g.phase(), Phase::Running { ms_left: 0 });
    assert!(g.out_of_time());
    g.finish();
    assert_eq!(g.phase(), Phase::Ended);
}

#[test]
fn each_crab_starts_off_its_corner_facing_the_centre() {
    let g = game(vec![Driver::Local, Driver::Ai, Driver::Ai, Driver::Ai], true);
    // half the 3150 mm board plus 275 mm along the diagonal (194 mm each way)
    let at = 1575 + 194;
    let starts: Vec<(i32, i32)> = g.crabs.iter().map(|c| (c.x, c.z)).collect();
    assert_eq!(starts, vec![(-at, -at), (-at, at), (at, at), (at, -at)]);
    // the red crab, south-west, faces north-east
    assert_eq!(g.crabs[0].yaw, 8192);
    assert_eq!(g.yaw, 8192, "the camera starts behind the local crab");
}

#[test]
fn w_walks_the_way_the_camera_faces_and_d_sidesteps_right() {
    let mut g = game(vec![Driver::Local, Driver::Ai], true);
    g.yaw = 0; // north
    let (x0, z0) = (g.crabs[0].x, g.crabs[0].z);
    g.keys.set("KeyW", true);
    g.advance(1000);
    // 1400 mm a second, north
    assert_eq!((g.crabs[0].x - x0, g.crabs[0].z - z0), (0, 1400));
    g.keys.set("KeyW", false);
    g.keys.set("KeyD", true);
    g.advance(500);
    assert_eq!(g.crabs[0].x - x0, 700, "D steps east, to the right of north");
    assert!(!g.keys.set("KeyQ", true));
}

#[test]
fn no_crab_moves_during_the_countdown_or_past_the_sand() {
    let mut g = game(vec![Driver::Local, Driver::Ai], false);
    g.keys.set("KeyW", true);
    let before = g.crabs.clone();
    g.advance(2900);
    assert_eq!(g.crabs[0].x, before[0].x);
    assert_eq!(g.crabs[1].x, before[1].x);
    g.advance(60_000);
    let e = g.extent_mm;
    assert!(g.crabs.iter().all(|c| c.x.abs() <= e && c.z.abs() <= e));
    // half the board and 3 m of sand beyond
    assert_eq!(e, 1575 + 3000);
}

#[test]
fn the_mouse_turns_the_camera_by_the_sensitivity_and_tilt_can_invert() {
    let mut g = game(vec![Driver::Local], true);
    g.yaw = 0;
    let p0 = g.pitch;
    g.look(100, 10, 100, false);
    // 27 units a pixel at 100%
    assert_eq!(g.yaw, 2700);
    assert_eq!(g.pitch, p0 + 270);
    g.look(100, 10, 200, true);
    assert_eq!(g.yaw, 2700 + 5400);
    assert_eq!(g.pitch, p0 + 270 - 540);
    g.look(0, 100_000, 100, false);
    assert_eq!(g.pitch, 9100, "the tilt stops at pitch_max");
}

#[test]
fn an_ai_crab_wanders_and_a_remote_crab_goes_where_it_is_told() {
    let mut g = game(vec![Driver::Remote, Driver::Ai], true);
    let start = (g.crabs[1].x, g.crabs[1].z);
    g.advance(5000);
    assert_ne!((g.crabs[1].x, g.crabs[1].z), start);
    g.set_remote(
        0,
        CrabState {
            p: [10, 20],
            v: [0, 0],
            y: 7,
        },
    );
    assert_eq!((g.crabs[0].x, g.crabs[0].z, g.crabs[0].yaw), (10, 20, 7));
    // a crab this browser moves is not placed from outside
    g.set_remote(
        1,
        CrabState {
            p: [0, 0],
            v: [0, 0],
            y: 0,
        },
    );
    assert_ne!((g.crabs[1].x, g.crabs[1].z), (0, 0));
}
