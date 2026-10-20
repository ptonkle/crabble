//! The tally's timetable and its panel. Times and points are worked out by
//! hand beside each check.

use super::*;
use crate::app::tests::wait;
use crate::gfx::geom::Affine;
use crate::sim::{State, Tile};
use crate::view::Item;

const STRINGS: &str = include_str!("../../../../../data/strings.json");
const NOTES: &str = include_str!("../../../../../data/patch-notes.json");
const TUNING: &str = include_str!("../../../../../data/tuning.json");
const LOOK: &str = include_str!("../../../../../data/look.json");

fn look(first: i32, line: i32, gap: i32, winner: i32) -> TallyLook {
    TallyLook {
        note: String::new(),
        first_ms: first,
        line_ms: line,
        gap_ms: gap,
        winner_ms: winner,
    }
}

/// Red with CAT on the star (+10), yellow with nothing, blue with ZQ (-40)
/// and 6 idle points lost.
fn three_players() -> Tally {
    let mut b = Board::new();
    b.put((7, 7), 'A', None, 0);
    b.put((7, 6), 'C', Some(0), 1);
    b.put((7, 8), 'T', Some(0), 2);
    b.put((1, 5), 'Z', Some(2), 3);
    b.put((1, 6), 'Q', Some(2), 4);
    score::tally(&b, 3, &[0, 0, 6])
}

#[test]
fn the_reveal_goes_column_by_column_a_line_at_a_time_then_the_winner() {
    // first 1000, a line every 500, 200 between columns, the winner 800
    // after the last line. Red's one line at 1000; red's turn ends 1500 +
    // 200: yellow's starts at 1700 and, with no line, takes 500 + 200;
    // blue's starts at 2400, ZQ at 2400 and the idle line at 2900; the
    // winner at 2900 + 800 = 3700
    let r = Reveal::new(three_players(), Some(&look(1000, 500, 200, 800)), 0);
    assert_eq!(r.times, vec![vec![1000], vec![], vec![2400, 2900]]);
    assert_eq!(r.starts, vec![1000, 1700, 2400]);
    assert_eq!(r.winner_ms, 3700);
    let at = |ms| {
        let s = r.at(ms);
        (s.lines, s.now, s.winner)
    };
    assert_eq!(at(999), (vec![0, 0, 0], None, false));
    assert_eq!(at(1000), (vec![1, 0, 0], Some(0), false));
    assert_eq!(at(1700), (vec![1, 0, 0], Some(1), false));
    assert_eq!(at(2400), (vec![1, 0, 1], Some(2), false));
    assert_eq!(at(3699), (vec![1, 0, 2], Some(2), false));
    assert_eq!(at(3700), (vec![1, 0, 2], None, true));
}

#[test]
fn a_column_counts_up_its_lines_and_the_idle_line_comes_last() {
    let t = three_players();
    let blue = &t.lines[2];
    // ZQ: Z on the triple letter 30 + Q 10 = 40, not a word
    assert_eq!(
        (total_of(blue, 0), total_of(blue, 1), total_of(blue, 2)),
        (0, -40, -46)
    );
    assert_eq!(line_count(blue), 2);
    assert_eq!(line_count(&t.lines[1]), 0);
}

#[test]
fn skip_shows_everything_at_once() {
    let mut r = Reveal::new(three_players(), Some(&look(1000, 500, 200, 800)), 5000);
    assert!(!r.at(100).winner);
    r.skip(5100);
    assert!(r.at(5100 - r.start_ms).winner);
}

/// A Local game with two AI crabs whose board holds the three players'
/// words, run to its end with the shipped look.
fn ended(fast: bool) -> App {
    let mut a = App::new(STRINGS, NOTES).unwrap();
    a.act("data.tuning", TUNING).unwrap();
    a.act("data.look", LOOK).unwrap();
    if fast {
        a.act("flag.fast", "on").unwrap();
    }
    a.act("flag.short", "1").unwrap();
    a.act("local.ai", "2").unwrap();
    a.act("local.start", "").unwrap();
    let tiles = &mut a.play.as_mut().unwrap().game.tiles.all;
    let base = tiles.len() as u16;
    for (k, (letter, square, seat, order)) in [
        ('A', (7, 7), None, 0),
        ('C', (7, 6), Some(0), 1),
        ('T', (7, 8), Some(0), 2),
        ('Z', (1, 5), Some(2), 3),
        ('Q', (1, 6), Some(2), 4),
    ]
    .into_iter()
    .enumerate()
    {
        let value = crate::letters::value(letter).unwrap();
        let mut t = Tile::new(base + k as u16, letter, value, Affine::IDENTITY, false);
        t.state = State::Placed { square, seat, order };
        tiles.push(t);
    }
    // the 1 s game and the 3-2-1
    wait(&mut a, 4100);
    a
}

