//! The AI's planning: a model of the board as the crab last saw it, the
//! words a letter would make on each square it could go to, what that is
//! worth to the crab, and which of the letters in view to fetch.
//!
//! What a placement is worth is the change in the crab's own standing
//! (BRIEF.md, Scoring): every word it makes becomes its own (a real word
//! scores, a fake one costs), and every word it grows stops being whoever
//! had it. A real word taken from another crab is worth `steal_pct` percent
//! of that word's score on top, as that crab loses it (a steal).
//!
//! The premium layout is the score's ([`crate::score::premium`]), so the AI
//! plans by the rules the tally counts. A word already on the board is read
//! as the referee reads it ([`crate::words::is_real`], ENABLE; the board's
//! marks show it). A word the crab would make counts as real to it only
//! when it is also common ([`crate::words::is_common`]): the AI makes only
//! words players know (TRIAGE F11; the person: "I guess let's just plan to
//! cut them from the AI"). A placement's `real` is still the referee's.

use crate::letters;
use crate::sim::{Square, State, Tiles};

/// Squares to a side.
pub const N: i32 = 15;

/// A letter on the board: whose it is (none for the centre tile) and the
/// how-manyth placement of the game it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub letter: char,
    pub seat: Option<usize>,
    pub order: u32,
}

/// The board as the crab knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    cells: Vec<Option<Cell>>,
    placed: u32,
}

impl Default for Board {
    fn default() -> Board {
        Board {
            cells: vec![None; (N * N) as usize],
            placed: 0,
        }
    }
}

fn on(sq: Square) -> bool {
    (0..N).contains(&sq.0) && (0..N).contains(&sq.1)
}

/// The two ways a word runs: along a row (across) and down a column.
const WAYS: [Square; 2] = [(0, 1), (1, 0)];

impl Board {
    /// The tiles placed in a game.
    pub fn from_tiles(tiles: &Tiles) -> Board {
        let mut b = Board::default();
        for t in &tiles.all {
            if let State::Placed { square, seat, order } = t.state {
                if on(square) {
                    b.cells[(square.0 * N + square.1) as usize] = Some(Cell {
                        letter: t.letter,
                        seat,
                        order,
                    });
                    b.placed = b.placed.max(order + 1);
                }
            }
        }
        b
    }

    /// A board from rows of text, for tests: `.` is empty, a capital is a
    /// letter placed by `seat` (`None` for none), in reading order.
    pub fn from_rows(rows: &[&str], seat: Option<usize>) -> Board {
        let mut b = Board::default();
        for (r, line) in rows.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                if ch.is_ascii_uppercase() {
                    b.put((r as i32, c as i32), ch, seat);
                }
            }
        }
        b
    }

    pub fn get(&self, sq: Square) -> Option<Cell> {
        if !on(sq) {
            return None;
        }
        self.cells[(sq.0 * N + sq.1) as usize]
    }

    /// Places `letter` on `sq` for `seat`, as the latest placement.
    pub fn put(&mut self, sq: Square, letter: char, seat: Option<usize>) {
        if on(sq) {
            self.cells[(sq.0 * N + sq.1) as usize] = Some(Cell {
                letter,
                seat,
                order: self.placed,
            });
            self.placed += 1;
        }
    }

    /// Whether a tile may go on `sq`: on the board, empty, and beside a
    /// placed tile (BRIEF.md, Board).
    pub fn legal(&self, sq: Square) -> bool {
        on(sq)
            && self.get(sq).is_none()
            && [(-1, 0), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|(dr, dc)| self.get((sq.0 + dr, sq.1 + dc)).is_some())
    }

    /// Every legal square, row by row.
    pub fn legal_squares(&self) -> Vec<Square> {
        (0..N)
            .flat_map(|r| (0..N).map(move |c| (r, c)))
            .filter(|sq| self.legal(*sq))
            .collect()
    }

    /// The squares of the run of letters through `sq` along `way`, with
    /// `sq` counted as filled.
    fn run(&self, sq: Square, way: Square) -> Vec<Square> {
        let filled = |s: Square| s == sq || self.get(s).is_some();
        let mut from = sq;
        while filled((from.0 - way.0, from.1 - way.1)) && on((from.0 - way.0, from.1 - way.1)) {
            from = (from.0 - way.0, from.1 - way.1);
        }
        let mut out = Vec::new();
        let mut at = from;
        while on(at) && filled(at) {
            out.push(at);
            at = (at.0 + way.0, at.1 + way.1);
        }
        out
    }

    /// A run of placed squares as a word: its letters, its score and its
    /// owner (whoever placed its latest letter). `extra` is a letter not
    /// yet placed, on its square, owned by `me`.
    fn word(&self, squares: &[Square], extra: Option<(Square, char, usize)>) -> Word {
        let mut text = String::new();
        let (mut sum, mut times) = (0, 1);
        let mut latest: Option<(u32, Option<usize>)> = None;
        for sq in squares {
            let (letter, seat, order) = match extra {
                Some((s, l, me)) if s == *sq => (l, Some(me), u32::MAX),
                _ => {
                    let c = self.get(*sq).expect("a run holds placed squares");
                    (c.letter, c.seat, c.order)
                }
            };
            text.push(letter);
            let (lm, wm) = premium(*sq);
            sum += letters::value(letter).unwrap_or(0) * lm;
            times *= wm;
            if latest.is_none_or(|(o, _)| order > o) {
                latest = Some((order, seat));
            }
        }
        Word {
            real: crate::words::is_real(&text),
            common: crate::words::is_common(&text),
            text,
            score: sum * times,
            owner: latest.and_then(|(_, s)| s),
        }
    }
}

