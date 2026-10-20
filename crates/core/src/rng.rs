//! Seeded dice. The core draws no random number of its own: every seed comes
//! from the page (`flag.seed`, from `crypto.getRandomValues`) or from a
//! message, so the same seed gives the same game everywhere.

/// SplitMix64's mixer: a well-spread 64-bit value from any input.
pub fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A stream of dice from one seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dice(pub u64);

impl Dice {
    pub fn roll(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix(self.0)
    }

    /// A number in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (self.roll() % n as u64) as u32
    }

    /// A number in `lo..=hi`.
    pub fn between(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below((hi - lo + 1) as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_dice_and_stays_in_range() {
        let (mut a, mut b) = (Dice(7), Dice(7));
        for _ in 0..100 {
            let x = a.between(-3, 5);
            assert_eq!(x, b.between(-3, 5));
            assert!((-3..=5).contains(&x));
        }
        assert_eq!(Dice(1).below(0), 0);
        assert_ne!(Dice(1).roll(), Dice(2).roll());
    }
}
