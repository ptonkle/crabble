//! Every expected value here is worked out by hand from the standard
//! layout and letter values, written beside it.

use super::*;

const RED: usize = 0;
const YELLOW: usize = 1;
const BLUE: usize = 2;

/// The centre tile, A, placed by no one.
fn centre_a() -> Board {
    let mut b = Board::new();
    b.put((7, 7), 'A', None, 0);
    b
}

fn found(b: &Board) -> Vec<(String, usize, bool, i32)> {
    words(b)
        .into_iter()
        .map(|w| (w.letters, w.word.owner, w.word.real, w.points))
        .collect()
}

#[test]
fn the_premium_layout_is_the_standard_one() {
    use Premium::*;
    let mut counts = [0; 4];
    for r in 0..SQUARES {
        for c in 0..SQUARES {
            match premium((r, c)) {
                Some(DoubleLetter) => counts[0] += 1,
                Some(TripleLetter) => counts[1] += 1,
                Some(DoubleWord) => counts[2] += 1,
                Some(TripleWord) => counts[3] += 1,
                None => {}
            }
        }
    }
    // 24 double letter, 12 triple letter, 17 double word (with the
    // centre star), 8 triple word
    assert_eq!(counts, [24, 12, 17, 8]);
    assert_eq!(premium((7, 7)), Some(DoubleWord));
    assert_eq!(premium((0, 0)), Some(TripleWord));
    assert_eq!(premium((14, 7)), Some(TripleWord));
    assert_eq!(premium((13, 9)), Some(TripleLetter));
    assert_eq!(premium((11, 14)), Some(DoubleLetter));
    assert_eq!(premium((12, 12)), Some(DoubleWord));
    assert_eq!(premium((7, 8)), None);
    assert_eq!(premium((15, 0)), None);
}

#[test]
fn a_word_through_the_centre_star_is_doubled_for_its_owner() {
    // red puts C left of the centre A, then T right of it: CAT,
    // C 3 + A 1 + T 1 = 5, the star doubles it: 10
    let mut b = centre_a();
    b.put((7, 6), 'C', Some(RED), 1);
    b.put((7, 8), 'T', Some(RED), 2);
    assert_eq!(found(&b), vec![("CAT".into(), RED, true, 10)]);
    let t = tally(&b, 2, &[]);
    assert_eq!((t.lines[0].total, t.lines[1].total), (10, 0));
    assert_eq!(t.winners, vec![RED]);
}

#[test]
fn extending_a_word_hands_it_to_whoever_added_the_last_letter() {
    // yellow adds S to red's CAT: CATS is the only word, 6 x 2 = 12, and
    // it is yellow's; CAT is no longer a word of its own
    let mut b = centre_a();
    b.put((7, 6), 'C', Some(RED), 1);
    b.put((7, 8), 'T', Some(RED), 2);
    b.put((7, 9), 'S', Some(YELLOW), 3);
    assert_eq!(found(&b), vec![("CATS".into(), YELLOW, true, 12)]);
    let t = tally(&b, 2, &[]);
    assert_eq!((t.lines[0].total, t.lines[1].total), (0, 12));
    assert!(t.lines[0].words.is_empty());
}

#[test]
fn the_owner_is_whoever_placed_the_latest_tile_not_the_last_square() {
    // row 10 from the double word at (10, 4): red puts C and T first,
    // yellow the A between them last. CAT = 5 x 2 = 10, yellow's
    let mut b = Board::new();
    b.put((10, 4), 'C', Some(RED), 1);
    b.put((10, 6), 'T', Some(RED), 2);
    b.put((10, 5), 'A', Some(YELLOW), 3);
    assert_eq!(found(&b), vec![("CAT".into(), YELLOW, true, 10)]);
}

#[test]
fn a_shared_letter_counts_in_both_words_and_so_does_the_star_under_it() {
    // red's CAT along row 7 and blue's BAT down column 7 share the centre
    // A on the star: each 5 x 2 = 10
    let mut b = centre_a();
    b.put((7, 6), 'C', Some(RED), 1);
    b.put((7, 8), 'T', Some(RED), 2);
    b.put((6, 7), 'B', Some(BLUE), 3);
    b.put((8, 7), 'T', Some(BLUE), 4);
    assert_eq!(
        found(&b),
        vec![("CAT".into(), RED, true, 10), ("BAT".into(), BLUE, true, 10)]
    );
    let t = tally(&b, 3, &[]);
    let totals: Vec<i32> = t.lines.iter().map(|l| l.total).collect();
    assert_eq!(totals, vec![10, 0, 10]);
    // a tie: both win
    assert_eq!(t.winners, vec![RED, BLUE]);
}

