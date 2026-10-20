//! AI crabs on the shipped map: four of them play the first minute of a
//! game through their `SeatInput`s alone, and tiles go onto the board.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::ai::{self, Brain};
use tool_core::app::App;
use tool_core::sim::{Driver, Event, Match, Setup, State};

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The files of `data/<dir>` bundled as the build bundles them.
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

#[test]
fn four_ai_crabs_place_tiles_on_the_shipped_map_in_a_minute() {
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    let t = app.tuning.as_ref().unwrap();
    let mut g = Match::new(
        t,
        app.view.map.as_ref().unwrap(),
        Setup {
            drivers: vec![Driver::Ai; 4],
            seed: 3,
            fast: true,
            length_ms: None,
        },
    );
    let mut brains: Vec<Brain> = (0..4).map(|k| Brain::new(t.ai.clone(), k)).collect();
    let mut placed = [0; 4];
    // a minute of 20 ms ticks
    for _ in 0..3000 {
        let inputs: Vec<_> = (0..4).map(|k| ai::think(&g, k, &mut brains[k])).collect();
        g.step(&inputs);
        for e in g.take_events() {
            if let Event::Placed { seat, .. } = e {
                placed[seat] += 1;
            }
        }
    }
    // a trip to a pile 2-4 m off and back takes about 20 s: each crab
    // places at least once in a minute, and together several times
    assert!(placed.iter().all(|n| *n >= 1), "{placed:?}");
    assert!(placed.iter().sum::<i32>() >= 6, "{placed:?}");
    let on_board = g
        .tiles
        .all
        .iter()
        .filter(|t| matches!(t.state, State::Placed { .. }))
        .count() as i32;
    assert_eq!(
        on_board,
        1 + placed.iter().sum::<i32>(),
        "the centre tile and theirs"
    );
}
