//! The score at the end of a game (BRIEF.md, Words and ownership, Scoring,
//! Anti-camping): pure, from the final board.
//!
//! - A **word** is a maximal run of two or more letters along a row or a
//!   column ([`words`]).
//! - Its **owner** is the player who placed its latest tile (by place
//!   order; the centre tile belongs to no one, SECOND-ORDER R1.11).
//! - It scores Scrabble's way: each letter's value, times 2 or 3 on a
//!   double or triple letter square, summed, then times 2 or 3 for each
//!   double or triple word square under it. A premium square counts for
//!   every word through it, all game (Q7); the centre star is a double word
//!   square (R1.10).
//! - A real word ([`crate::words::is_real`]) adds its score to its owner's
//!   total; a word that is not real takes the same score away (Q8).
//! - Each player's idle penalty is taken from their total.
//!
//! The board comes from the game's tiles through [`Board::from_tiles`], the
//! one door from the simulation to the tally.

use crate::letters;
use crate::marks::{self, SQUARES};
use crate::sim::{Square, State, Tiles};

/// A premium square's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Premium {
    DoubleLetter,
    TripleLetter,
    DoubleWord,
    TripleWord,
}

impl Premium {
    /// The label stamped on the square (`data/models/board.json`).
    pub fn label(self) -> &'static str {
        match self {
            Premium::DoubleLetter => "2L",
            Premium::TripleLetter => "3L",
            Premium::DoubleWord => "2W",
            Premium::TripleWord => "3W",
        }
    }
}

/// One quarter of the standard layout, (row, column) in the top-left
/// quarter including the middle row and column; the rest is its mirror.
const QUARTER: [(Premium, &[(i32, i32)]); 4] = [
    (Premium::TripleWord, &[(0, 0), (0, 7), (7, 0)]),
    (Premium::DoubleWord, &[(1, 1), (2, 2), (3, 3), (4, 4), (7, 7)]),
    (Premium::TripleLetter, &[(1, 5), (5, 1), (5, 5)]),
    (
        Premium::DoubleLetter,
        &[(0, 3), (3, 0), (2, 6), (6, 2), (3, 7), (7, 3), (6, 6)],
    ),
];

/// The premium on a square of the standard 15 x 15 layout, if any.
pub fn premium(square: Square) -> Option<Premium> {
    let last = SQUARES - 1;
    let fold = |v: i32| v.min(last - v);
    let (r, c) = (fold(square.0), fold(square.1));
    if !(0..SQUARES).contains(&square.0) || !(0..SQUARES).contains(&square.1) {
        return None;
    }
    QUARTER
        .iter()
        .find(|(_, cells)| cells.contains(&(r, c)))
        .map(|(p, _)| *p)
}

/// A placed tile as the tally sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub letter: char,
    pub value: i32,
    /// who placed it (`None` for the centre tile)
    pub seat: Option<usize>,
    /// the how-manyth placement of the game it was
    pub order: u32,
}

/// The board's placed tiles, square by square.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    cells: Vec<Option<Cell>>,
}

impl Default for Board {
    fn default() -> Board {
        Board::new()
    }
}

impl Board {
    pub fn new() -> Board {
        Board {
            cells: vec![None; (SQUARES * SQUARES) as usize],
        }
    }

    /// The board as the game's tiles leave it: every placed tile.
    pub fn from_tiles(tiles: &Tiles) -> Board {
        let mut b = Board::new();
        for t in &tiles.all {
            if let State::Placed { square, seat, order } = t.state {
                b.set(
                    square,
                    Cell {
                        letter: t.letter,
                        value: t.value,
                        seat,
                        order,
                    },
                );
            }
        }
        b
    }

    fn index(square: Square) -> Option<usize> {
        let ok = (0..SQUARES).contains(&square.0) && (0..SQUARES).contains(&square.1);
        ok.then_some((square.0 * SQUARES + square.1) as usize)
    }

    pub fn set(&mut self, square: Square, cell: Cell) {
        if let Some(k) = Board::index(square) {
            self.cells[k] = Some(cell);
        }
    }

