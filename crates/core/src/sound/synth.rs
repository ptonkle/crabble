//! A small integer synthesiser: renders a recipe (`recipe.rs`) into
//! samples. Every value is a whole number (BUILDER-SPEC D2): samples and
//! gains are Q16.16 (`ONE` is full scale), pitches are worked in octaves
//! (log2 Hz, Q16.16) so glides, wobbles and the seed's pitch variation all
//! add, and the only source of chance is the seed the caller passes.
//!
//! After Ape Race's `sound.rs` (phase accumulator oscillators, envelopes,
//! noise from dice, seamless loops by running the filter round twice), with
//! Honey Hole's recipe features: filters with Q and sweeps, glides, wobble.
//! The filter is Chamberlin's state-variable filter run twice per sample.

use super::decimal::MILLI;
use super::recipe::{Cue, Fall, FilterKind, Glide, Layer, Wave};

/// One, in Q16.16.
pub const ONE: i64 = 1 << 16;

// ---- fixed-point maths ----

/// The product of two Q16.16 values.
pub fn mul(a: i64, b: i64) -> i64 {
    (a * b) >> 16
}

/// sin(2π · phase / 2^32) in Q16.16, from a quarter-wave polynomial
/// (largest error about 1/10 000 of full scale).
pub fn sin(phase: u32) -> i64 {
    let quarter = phase >> 30;
    let t = ((phase >> 14) & 0xFFFF) as i64;
    let t = if quarter & 1 == 1 { ONE - t } else { t };
    // sin(π/2 · t) ≈ t (a − t² (b − c t²)), a = π/2, b = 0.645964, c = 0.079693
    let t2 = mul(t, t);
    let r = 102_944 - mul(42_334 - mul(5_223, t2), t2);
    let s = mul(r, t).min(ONE);
    if quarter >= 2 {
        -s
    } else {
        s
    }
}

/// 2^x for `x` in Q16.16, as Q16.16. From -16 to 30 octaves.
pub fn exp2(x: i64) -> i64 {
    let n = x >> 16;
    let f = x & 0xFFFF;
    // 2^f, f in [0, 1), by its Taylor series in Q30 (error below 2e-5)
    const C: [i64; 7] = [
        1 << 30,
        744_261_118,
        257_946_580,
        59_597_034,
        10_327_368,
        1_431_686,
        165_384,
    ];
    let f30 = f << 14;
    let mut acc = C[6];
    for c in C[..6].iter().rev() {
        acc = c + ((acc * f30) >> 30);
    }
    let v = acc >> 14;
    if n >= 0 {
        v << n.min(30)
    } else {
        v >> (-n).min(40)
    }
}

/// log2 of a positive Q16.16 value, as Q16.16 (bit by bit).
pub fn log2(v: i64) -> i64 {
    let v = v.max(1);
    let top = 63 - v.leading_zeros() as i64;
    let mut out = (top - 16) << 16;
    // y in [1, 2) as Q30
    let mut y = if top >= 30 {
        v >> (top - 30)
    } else {
        v << (30 - top)
    };
    let mut bit = ONE >> 1;
    while bit > 0 {
        y = (y * y) >> 30;
        if y >= 2 << 30 {
            y >>= 1;
            out += bit;
        }
        bit >>= 1;
    }
    out
}

/// Thousandths of a Hz as log2 Hz (Q16.16).
fn octave_of(milli_hz: i64) -> i64 {
    log2(milli_hz * ONE / MILLI)
}

/// Cents (thousandths) as octaves (Q16.16).
pub fn cents_to_octaves(milli_cents: i64) -> i64 {
    milli_cents * ONE / (1200 * MILLI)
}

/// The dice: SplitMix64, seeded by the caller. The same seed always rolls
/// the same numbers.
#[derive(Debug, Clone)]
pub struct Dice(pub u64);

