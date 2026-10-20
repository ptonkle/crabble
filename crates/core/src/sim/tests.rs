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
    m.slots.push((Affine::translate(mm([x, 0, z])), false));
    // the last slot is the centre tile's
    m.slots.push((Affine::IDENTITY, false));
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
        map.slots.push((Affine::translate(mm([k * 10, 0, 3000])), false));
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

// ---- grab, place, drop (stage 2) ----

/// A game whose map has a tile lying flat `ahead_mm` straight ahead of the
/// red crab's start, and the centre tile (the last slot, which the game
/// moves to the centre square).
fn tile_ahead(ahead_mm: i32, seats: usize) -> Match {
    let t = tuning();
    let (at, yaw) = starts(&t, 1)[0];
    let ahead = at.add(crate::gfx::geom::facing(yaw, 0).scale(mm1(ahead_mm)));
    let mut m = flat();
    let turn = Affine::translate(ahead).then(&Affine::rot_y(yaw));
    m.slots.push((turn, false));
    m.slots.push((Affine::IDENTITY, false));
    let mut drivers = vec![Driver::Local];
    drivers.extend(std::iter::repeat_n(Driver::Ai, seats - 1));
    game_on(&m, drivers)
}

fn click(g: &Match, seat: usize, claw: Claw) -> SeatInput {
    let mut i = looking(g, seat);
    i.click[claw] = true;
    i
}

#[test]
fn the_centre_square_starts_with_a_random_letter_placed() {
    let g = tile_ahead(200, 1);
    let centre = g.tiles.all.last().unwrap();
    assert_eq!(
        centre.state,
        State::Placed {
            square: (7, 7),
            seat: None,
            order: 0
        }
    );
    assert_eq!((centre.pose.t.x, centre.pose.t.z), (0, 0));
    assert_eq!(g.placed_at((7, 7)), Some(centre.id));
}

#[test]
fn the_crosshair_grabs_the_tile_it_hits_within_reach() {
    let mut g = tile_ahead(200, 1);
    let aim = g.aim(0);
    assert_eq!(aim.target, Target::Tile(0));
    assert!(aim.in_reach);
    let i = click(&g, 0, tiles::RIGHT);
    g.step(&[i]);
    assert_eq!(g.crabs[0].claws, [None, Some(0)]);
    assert_eq!(
        g.tiles.all[0].state,
        State::Held {
            seat: 0,
            claw: tiles::RIGHT
        }
    );
    assert!(g.take_events().contains(&Event::Grabbed {
        seat: 0,
        claw: tiles::RIGHT,
        tile: 0
    }));
    // a tile under the crosshair but out of reach is not grabbed: looking
    // nearly level, the ray meets a tile 650 mm ahead
    let mut far = tile_ahead(650, 1);
    let mut i = click(&far, 0, tiles::LEFT);
    i.pitch = 1200;
    let mut look = i;
    look.click = [false; 2];
    far.step(&[look]);
    let aim = far.aim(0);
    assert_eq!(aim.target, Target::Tile(0));
    assert!(!aim.in_reach);
    far.step(&[i]);
    assert_eq!(far.crabs[0].claws, [None, None]);
}

#[test]
fn grabbing_the_bottom_tile_of_a_stack_tumbles_the_one_on_it() {
    let mut g = tile_ahead(200, 1);
    // a second tile on the first, the stack's top 90 mm up
    let under = g.tiles.all[0].pose;
    let mut top = crate::sim::tiles::Tile::new(2, 'Z', 10, under, false);
    top.pose.t.y += mm1(45);
    g.tiles.all.push(top);
    g.tiles.find_rests();
    assert_eq!(g.tiles.all[2].rests_on, vec![0]);
    g.submit_claim(Claim::Grab {
        seat: 0,
        claw: tiles::LEFT,
        tile: 0,
    });
    let i = looking(&g, 0);
    run(&mut g, 1500, i);
    assert_eq!(g.tiles.all[2].state, State::Loose { awake: false });
    assert!(g.tiles.all[2].pose.t.y < mm1(3), "it fell to the sand");
}

#[test]
fn two_claims_on_one_tile_the_first_wins() {
    let mut g = tile_ahead(200, 2);
    // yellow's claim came from far away first; red clicks in the same tick
    g.submit_claim(Claim::Grab {
        seat: 1,
        claw: tiles::LEFT,
        tile: 0,
    });
    let i = click(&g, 0, tiles::RIGHT);
    let idle = looking(&g, 1);
    g.step(&[i, idle]);
    assert_eq!(
        g.tiles.all[0].state,
        State::Held {
            seat: 1,
            claw: tiles::LEFT
        }
    );
    assert_eq!(g.crabs[0].claws, [None, None]);
    assert!(g.take_events().contains(&Event::Refused {
        seat: 0,
        claw: tiles::RIGHT,
        tile: 0
    }));
}

