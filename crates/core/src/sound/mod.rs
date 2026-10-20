//! Sounds (DECISIONS.md, 2026-10-19 sounds). There are no audio files:
//! every sound is a recipe in `data/sounds.json`, and the core renders it as
//! a complete WAV file (16-bit mono PCM) that the page decodes and plays.
//! The page holds no frequencies, lengths or levels.
//!
//! - `recipe.rs`: the recipes, checked (Honey Hole's format).
//! - `synth.rs`: the integer synthesiser that renders them.
//! - `wav.rs`: the WAV writer, and a reader for the tests.
//!
//! A sound heard again and again (a grab, a placement, a tally tick) varies
//! its pitch by the seed the caller passes, never by a random source.

use serde_json::Value;

pub mod decimal;
mod read;
pub mod recipe;
pub mod synth;
#[cfg(test)]
mod tests;
pub mod wav;

use recipe::Cue;

/// The sounds the game asks for, by key. Each is in `data/sounds.json`
/// (`tests/sounds.rs`).
pub const KEYS: &[&str] = &[
    "grab",
    "place",
    "word",
    "drop",
    "jump",
    "land",
    "idle_tick",
    "penalty",
    "countdown",
    "go",
    "end",
    "tally",
    "fanfare",
    "ambience",
];

/// The volume (percent) the page uses until a setting gives another.
pub const DEFAULT_VOLUME: u32 = 50;

/// The longest a sound may last as written.
const MAX_MS: i64 = 20_000;

/// `data/sounds.json`, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    /// samples a second
    pub sample_rate: i64,
    /// the 16-bit level of a full-scale sound at 100% volume
    pub master: i64,
    /// in file order
    pub cues: Vec<Cue>,
}

/// A cue id: lowercase words joined by `_`.
fn check_id(id: &str, at: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!("{at} id \"{id}\" must be lowercase words joined by _."));
    }
    Ok(())
}

impl Bank {
    pub fn parse(sounds_json: &str) -> Result<Bank, String> {
        let all: Value = serde_json::from_str(sounds_json)
            .map_err(|e| format!("The sounds file could not be read: {e}"))?;
        let top = "The sounds file";
        let o = read::obj(&all, top)?;
        read::keys(o, &["sample_rate", "master", "cues"], top)?;
        let sample_rate = read::whole(o, "sample_rate", top)?;
        if !(8_000..=48_000).contains(&sample_rate) {
            return Err("The sounds file sample_rate must be 8000 to 48000.".into());
        }
        let master = read::whole(o, "master", top)?;
        if !(1..=i16::MAX as i64).contains(&master) {
            return Err(format!("The sounds file master must be 1 to {}.", i16::MAX));
        }
        // a pitch or a filter must stay below half the sample rate
        let top_hz = sample_rate * 2 / 5;
        let list = o
            .get("cues")
            .and_then(Value::as_array)
            .ok_or_else(|| "The sounds file needs a list of cues.".to_string())?;
        let mut cues: Vec<Cue> = Vec::new();
        for (k, v) in list.iter().enumerate() {
            let c = read::obj(v, &format!("Cue {}", k + 1))?;
            let id = read::text(c, "id", &format!("Cue {}", k + 1))?;
            let at = format!("Cue {id}");
            check_id(id, &at)?;
            if cues.iter().any(|q| q.id == id) {
                return Err(format!("{at} is listed twice."));
            }
            let cue = Cue::parse(c, id, &at, top_hz)?;
            if cue.length_ms() > MAX_MS {
                return Err(format!(
                    "{at} lasts {} ms; a sound lasts at most {MAX_MS} ms.",
                    cue.length_ms()
                ));
            }
            cues.push(cue);
        }
        Ok(Bank {
            sample_rate,
            master,
            cues,
        })
    }

    pub fn cue(&self, id: &str) -> Option<&Cue> {
        self.cues.iter().find(|c| c.id == id)
    }

    /// The cue `id` as 16-bit samples at `volume` percent (0 to 100), its
    /// pitch and noise moved by `seed`.
    pub fn samples(&self, id: &str, volume: u32, seed: u64) -> Option<Vec<i16>> {
        let cue = self.cue(id)?;
        let mix = synth::render(cue, self.sample_rate, seed);
        Some(to_pcm(&mix, self.master, volume))
    }

    /// The cue `id` as a complete WAV file, or a refusal naming it.
    pub fn wav(&self, id: &str, volume: u32, seed: u64) -> Result<Vec<u8>, String> {
        let pcm = self
            .samples(id, volume, seed)
            .ok_or_else(|| format!("There is no sound \"{id}\"."))?;
        Ok(wav::file(&pcm, self.sample_rate as u32))
    }
}

/// A Q16.16 mix (ONE is full scale) as 16-bit samples: full scale is
/// `master` at 100% volume.
pub fn to_pcm(mix: &[i64], master: i64, volume: u32) -> Vec<i16> {
    let vol = volume.min(100) as i64;
    mix.iter()
        .map(|v| {
            let s = synth::mul(*v, master) * vol / 100;
            s.clamp(i16::MIN as i64, i16::MAX as i64) as i16
        })
        .collect()
}
