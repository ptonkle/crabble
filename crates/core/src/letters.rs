//! Each letter's point value, as in standard English Scrabble (BRIEF.md,
//! Tiles), for the value drawn in a tile's corner. The tile set (how many
//! of each letter) belongs to the game's rules, not here (SECOND-ORDER
//! R1.63).

/// The letters and their points, A to Z.
pub const VALUES: [(char, i32); 26] = [
    ('A', 1),
    ('B', 3),
    ('C', 3),
    ('D', 2),
    ('E', 1),
    ('F', 4),
    ('G', 2),
    ('H', 4),
    ('I', 1),
    ('J', 8),
    ('K', 5),
    ('L', 1),
    ('M', 3),
    ('N', 1),
    ('O', 1),
    ('P', 3),
    ('Q', 10),
    ('R', 1),
    ('S', 1),
    ('T', 1),
    ('U', 1),
    ('V', 4),
    ('W', 4),
    ('X', 8),
    ('Y', 4),
    ('Z', 10),
];

/// A letter's points, or `None` for anything but A to Z.
pub fn value(letter: char) -> Option<i32> {
    VALUES.iter().find(|(c, _)| *c == letter).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_are_scrabbles() {
        // one-point letters, the tens, and a few between, from the rules
        for c in "AEILNORSTU".chars() {
            assert_eq!(value(c), Some(1), "{c}");
        }
        assert_eq!((value('Q'), value('Z')), (Some(10), Some(10)));
        assert_eq!((value('J'), value('X')), (Some(8), Some(8)));
        assert_eq!((value('K'), value('D'), value('G')), (Some(5), Some(2), Some(2)));
        assert_eq!(value('a'), None);
        // the 26 values, one of each letter, sum to 87
        assert_eq!(VALUES.iter().map(|(_, v)| v).sum::<i32>(), 87);
    }
}
