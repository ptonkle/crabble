//! The real sounds file: it is one the core accepts, every sound the game
//! asks for is in it, and no sound clips, starts or ends away from silence,
//! at any seed.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::sound::{self, synth, Bank};

fn bank() -> Bank {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sounds.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Bank::parse(&text).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_sound_the_game_asks_for_is_in_the_sounds_file() {
    let b = bank();
    for key in sound::KEYS {
        assert!(b.cue(key).is_some(), "the sounds file has no \"{key}\"");
    }
    assert_eq!(b.cues.len(), sound::KEYS.len(), "a sound the game never asks for");
    assert!(b.cue("ambience").is_some_and(|c| c.loop_ms.is_some()));
}

#[test]
fn no_sound_clips_at_full_volume_at_any_seed() {
    let b = bank();
    for cue in &b.cues {
        for seed in 0..24 {
            let mix = synth::render(cue, b.sample_rate, seed);
            // before the clamp: full scale at 100% is `master`
            let peak = mix
                .iter()
                .map(|v| synth::mul(*v, b.master).abs())
                .max()
                .unwrap_or(0);
            assert!(
                peak < i16::MAX as i64,
                "{} at seed {seed} peaks at {peak}",
                cue.id
            );
            assert!(peak > 0, "{} is silent", cue.id);
        }
    }
}

#[test]
fn every_sound_starts_and_ends_at_silence_with_no_offset() {
    let b = bank();
    for cue in &b.cues {
        let pcm = b.samples(&cue.id, 100, 1).unwrap();
        let mean = pcm.iter().map(|s| *s as i64).sum::<i64>() / pcm.len() as i64;
        // under 1% of full scale
        assert!(mean.abs() < 327, "{} has a DC offset of {mean}", cue.id);
        if cue.loop_ms.is_none() {
            assert_eq!(pcm[0], 0, "{} starts with a click", cue.id);
            assert_eq!(*pcm.last().unwrap(), 0, "{} ends with a click", cue.id);
        }
    }
}