/// A word on the board, or one a placement would make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub score: i32,
    pub real: bool,
    /// in the AI's vocabulary (real and common)
    pub common: bool,
    pub owner: Option<usize>,
}

/// What a word is worth to seat `me` at the end, if `real`: its score to
/// its owner (lost if it is fake), and to anyone else `steal_pct` percent
/// of what its owner gets, the other way.
fn worth(w: &Word, real: bool, me: usize, steal_pct: i32) -> i32 {
    let s = if real { w.score } else { -w.score };
    match w.owner {
        Some(o) if o == me => s,
        Some(_) => -s * steal_pct / 100,
        None => 0,
    }
}

/// One letter on one square, rated for the crab placing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub square: Square,
    pub letter: char,
    /// the words it makes (two letters or more), across first
    pub words: Vec<Word>,
    /// its worth to the crab (points)
    pub gain: i32,
    /// every word it makes is real (the referee's view)
    pub real: bool,
    /// every word it makes is in the AI's vocabulary (the crab's view)
    pub known: bool,
    /// it grows a real word another crab owns
    pub steal: bool,
}

/// `letter` on `sq` for seat `me`. `sq` must be legal.
pub fn rate(b: &Board, sq: Square, letter: char, me: usize, steal_pct: i32) -> Placement {
    let mut words = Vec::new();
    let mut gain = 0;
    let mut steal = false;
    for way in WAYS {
        let run = b.run(sq, way);
        if run.len() < 2 {
            continue;
        }
        let made = b.word(&run, Some((sq, letter, me)));
        gain += worth(&made, made.common, me, steal_pct);
        // the words it grows: the runs on either side of the square
        let at = run.iter().position(|s| *s == sq).unwrap_or(0);
        for piece in [&run[..at], &run[at + 1..]] {
            if piece.len() >= 2 {
                let old = b.word(piece, None);
                gain -= worth(&old, old.real, me, steal_pct);
                steal |= old.real && old.owner.is_some_and(|o| o != me);
            }
        }
        words.push(made);
    }
    Placement {
        square: sq,
        letter,
        real: words.iter().all(|w| w.real),
        known: words.iter().all(|w| w.common),
        words,
        gain,
        steal,
    }
}

/// The best placement of any of `letters` for seat `me` whose every word
/// it knows (the only placements the AI makes, in a hurry too: the person,
/// "ai is still playing bs words"), and which of them it uses: the highest
/// gain, the first square in reading order on a tie. With two letters, a
/// placement also counts what the other letter would then make (a letter
/// that opens a word for the other goes first).
pub fn best(b: &Board, letters: &[char], me: usize, steal_pct: i32) -> Option<(usize, Placement)> {
    choose(b, letters, me, steal_pct).map(|(k, p, _)| (k, p))
}

/// [`best`], less `away(square)` thousandths of a point for each square
/// (the walk to it): a near word before a far one a little better. And
/// `open_pts` points more for each square next to it that some letter could
/// still take with a word it knows (see [`open_after`]), so it keeps the
/// board open rather than close it up.
pub fn best_from(
    b: &Board,
    letters: &[char],
    me: usize,
    steal_pct: i32,
    away: &dyn Fn(Square) -> i64,
    open_pts: i32,
) -> Option<(usize, Placement)> {
    choose_from(b, letters, me, steal_pct, away, open_pts).map(|(k, p, _)| (k, p))
}

/// With `p` placed, how many of the empty squares beside it (on its four
/// sides, and past each end of the words it makes) some letter could still
/// take with every word one it knows.
pub fn open_after(b: &Board, p: &Placement, me: usize) -> i32 {
    let mut after = b.clone();
    after.put(p.square, p.letter, Some(me));
    let (r, c) = p.square;
    let mut near: Vec<Square> = vec![(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)];
    for way in WAYS {
        let run = after.run(p.square, way);
        if let (Some(first), Some(last)) = (run.first(), run.last()) {
            near.push((first.0 - way.0, first.1 - way.1));
            near.push((last.0 + way.0, last.1 + way.1));
        }
    }
    near.sort();
    near.dedup();
    near.iter()
        .filter(|sq| after.legal(**sq))
        .filter(|sq| ('A'..='Z').any(|l| rate(&after, **sq, l, me, 0).known))
        .count() as i32
}