impl Dice {
    pub fn roll(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A whole number from 0 to `n - 1` (0 when `n` is 0).
    pub fn below(&mut self, n: i64) -> i64 {
        if n <= 0 {
            return 0;
        }
        (self.roll() % n as u64) as i64
    }

    /// A whole number from `-n` to `n`.
    pub fn spread(&mut self, n: i64) -> i64 {
        self.below(2 * n + 1) - n
    }

    /// White noise in [-ONE, ONE).
    pub fn white(&mut self) -> i64 {
        (self.roll() >> 47) as i64 - ONE
    }
}

// ---- rendering ----

/// Milliseconds as samples at `rate`.
pub fn samples_of(ms: i64, rate: i64) -> usize {
    (ms * rate / 1000) as usize
}

/// Chamberlin's state-variable filter, run twice per sample so it stays
/// steady up to a third of the sample rate.
#[derive(Default)]
struct Svf {
    low: i64,
    band: i64,
}

impl Svf {
    /// One sample through the filter with coefficient `f` and damping `d`
    /// (both Q16.16); the band output is scaled by `d` so its peak is 1.
    fn step(&mut self, x: i64, f: i64, d: i64, kind: FilterKind) -> i64 {
        let mut high = 0;
        for _ in 0..2 {
            self.low += mul(f, self.band);
            high = x - self.low - mul(d, self.band);
            self.band += mul(f, high);
        }
        match kind {
            FilterKind::Lowpass => self.low,
            FilterKind::Highpass => high,
            FilterKind::Bandpass => mul(d, self.band),
        }
    }
}

/// The filter coefficient for a cutoff at `octave` (log2 Hz, Q16.16),
/// held where the filter stays steady for damping `d`.
fn coefficient(octave: i64, d: i64, rate: i64) -> i64 {
    let hz = exp2(octave);
    // f = 2 sin(π fc / 2·rate); the phase of π·x is x/2 of a turn
    let phase = ((hz << 14) / rate).clamp(0, u32::MAX as i64) as u32;
    let f = 2 * sin(phase);
    f.min((2 * ONE - d) * 7 / 8).max(1)
}

/// How a layer is heard this time: its start, and how far the seed moved
/// its pitch (octaves, Q16.16).
pub struct Placed<'a> {
    pub layer: &'a Layer,
    pub at_ms: i64,
    pub octaves: i64,
}

