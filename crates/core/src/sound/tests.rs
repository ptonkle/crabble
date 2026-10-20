//! The sound engine's rules. Expected values are worked out by hand from the
//! small recipes below, not read back from the functions under test.

use super::synth::{self, ONE};
use super::*;

/// 8000 samples a second, so a millisecond is 8 samples; full scale at
/// 100% volume is 10 000.
fn file(cues: &str) -> String {
    format!(r#"{{"_note": "test", "sample_rate": 8000, "master": 10000, "cues": [{cues}]}}"#)
}

/// A steady 1000 Hz sine: 1 ms in, 100 ms held, 1 ms out, at half gain.
const BEEP: &str = r#"{"id": "beep", "layers": [
  {"wave": "sine", "freq": 1000, "attack_ms": 1, "hold_ms": 100, "release_ms": 1, "fall": "linear", "gain": 0.5}]}"#;

fn bank(cues: &str) -> Bank {
    Bank::parse(&file(cues)).unwrap_or_else(|e| panic!("{e}"))
}

fn refusal(cues: &str) -> String {
    Bank::parse(&file(cues)).unwrap_err()
}

/// Upward zero crossings in `pcm[from..to]`: one a cycle.
fn cycles(pcm: &[i16], from: usize, to: usize) -> usize {
    pcm[from..to].windows(2).filter(|w| w[0] <= 0 && w[1] > 0).count()
}

// ---- the file's format ----

