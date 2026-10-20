use super::*;
use crate::gfx::fx::ONE;
use crate::gfx::geom::Affine;
use crate::model::mm;
use crate::tuning::{self, tests::SHIPPED};

pub(crate) fn tuning() -> Tuning {
    tuning::parse(SHIPPED).unwrap()
}

pub(crate) fn game_on(map: &Map, drivers: Vec<Driver>) -> Match {
    Match::new(
        &tuning(),
        map,
        Setup {
            drivers,
            seed: 9,
            fast: true,
            length_ms: None,
        },
    )
}

pub(crate) fn flat() -> Map {
    Map::flat(6000)
}

/// Runs `ms` of ticks with crab 0 given `input` (its look kept).
pub(crate) fn run(g: &mut Match, ms: i64, input: SeatInput) {
    let ticks = ms / g.rules.tick_ms;
    for k in 0..ticks {
        let mut i = input;
        // a press lasts one tick
        if k > 0 {
            i.jump = false;
            i.click = [false; 2];
        }
        g.step(&[i]);
    }
}

pub(crate) fn looking(g: &Match, seat: usize) -> SeatInput {
    SeatInput {
        yaw: g.crabs[seat].yaw,
        pitch: g.crabs[seat].pitch,
        ..Default::default()
    }
}

fn mm_of(q: i32) -> i32 {
    (q as i64 * 1000 / ONE as i64) as i32
}

#[test]
fn a_game_counts_three_two_one_then_four_minutes_down_to_the_end() {
    let mut g = Match::new(
        &tuning(),
        &flat(),
        Setup {
            drivers: vec![Driver::Local, Driver::Ai],
            seed: 1,
            fast: false,
            length_ms: None,
        },
    );
    assert_eq!(g.phase(), Phase::Countdown { seconds: 3 });
    run(&mut g, 1500, SeatInput::default());
    assert_eq!(g.phase(), Phase::Countdown { seconds: 2 });
    run(&mut g, 1500, SeatInput::default());
    assert_eq!(g.phase(), Phase::Running { ms_left: 240_000 });
    run(&mut g, 239_000, SeatInput::default());
    assert_eq!(g.phase(), Phase::Running { ms_left: 1000 });
    run(&mut g, 1000, SeatInput::default());
    assert_eq!(g.phase(), Phase::Ended);
    assert!(g.take_events().contains(&Event::Ended));
}

#[test]
fn a_joiner_holds_at_zero_until_the_end_arrives() {
    let mut g = game_on(&flat(), vec![Driver::Remote, Driver::Local]);
    g.hold_at_zero = true;
    run(&mut g, 300_000, SeatInput::default());
    assert_eq!(g.phase(), Phase::Running { ms_left: 0 });
    assert!(g.out_of_time());
    g.finish();
    assert_eq!(g.phase(), Phase::Ended);
}

#[test]
fn each_crab_starts_off_its_corner_facing_the_middle() {
    let g = game_on(&flat(), vec![Driver::Local, Driver::Ai, Driver::Ai, Driver::Ai]);
    // half the 3150 mm board plus 250 mm along the diagonal (176 mm each way)
    let at = 1575 + 175;
    let starts: Vec<(i32, i32)> = g.crabs.iter().map(|c| (mm_of(c.pos.x), mm_of(c.pos.z))).collect();
    // red near-left, then round the board: far-left, far-right, near-right
    assert_eq!(starts, vec![(-at, at), (-at, -at), (at, -at), (at, at)]);
    // the red crab faces the middle: forward is +x and -z
    let f = g.crabs[0].forward();
    assert!(f.x > ONE / 2 && f.z < -ONE / 2, "{f:?}");
}

#[test]
fn w_walks_the_crab_forward_at_its_speed_and_a_and_d_scuttle_it_sideways() {
    let mut g = game_on(&flat(), vec![Driver::Local]);
    let start = g.crabs[0].pos;
    let f = g.crabs[0].forward();
    let mut w = looking(&g, 0);
    w.fwd = 1;
    run(&mut g, 1000, w);
    let gone = g.crabs[0].pos.sub(start);
    // 394 mm in a second, straight ahead
    assert!((mm_of(gone.len()) - 394).abs() <= 3, "{}", mm_of(gone.len()));
    assert!(gone.dot(f) > 0);
    assert_eq!(g.crabs[0].gait, Gait::Forward);
    // D goes to its right, at the same speed
    let here = g.crabs[0].pos;
    let mut d = looking(&g, 0);
    d.side = 1;
    run(&mut g, 500, d);
    let gone = g.crabs[0].pos.sub(here);
    assert!(gone.dot(g.crabs[0].right()) > mm1(190));
    assert_eq!(g.crabs[0].gait, Gait::Right);
}

