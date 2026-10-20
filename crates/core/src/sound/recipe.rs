//! Recipes: how each sound is made from oscillators, noise, envelopes and
//! filters (`data/sounds.json`), checked when the file is read. The format
//! is Honey Hole's (its page plays recipes with WebAudio); here the core
//! renders them itself (`synth.rs`), so the page plays finished WAV bytes.
//!
//! A **cue** is a list of layers, each a waveform or noise with a pitch (and
//! an optional glide to a second pitch), an envelope (attack, hold, release)
//! and an optional filter (with its own sweep). A layer can repeat itself
//! (`repeat`), play a little tune (`notes`), or be strewn about by the seed
//! (`scatter`: bubbles, a tumble). A cue may vary its pitch by the seed
//! (`vary_cents`), so a sound heard again and again is never quite the same.
//!
//! A cue with `loop_ms` is a **loop**: everything in it wraps round that
//! length so the end runs into the start, and a `whole` noise layer fills it
//! from end to end (an ambience bed).
//!
//! Units: frequencies in Hz, times in milliseconds, gains 0 to 1, Q as
//! WebAudio's, cents as a musician's. Held as thousandths (`decimal.rs`).

use serde_json::Value;

use super::decimal::{parse_milli, MILLI};
use super::read::{self, Obj};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Noise,
}

impl Wave {
    const ALL: [(&'static str, Wave); 5] = [
        ("sine", Wave::Sine),
        ("square", Wave::Square),
        ("sawtooth", Wave::Sawtooth),
        ("triangle", Wave::Triangle),
        ("noise", Wave::Noise),
    ];

    fn parse(o: &Obj, at: &str) -> Result<Wave, String> {
        let name = read::text(o, "wave", at)?;
        Wave::ALL
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, w)| *w)
            .ok_or_else(|| {
                format!("{at} has wave \"{name}\"; a wave is sine, square, sawtooth, triangle or noise.")
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Lowpass,
    Highpass,
    Bandpass,
}

/// Q when a filter names none (WebAudio's own default).
const DEFAULT_Q: i64 = 707;

/// How a pitch or a filter sweep moves: exponential sounds even to the
/// ear; linear is for a sweep that should rush at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glide {
    Exp,
    Linear,
}

/// How the release falls: `curved` (quadratic, a pluck or a knock dying
/// away) or `linear`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fall {
    Curved,
    Linear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub kind: FilterKind,
    /// Hz, thousandths
    pub freq: i64,
    pub to_freq: Option<i64>,
    /// thousandths
    pub q: i64,
}

impl Filter {
    fn parse(v: &Value, at: &str, top_hz: i64) -> Result<Filter, String> {
        let at = format!("{at} filter");
        let o = read::obj(v, &at)?;
        read::keys(o, &["type", "freq", "to_freq", "q"], &at)?;
        let kind = match read::text(o, "type", &at)? {
            "lowpass" => FilterKind::Lowpass,
            "highpass" => FilterKind::Highpass,
            "bandpass" => FilterKind::Bandpass,
            name => {
                return Err(format!(
                    "{at} has type \"{name}\"; a filter is lowpass, highpass or bandpass."
                ))
            }
        };
        let hz = |key: &str| -> Result<Option<i64>, String> {
            let what = format!("{at} {key}");
            read::opt_milli(o, key, &at)?
                .map(|f| {
                    read::within(
                        f,
                        1,
                        top_hz * MILLI,
                        &what,
                        &format!("more than 0 and at most {top_hz}"),
                    )
                })
                .transpose()
        };
        let freq = hz("freq")?.ok_or_else(|| format!("{at} has no \"freq\"."))?;
        let q = read::opt_milli(o, "q", &at)?.unwrap_or(DEFAULT_Q);
        Ok(Filter {
            kind,
            freq,
            to_freq: hz("to_freq")?,
            q: read::within(q, 100, 30 * MILLI, &format!("{at} q"), "0.1 to 30")?,
        })
    }
}

/// A slow bend of the pitch, `cents` either way, `rate` times a second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wobble {
    /// Hz, thousandths
    pub rate: i64,
    /// cents, thousandths
    pub cents: i64,
}

/// Copies of a layer strewn by the seed: `times` of them, each starting
/// somewhere in the first `over_ms`, each up to `cents` off pitch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scatter {
    pub times: i64,
    pub over_ms: i64,
    /// cents, thousandths
    pub cents: i64,
}

/// The widest a pitch may be moved by the seed or a wobble: an octave.
const MOST_CENTS: i64 = 1200 * MILLI;

fn positive(v: i64, what: &str) -> Result<i64, String> {
    if v > 0 {
        Ok(v)
    } else {
        Err(format!("{what} must be more than zero."))
    }
}