fn tally_of(a: &App) -> (Vec<crate::view::TallyColumn>, Vec<Item>) {
    let ui = a.ui();
    assert_eq!(ui.phase, "ended");
    let items = ui.panels[0].items.clone();
    let cols = items
        .iter()
        .find_map(|i| match i {
            Item::Tally { columns } => Some(columns.clone()),
            _ => None,
        })
        .expect("a tally");
    (cols, items)
}

fn buttons(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Button { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_end_reveals_each_players_words_in_their_colour_then_the_winner() {
    let mut a = ended(false);
    // the end has just come: nothing shown yet, and Skip
    let (cols, items) = tally_of(&a);
    assert_eq!(cols.len(), 3);
    assert!(cols.iter().all(|c| c.lines.is_empty() && c.total == "0"));
    assert_eq!(buttons(&items), vec!["end.skip"]);
    // the shipped look: first 1200, a line every 450, 600 between: red's
    // CAT shows at 1200
    wait(&mut a, 1300);
    let (cols, _) = tally_of(&a);
    assert_eq!(cols[0].name, "Red (you)");
    assert_eq!(cols[0].tone, "red");
    assert!(cols[0].now);
    assert_eq!(cols[0].lines[0].text, "CAT");
    assert_eq!(cols[0].lines[0].points, "+10");
    assert!(!cols[0].lines[0].minus);
    assert_eq!(cols[0].total, "10");
    assert!(cols[2].lines.is_empty(), "blue's turn is still to come");
    // to the end: blue's ZQ in minus, then its idle line (none lost here,
    // so no idle line), and red wins
    wait(&mut a, 5000);
    let (cols, items) = tally_of(&a);
    assert_eq!(cols[2].name, "Blue");
    assert_eq!(cols[2].lines.len(), 1);
    assert_eq!(
        (cols[2].lines[0].text.as_str(), cols[2].lines[0].points.as_str()),
        ("ZQ", "-40")
    );
    assert!(cols[2].lines[0].minus);
    assert_eq!(cols[2].total, "-40");
    assert!(cols[0].winner && !cols[1].winner && !cols[2].winner);
    assert!(items.contains(&Item::Lines {
        lines: vec!["Red (you) wins".into()]
    }));
    assert_eq!(buttons(&items), vec!["end.title"], "no Rematch in a Local game");
    // each line ticked, then the fanfare
    let keys: Vec<&str> = a.cues.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(&keys[keys.len() - 3..], &["tally", "tally", "fanfare"]);
}

#[test]
fn skip_and_fast_mode_show_the_whole_tally() {
    let mut a = ended(false);
    a.act("end.skip", "").unwrap();
    let (cols, items) = tally_of(&a);
    assert_eq!(cols[0].total, "10");
    assert_eq!(cols[2].total, "-40");
    assert_eq!(buttons(&items), vec!["end.title"]);
    let a = ended(true);
    let (cols, items) = tally_of(&a);
    assert_eq!(cols[2].lines.len(), 1);
    assert_eq!(buttons(&items), vec!["end.title"]);
}

#[test]
fn a_tie_says_so() {
    // nobody placed a word: everyone 0
    let mut a = App::new(STRINGS, NOTES).unwrap();
    a.act("data.tuning", TUNING).unwrap();
    a.act("flag.fast", "on").unwrap();
    a.act("flag.short", "1").unwrap();
    a.act("local.start", "").unwrap();
    wait(&mut a, 1100);
    let (cols, items) = tally_of(&a);
    assert!(cols.iter().all(|c| c.winner));
    assert!(items.contains(&Item::Lines {
        lines: vec!["A tie".into()]
    }));
}
