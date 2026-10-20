//! Nothing floats on the shipped map (SECOND-ORDER R16.*): at the start of
//! a game, and after pulling each tile that others lie on and running 5 s,
//! no loose tile's lowest point is more than 2 mm over what is under it
//! (the ground or another tile's top), sampled every 10 mm over its
//! underside and sides.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::app::App;
use tool_core::gfx::geom::{v3, V3};
use tool_core::sim::{Driver, Match, Setup, State, Tiles};

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

/// Millimetres in the core's units (Q16 metres).
fn q(mm: i32) -> i32 {
    mm * 65536 / 1000
}

/// How far (mm) tile `i`'s lowest point is over what is under it.
fn hover_mm(g: &Match, ts: &Tiles, i: usize) -> i32 {
    let t = &ts.all[i];
    let near: Vec<usize> = (0..ts.all.len())
        .filter(|&j| j != i && !ts.all[j].is_held())
        .filter(|&j| {
            let o = ts.all[j].pose.t;
            (o.x - t.pose.t.x).abs() < q(300) && (o.z - t.pose.t.z).abs() < q(300)
        })
        .collect();
    let mut pts: Vec<V3> = Vec::new();
    for a in (-100..=100).step_by(10) {
        for b in (-100..=100).step_by(10) {
            pts.push(t.pose.apply(v3(q(a), 0, q(b))));
        }
        // up its sides every 9 mm, and half way up (where the physics samples)
        for y in [0, q(9), q(18), q(45) / 2, q(27), q(36), q(45)] {
            for (u, w) in [(a, -100), (a, 100), (-100, a), (100, a)] {
                pts.push(t.pose.apply(v3(q(u), y, q(w))));
            }
        }
    }
    let mut best = i32::MAX;
    for p in pts {
        let mut s = g.ground.height(p.x, p.z);
        for &j in &near {
            if let Some(h) = ts.all[j].top_at(p.x, p.z).filter(|h| *h <= p.y + q(1)) {
                s = s.max(h);
            }
        }
        best = best.min(p.y - s);
    }
    best * 1000 / 65536
}

#[test]
fn no_loose_tile_floats_at_the_start_or_after_any_supporting_tile_is_pulled() {
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    let t = app.tuning.as_ref().unwrap();
    let fresh = Match::new(
        t,
        app.view.map.as_ref().unwrap(),
        Setup {
            drivers: vec![Driver::Ai; 4],
            seed: 1,
            fast: true,
            length_ms: None,
        },
    );
    // the piles stand at most about half a metre high: a tile higher is
    // on something that is not there to see (a light shaft was ground
    // until R16.11, and the start's settle lifted five tiles up them)
    for t in fresh.tiles.all.iter().filter(|t| t.is_loose()) {
        assert!(
            t.pose.t.y < q(700),
            "tile {} starts {} mm up",
            t.id,
            t.pose.t.y * 1000 / 65536
        );
    }
    for i in 0..fresh.tiles.all.len() {
        if fresh.tiles.all[i].is_loose() {
            let h = hover_mm(&fresh, &fresh.tiles, i);
            assert!(
                h <= 2,
                "at the start tile {i} hovers {h} mm over what is under it"
            );
        }
    }
    let ph = fresh.rules.physics.clone();
    let dt = fresh.rules.tick_ms as i32;
    let mut pulls = 0;
    for id in 0..fresh.tiles.all.len() {
        let id16 = id as u16;
        if !fresh.tiles.all[id].is_loose() || !fresh.tiles.all.iter().any(|o| o.rests_on.contains(&id16)) {
            continue;
        }
        pulls += 1;
        let mut g = fresh.clone();
        g.tiles.all[id].state = State::Held { seat: 0, claw: 0 };
        g.tiles.wake_above(id16);
        // 5 s
        for _ in 0..250 {
            g.tiles.step(&g.ground, &ph, dt);
        }
        // every loose tile near the pulled one (the rest have not moved,
        // and nothing under them has)
        let at = fresh.tiles.all[id].pose.t;
        for k in 0..g.tiles.all.len() {
            let p = g.tiles.all[k].pose.t;
            if !g.tiles.all[k].is_loose() || (p.x - at.x).abs() > q(800) || (p.z - at.z).abs() > q(800) {
                continue;
            }
            let h = hover_mm(&g, &g.tiles, k);
            assert!(h <= 2, "pulling tile {id}, tile {k} hovers {h} mm over what is under it (it lay on {:?}, now {:?}, {:?}; before the run {})", fresh.tiles.all[k].rests_on, g.tiles.all[k].rests_on, g.tiles.all[k].state, { let mut f = fresh.clone(); f.tiles.all[id].state = State::Held { seat: 0, claw: 0 }; hover_mm(&f, &f.tiles, k) });
        }
    }
    assert!(pulls > 100, "the map's piles: {pulls} tiles others lie on");
}