fn cents(o: &Obj, key: &str, at: &str) -> Result<i64, String> {
    let c = read::milli(o, key, at)?;
    read::within(c, 0, MOST_CENTS, &format!("{at} {key}"), "0 to 1200")
}

/// One sounding part of a cue, its notes and repeats already laid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub wave: Wave,
    /// when it starts after the cue does
    pub at_ms: i64,
    /// Hz in thousandths; zero for noise
    pub freq: i64,
    pub to_freq: Option<i64>,
    pub glide: Glide,
    pub attack_ms: i64,
    pub hold_ms: i64,
    pub release_ms: i64,
    pub fall: Fall,
    /// thousandths
    pub gain: i64,
    pub filter: Option<Filter>,
    pub wobble: Option<Wobble>,
    pub scatter: Option<Scatter>,
    /// fills a loop from end to end at a steady gain (noise only)
    pub whole: bool,
}

impl Layer {
    /// When it has finished, as written (a scatter's latest copy included).
    pub fn end_ms(&self) -> i64 {
        let spread = self.scatter.as_ref().map_or(0, |s| s.over_ms);
        self.at_ms + spread + self.attack_ms + self.hold_ms + self.release_ms
    }

    /// The layer as written, then one copy per note and per repeat.
    fn parse(v: &Value, at: &str, top_hz: i64) -> Result<Vec<Layer>, String> {
        let o = read::obj(v, at)?;
        read::keys(
            o,
            &[
                "wave",
                "at_ms",
                "freq",
                "to_freq",
                "glide",
                "attack_ms",
                "hold_ms",
                "release_ms",
                "fall",
                "gain",
                "filter",
                "wobble",
                "notes",
                "repeat",
                "scatter",
                "whole",
            ],
            at,
        )?;
        let wave = Wave::parse(o, at)?;
        let pitch = |key: &str| -> Result<Option<i64>, String> {
            let what = format!("{at} {key}");
            read::opt_milli(o, key, at)?
                .map(|f| {
                    read::within(
                        f,
                        1,
                        top_hz * MILLI,
                        &what,
                        &format!("more than 0 and at most {top_hz}"),
                    )
                })
                .transpose()
        };
        let (freq, to_freq) = if wave == Wave::Noise {
            if o.contains_key("freq") || o.contains_key("to_freq") {
                return Err(format!(
                    "{at} is noise, which has no freq; colour it with a filter."
                ));
            }
            (0, None)
        } else {
            let f = pitch("freq")?.ok_or_else(|| format!("{at} has no \"freq\"."))?;
            (f, pitch("to_freq")?)
        };
        let glide = match read::opt_text(o, "glide", at)? {
            None | Some("exp") => Glide::Exp,
            Some("linear") => Glide::Linear,
            Some(other) => return Err(format!("{at} has glide \"{other}\"; a glide is exp or linear.")),
        };
        let fall = match read::opt_text(o, "fall", at)? {
            None | Some("curved") => Fall::Curved,
            Some("linear") => Fall::Linear,
            Some(other) => return Err(format!("{at} has fall \"{other}\"; a fall is curved or linear.")),
        };
        let ms = |key: &str, least: i64| -> Result<i64, String> {
            let v = read::opt_whole(o, key, at, least)?;
            if v < least {
                Err(format!("{at} {key} must be at least {least}."))
            } else {
                Ok(v)
            }
        };
        let gain = read::milli(o, "gain", at)?;
        let wobble = match o.get("wobble") {
            None => None,
            Some(w) => {
                let what = format!("{at} wobble");
                let wo = read::obj(w, &what)?;
                read::keys(wo, &["rate", "cents"], &what)?;
                Some(Wobble {
                    rate: positive(read::milli(wo, "rate", &what)?, &format!("{what} rate"))?,
                    cents: cents(wo, "cents", &what)?,
                })
            }
        };
        let scatter = match o.get("scatter") {
            None => None,
            Some(s) => {
                let what = format!("{at} scatter");
                let so = read::obj(s, &what)?;
                read::keys(so, &["times", "over_ms", "cents"], &what)?;
                let times = read::whole(so, "times", &what)?;
                let over_ms = read::whole(so, "over_ms", &what)?;
                if !(1..=64).contains(&times) || over_ms < 1 {
                    return Err(format!("{what} must strew 1 to 64 copies over at least 1 ms."));
                }
                Some(Scatter {
                    times,
                    over_ms,
                    cents: cents(so, "cents", &what)?,
                })
            }
        };
        let whole = read::flag(o, "whole", at)?;
        if whole && (wave != Wave::Noise || o.contains_key("filter") && filter_sweeps(o)) {
            return Err(format!(
                "{at} is whole, which only a noise layer with a steady filter can be."
            ));
        }
        let base = Layer {
            wave,
            at_ms: ms("at_ms", 0)?,
            freq,
            to_freq,
            glide,
            // at least a millisecond each way, or the sound clicks
            attack_ms: ms("attack_ms", 1)?,
            hold_ms: ms("hold_ms", 0)?,
            release_ms: ms("release_ms", 1)?,
            fall,
            gain: read::within(gain, 1, MILLI, &format!("{at} gain"), "more than 0 and at most 1")?,
            filter: o
                .get("filter")
                .map(|f| Filter::parse(f, at, top_hz))
                .transpose()?,
            wobble,
            scatter,
            whole,
        };

        let mut out = vec![base.clone()];
        if let Some(notes) = o.get("notes") {
            if wave == Wave::Noise {
                return Err(format!("{at} is noise, which sounds no notes."));
            }
            let list = notes
                .as_array()
                .filter(|l| !l.is_empty())
                .ok_or_else(|| format!("{at} notes must be a list of [at_ms, freq] pairs."))?;
            out = Vec::new();
            for (k, n) in list.iter().enumerate() {
                let what = format!("{at} note {}", k + 1);
                let pair = n
                    .as_array()
                    .filter(|p| p.len() == 2)
                    .ok_or_else(|| format!("{what} must be [at_ms, freq]."))?;
                let when = pair[0]
                    .as_i64()
                    .filter(|w| *w >= 0)
                    .ok_or_else(|| format!("{what}: at_ms must be a whole number from 0."))?;
                let f = read::within(
                    parse_milli(&pair[1], &what)?,
                    1,
                    top_hz * MILLI,
                    &what,
                    &format!("more than 0 and at most {top_hz}"),
                )?;
                let mut l = base.clone();
                l.at_ms += when;
                l.freq = f;
                // a glide keeps its shape on every note
                l.to_freq = base.to_freq.map(|t| t * f / base.freq);
                out.push(l);
            }
        }
        if let Some(r) = o.get("repeat") {
            let what = format!("{at} repeat");
            let ro = read::obj(r, &what)?;
            read::keys(ro, &["times", "every_ms"], &what)?;
            let times = read::whole(ro, "times", &what)?;
            let every = read::whole(ro, "every_ms", &what)?;
            if !(1..=64).contains(&times) || every < 1 {
                return Err(format!("{what} must sound 1 to 64 times, at least 1 ms apart."));
            }
            let once = out;
            out = Vec::new();
            for k in 0..times {
                for l in &once {
                    let mut c = l.clone();
                    c.at_ms += k * every;
                    out.push(c);
                }
            }
        }
        Ok(out)
    }
}

