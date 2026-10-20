//! The words on the board, for the marks and the word preview, taken from
//! the one word finder, `crate::score::words` (SECOND-ORDER R3.5): a word
//! is a maximal run of two or more letters; its owner placed its latest
//! letter; it is real when ENABLE lists it.

use crate::score::{self, Board, Cell, Scored};

use super::tiles::{Square, TileId};
use super::Match;

/// Whether word `w` runs through square `sq`.
fn through(w: &Scored, sq: Square) -> bool {
    w.word.cells().contains(&sq)
}

impl Match {
    /// The placed letters, as the score reads them.
    pub fn board(&self) -> Board {
        Board::from_tiles(&self.tiles)
    }

    /// Every word on the board now.
    pub fn words(&self) -> Vec<Scored> {
        score::words(&self.board())
    }

    /// The words through square `sq` now.
    pub fn words_through(&self, sq: Square) -> Vec<Scored> {
        self.words().into_iter().filter(|w| through(w, sq)).collect()
    }

    /// The words seat `i` would form placing tile `tile` on `sq` (the word
    /// preview): each word through the square with the tile in it, which
    /// would be seat `i`'s.
    pub fn preview(&self, i: usize, tile: TileId, sq: Square) -> Vec<Scored> {
        let Some(t) = self.tiles.get(tile) else {
            return Vec::new();
        };
        let mut b = self.board();
        b.set(
            sq,
            Cell {
                letter: t.letter,
                value: t.value,
                seat: Some(i),
                order: u32::MAX,
            },
        );
        score::words(&b).into_iter().filter(|w| through(w, sq)).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::gfx::geom::Affine;
    use crate::sim::tests::{flat, game_on};
    use crate::sim::{Driver, State, Tile};

    #[test]
    fn the_preview_shows_each_word_a_placement_would_make_and_whose_it_would_be() {
        let mut m = flat();
        m.slots.push((Affine::IDENTITY, false));
        m.slots.push((Affine::IDENTITY, false));
        let mut g = game_on(&m, vec![Driver::Local, Driver::Ai]);
        // the centre holds A (the last tile dealt); tile 0 is a T
        g.tiles.all[1] = Tile {
            letter: 'A',
            value: 1,
            ..g.tiles.all[1].clone()
        };
        g.tiles.all[0].letter = 'T';
        let words = g.preview(1, 0, (7, 8));
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].letters, "AT");
        assert!(words[0].word.real);
        assert_eq!(words[0].word.owner, 1);
        // nothing is placed by a preview
        assert!(g.words().is_empty());
        assert!(matches!(g.tiles.all[0].state, State::Loose { .. }));
    }
}