/// [`best`], with the gain it counted (its own and the other letter's after
/// it).
fn choose(b: &Board, letters: &[char], me: usize, steal_pct: i32) -> Option<(usize, Placement, i32)> {
    choose_from(b, letters, me, steal_pct, &|_| 0, 0)
}

fn choose_from(
    b: &Board,
    letters: &[char],
    me: usize,
    steal_pct: i32,
    away: &dyn Fn(Square) -> i64,
    open_pts: i32,
) -> Option<(usize, Placement, i32)> {
    let mut out: Option<(usize, Placement, i32, i64)> = None;
    for sq in b.legal_squares() {
        for (k, l) in letters.iter().enumerate() {
            let p = rate(b, sq, *l, me, steal_pct);
            if !p.known {
                continue;
            }
            let mut total = p.gain;
            if letters.len() == 2 {
                total += then(b, &p, letters[1 - k], me, steal_pct).unwrap_or(0);
            }
            let mut worth = total as i64 * 1000 - away(sq);
            if open_pts > 0 {
                worth += open_pts as i64 * 1000 * open_after(b, &p, me) as i64;
            }
            let better = match &out {
                None => true,
                Some((_, _, _, w)) => worth > *w,
            };
            if better {
                out = Some((k, p, total, worth));
            }
        }
    }
    out.map(|(k, p, t, _)| (k, p, t))
}

/// The gain of the best known placement of `letter` once `p` is on the
/// board.
fn then(b: &Board, p: &Placement, letter: char, me: usize, steal_pct: i32) -> Option<i32> {
    let mut after = b.clone();
    after.put(p.square, p.letter, Some(me));
    choose(&after, &[letter], me, steal_pct).map(|(_, q, _)| q.gain)
}

/// A loose tile the crab can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    pub tile: crate::sim::TileId,
    pub letter: char,
    /// how far it is from the crab, mm, a climb counted three times
    pub mm: i32,
}

/// The tile to fetch, and the word plan that chose it (none when no letter
/// in view makes a real word now).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    pub tile: crate::sim::TileId,
    pub letter: char,
    pub plan: Option<Placement>,
}

/// Which of the tiles in view to fetch for seat `me`, holding `held`: the
/// one whose best known placement is worth most, less `walk_cost` points a
/// metre to it. A held letter with no word of its own counts too: a tile
/// whose word then opens one for it is worth both. When none makes a word
/// it knows, the commonest letter (the most of it in a set, so the
/// likeliest to fit later), nearest first.
pub fn pick(
    b: &Board,
    seen: &[Seen],
    me: usize,
    steal_pct: i32,
    walk_cost: i32,
    held: &[char],
) -> Option<Pick> {
    // a held letter with no word of its own waits on the one fetched
    let stuck: Vec<char> = held
        .iter()
        .copied()
        .filter(|h| best(b, &[*h], me, steal_pct).is_none())
        .take(1)
        .collect();
    // the best plan for each letter (and what it is worth), worked out once
    let mut plans: Vec<(char, Option<(Placement, i32)>)> = Vec::new();
    for s in seen {
        if !plans.iter().any(|(l, _)| *l == s.letter) {
            let mut ls = vec![s.letter];
            ls.extend(&stuck);
            let plan = choose(b, &ls, me, steal_pct).filter(|(k, _, _)| *k == 0);
            plans.push((s.letter, plan.map(|(_, p, t)| (p, t))));
        }
    }
    let plan_of = |l: char| plans.iter().find(|(x, _)| *x == l).and_then(|(_, p)| p.clone());
    // points a thousand times over, so a metre's cost can be a fraction
    let value = |s: &Seen| -> Option<i64> {
        plan_of(s.letter).map(|(_, t)| t as i64 * 1000 - walk_cost as i64 * s.mm as i64)
    };
    let planned = seen
        .iter()
        .filter_map(|s| value(s).map(|v| (v, s)))
        .filter(|(v, _)| *v > 0)
        .fold(None::<(i64, &Seen)>, |acc, (v, s)| match acc {
            Some((a, _)) if a >= v => acc,
            _ => Some((v, s)),
        });
    if let Some((_, s)) = planned {
        return Some(Pick {
            tile: s.tile,
            letter: s.letter,
            plan: plan_of(s.letter).map(|(p, _)| p),
        });
    }
    let common = |l: char| {
        letters::COUNTS
            .iter()
            .find(|(c, _)| *c == l)
            .map_or(0, |(_, n)| *n)
    };
    seen.iter()
        .max_by_key(|s| (common(s.letter), -s.mm, -(s.tile as i32)))
        .map(|s| Pick {
            tile: s.tile,
            letter: s.letter,
            plan: None,
        })
}

/// A square's premium as (letter times, word times), from the score's
/// layout ([`crate::score::premium`]).
pub fn premium(sq: Square) -> (i32, i32) {
    use crate::score::Premium;
    match crate::score::premium(sq) {
        Some(Premium::DoubleLetter) => (2, 1),
        Some(Premium::TripleLetter) => (3, 1),
        Some(Premium::DoubleWord) => (1, 2),
        Some(Premium::TripleWord) => (1, 3),
        None => (1, 1),
    }
}