#[test]
fn a_misspelt_key_is_refused_by_name() {
    let e = refusal(r#"{"id": "a", "layers": [{"wave": "sine", "freq": 440, "gian": 0.5}]}"#);
    assert!(e.contains("\"gian\""), "{e}");
    let e = Bank::parse(r#"{"sample_rate": 8000, "master": 1, "cues": [], "volume": 3}"#).unwrap_err();
    assert!(e.contains("\"volume\""), "{e}");
}

#[test]
fn a_bad_layer_is_refused_by_name() {
    let e = refusal(r#"{"id": "a", "layers": [{"wave": "hum", "freq": 440, "gain": 0.5}]}"#);
    assert!(e.starts_with("Cue a layer 1 has wave \"hum\""), "{e}");
    let e = refusal(r#"{"id": "a", "layers": [{"wave": "noise", "freq": 440, "gain": 0.5}]}"#);
    assert!(e.contains("noise, which has no freq"), "{e}");
    let e = refusal(r#"{"id": "a", "layers": [{"wave": "sine", "freq": 440, "gain": 1.5}]}"#);
    assert_eq!(e, "Cue a layer 1 gain must be more than 0 and at most 1.");
    // at 8000 samples a second a pitch stays at or below 3200 Hz
    let e = refusal(r#"{"id": "a", "layers": [{"wave": "sine", "freq": 3300, "gain": 0.5}]}"#);
    assert_eq!(e, "Cue a layer 1 freq must be more than 0 and at most 3200.");
    let e = refusal(
        r#"{"id": "a", "layers": [{"wave": "noise", "gain": 0.5, "filter": {"type": "notch", "freq": 300}}]}"#,
    );
    assert!(e.contains("type \"notch\""), "{e}");
    let e = refusal(
        r#"{"id": "a", "layers": [{"wave": "sine", "freq": 440, "gain": 0.5, "scatter": {"times": 0, "over_ms": 9, "cents": 0}}]}"#,
    );
    assert!(e.contains("scatter"), "{e}");
}

#[test]
fn a_cue_listed_twice_or_badly_named_is_refused() {
    let e = refusal(&format!("{BEEP}, {BEEP}"));
    assert_eq!(e, "Cue beep is listed twice.");
    let e = refusal(r#"{"id": "Beep", "layers": [{"wave": "sine", "freq": 440, "gain": 0.5}]}"#);
    assert!(e.contains("lowercase"), "{e}");
}

#[test]
fn a_whole_layer_belongs_to_a_loop_and_a_loop_holds_its_layers() {
    let e = refusal(r#"{"id": "bed", "layers": [{"wave": "noise", "whole": true, "gain": 0.5}]}"#);
    assert_eq!(e, "Cue bed has a whole layer but is not a loop.");
    let e = refusal(
        r#"{"id": "bed", "loop_ms": 50, "layers": [{"wave": "sine", "freq": 440, "hold_ms": 60, "gain": 0.5}]}"#,
    );
    // 1 + 60 + 1 ms
    assert_eq!(
        e,
        "Cue bed has a layer lasting 62 ms, longer than its loop of 50 ms."
    );
    let e = refusal(
        r#"{"id": "bed", "loop_ms": 50, "layers": [{"wave": "sine", "freq": 440, "whole": true, "gain": 0.5}]}"#,
    );
    assert!(e.contains("only a noise layer"), "{e}");
}

// ---- what is rendered ----

#[test]
fn a_cue_lasts_its_layers_and_a_short_tail() {
    let b = bank(BEEP);
    // 1 + 100 + 1 ms, then 10 ms of silence: 112 ms at 8 samples a ms
    let pcm = b.samples("beep", 100, 1).unwrap();
    assert_eq!(pcm.len(), 896);
    assert!(pcm[102 * 8..].iter().all(|s| *s == 0));
}

#[test]
fn a_sine_sounds_at_its_pitch_and_level() {
    let b = bank(BEEP);
    let pcm = b.samples("beep", 100, 1).unwrap();
    // 1000 Hz over the 100 ms hold (samples 8 to 808) is 100 cycles
    let c = cycles(&pcm, 8, 808);
    assert!((99..=101).contains(&c), "{c} cycles");
    // half gain of a 10 000 full scale is a peak of 5000
    let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap();
    assert!((4990..=5000).contains(&peak), "{peak}");
}

#[test]
fn a_glide_moves_from_its_pitch_to_its_second() {
    let b = bank(
        r#"{"id": "up", "layers": [{"wave": "sine", "freq": 200, "to_freq": 800, "attack_ms": 1, "hold_ms": 400, "release_ms": 1, "gain": 0.5}]}"#,
    );
    let pcm = b.samples("up", 100, 1).unwrap();
    // an exponential glide over two octaves in 402 ms, f = 200·4^(t/402):
    // averaged over the first 50 ms it is 218 Hz (10.9 cycles), over the
    // last 50 ms (352 to 402 ms) 735 Hz (36.8 cycles)
    let first = cycles(&pcm, 0, 400);
    let last = cycles(&pcm, 2816, 3216);
    assert!((10..=12).contains(&first), "{first}");
    assert!((36..=38).contains(&last), "{last}");
}

#[test]
fn volume_scales_the_sound_and_zero_is_silent() {
    let b = bank(BEEP);
    let full = b.samples("beep", 100, 1).unwrap();
    let half = b.samples("beep", 50, 1).unwrap();
    assert!(b.samples("beep", 0, 1).unwrap().iter().all(|s| *s == 0));
    for (f, h) in full.iter().zip(&half) {
        assert!((*f as i32 / 2 - *h as i32).abs() <= 1, "{f} {h}");
    }
    // above 100% is 100%
    assert_eq!(b.samples("beep", 250, 1).unwrap(), full);
}

#[test]
fn the_same_seed_gives_the_same_sound_and_another_seed_another_pitch() {
    let b = bank(
        r#"{"id": "tick", "vary_cents": 1200, "layers": [{"wave": "sine", "freq": 1000, "attack_ms": 1, "hold_ms": 100, "release_ms": 1, "gain": 0.5}]},
           {"id": "hiss", "layers": [{"wave": "noise", "attack_ms": 1, "hold_ms": 20, "release_ms": 1, "gain": 0.5}]}"#,
    );
    assert_eq!(b.wav("tick", 100, 7), b.wav("tick", 100, 7));
    assert_eq!(b.wav("hiss", 100, 7), b.wav("hiss", 100, 7));
    assert_ne!(b.wav("hiss", 100, 7), b.wav("hiss", 100, 8));
    // up to an octave either way: 50 to 200 cycles over the hold, and not
    // the same for every seed
    let counts: Vec<usize> = (0..12)
        .map(|seed| cycles(&b.samples("tick", 100, seed).unwrap(), 8, 808))
        .collect();
    assert!(counts.iter().all(|c| (49..=201).contains(c)), "{counts:?}");
    assert!(counts.iter().any(|c| *c != counts[0]), "{counts:?}");
    // with no vary_cents the seed leaves a tone alone
    let b = bank(BEEP);
    assert_eq!(b.wav("beep", 100, 1), b.wav("beep", 100, 99));
}

#[test]
fn a_loop_wraps_so_its_end_runs_into_its_start() {
    let b = bank(
        r#"{"id": "bed", "loop_ms": 100, "layers": [{"wave": "sine", "freq": 500, "at_ms": 95, "attack_ms": 1, "hold_ms": 18, "release_ms": 1, "gain": 0.5}]}"#,
    );
    let pcm = b.samples("bed", 100, 1).unwrap();
    // exactly the loop's length, and the tone from 95 ms comes round to 0..15 ms
    assert_eq!(pcm.len(), 800);
    assert!(pcm[8..100].iter().any(|s| s.unsigned_abs() > 4000));
    assert!(pcm[200..700].iter().all(|s| *s == 0));
}

#[test]
fn notes_and_repeats_lay_out_copies() {
    let b = bank(
        r#"{"id": "run", "layers": [{"wave": "sine", "freq": 400, "attack_ms": 1, "release_ms": 9, "gain": 0.5, "notes": [[0, 400], [50, 800]], "repeat": {"times": 2, "every_ms": 200}}]}"#,
    );
    let cue = b.cue("run").unwrap();
    let starts: Vec<(i64, i64)> = cue.layers.iter().map(|l| (l.at_ms, l.freq)).collect();
    assert_eq!(
        starts,
        vec![(0, 400_000), (50, 800_000), (200, 400_000), (250, 800_000)]
    );
    // the last copy ends at 250 + 10 ms
    assert_eq!(cue.length_ms(), 260);
}

#[test]
fn a_wav_file_has_its_header_and_its_samples() {
    let b = bank(BEEP);
    let bytes = b.wav("beep", 100, 1).unwrap();
    // 44 header bytes and 896 samples of 2 bytes
    assert_eq!(bytes.len(), 44 + 1792);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        36 + 1792
    );
    assert_eq!(
        u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        8000
    );
    let (rate, pcm) = wav::parse(&bytes).unwrap();
    assert_eq!(rate, 8000);
    assert_eq!(pcm, b.samples("beep", 100, 1).unwrap());
    assert_eq!(
        b.wav("nothing", 100, 1),
        Err("There is no sound \"nothing\".".into())
    );
}

// ---- the maths ----

#[test]
fn the_fixed_point_maths_is_close() {
    assert_eq!(synth::sin(0), 0);
    let quarter = 1u32 << 30;
    assert!((synth::sin(quarter) - ONE).abs() <= 8);
    assert!((synth::sin(quarter * 3) + ONE).abs() <= 8);
    // sin 30° is a half
    assert!((synth::sin(u32::MAX / 12) - ONE / 2).abs() <= 8);
    assert!((synth::exp2(ONE) - 2 * ONE).abs() <= 2);
    assert!((synth::exp2(ONE / 2) - 92_682).abs() <= 4); // √2
    assert!((synth::exp2(-ONE) - ONE / 2).abs() <= 2);
    assert!((synth::log2(8 * ONE) - 3 * ONE).abs() <= 2);
    assert!((synth::log2(ONE / 4) + 2 * ONE).abs() <= 2);
    // 1200 cents is an octave
    assert_eq!(synth::cents_to_octaves(1_200_000), ONE);
}
