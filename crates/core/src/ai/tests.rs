use super::plan::{self, Board, Seen};
use super::*;
use crate::gfx::geom::Affine;
use crate::model::mm;
use crate::sim::tests::{flat, game_on, tuning};
use crate::sim::{Driver, Tile};

fn numbers() -> Numbers {
    tuning().ai
}

/// A centre row: `text` placed by `seat` from column `col` of row 7.
fn row7(text: &str, col: usize, seat: Option<usize>) -> Board {
    let line = format!("{}{}", ".".repeat(col), text);
    let mut rows = vec![""; 15];
    rows[7] = &line;
    Board::from_rows(&rows, seat)
}

#[test]
fn the_premiums_are_the_standard_layout() {
    assert_eq!(plan::premium((0, 0)), (1, 3));
    assert_eq!(plan::premium((14, 7)), (1, 3));
    assert_eq!(plan::premium((7, 7)), (1, 2));
    assert_eq!(plan::premium((13, 1)), (1, 2));
    assert_eq!(plan::premium((1, 9)), (3, 1));
    assert_eq!(plan::premium((14, 11)), (2, 1));
    assert_eq!(plan::premium((7, 8)), (1, 1));
    let mut counts = [0; 4];
    for r in 0..15 {
        for c in 0..15 {
            match plan::premium((r, c)) {
                (1, 3) => counts[0] += 1,
                (1, 2) => counts[1] += 1,
                (3, 1) => counts[2] += 1,
                (2, 1) => counts[3] += 1,
                _ => {}
            }
        }
    }
    assert_eq!(counts, [8, 17, 12, 24]);
}

#[test]
fn an_empty_board_has_no_legal_square_and_a_lone_centre_tile_has_four() {
    assert!(Board::default().legal_squares().is_empty());
    let b = row7("A", 7, None);
    assert_eq!(b.legal_squares(), vec![(6, 7), (7, 6), (7, 8), (8, 7)]);
}

#[test]
fn extending_another_crabs_word_is_a_steal_worth_its_score_and_half_theirs() {
    // CAT on 7,6..8 (the centre doubles): (3+1+1) x 2 = 10, seat 1's
    let b = row7("CAT", 6, Some(1));
    let p = plan::rate(&b, (7, 9), 'S', 0, 50);
    assert_eq!(p.words.len(), 1);
    assert_eq!(p.words[0].text, "CATS");
    // CATS: (3+1+1+1) x 2 = 12, and seat 1 loses its 10: 12 + 5
    assert_eq!((p.words[0].score, p.gain, p.real, p.steal), (12, 17, true, true));
    // growing its own word earns only the difference
    let mine = row7("CAT", 6, Some(0));
    let q = plan::rate(&mine, (7, 9), 'S', 0, 50);
    assert_eq!((q.gain, q.steal), (2, false));
}

#[test]
fn a_placement_reads_its_word_across_and_down() {
    // CAT across row 7; an X under its A makes a word down, none across
    let b = row7("CAT", 6, Some(1));
    let p = plan::rate(&b, (8, 7), 'X', 0, 50);
    // A over X: AX, real; the centre doubles: (1 + 8) x 2 = 18
    assert_eq!(
        p.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        vec!["AX"]
    );
    assert_eq!((p.gain, p.real, p.steal), (18, true, false));
    // an S at 6,9 is beside nothing: the T is at 7,8
    assert!(!b.legal((6, 9)));
}

#[test]
fn a_fake_word_is_left_out_unless_nothing_else_will_do() {
    let b = row7("A", 7, None);
    // Q beside an A makes QA or AQ, neither real
    assert_eq!(plan::best(&b, &['Q'], 0, 50, true), None);
    let (k, p) = plan::best(&b, &['Q'], 0, 50, false).unwrap();
    // (10 + 1) x 2 = 22 lost, on the first square in reading order
    assert_eq!((k, p.square, p.real, p.gain), (0, (6, 7), false, -22));
    // with a T as well, the T's real word wins: AT, (1 + 1) x 2
    let (k, p) = plan::best(&b, &['Q', 'T'], 0, 50, false).unwrap();
    assert_eq!((k, p.real, p.known, p.gain), (1, true, true, 4));
}

#[test]
fn a_real_word_players_do_not_know_counts_as_fake_to_the_ai() {
    // TRIAGE F11: AE is in ENABLE but not common, so the AI will not make
    // it, and thinks of it as costing (1 + 1) x 2 (EA is not real at all)
    let b = row7("A", 7, None);
    assert_eq!(plan::best(&b, &['E'], 0, 50, true), None);
    let p = plan::rate(&b, (7, 8), 'E', 0, 50);
    assert_eq!(p.words[0].text, "AE");
    assert_eq!((p.real, p.known, p.gain), (true, false, -4));
    // a word already on the board is read as the referee reads it: seat
    // 1's OE (real, (1 + 1) x 2 = 4) grown into TOE ((1 + 1 + 1) x 2 = 6)
    // is a steal, worth 6 and half of OE's 4
    let p = plan::rate(&row7("OE", 7, Some(1)), (7, 6), 'T', 0, 50);
    assert_eq!(p.words[0].text, "TOE");
    assert_eq!((p.gain, p.real, p.known, p.steal), (8, true, true, true));
}