/// Gives seat `seat` tile `tile` in `claw` through the referee.
fn hand(g: &mut Match, seat: usize, claw: Claw, tile: TileId) {
    g.submit_claim(Claim::Grab { seat, claw, tile });
    let inputs: Vec<SeatInput> = (0..g.crabs.len()).map(|k| looking(g, k)).collect();
    g.step(&inputs);
    assert_eq!(g.crabs[seat].claws[claw], Some(tile));
}

#[test]
fn two_placements_on_one_square_the_first_wins_and_a_placed_tile_never_moves() {
    let mut m = flat();
    for k in 0..3 {
        m.slots
            .push((Affine::translate(mm([2500 + k * 300, 0, 0])), false));
    }
    let mut g = game_on(&m, vec![Driver::Local, Driver::Ai]);
    hand(&mut g, 0, tiles::LEFT, 0);
    hand(&mut g, 1, tiles::RIGHT, 1);
    // both put theirs beside the centre, red first
    for (seat, claw, tile) in [(0, tiles::LEFT, 0), (1, tiles::RIGHT, 1)] {
        g.submit_claim(Claim::Place {
            seat,
            claw,
            tile,
            square: (7, 8),
        });
    }
    let inputs: Vec<SeatInput> = (0..2).map(|k| looking(&g, k)).collect();
    g.step(&inputs);
    assert_eq!(g.placed_at((7, 8)), Some(0));
    assert_eq!(
        g.tiles.all[0].state,
        State::Placed {
            square: (7, 8),
            seat: Some(0),
            order: 1
        }
    );
    assert_eq!(g.crabs[1].claws[tiles::RIGHT], Some(1), "yellow keeps its tile");
    // the placed tile cannot be grabbed, and stays where it is
    let pose = g.tiles.all[0].pose;
    g.submit_claim(Claim::Grab {
        seat: 0,
        claw: tiles::LEFT,
        tile: 0,
    });
    let i = looking(&g, 0);
    run(&mut g, 2000, i);
    assert_eq!(g.tiles.all[0].pose, pose);
    assert_eq!(g.crabs[0].claws[tiles::LEFT], None);
}

#[test]
fn a_tile_must_touch_one_already_placed() {
    let g = tile_ahead(200, 1);
    assert!(g.legal_square((7, 8)) && g.legal_square((6, 7)));
    assert!(!g.legal_square((7, 9)), "two squares off");
    assert!(!g.legal_square((8, 8)), "a corner does not touch");
    assert!(!g.legal_square((7, 7)), "the centre is full");
    assert!(!g.legal_square((7, 15)), "off the board");
}

#[test]
fn a_full_claw_aimed_at_a_legal_square_places_and_elsewhere_drops() {
    let mut m = flat();
    m.slots.push((Affine::translate(mm([2500, 0, 0])), false));
    m.slots.push((Affine::translate(mm([2800, 0, 0])), false));
    m.slots.push((Affine::IDENTITY, false));
    let mut g = game_on(&m, vec![Driver::Local]);
    hand(&mut g, 0, tiles::LEFT, 0);
    // stand on the board two squares from the centre, facing it along -x
    // (yaw -90°): the crosshair falls on (7, 8)
    g.crabs[0].pos = v3(mm1(520), g.rules.board_top, 0);
    let face = SeatInput {
        yaw: -crate::gfx::fx::QUARTER,
        pitch: tuning().look.pitch_start,
        ..Default::default()
    };
    g.step(&[face]);
    assert_eq!(g.ghost(0), Some((tiles::LEFT, 0, (7, 8))));
    let mut put = face;
    put.click[tiles::LEFT] = true;
    g.step(&[put]);
    assert_eq!(g.placed_at((7, 8)), Some(0));
    assert_eq!(g.crabs[0].placed_ms, g.play_ms(), "the idle clock starts again");
    // aimed off the board, a full claw drops its tile
    hand(&mut g, 0, tiles::RIGHT, 1);
    let mut away = face;
    away.yaw = crate::gfx::fx::QUARTER;
    away.click[tiles::RIGHT] = true;
    g.step(&[away]);
    assert_eq!(g.crabs[0].claws, [None, None]);
    assert_eq!(g.tiles.all[1].state, State::Loose { awake: true });
    assert!(g
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Dropped { tile: 1, .. })));
    let i = looking(&g, 0);
    run(&mut g, 2000, i);
    assert_eq!(
        g.tiles.all[1].state,
        State::Loose { awake: false },
        "it falls and settles"
    );
}

