//! The words on the board (BRIEF.md, Words and ownership): a word is a
//! maximal unbroken run of two or more letters along a row or a column;
//! its owner is whoever placed its latest letter (by place order). Whether
//! it is a real word is `crate::words::is_real` (ENABLE). The scores are
//! the tally's (`crate::words` and the end screen), not here.

use crate::marks::Word;

use super::tiles::{Square, State, TileId};
use super::Match;

/// One placed letter, as the board holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Letter {
    pub tile: TileId,
    pub letter: char,
    pub value: i32,
    /// who placed it (none for the centre tile) and when, in place order
    pub seat: Option<usize>,
    pub order: u32,
}

/// A word on the board: its squares, letters, owner and whether it is real.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardWord {
    pub from: Square,
    pub to: Square,
    pub text: String,
    /// the seat that placed its latest letter; none only for a word of
    /// centre tile alone (which cannot be: a word has two letters)
    pub owner: Option<usize>,
    pub real: bool,
}

impl BoardWord {
    /// As the word marks draw it.
    pub fn mark(&self) -> Option<Word> {
        Some(Word {
            from: self.from,
            to: self.to,
            owner: self.owner?,
            real: self.real,
        })
    }
}

/// The board's letters by square, row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub size: i32,
    pub cells: Vec<Option<Letter>>,
}

impl Grid {
    pub fn get(&self, (r, c): Square) -> Option<Letter> {
        if r < 0 || c < 0 || r >= self.size || c >= self.size {
            return None;
        }
        self.cells[(r * self.size + c) as usize]
    }

    pub fn set(&mut self, (r, c): Square, l: Letter) {
        if (0..self.size).contains(&r) && (0..self.size).contains(&c) {
            self.cells[(r * self.size + c) as usize] = Some(l);
        }
    }

    /// The run of letters through `sq` along (dr, dc), if two or more long.
    fn run(&self, sq: Square, (dr, dc): (i32, i32)) -> Option<BoardWord> {
        self.get(sq)?;
        let mut from = sq;
        while self.get((from.0 - dr, from.1 - dc)).is_some() {
            from = (from.0 - dr, from.1 - dc);
        }
        let mut to = sq;
        while self.get((to.0 + dr, to.1 + dc)).is_some() {
            to = (to.0 + dr, to.1 + dc);
        }
        if from == to {
            return None;
        }
        let mut text = String::new();
        let mut latest: Option<Letter> = None;
        let mut at = from;
        loop {
            let l = self.get(at)?;
            text.push(l.letter);
            if latest.is_none_or(|x| l.order > x.order) {
                latest = Some(l);
            }
            if at == to {
                break;
            }
            at = (at.0 + dr, at.1 + dc);
        }
        let real = crate::words::is_real(&text);
        Some(BoardWord {
            from,
            to,
            text,
            owner: latest.and_then(|l| l.seat),
            real,
        })
    }

    /// Every word on the board: the rows' first, top to bottom, then the
    /// columns', left to right.
    pub fn words(&self) -> Vec<BoardWord> {
        let mut out = Vec::new();
        for (dir, starts) in [((0, 1), true), ((1, 0), false)] {
            for a in 0..self.size {
                for b in 0..self.size {
                    let sq = if starts { (a, b) } else { (b, a) };
                    let before = (sq.0 - dir.0, sq.1 - dir.1);
                    if self.get(sq).is_some() && self.get(before).is_none() {
                        if let Some(w) = self.run(sq, dir) {
                            out.push(w);
                        }
                    }
                }
            }
        }
        out
    }

    /// The words through `sq`: across, then down.
    pub fn words_through(&self, sq: Square) -> Vec<BoardWord> {
        [(0, 1), (1, 0)].iter().filter_map(|d| self.run(sq, *d)).collect()
    }
}

impl Match {
    /// The placed letters by square.
    pub fn grid(&self) -> Grid {
        let n = self.rules.board_size;
        let mut g = Grid {
            size: n,
            cells: vec![None; (n * n) as usize],
        };
        for t in &self.tiles.all {
            if let State::Placed { square, seat, order } = t.state {
                g.set(
                    square,
                    Letter {
                        tile: t.id,
                        letter: t.letter,
                        value: t.value,
                        seat,
                        order,
                    },
                );
            }
        }
        g
    }

    /// The words seat `i` would form placing tile `tile` on `sq` (the word
    /// preview): each word through the square, with the tile in it, owned
    /// by seat `i`.
    pub fn preview(&self, i: usize, tile: TileId, sq: Square) -> Vec<BoardWord> {
        let Some(t) = self.tiles.get(tile) else {
            return Vec::new();
        };
        let mut g = self.grid();
        g.set(
            sq,
            Letter {
                tile,
                letter: t.letter,
                value: t.value,
                seat: Some(i),
                order: u32::MAX,
            },
        );
        g.words_through(sq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[(Square, char, usize, u32)]) -> Grid {
        let mut g = Grid {
            size: 15,
            cells: vec![None; 225],
        };
        for (sq, c, seat, order) in rows {
            g.set(
                *sq,
                Letter {
                    tile: *order as TileId,
                    letter: *c,
                    value: 1,
                    seat: Some(*seat),
                    order: *order,
                },
            );
        }
        g
    }

    #[test]
    fn a_word_is_a_whole_run_and_its_last_letter_names_its_owner() {
        // CAT across row 7 by seat 0, then seat 1 adds an S: CATS is seat 1's
        let mut rows = vec![((7, 6), 'C', 0, 1), ((7, 7), 'A', 0, 0), ((7, 8), 'T', 0, 2)];
        let g = grid(&rows);
        let w = g.words();
        assert_eq!(w.len(), 1);
        assert_eq!(
            (w[0].text.as_str(), w[0].owner, w[0].real),
            ("CAT", Some(0), true)
        );
        rows.push(((7, 9), 'S', 1, 3));
        let w = grid(&rows).words();
        assert_eq!(
            (w[0].text.as_str(), w[0].owner, w[0].from, w[0].to),
            ("CATS", Some(1), (7, 6), (7, 9))
        );
    }

    #[test]
    fn crossing_words_are_both_found_and_a_lone_letter_is_no_word() {
        // CAT across, AX down through the A; a Q off by itself
        let g = grid(&[
            ((7, 6), 'C', 0, 1),
            ((7, 7), 'A', 0, 0),
            ((7, 8), 'T', 0, 2),
            ((8, 7), 'X', 1, 3),
            ((2, 2), 'Q', 1, 4),
        ]);
        let texts: Vec<String> = g.words().into_iter().map(|w| w.text).collect();
        assert_eq!(texts, vec!["CAT".to_string(), "AX".to_string()]);
        let fake = grid(&[((7, 7), 'Q', 0, 0), ((7, 8), 'J', 1, 1)]).words();
        assert!(!fake[0].real, "QJ is not in the list");
    }
}
