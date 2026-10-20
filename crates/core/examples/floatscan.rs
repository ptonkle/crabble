//! Floating tiles on the shipped map (SECOND-ORDER R16.*): for each loose
//! tile, how far its lowest point is over what is under it (the ground or
//! another tile's face), sampled on a 4 mm grid over its underside and
//! sides, and whether its middle is over what it touches. Run at the start
//! of a game, and after pulling each tile others lie on and running 5 s.
//!
//! `cargo run --release -p tool_core --example floatscan [-- quick]`
//! (`PULL=<id>` traces the tiles awake through that pull, tick by tick)

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::app::App;
use tool_core::gfx::geom::{v3, V3};
use tool_core::sim::{Driver, Match, Setup, State, Tiles};

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap()
}

fn bundle(dir: &str) -> String {
    let at = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data").join(dir);
    let mut paths: Vec<PathBuf> = fs::read_dir(&at).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    let parts: Vec<String> = paths
        .iter()
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|p| {
            format!(
                "\"{}\":{}",
                p.file_stem().unwrap().to_str().unwrap(),
                fs::read_to_string(p).unwrap()
            )
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn mm(q: i32) -> f64 {
    q as f64 * 1000.0 / 65536.0
}

/// The gap (Q16) between tile `i`'s lowest point and what is under it.
fn gap(g: &Match, ts: &Tiles, i: usize) -> (i32, bool) {
    let t = &ts.all[i];
    let step = 4;
    let mut pts: Vec<V3> = Vec::new();
    let q = |mm: i32| mm * 65536 / 1000;
    for a in (-100..=100).step_by(step) {
        for b in (-100..=100).step_by(step) {
            pts.push(t.pose.apply(v3(q(a), 0, q(b))));
        }
        for y in (0..=45).step_by(5) {
            for (u, w) in [(a, -100), (a, 100), (-100, a), (100, a)] {
                pts.push(t.pose.apply(v3(q(u), q(y), q(w))));
            }
        }
    }
    let mut best = i32::MAX;
    let mut touch = Vec::new();
    let mut side = false;
    for p in pts {
        let mut s = g.ground.height(p.x, p.z);
        for (j, o) in ts.all.iter().enumerate() {
            if j == i || o.is_held() || (o.pose.t.x - p.x).abs() > q(160) || (o.pose.t.z - p.z).abs() > q(160)
            {
                continue;
            }
            let d = p.sub(o.pose.t);
            let (lx, ly, lz) = (d.dot(o.pose.c0), d.dot(o.pose.c1), d.dot(o.pose.c2));
            if lx.abs() <= q(103)
                && lz.abs() <= q(103)
                && ly >= q(-3)
                && ly <= q(43)
                && (lx.abs() > q(97) || lz.abs() > q(97))
            {
                side = true;
            }
            if let Some(h) = o.top_at(p.x, p.z) {
                if h <= p.y + q(1) && h > s {
                    s = h;
                }
            }
        }
        best = best.min(p.y - s);
        if p.y - s <= q(2) {
            touch.push(p);
        }
    }
    // its middle over what it touches: inside their outline from above
    let m = t.middle();
    let over = !touch.is_empty()
        && (0..64).all(|k| {
            let a = k as f64 * std::f64::consts::TAU / 64.0;
            let (ux, uz) = (a.cos(), a.sin());
            touch
                .iter()
                .map(|c| (c.x - m.x) as f64 * ux + (c.z - m.z) as f64 * uz)
                .fold(f64::MIN, f64::max)
                >= -(q(3) as f64)
        });
    (best, over || side)
}

fn report(label: &str, g: &Match, ts: &Tiles) -> Vec<(usize, f64)> {
    let mut out = Vec::new();
    let mut tipping = 0;
    for i in 0..ts.all.len() {
        if !ts.all[i].is_loose() {
            continue;
        }
        let (d, over) = gap(g, ts, i);
        let d = mm(d);
        if d > 2.0 {
            out.push((i, d));
        } else if !over {
            tipping += 1;
        }
    }
    println!("{label}: {tipping} touching with their middle past what they touch");
    let worst = out.iter().map(|o| o.1).fold(0f64, f64::max);
    println!(
        "{label}: {} loose tiles over 2 mm above what is under them, worst {worst:.1} mm",
        out.len()
    );
    out
}

fn main() {
    let quick = std::env::args().nth(1).is_some();
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.glyphs", &data("glyphs.json")).unwrap();
    app.act("data.look", &data("look.json")).unwrap();
    let t0 = std::time::Instant::now();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    println!(
        "the map read and its piles settled in {:.0} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    let t = app.tuning.as_ref().unwrap();
    let map = app.view.map.as_ref().unwrap();
    let fresh = Match::new(
        t,
        map,
        Setup {
            drivers: vec![Driver::Ai; 4],
            seed: 1,
            fast: true,
            length_ms: None,
        },
    );
    let start = report("start", &fresh, &fresh.tiles);
    for (i, d) in start.iter().take(12) {
        let tl = &fresh.tiles.all[*i];
        println!(
            "  tile {i} at {:.0},{:.0},{:.0} up {:.2} gap {d:.1} mm rests_on {:?}",
            mm(tl.pose.t.x),
            mm(tl.pose.t.y),
            mm(tl.pose.t.z),
            tl.pose.c1.y as f64 / 65536.0,
            tl.rests_on
        );
    }
    let ph = fresh.rules.physics.clone();
    let dt = fresh.rules.tick_ms as i32;
    let (mut pulls, mut floats, mut worst, mut forced, mut unheld) = (0, 0, 0f64, 0, 0);
    for id in 0..fresh.tiles.all.len() {
        if !fresh.tiles.all.iter().any(|o| o.rests_on.contains(&(id as u16)))
            || !fresh.tiles.all[id].is_loose()
        {
            continue;
        }
        if quick && pulls >= 30 {
            break;
        }
        pulls += 1;
        let mut g = fresh.clone();
        g.tiles.all[id].state = State::Held { seat: 0, claw: 0 };
        g.tiles.wake_above(id as u16);
        let mut woke = vec![false; g.tiles.all.len()];
        for _ in 0..250 {
            for (k, tl) in g.tiles.all.iter().enumerate() {
                woke[k] |= tl.state == (State::Loose { awake: true });
            }
            if std::env::var("PULL").ok().and_then(|v| v.parse::<usize>().ok()) == Some(id) {
                let aw: Vec<String> = g
                    .tiles
                    .all
                    .iter()
                    .filter(|t| t.state == (State::Loose { awake: true }))
                    .map(|t| {
                        format!(
                            "{}@{} y{:.1} up{:.3} on{:?}",
                            t.id,
                            t.awake_ticks,
                            mm(t.pose.t.y),
                            t.pose.c1.y as f64 / 65536.0,
                            t.rests_on
                        )
                    })
                    .collect();
                eprintln!("{}", aw.join(" "));
            }
            for s in g.tiles.step(&g.ground, &ph, dt) {
                if g.tiles.all[s as usize].awake_ticks >= ph.settle_ticks {
                    forced += 1;
                }
            }
        }
        for tl in &g.tiles.all {
            if tl.state == (State::Loose { awake: true }) {
                println!(
                    "  pull {id}: tile {} still awake after 5 s ({} ticks awake) at {:.0},{:.0},{:.0} up {:.2} vel {:?} spin {:?}",
                    tl.id, tl.awake_ticks, mm(tl.pose.t.x), mm(tl.pose.t.y), mm(tl.pose.t.z), tl.pose.c1.y as f64 / 65536.0, tl.vel, tl.spin
                );
            }
        }
        for (k, &w) in woke.iter().enumerate() {
            if !w {
                continue;
            }
            let (d, over) = gap(&g, &g.tiles, k);
            let d = mm(d);
            if !over && d <= 2.0 {
                unheld += 1;
                if unheld <= 10 {
                    let tl = &g.tiles.all[k];
                    println!(
                        "  pull {id}: tile {k} middle past what it touches, up {:.2} awake {}",
                        tl.pose.c1.y as f64 / 65536.0,
                        tl.awake_ticks
                    );
                }
            }
            if d > 2.0 {
                floats += 1;
                worst = worst.max(d);
                if floats <= 15 {
                    let tl = &g.tiles.all[k];
                    println!(
                        "  pull {id}: tile {k} floats {d:.1} mm at {:.0},{:.0},{:.0} up {:.2} awake {}",
                        mm(tl.pose.t.x),
                        mm(tl.pose.t.y),
                        mm(tl.pose.t.z),
                        tl.pose.c1.y as f64 / 65536.0,
                        tl.awake_ticks
                    );
                }
            }
        }
    }
    println!("{pulls} pulls, 5 s each: {unheld} woken tiles with their middle past what they touch; {floats} woken tiles over 2 mm above what is under them (worst {worst:.1} mm); {forced} put down at the time limit");
}