    /// Places `letter` with its standard value (for tests and tools).
    pub fn put(&mut self, square: Square, letter: char, seat: Option<usize>, order: u32) {
        let value = letters::value(letter).unwrap_or(0);
        self.set(
            square,
            Cell {
                letter,
                value,
                seat,
                order,
            },
        );
    }

    pub fn get(&self, square: Square) -> Option<Cell> {
        Board::index(square).and_then(|k| self.cells[k])
    }
}

/// A word on the final board with its score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scored {
    /// its squares, owner and whether it is real (the marks' word)
    pub word: marks::Word,
    pub letters: String,
    /// its score, premiums applied, before its sign
    pub points: i32,
    /// the place order of its latest tile, when its owner claimed it
    pub claimed: u32,
}

impl Scored {
    /// What it adds to its owner's total: its points, or minus them when
    /// it is not a real word.
    pub fn signed(&self) -> i32 {
        if self.word.real {
            self.points
        } else {
            -self.points
        }
    }
}

/// Scores the run of squares `cells` (two or more, all filled).
fn score(board: &Board, cells: &[Square]) -> Option<Scored> {
    let mut letters = String::new();
    let mut sum = 0;
    let mut times = 1;
    let mut latest: Option<(u32, usize)> = None;
    for sq in cells {
        let cell = board.get(*sq)?;
        letters.push(cell.letter);
        let (l, w) = match premium(*sq) {
            Some(Premium::DoubleLetter) => (2, 1),
            Some(Premium::TripleLetter) => (3, 1),
            Some(Premium::DoubleWord) => (1, 2),
            Some(Premium::TripleWord) => (1, 3),
            None => (1, 1),
        };
        sum += cell.value * l;
        times *= w;
        if let Some(seat) = cell.seat {
            if latest.is_none_or(|(o, _)| cell.order > o) {
                latest = Some((cell.order, seat));
            }
        }
    }
    // a run of tiles no player placed has no owner (the centre tile is the
    // only such tile, so a word always has one)
    let (claimed, owner) = latest?;
    let real = crate::words::is_real(&letters);
    Some(Scored {
        word: marks::Word {
            from: cells[0],
            to: cells[cells.len() - 1],
            owner,
            real,
        },
        letters,
        points: sum * times,
        claimed,
    })
}

/// Every word on the board: the rows' words top to bottom, then the
/// columns' left to right, each first square to last.
pub fn words(board: &Board) -> Vec<Scored> {
    let mut out = Vec::new();
    for along_rows in [true, false] {
        for line in 0..SQUARES {
            let mut run: Vec<Square> = Vec::new();
            for k in 0..=SQUARES {
                let sq = if along_rows { (line, k) } else { (k, line) };
                if k < SQUARES && board.get(sq).is_some() {
                    run.push(sq);
                    continue;
                }
                if run.len() >= 2 {
                    out.extend(score(board, &run));
                }
                run.clear();
            }
        }
    }
    out
}

/// One player's part of the tally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub seat: usize,
    /// the words they own, in the order they claimed them
    pub words: Vec<Scored>,
    /// points lost to the idle penalty (0 or more)
    pub idle: i32,
    pub total: i32,
}

/// The whole tally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tally {
    /// one line a seat, in seat order
    pub lines: Vec<Line>,
    /// the seats with the highest total (more than one on a tie)
    pub winners: Vec<usize>,
}

/// The tally for `seats` players from the final board and each seat's idle
/// penalty (points lost; a missing seat lost none).
pub fn tally(board: &Board, seats: usize, idle: &[i32]) -> Tally {
    let all = words(board);
    let lines: Vec<Line> = (0..seats)
        .map(|seat| {
            let mut words: Vec<Scored> = all.iter().filter(|w| w.word.owner == seat).cloned().collect();
            words.sort_by_key(|w| w.claimed);
            let idle = idle.get(seat).copied().unwrap_or(0).max(0);
            let total = words.iter().map(Scored::signed).sum::<i32>() - idle;
            Line {
                seat,
                words,
                idle,
                total,
            }
        })
        .collect();
    let best = lines.iter().map(|l| l.total).max();
    let winners = lines
        .iter()
        .filter(|l| Some(l.total) == best)
        .map(|l| l.seat)
        .collect();
    Tally { lines, winners }
}

#[cfg(test)]
mod tests;
