//! Writes every sound in data/sounds.json to a directory as .wav files, to
//! listen to (scripts/sounds.py runs it). A sound that varies by the seed is
//! also written with two more seeds (`grab-2.wav`, `grab-3.wav`).
//!
//!     cargo run -p tool_core --example write_sounds -- OUT_DIR [VOLUME]

use std::fs;
use std::path::PathBuf;

use tool_core::sound::Bank;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("usage: write_sounds OUT_DIR [VOLUME]"));
    let volume: u32 = args
        .next()
        .map_or(100, |v| v.parse().expect("VOLUME is a percent"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = fs::read_to_string(root.join("data/sounds.json")).expect("read data/sounds.json");
    let bank = Bank::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    fs::create_dir_all(&out).expect("make the output directory");
    for cue in &bank.cues {
        let varies = cue.vary_cents > 0 || cue.layers.iter().any(|l| l.scatter.is_some());
        let seeds: &[(u64, &str)] = if varies {
            &[(1, ""), (2, "-2"), (3, "-3")]
        } else {
            &[(1, "")]
        };
        for (seed, suffix) in seeds {
            let bytes = bank.wav(&cue.id, volume, *seed).expect("a listed sound");
            let path = out.join(format!("{}{suffix}.wav", cue.id));
            fs::write(&path, bytes).expect("write a sound");
            println!("{}", path.display());
        }
    }
}