#[test]
fn the_crab_faces_where_its_seat_looks() {
    let mut g = game_on(&flat(), vec![Driver::Local]);
    let mut i = looking(&g, 0);
    i.yaw = 0;
    i.fwd = 1;
    let z = g.crabs[0].pos.z;
    run(&mut g, 200, i);
    assert_eq!(g.crabs[0].yaw, 0);
    assert!(g.crabs[0].pos.z > z, "yaw 0 looks along +z");
}

#[test]
fn a_jump_is_floaty_and_rises_just_over_a_tile() {
    let mut g = game_on(&flat(), vec![Driver::Local]);
    let mut i = looking(&g, 0);
    i.jump = true;
    let mut top = 0;
    let mut air = 0;
    run(&mut g, 20, i);
    assert!(g.take_events().contains(&Event::Jumped { seat: 0 }));
    while !g.crabs[0].on_ground {
        top = top.max(g.crabs[0].pos.y);
        air += 20;
        let i = looking(&g, 0);
        run(&mut g, 20, i);
        assert!(air < 2000, "it comes down");
    }
    // apex 65 mm, about 700 ms in the air
    assert!((mm_of(top) - 65).abs() <= 3, "{}", mm_of(top));
    assert!((660..=740).contains(&air), "{air}");
    assert!(g.take_events().contains(&Event::Landed { seat: 0 }));
}

/// A map with one tile lying flat on the sand at (x, z) mm.
fn one_tile(x: i32, z: i32) -> Map {
    let mut m = flat();
    m.slots.push(Affine::translate(mm([x, 0, z])));
    m
}

#[test]
fn a_lying_tile_needs_a_jump_and_the_crab_can_stand_on_it() {
    let g0 = game_on(&flat(), vec![Driver::Local]);
    let c = &g0.crabs[0];
    // a tile 250 mm straight ahead
    let ahead = c.pos.add(c.forward().scale(mm1(250)));
    let map = one_tile(mm_of(ahead.x), mm_of(ahead.z));
    let mut g = game_on(&map, vec![Driver::Local]);
    let mut w = looking(&g, 0);
    w.fwd = 1;
    run(&mut g, 1500, w);
    let stopped = g.crabs[0].pos;
    let to_tile = ahead.sub(stopped).len();
    assert!(
        mm_of(to_tile) > 100,
        "walking stops at the tile's edge ({} mm)",
        mm_of(to_tile)
    );
    assert_eq!(g.crabs[0].pos.y, 0);
    // a jump forward lands on its top, 45 mm up
    w.jump = true;
    run(&mut g, 800, w);
    let mut still = looking(&g, 0);
    still.fwd = 0;
    run(&mut g, 400, still);
    assert!(g.crabs[0].on_ground);
    assert!(
        (mm_of(g.crabs[0].pos.y) - 45).abs() <= 2,
        "{}",
        mm_of(g.crabs[0].pos.y)
    );
}

#[test]
fn a_crab_stands_on_the_drawn_ground() {
    // a raised square 100 mm high round the red crab's start
    let t = tuning();
    let (at, _) = starts(&t, 1)[0];
    let mut tris = crate::gfx::render::Tris::default();
    let p = |x: i32, z: i32| at.add(mm([x, 100, z]));
    let up = v3(0, ONE, 0);
    let paint = crate::gfx::render::Paint::lit([0; 3]);
    tris.push([p(-300, -300), p(300, -300), p(300, 300)], up, paint);
    tris.push([p(-300, -300), p(300, 300), p(-300, 300)], up, paint);
    let map = Map {
        ground: Ground::from_tris(&tris.solid, 6000),
        slots: Vec::new(),
    };
    let g = game_on(&map, vec![Driver::Local]);
    assert!((mm_of(g.crabs[0].pos.y) - 100).abs() <= 1);
}

