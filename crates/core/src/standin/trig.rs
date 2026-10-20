//! Angles for the stand-in: 65536 units a full turn, sines in Q16 (65536 is
//! one), read from a quarter-wave table with straight lines between its 256
//! steps. No floats in the core (BUILDER-SPEC D2). Kept inside `standin` so
//! it does not meet the renderer's own fixed-point maths (`gfx/`) built in
//! parallel; the real game can take the renderer's.

/// One full turn.
pub const TURN: i32 = 65536;
/// One, in Q16.
pub const ONE: i64 = 65536;

/// sin over a quarter turn, 257 points (0 to 90 degrees), Q16.
const QUARTER: [i32; 257] = [
    0, 402, 804, 1206, 1608, 2010, 2412, 2814, 3216, 3617, 4019, 4420, 4821, 5222, 5623, 6023, 6424, 6824,
    7224, 7623, 8022, 8421, 8820, 9218, 9616, 10014, 10411, 10808, 11204, 11600, 11996, 12391, 12785, 13180,
    13573, 13966, 14359, 14751, 15143, 15534, 15924, 16314, 16703, 17091, 17479, 17867, 18253, 18639, 19024,
    19409, 19792, 20175, 20557, 20939, 21320, 21699, 22078, 22457, 22834, 23210, 23586, 23961, 24335, 24708,
    25080, 25451, 25821, 26190, 26558, 26925, 27291, 27656, 28020, 28383, 28745, 29106, 29466, 29824, 30182,
    30538, 30893, 31248, 31600, 31952, 32303, 32652, 33000, 33347, 33692, 34037, 34380, 34721, 35062, 35401,
    35738, 36075, 36410, 36744, 37076, 37407, 37736, 38064, 38391, 38716, 39040, 39362, 39683, 40002, 40320,
    40636, 40951, 41264, 41576, 41886, 42194, 42501, 42806, 43110, 43412, 43713, 44011, 44308, 44604, 44898,
    45190, 45480, 45769, 46056, 46341, 46624, 46906, 47186, 47464, 47741, 48015, 48288, 48559, 48828, 49095,
    49361, 49624, 49886, 50146, 50404, 50660, 50914, 51166, 51417, 51665, 51911, 52156, 52398, 52639, 52878,
    53114, 53349, 53581, 53812, 54040, 54267, 54491, 54714, 54934, 55152, 55368, 55582, 55794, 56004, 56212,
    56418, 56621, 56823, 57022, 57219, 57414, 57607, 57798, 57986, 58172, 58356, 58538, 58718, 58896, 59071,
    59244, 59415, 59583, 59750, 59914, 60075, 60235, 60392, 60547, 60700, 60851, 60999, 61145, 61288, 61429,
    61568, 61705, 61839, 61971, 62101, 62228, 62353, 62476, 62596, 62714, 62830, 62943, 63054, 63162, 63268,
    63372, 63473, 63572, 63668, 63763, 63854, 63944, 64031, 64115, 64197, 64277, 64354, 64429, 64501, 64571,
    64639, 64704, 64766, 64827, 64884, 64940, 64993, 65043, 65091, 65137, 65180, 65220, 65259, 65294, 65328,
    65358, 65387, 65413, 65436, 65457, 65476, 65492, 65505, 65516, 65525, 65531, 65535, 65536,
];

fn quarter(a: i32) -> i32 {
    // a in 0..=16384: 64 units a table step
    let i = (a / 64) as usize;
    if i >= 256 {
        return QUARTER[256];
    }
    let f = a % 64;
    QUARTER[i] + (QUARTER[i + 1] - QUARTER[i]) * f / 64
}

/// sin(a), Q16.
pub fn sin(a: i32) -> i32 {
    let a = a.rem_euclid(TURN);
    match a / 16384 {
        0 => quarter(a),
        1 => quarter(32768 - a),
        2 => -quarter(a - 32768),
        _ => -quarter(TURN - a),
    }
}

/// cos(a), Q16.
pub fn cos(a: i32) -> i32 {
    sin(a.wrapping_add(16384))
}

/// The yaw that faces from the origin towards (x, z), x east and z north:
/// the angle whose (sin, cos) points along (x, z). 0 for (0, 0).
pub fn atan2(x: i64, z: i64) -> i32 {
    if x == 0 && z == 0 {
        return 0;
    }
    let (ax, az) = (x.abs(), z.abs());
    // the angle in 0..=16384 with sin(a) * az = cos(a) * ax, by halving
    let (mut lo, mut hi) = (0, 16384);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if (sin(mid) as i64) * az < (cos(mid) as i64) * ax {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let a = if ax == 0 {
        0
    } else if az == 0 {
        16384
    } else {
        hi
    };
    match (x >= 0, z >= 0) {
        (true, true) => a,
        (true, false) => 32768 - a,
        (false, false) => 32768 + a,
        (false, true) => (TURN - a) % TURN,
    }
}

/// The square root of a non-negative number, rounded down.
pub fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sines_of_the_quarter_turns_and_an_eighth() {
        assert_eq!(sin(0), 0);
        assert_eq!(sin(16384), 65536);
        assert_eq!(sin(32768), 0);
        assert_eq!(sin(49152), -65536);
        assert_eq!(cos(0), 65536);
        assert_eq!(sin(-16384), -65536);
        // sin 45 degrees is 0.70711: 46341 in Q16
        assert!((sin(8192) - 46341).abs() <= 1);
        assert!((cos(8192) - 46341).abs() <= 1);
    }

    #[test]
    fn atan2_finds_each_compass_point_and_isqrt_rounds_down() {
        assert_eq!(atan2(0, 1000), 0);
        assert_eq!(atan2(1000, 0), 16384);
        assert_eq!(atan2(0, -1000), 32768);
        assert_eq!(atan2(-1000, 0), 49152);
        assert!((atan2(1000, 1000) - 8192).abs() <= 1);
        assert!((atan2(-1000, -1000) - 40960).abs() <= 1);
        assert_eq!(isqrt(1_000_000), 1000);
        assert_eq!(isqrt(99), 9);
    }
}