/// Adds one layer to `mix`. A loop's `mix` wraps: a sound running off its
/// end comes in again at its start.
pub fn add_layer(p: &Placed, rate: i64, dice: &mut Dice, mix: &mut [i64], wraps: bool) {
    let l = p.layer;
    let len = mix.len();
    if len == 0 {
        return;
    }
    let mut svf = Svf::default();
    let filter = l.filter.as_ref().map(|f| {
        let d = (ONE * MILLI / f.q).max(1);
        let from = octave_of(f.freq) + p.octaves;
        let to = f.to_freq.map_or(from, |t| octave_of(t) + p.octaves);
        (f.kind, d, from, to)
    });
    let gain = l.gain * ONE / MILLI;

    if l.whole {
        // noise from end to end: the filter runs round twice, so its state
        // at the end carries into the start and the loop has no seam
        let raw: Vec<i64> = (0..len).map(|_| dice.white()).collect();
        let coef = filter.map(|(kind, d, from, _)| (kind, d, coefficient(from, d, rate)));
        for pass in 0..2 {
            for (i, x) in raw.iter().enumerate() {
                let s = match coef {
                    Some((kind, d, f)) => svf.step(*x, f, d, kind),
                    None => *x,
                };
                if pass == 1 {
                    mix[i] += mul(s, gain);
                }
            }
        }
        return;
    }

    let attack = samples_of(l.attack_ms, rate).max(1);
    let hold = samples_of(l.hold_ms, rate);
    let release = samples_of(l.release_ms, rate).max(1);
    let count = attack + hold + release;
    let start = samples_of(p.at_ms, rate);
    let span = count as i64;

    let (from, to) = if l.wave == Wave::Noise {
        (0, 0)
    } else {
        let from = octave_of(l.freq) + p.octaves;
        (from, l.to_freq.map_or(from, |t| octave_of(t) + p.octaves))
    };
    let (lin_from, lin_to) = (exp2(from), exp2(to));
    let wobble = l
        .wobble
        .as_ref()
        .map(|w| (((w.rate * ONE / MILLI) << 16) / rate, cents_to_octaves(w.cents)));
    let mut phase: u32 = 0;
    let mut wobble_phase: u32 = 0;

    for i in 0..count {
        let k = i as i64;
        let raw = if l.wave == Wave::Noise {
            dice.white()
        } else {
            let mut octave = match l.glide {
                Glide::Exp => from + (to - from) * k / span,
                Glide::Linear => log2(lin_from + (lin_to - lin_from) * k / span),
            };
            if let Some((step, depth)) = wobble {
                wobble_phase = wobble_phase.wrapping_add(step as u32);
                octave += mul(depth, sin(wobble_phase));
            }
            let hz = exp2(octave);
            phase = phase.wrapping_add(((hz << 16) / rate) as u32);
            match l.wave {
                Wave::Sine => sin(phase),
                Wave::Square => {
                    if phase < 1 << 31 {
                        ONE
                    } else {
                        -ONE
                    }
                }
                Wave::Sawtooth => (phase >> 15) as i64 - ONE,
                // rises from the middle, as a sine does
                Wave::Triangle => {
                    let a = (phase >> 14) as i64; // 0 .. 4·ONE
                    if a < ONE {
                        a
                    } else if a < 3 * ONE {
                        2 * ONE - a
                    } else {
                        a - 4 * ONE
                    }
                }
                Wave::Noise => 0,
            }
        };
        let s = match filter {
            Some((kind, d, ff, ft)) => {
                let f = coefficient(ff + (ft - ff) * k / span, d, rate);
                svf.step(raw, f, d, kind)
            }
            None => raw,
        };
        let level = if i < attack {
            k * ONE / attack as i64
        } else if i < attack + hold {
            ONE
        } else {
            let rem = (count - i) as i64;
            let r = release as i64;
            match l.fall {
                Fall::Linear => rem * ONE / r,
                Fall::Curved => rem * rem * ONE / (r * r),
            }
        };
        let at = start + i;
        let slot = if wraps {
            Some(at % len)
        } else {
            (at < len).then_some(at)
        };
        if let Some(j) = slot {
            mix[j] += mul(mul(s, level), gain);
        }
    }
}

/// Silence after the last layer, so a player that trims nothing still ends
/// cleanly.
pub const TAIL_MS: i64 = 10;

/// A cue rendered as Q16.16 samples (ONE is full scale), before the volume.
/// `seed` moves the pitch by up to `vary_cents`, strews `scatter` layers
/// and rolls the noise.
pub fn render(cue: &Cue, rate: i64, seed: u64) -> Vec<i64> {
    let mut dice = Dice(seed ^ fnv(&cue.id));
    let cue_octaves = cents_to_octaves(dice.spread(cue.vary_cents));
    let len_ms = match cue.loop_ms {
        Some(l) => l,
        None => cue.length_ms() + TAIL_MS,
    };
    let mut mix = vec![0i64; samples_of(len_ms, rate)];
    let wraps = cue.loop_ms.is_some();
    for layer in &cue.layers {
        match &layer.scatter {
            None => add_layer(
                &Placed {
                    layer,
                    at_ms: layer.at_ms,
                    octaves: cue_octaves,
                },
                rate,
                &mut dice,
                &mut mix,
                wraps,
            ),
            Some(s) => {
                for _ in 0..s.times {
                    let at_ms = layer.at_ms + dice.below(s.over_ms);
                    let octaves = cue_octaves + cents_to_octaves(dice.spread(s.cents));
                    add_layer(
                        &Placed {
                            layer,
                            at_ms,
                            octaves,
                        },
                        rate,
                        &mut dice,
                        &mut mix,
                        wraps,
                    );
                }
            }
        }
    }
    mix
}

/// FNV-1a of a cue's id, so two cues given one seed roll differently.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01B3)
    })
}