#[test]
fn two_crabs_that_bump_push_each_other_a_bit() {
    let mut g = game_on(&flat(), vec![Driver::Local, Driver::Ai]);
    g.crabs[1].pos = g.crabs[0].pos.add(v3(mm1(30), 0, 0));
    let before = g.crabs[1].pos.sub(g.crabs[0].pos).len();
    let i = looking(&g, 0);
    run(&mut g, 20, i);
    let after = g.crabs[1].pos.sub(g.crabs[0].pos).len();
    assert!(after > before, "pushed apart");
    // a bit, not all at once
    assert!(after < 2 * g.rules.radius);
    let i = looking(&g, 0);
    run(&mut g, 2000, i);
    let apart = g.crabs[1].pos.sub(g.crabs[0].pos).len();
    assert!(apart >= 2 * g.rules.radius - mm1(2), "{}", mm_of(apart));
}

#[test]
fn two_games_with_the_same_inputs_stay_identical() {
    let map = one_tile(-1300, 1300);
    let mut a = game_on(&map, vec![Driver::Local, Driver::Ai]);
    let mut b = game_on(&map, vec![Driver::Local, Driver::Ai]);
    for k in 0..300 {
        let i = SeatInput {
            fwd: (k % 3) as i8 - 1,
            side: (k % 5 / 2) as i8 - 1,
            jump: k % 37 == 0,
            yaw: k * 97,
            pitch: 3000,
            click: [false; 2],
        };
        a.step(&[i]);
        b.step(&[i]);
    }
    assert_eq!(a, b);
    assert_ne!(a.crabs[0].pos, game_on(&map, vec![Driver::Local]).crabs[0].pos);
}

#[test]
fn the_tiles_are_dealt_three_sets_without_blanks_by_the_seed() {
    let mut map = flat();
    for k in 0..294 {
        map.slots.push(Affine::translate(mm([k * 10, 0, 3000])));
    }
    let a = game_on(&map, vec![Driver::Local]);
    let letters: String = a.tiles.all.iter().map(|t| t.letter).collect();
    let mut sorted: Vec<char> = letters.chars().collect();
    sorted.sort();
    assert_eq!(sorted, crate::letters::tile_set(3));
    // ids are their places, 0 to 293
    assert!(a.tiles.all.iter().enumerate().all(|(i, t)| t.id as usize == i));
    let b = Match::new(
        &tuning(),
        &map,
        Setup {
            drivers: vec![Driver::Local],
            seed: 10,
            fast: true,
            length_ms: None,
        },
    );
    let other: String = b.tiles.all.iter().map(|t| t.letter).collect();
    assert_ne!(letters, other, "another seed deals another order");
}

#[test]
fn a_remote_crab_goes_where_it_is_told_and_a_local_one_does_not() {
    let mut g = game_on(&flat(), vec![Driver::Remote, Driver::Ai]);
    g.set_remote(
        0,
        CrabState {
            p: [10, 20],
            v: [0, 0],
            y: 7,
            ..Default::default()
        },
    );
    let s = g.crabs[0].state();
    assert_eq!((s.p, s.y), ([10, 20], 7));
    // a crab this browser moves is not placed from outside
    let before = g.crabs[1].pos;
    g.set_remote(
        1,
        CrabState {
            p: [0, 0],
            v: [0, 0],
            y: 0,
            ..Default::default()
        },
    );
    assert_eq!(g.crabs[1].pos, before);
}

#[test]
fn a_remote_crab_in_the_air_is_drawn_at_its_owners_height_and_on_the_ground_on_ours() {
    let mut g = game_on(&flat(), vec![Driver::Remote, Driver::Ai]);
    let air = |h: i32| CrabState {
        h,
        vh: 500,
        a: true,
        g: 3,
        ..Default::default()
    };
    g.set_remote(0, air(60));
    assert_eq!(g.crabs[0].pos.y, crate::model::mm1(60));
    assert!(!g.crabs[0].on_ground);
    assert_eq!(g.crabs[0].gait, Gait::Left);
    // never under the ground, however late its state
    g.set_remote(0, air(-40));
    assert_eq!(g.crabs[0].pos.y, 0);
    // on the ground: on this browser's ground, whatever height it said
    g.set_remote(
        0,
        CrabState {
            h: 30,
            ..Default::default()
        },
    );
    assert_eq!(g.crabs[0].pos.y, 0);
    assert!(g.crabs[0].on_ground);
    assert_eq!(g.crabs[0].gait, Gait::Still);
}