#[test]
fn the_letter_chosen_is_the_best_word_less_the_walk_to_it() {
    let b = row7("A", 7, None);
    let near = |tile, letter, mm| Seen { tile, letter, mm };
    // X makes AX (18), M makes AM or MA (8), T makes AT (4)
    let p = plan::pick(
        &b,
        &[near(0, 'T', 200), near(1, 'M', 200), near(2, 'X', 300)],
        0,
        50,
        3,
    )
    .unwrap();
    assert_eq!((p.tile, p.letter), (2, 'X'));
    assert_eq!(p.plan.map(|q| (q.square, q.gain)), Some(((7, 8), 18)));
    // the X 4 m further: 18 - 12.9 < 8 - 0.6, so the M
    let p = plan::pick(&b, &[near(1, 'M', 200), near(2, 'X', 4300)], 0, 50, 3).unwrap();
    assert_eq!(p.letter, 'M');
}

#[test]
fn with_no_word_to_make_it_fetches_the_commonest_letter() {
    let b = Board::default();
    let seen = [
        Seen {
            tile: 0,
            letter: 'Q',
            mm: 100,
        },
        Seen {
            tile: 1,
            letter: 'T',
            mm: 100,
        },
        Seen {
            tile: 2,
            letter: 'E',
            mm: 500,
        },
    ];
    let p = plan::pick(&b, &seen, 0, 50, 3).unwrap();
    // twelve Es in a set, six Ts, one Q
    assert_eq!((p.tile, p.plan), (2, None));
    assert_eq!(plan::pick(&b, &[], 0, 50, 3), None);
}

// ---- the brain on a game (Stage 1: clicks do nothing yet) -------------

fn tile(id: u16, letter: char, at_mm: [i32; 3], state: State) -> Tile {
    let value = crate::letters::value(letter).unwrap_or(0);
    let mut t = Tile::new(id, letter, value, Affine::translate(mm(at_mm)), false);
    t.state = state;
    t
}

const LOOSE: State = State::Loose { awake: false };

/// One AI crab at its corner with an A on the centre square and a small
/// pile of X, B, E off the board's left edge.
fn a_game() -> Match {
    let mut g = game_on(&flat(), vec![Driver::Ai]);
    g.tiles.all = vec![
        tile(
            0,
            'A',
            [0, 40, 0],
            State::Placed {
                square: (7, 7),
                seat: None,
                order: 0,
            },
        ),
        tile(1, 'B', [-2200, 0, 200], LOOSE),
        tile(2, 'X', [-2200, 0, 0], LOOSE),
        tile(3, 'E', [-2200, 0, -200], LOOSE),
    ];
    g
}

/// Runs the AI for `ms`, calling `each` before every step with the game,
/// the brain and its input.
fn play(g: &mut Match, b: &mut Brain, ms: i64, mut each: impl FnMut(&Match, &Brain, &SeatInput)) {
    for _ in 0..ms / g.rules.tick_ms {
        let i = think(g, 0, b);
        each(g, b, &i);
        g.step(&[i]);
    }
}

#[test]
fn the_ai_walks_to_a_pile_and_thinks_before_it_chooses_the_letter_for_its_best_word() {
    let mut g = a_game();
    let mut b = Brain::new(numbers(), 5);
    let mut arrived = None;
    let mut most_turn = 0;
    let mut last_yaw = None;
    play(&mut g, &mut b, 12_000, |g, b, i| {
        if arrived.is_none() && matches!(b.doing, Doing::Choose { .. }) {
            arrived = Some(g.play_ms());
        }
        if let Some(y) = last_yaw {
            most_turn = most_turn.max(fx::wrap(i.yaw - y).abs());
        }
        last_yaw = Some(i.yaw);
    });
    let arrived = arrived.expect("it reached the pile");
    let (chose, tile) = b
        .thoughts
        .iter()
        .find_map(|t| match t {
            Thought::Chose { ms, tile, .. } => Some((*ms, *tile)),
            _ => None,
        })
        .expect("it chose");
    // X makes AX, the best word
    assert_eq!(tile, 2);
    // three letters in view: (800 + 3 x 120) + 1000 ms, each part 70% to 130%
    let thought = chose - arrived;
    assert!((1512..=2828).contains(&thought), "{thought} ms");
    // the pile is 1.8 m from its corner, and it stops 0.47 m short (two
    // thirds of what it sees): over 3 s at 394 mm/s, with its turn
    assert!(arrived >= 2800, "{arrived}");
    // 240 degrees a second is 4.8 degrees (874 units) a 20 ms tick
    assert!(most_turn <= 874, "{most_turn}");
}