fn filter_sweeps(o: &Obj) -> bool {
    o.get("filter")
        .and_then(Value::as_object)
        .is_some_and(|f| f.contains_key("to_freq"))
}

/// A recipe: a sound the game asks for by its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    pub id: String,
    pub layers: Vec<Layer>,
    /// how far the seed may move the whole cue's pitch, either way
    /// (cents, thousandths)
    pub vary_cents: i64,
    /// a loop's length; `None` for a sound that plays once
    pub loop_ms: Option<i64>,
}

impl Cue {
    pub fn parse(o: &Obj, id: &str, at: &str, top_hz: i64) -> Result<Cue, String> {
        read::keys(o, &["id", "layers", "vary_cents", "loop_ms"], at)?;
        let list = o
            .get("layers")
            .and_then(Value::as_array)
            .filter(|l| !l.is_empty())
            .ok_or_else(|| format!("{at} needs a list of layers."))?;
        let mut layers = Vec::new();
        for (k, l) in list.iter().enumerate() {
            layers.extend(Layer::parse(l, &format!("{at} layer {}", k + 1), top_hz)?);
        }
        let vary_cents = match o.get("vary_cents") {
            None => 0,
            Some(_) => cents(o, "vary_cents", at)?,
        };
        let loop_ms = match o.get("loop_ms") {
            None => None,
            Some(_) => Some(positive(
                read::whole(o, "loop_ms", at)?,
                &format!("{at} loop_ms"),
            )?),
        };
        let cue = Cue {
            id: id.to_string(),
            layers,
            vary_cents,
            loop_ms,
        };
        match loop_ms {
            Some(len) => {
                let once = |l: &Layer| l.attack_ms + l.hold_ms + l.release_ms;
                if let Some(l) = cue.layers.iter().find(|l| !l.whole && once(l) > len) {
                    return Err(format!(
                        "{at} has a layer lasting {} ms, longer than its loop of {len} ms.",
                        once(l)
                    ));
                }
            }
            None => {
                if cue.layers.iter().any(|l| l.whole) {
                    return Err(format!("{at} has a whole layer but is not a loop."));
                }
            }
        }
        Ok(cue)
    }

    /// How long the cue lasts as written: a loop's length, or when its last
    /// layer ends.
    pub fn length_ms(&self) -> i64 {
        self.loop_ms
            .unwrap_or_else(|| self.layers.iter().map(Layer::end_ms).max().unwrap_or(0))
    }
}