#[test]
fn a_triple_word_corner_triples_both_words_through_it() {
    // red's CAT along row 0 from the corner: 5 x 3 = 15; yellow's COX
    // down column 0 through the same C: 3 + 1 + 8 = 12, x 3 = 36
    let mut b = Board::new();
    b.put((0, 0), 'C', Some(RED), 1);
    b.put((0, 1), 'A', Some(RED), 2);
    b.put((0, 2), 'T', Some(RED), 3);
    b.put((1, 0), 'O', Some(YELLOW), 4);
    b.put((2, 0), 'X', Some(YELLOW), 5);
    assert_eq!(
        found(&b),
        vec![("CAT".into(), RED, true, 15), ("COX".into(), YELLOW, true, 36)]
    );
}

#[test]
fn a_fake_word_costs_its_owner_its_full_score_letter_premium_included() {
    // ZQ along row 1: Z on the triple letter (1, 5) is 30, Q 10: 40, so
    // red loses 40
    let mut b = Board::new();
    b.put((1, 5), 'Z', Some(RED), 1);
    b.put((1, 6), 'Q', Some(RED), 2);
    assert_eq!(found(&b), vec![("ZQ".into(), RED, false, 40)]);
    let t = tally(&b, 1, &[]);
    assert_eq!(t.lines[0].total, -40);
    assert_eq!(t.lines[0].words[0].signed(), -40);
}

#[test]
fn word_premiums_multiply_together() {
    // eight As down column 0, rows 0 to 7: triple word at (0, 0) and
    // (7, 0), double letter at (3, 0). Letters 8 + 1 = 9, x 3 x 3 = 81;
    // not a word, so blue loses 81
    let mut b = Board::new();
    for r in 0..8 {
        b.put((r, 0), 'A', Some(BLUE), r as u32 + 1);
    }
    assert_eq!(found(&b), vec![("AAAAAAAA".into(), BLUE, false, 81)]);
}

#[test]
fn the_idle_penalty_comes_off_each_players_total() {
    // red's CAT on the star (10) less 14 idle points: -4; yellow has no
    // word and lost 6: -6; red still wins
    let mut b = centre_a();
    b.put((7, 6), 'C', Some(RED), 1);
    b.put((7, 8), 'T', Some(RED), 2);
    let t = tally(&b, 2, &[14, 6]);
    assert_eq!((t.lines[0].idle, t.lines[0].total), (14, -4));
    assert_eq!((t.lines[1].idle, t.lines[1].total), (6, -6));
    assert_eq!(t.winners, vec![RED]);
}

#[test]
fn a_players_words_come_in_the_order_they_claimed_them() {
    // red claims ZQ (fake) at placement 4, CAT at 2: CAT first
    let mut b = centre_a();
    b.put((7, 6), 'C', Some(RED), 1);
    b.put((7, 8), 'T', Some(RED), 2);
    b.put((1, 5), 'Z', Some(RED), 3);
    b.put((1, 6), 'Q', Some(RED), 4);
    let t = tally(&b, 1, &[]);
    let order: Vec<&str> = t.lines[0].words.iter().map(|w| w.letters.as_str()).collect();
    assert_eq!(order, vec!["CAT", "ZQ"]);
    // 10 - 40
    assert_eq!(t.lines[0].total, -30);
}

#[test]
fn a_lone_tile_is_no_word() {
    let b = centre_a();
    assert!(words(&b).is_empty());
    let t = tally(&b, 2, &[]);
    assert_eq!(t.winners, vec![RED, YELLOW]);
}

#[test]
fn the_board_is_read_from_the_games_placed_tiles_only() {
    use crate::gfx::geom::Affine;
    use crate::sim::{Tile, Tiles};
    let tile = |id: u16, letter: char, value: i32, state: State| Tile {
        id,
        letter,
        value,
        state,
        pose: Affine::IDENTITY,
        vy: 0,
        rests_on: Vec::new(),
        awake_ticks: 0,
        since_ms: 0,
    };
    let placed = |square, seat, order| State::Placed { square, seat, order };
    let tiles = Tiles {
        all: vec![
            tile(0, 'A', 1, placed((7, 7), None, 0)),
            tile(1, 'X', 8, placed((7, 8), Some(YELLOW), 1)),
            // held and loose tiles are not on the board
            tile(2, 'E', 1, State::Held { seat: RED, claw: 0 }),
            tile(3, 'S', 1, State::Loose { awake: false }),
        ],
    };
    let b = Board::from_tiles(&tiles);
    // AX: (1 + 8) x 2 on the star = 18, yellow's
    assert_eq!(found(&b), vec![("AX".into(), YELLOW, true, 18)]);
    assert_eq!(b.get((7, 9)), None);
    assert_eq!(b.get((7, 6)), None);
}