#[test]
fn a_remote_referee_queues_claims_and_applies_verdicts() {
    let mut g = tile_ahead(200, 1);
    g.referee = Referee::Remote;
    let i = click(&g, 0, tiles::LEFT);
    g.step(&[i]);
    // nothing happens here until the host says so
    assert_eq!(g.crabs[0].claws, [None, None]);
    let sent = g.take_claims();
    assert_eq!(
        sent,
        vec![Claim::Grab {
            seat: 0,
            claw: tiles::LEFT,
            tile: 0
        }]
    );
    g.apply_verdict(Verdict::Granted(sent[0]));
    assert_eq!(g.crabs[0].claws[tiles::LEFT], Some(0));
    // a host keeps its verdicts for everyone
    let mut h = tile_ahead(200, 2);
    h.record_verdicts = true;
    h.submit_claim(Claim::Grab {
        seat: 1,
        claw: tiles::RIGHT,
        tile: 0,
    });
    let inputs: Vec<SeatInput> = (0..2).map(|k| looking(&h, k)).collect();
    h.step(&inputs);
    assert_eq!(
        h.take_verdicts(),
        vec![Verdict::Granted(Claim::Grab {
            seat: 1,
            claw: tiles::RIGHT,
            tile: 0
        })]
    );
}

#[test]
fn grabbing_the_tile_a_crab_stands_on_drops_the_crab() {
    let mut g = tile_ahead(200, 2);
    // yellow stands on the tile
    let top = g.tiles.all[0].pose.t;
    g.crabs[1].pos = v3(top.x, mm1(45), top.z);
    g.submit_claim(Claim::Grab {
        seat: 0,
        claw: tiles::LEFT,
        tile: 0,
    });
    let inputs: Vec<SeatInput> = (0..2).map(|k| looking(&g, k)).collect();
    g.step(&inputs);
    let inputs: Vec<SeatInput> = (0..2).map(|k| looking(&g, k)).collect();
    for _ in 0..40 {
        g.step(&inputs);
    }
    assert!(
        g.crabs[1].pos.y < mm1(5) && g.crabs[1].on_ground,
        "it falls to the sand"
    );
}

#[test]
fn a_host_drops_a_joiners_tile_where_the_joiner_let_go_and_logs_its_changes_in_order() {
    let mut h = tile_ahead(200, 2);
    h.record_verdicts = true;
    let grab = Claim::Grab {
        seat: 1,
        claw: tiles::LEFT,
        tile: 0,
    };
    h.submit_claim(grab);
    let inputs: Vec<SeatInput> = (0..2).map(|k| looking(&h, k)).collect();
    h.step(&inputs);
    let at_grab = h.play_ms();
    // the joiner let go 1 m east of the middle, 100 mm up
    let there = Affine::translate(mm([1000, 100, 0]));
    h.submit_drop_at(1, tiles::LEFT, there);
    h.submit_claim(Claim::Drop {
        seat: 1,
        claw: tiles::LEFT,
        tile: 0,
    });
    for _ in 0..100 {
        h.step(&inputs);
    }
    let changes = h.take_changes();
    assert_eq!(
        changes[0],
        Change::Judged {
            verdict: Verdict::Granted(grab),
            at_ms: at_grab,
            drop: None
        }
    );
    match &changes[1] {
        Change::Judged {
            verdict: Verdict::Granted(Claim::Drop { .. }),
            drop,
            ..
        } => assert_eq!(*drop, Some(there)),
        other => panic!("{other:?}"),
    }
    // it fell to the sand there, and its rest is logged last
    let t = &h.tiles.all[0];
    assert_eq!((t.pose.t.x, t.pose.t.z), (mm1(1000), 0));
    assert!(matches!(changes.last(), Some(Change::Settled { tile: 0, .. })));
}

#[test]
fn a_joiner_takes_the_hosts_times_and_a_rest_for_a_loose_tile_only() {
    let mut g = tile_ahead(200, 2);
    g.referee = Referee::Remote;
    let grab = Claim::Grab {
        seat: 1,
        claw: tiles::LEFT,
        tile: 0,
    };
    g.apply_judged(Verdict::Granted(grab), 1234, None);
    assert_eq!(
        g.tiles.all[0].since_ms, 1234,
        "the host's time, not this browser's"
    );
    assert_eq!(g.crabs[1].reach[tiles::LEFT], Some((Reach::Grab, 1234)));
    // a rest for a tile held here since is left alone
    let pose = g.tiles.all[0].pose;
    g.apply_settled(0, Affine::translate(mm([5, 5, 5])), true, vec![]);
    assert_eq!(g.tiles.all[0].pose, pose);
    // a rest for a loose one puts it where the host's lies, asleep
    let there = Affine::translate(mm([700, 0, 700]));
    g.apply_judged(
        Verdict::Granted(Claim::Drop {
            seat: 1,
            claw: tiles::LEFT,
            tile: 0,
        }),
        2000,
        Some(Affine::translate(mm([700, 300, 700]))),
    );
    g.apply_settled(0, there, true, vec![]);
    let t = &g.tiles.all[0];
    assert_eq!(
        (t.pose, t.face_down, t.state),
        (there, true, State::Loose { awake: false })
    );
    g.set_penalties(&[-4, -6]);
    assert_eq!((g.crabs[0].penalty, g.crabs[1].penalty), (-4, -6));
}
