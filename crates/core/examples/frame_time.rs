//! How long the core takes to build a game frame on the shipped map
//! (`app::view3d`): 4 crabs (one local, three AI), the page's 16 ms
//! frames, natively in release. The page also asks for the static layers
//! again when they change; that is timed apart, once.
//!
//!     cargo run --release -p tool_core --example frame_time [FRAMES]
//!
//! Prints the mean and the slowest frame build, and the triangles drawn
//! each frame (solid and see-through, both passes).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tool_core::app::App;

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn bundle(dir: &str) -> String {
    let at = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data").join(dir);
    let mut paths: Vec<PathBuf> = fs::read_dir(&at).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    let parts: Vec<String> = paths
        .iter()
        .map(|p| {
            let id = p.file_stem().unwrap().to_str().unwrap();
            format!("\"{id}\":{}", fs::read_to_string(p).unwrap())
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.glyphs", &data("glyphs.json")).unwrap();
    app.act("data.look", &data("look.json")).unwrap();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    app.act("flag.fast", "on").unwrap();
    app.act("local.ai", "3").unwrap();
    app.act("local.start", "").unwrap();
    app.resize(1920, 1080, 100);
    // the static layers, once (the world is built on first ask)
    let t0 = Instant::now();
    let mut layer_tris = 0;
    for k in 0..12 {
        layer_tris += app.layer(k).len();
    }
    let layers_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let mut clock = 0i64;
    let (mut total, mut worst, mut tris) = (0.0f64, 0.0f64, 0usize);
    for _ in 0..frames {
        clock += 16_000;
        app.advance(clock);
        let t = Instant::now();
        app.build_frame();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        total += ms;
        worst = worst.max(ms);
        tris = app
            .view
            .frame
            .iter()
            .map(|p| p.solid.len() + p.clear.len())
            .sum();
    }
    println!(
        "{frames} frames: build mean {:.3} ms, slowest {:.3} ms; last frame's per-frame data {} ints; static layers {} ints built in {:.1} ms",
        total / frames as f64,
        worst,
        tris,
        layer_tris,
        layers_ms
    );
}