#[test]
fn the_ai_clicks_a_claw_only_with_the_crosshair_on_its_tile_and_grabs_it() {
    let mut g = a_game();
    // an aim that starts up to 40 cm off, two tiles' width
    let mut n = numbers();
    n.aim_error_mm = 400;
    n.aim_settle_ms = 400;
    let mut b = Brain::new(n, 5);
    let mut first = None;
    play(&mut g, &mut b, 14_000, |g, _, i| {
        if first.is_none() && i.click != [false; 2] {
            // by hand: the camera's centre ray meets the X's top (45 mm
            // up) inside its 200 mm square
            let (eye, fwd) = g.camera(0);
            let x = g.tiles.all[2].pose.t.add(v3(0, mm1(45), 0));
            let s = fx::div(x.y - eye.y, fwd.y);
            let off = eye.add(fwd.scale(s)).sub(x);
            first = Some((i.click, off.x.abs() <= mm1(100) && off.z.abs() <= mm1(100)));
        }
    });
    assert_eq!(first, Some(([true, false], true)));
    assert_eq!(g.crabs[0].claws[0], Some(2), "the X is in its left claw");
}

/// The game with tile `letter` in crab 0's left claw, near the board.
fn holding(letter: char) -> Match {
    let mut g = a_game();
    g.tiles.all[2].letter = letter;
    g.tiles.all[2].state = State::Held { seat: 0, claw: 0 };
    g.crabs[0].claws[0] = Some(2);
    g
}

#[test]
fn holding_a_letter_with_a_word_it_goes_to_the_board_thinks_and_places_it_there() {
    let mut g = holding('X');
    let mut b = Brain::new(numbers(), 9);
    let (mut settled, mut chose) = (None, None);
    play(&mut g, &mut b, 15_000, |g, b, _| {
        if settled.is_none() && matches!(b.doing, Doing::Settle { .. }) {
            settled = Some(g.play_ms());
        }
        if chose.is_none() && matches!(b.doing, Doing::Place { .. }) {
            chose = Some(g.play_ms());
        }
    });
    let settled = settled.expect("it reached the board");
    let chose = chose.expect("it chose a square");
    let placing = b.thoughts.iter().find_map(|t| match t {
        Thought::Placing {
            ms,
            square,
            letter,
            real,
            steal,
        } => Some((*ms, *square, *letter, *real, *steal)),
        _ => None,
    });
    let (ms, square, letter, real, steal) = placing.expect("it clicked to place");
    // AX across at 7,8 (18 points; AX down at 8,7 ties, after it in reading order)
    assert_eq!((square, letter, real, steal), ((7, 8), 'X', true, false));
    // 900 ms to settle, at least 70% of it
    assert!(chose - settled >= 630, "{} ms", chose - settled);
    assert!(ms >= chose);
}

#[test]
fn a_letter_with_no_word_waits_for_another_unless_it_is_late() {
    // a Q makes no real word beside the A: it goes for a second letter
    let mut g = holding('Q');
    g.tiles.all.push(tile(4, 'U', [-2200, 0, 400], LOOSE));
    let mut b = Brain::new(numbers(), 9);
    play(&mut g, &mut b, 1000, |_, _, _| {});
    assert!(matches!(b.doing, Doing::ToPile { .. }), "{:?}", b.doing);
    // 25 s without placing (past hurry_ms, 20 s): it places the Q anyway
    let mut g = holding('Q');
    g.crabs[0].placed_ms = -25_000;
    let mut b = Brain::new(numbers(), 9);
    play(&mut g, &mut b, 15_000, |_, _, _| {});
    let fake = b.thoughts.iter().any(|t| {
        matches!(
            t,
            Thought::Placing {
                letter: 'Q',
                real: false,
                ..
            }
        )
    });
    assert!(fake, "{:?}", b.thoughts);
}

#[test]
fn two_brains_with_one_seed_send_the_same_inputs() {
    let run = || {
        let mut g = a_game();
        let mut b = Brain::new(numbers(), 77);
        let mut all = Vec::new();
        play(&mut g, &mut b, 8000, |_, _, i| all.push(*i));
        all
    };
    assert_eq!(run(), run());
}

#[test]
fn an_ai_number_out_of_range_is_refused_by_name() {
    let mut n = numbers();
    n.jitter_pct = 95;
    assert_eq!(
        n.check().unwrap_err(),
        "The tuning file's ai.jitter_pct is 95; it must be 0 to 90."
    );
}
