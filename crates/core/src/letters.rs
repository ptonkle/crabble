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

/// How many of each letter one standard English set holds, A to Z,
/// without its two blanks: 98 tiles.
pub const COUNTS: [(char, u32); 26] = [
    ('A', 9),
    ('B', 2),
    ('C', 2),
    ('D', 4),
    ('E', 12),
    ('F', 2),
    ('G', 3),
    ('H', 2),
    ('I', 9),
    ('J', 1),
    ('K', 1),
    ('L', 4),
    ('M', 2),
    ('N', 6),
    ('O', 8),
    ('P', 2),
    ('Q', 1),
    ('R', 6),
    ('S', 4),
    ('T', 6),
    ('U', 4),
    ('V', 2),
    ('W', 2),
    ('X', 1),
    ('Y', 2),
    ('Z', 1),
];

/// `sets` standard sets without blanks, in letter order.
pub fn tile_set(sets: u32) -> Vec<char> {
    COUNTS
        .iter()
        .flat_map(|(c, n)| std::iter::repeat_n(*c, (*n * sets) as usize))
        .collect()
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

    #[test]
    fn three_sets_without_blanks_are_294_tiles_in_the_standard_counts() {
        let t = tile_set(3);
        assert_eq!(t.len(), 294);
        let count = |c: char| t.iter().filter(|x| **x == c).count();
        // twelve Es, nine As and Is, one each of J K Q X Z, in one set
        assert_eq!((count('E'), count('A'), count('I')), (36, 27, 27));
        for c in "JKQXZ".chars() {
            assert_eq!(count(c), 3, "{c}");
        }
        // one set's 98 tiles carry 187 points (the standard set's, blanks 0)
        let points: i32 = tile_set(1).iter().map(|c| value(*c).unwrap()).sum();
        assert_eq!(points, 187);
    }
}
