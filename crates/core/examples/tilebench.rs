//! The tile physics' cost and calm on the shipped map (SECOND-ORDER R8.*):
//! the 294 tiles are dealt, then every loose tile is woken at once (the
//! worst case: every pile tumbling together) and the game's tile step is
//! timed tick by tick until all sleep again. Also reports how far the
//! piles moved (a settled pile should barely move when woken) and that
//! no tile ended under the sand.
//!
//! `cargo run --release -p tool_core --example tilebench [-- RUNS]`

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tool_core::ai::{self, Brain};
use tool_core::app::App;
use tool_core::sim::{Driver, Event, Match, Phase, Setup, State};

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
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|p| {
            let id = p.file_stem().unwrap().to_str().unwrap();
            format!("\"{id}\":{}", fs::read_to_string(p).unwrap())
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

fn mm(q: i32) -> f64 {
    q as f64 * 1000.0 / 65536.0
}

fn main() {
    let runs: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(5);
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.glyphs", &data("glyphs.json")).unwrap();
    app.act("data.look", &data("look.json")).unwrap();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    let t = app.tuning.as_ref().unwrap();
    let map = app.view.map.as_ref().unwrap();
    let mut worst = 0f64;
    let mut sum = 0f64;
    let mut ticks_all = 0usize;
    for run in 0..runs {
        let mut g = Match::new(
            t,
            map,
            Setup {
                drivers: vec![Driver::Ai; 4],
                seed: 1 + run as u64,
                fast: true,
                length_ms: None,
            },
        );
        let start: Vec<_> = g.tiles.all.iter().map(|t| t.pose.t).collect();
        if std::env::var("TRACE").is_ok() {
            let mut hist = [0; 10];
            for t in &g.tiles.all {
                let deg = (t.pose.c1.y as f64 / 65536.0).acos().to_degrees();
                hist[((deg / 10.0) as usize).min(9)] += 1;
            }
            eprintln!("tilts by 10 degrees: {hist:?}");
        }
        let mut loose = 0;
        for tile in &mut g.tiles.all {
            if tile.is_loose() {
                tile.state = State::Loose { awake: true };
                loose += 1;
            }
        }
        let ph = g.rules.physics.clone();
        let dt = g.rules.tick_ms as i32;
        let mut ticks = 0;
        let mut most_awake = 0;
        let mut run_worst = 0f64;
        while g.tiles.any_awake() && ticks < 1000 {
            let awake = g
                .tiles
                .all
                .iter()
                .filter(|t| t.state == State::Loose { awake: true })
                .count();
            most_awake = most_awake.max(awake);
            let mut c = g.tiles.clone();
            let t0 = Instant::now();
            c.step(&g.ground, &ph, dt);
            let first = t0.elapsed().as_secs_f64() * 1000.0;
            let t0 = Instant::now();
            g.tiles.step(&g.ground, &ph, dt);
            let ms = first.min(t0.elapsed().as_secs_f64() * 1000.0);
            run_worst = run_worst.max(ms);
            sum += ms;
            ticks += 1;
            if std::env::var("TRACE").is_ok() && ticks % 50 == 0 {
                let aw: Vec<_> = g
                    .tiles
                    .all
                    .iter()
                    .filter(|t| t.state == State::Loose { awake: true })
                    .collect();
                eprintln!("tick {ticks}: {} awake", aw.len());
                for t in aw.iter().take(3) {
                    eprintln!(
                        "  {} at {:.0},{:.0},{:.0} vel {:?} spin {:?} quiet {} awake {} c1 {:?}",
                        t.id,
                        mm(t.pose.t.x),
                        mm(t.pose.t.y),
                        mm(t.pose.t.z),
                        t.vel,
                        t.spin,
                        t.quiet,
                        t.awake_ticks,
                        t.pose.c1
                    );
                }
            }
        }
        ticks_all += ticks;
        worst = worst.max(run_worst);
        let moved: Vec<f64> = g
            .tiles
            .all
            .iter()
            .zip(&start)
            .filter(|(t, _)| t.is_loose())
            .map(|(t, s)| mm(t.pose.t.sub(*s).len()))
            .collect();
        let over = |d: f64| moved.iter().filter(|m| **m > d).count();
        let mut under = 0;
        for tile in &g.tiles.all {
            if !tile.is_loose() {
                continue;
            }
            for q in tile.underside() {
                if q.y < g.ground.height(q.x, q.z) - 65 {
                    under += 1;
                    break;
                }
            }
        }
        println!(
            "run {run}: {loose} loose woken, {most_awake} awake at most, asleep after {ticks} ticks; worst tick {run_worst:.2} ms; moved >5 mm: {}, >20 mm: {}, >100 mm: {}; under the sand: {under}",
            over(5.0),
            over(20.0),
            over(100.0)
        );
    }
    println!(
        "all woken: worst tick {worst:.2} ms, mean {:.3} ms over {ticks_all} ticks (a tick is 20 ms)",
        sum / ticks_all.max(1) as f64
    );
    // a grab: each tile that others lie on is pulled out of a fresh game
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
    let ph = fresh.rules.physics.clone();
    let dt = fresh.rules.tick_ms as i32;
    let (mut pulls, mut worst, mut worst_ticks, mut most_awake, mut forced, mut sum, mut n) =
        (0, 0f64, 0, 0, 0, 0f64, 0);
    let mut far = 0;
    for id in 0..fresh.tiles.all.len() {
        if !fresh.tiles.all.iter().any(|o| o.rests_on.contains(&(id as u16)))
            || !fresh.tiles.all[id].is_loose()
        {
            continue;
        }
        pulls += 1;
        let mut g = fresh.clone();
        g.tiles.all[id].state = State::Held { seat: 0, claw: 0 };
        g.tiles.wake_above(id as u16);
        let start: Vec<_> = g.tiles.all.iter().map(|t| t.pose.t).collect();
        let mut ticks = 0;
        while g.tiles.any_awake() && ticks < 1000 {
            let awake = g
                .tiles
                .all
                .iter()
                .filter(|t| t.state == State::Loose { awake: true })
                .count();
            most_awake = most_awake.max(awake);
            // the least of three runs of the same tick: the machine's own
            // noise left out
            let mut ms = f64::MAX;
            for _ in 0..2 {
                let mut c = g.tiles.clone();
                let t0 = Instant::now();
                c.step(&g.ground, &ph, dt);
                ms = ms.min(t0.elapsed().as_secs_f64() * 1000.0);
            }
            let t0 = Instant::now();
            let settled = g.tiles.step(&g.ground, &ph, dt);
            let ms = ms.min(t0.elapsed().as_secs_f64() * 1000.0);
            if std::env::var("TRACE").is_ok() {
                for t in &g.tiles.all {
                    let watch = std::env::var("WATCH").ok().and_then(|w| w.parse::<u16>().ok());
                    let hit = watch.map_or(t.awake_ticks % 30 == 0 && t.awake_ticks >= 90, |w| w == t.id);
                    if t.state == (State::Loose { awake: true }) && hit {
                        eprintln!("pull {id} tile {} awake {} at {:.0},{:.0},{:.0} vel {:?} spin {:?} quiet {} c1 {:?}", t.id, t.awake_ticks, mm(t.pose.t.x), mm(t.pose.t.y), mm(t.pose.t.z), t.vel, t.spin, t.quiet, t.pose.c1);
                    }
                }
            }
            for s in settled {
                if g.tiles.all[s as usize].awake_ticks >= ph.settle_ticks {
                    forced += 1;
                }
            }
            worst = worst.max(ms);
            sum += ms;
            n += 1;
            ticks += 1;
        }
        worst_ticks = worst_ticks.max(ticks);
        far += g
            .tiles
            .all
            .iter()
            .zip(&start)
            .filter(|(t, s)| t.is_loose() && mm(t.pose.t.sub(**s).len()) > 100.0)
            .count();
    }
    println!(
        "{pulls} grabs of a tile others lie on: worst tick {worst:.3} ms, mean {:.3} ms; {most_awake} awake at most; all asleep within {worst_ticks} ticks; {forced} put down at the time limit; {far} tiles moved over 100 mm in all",
        sum / n.max(1) as f64
    );

    // a whole game of four AI crabs: the game's own tick (crabs, nudges,
    // claims, tiles), the AI's thinking left out
    let mut g = fresh.clone();
    let mut brains: Vec<Brain> = (0..4)
        .map(|k| Brain::new(t.ai.clone(), tool_core::rng::mix(1 ^ k as u64)))
        .collect();
    let (mut worst, mut sum, mut n, mut most_awake, mut nudged) = (0f64, 0f64, 0, 0, 0);
    let start: Vec<_> = g.tiles.all.iter().map(|t| (t.pose, t.is_loose())).collect();
    while g.phase() != Phase::Ended {
        let inputs: Vec<_> = (0..4).map(|k| ai::think(&g, k, &mut brains[k])).collect();
        let asleep: Vec<bool> = g
            .tiles
            .all
            .iter()
            .map(|t| t.state == State::Loose { awake: false })
            .collect();
        // the least of two runs of the same tick: the machine's noise out
        let mut c = g.clone();
        let t0 = Instant::now();
        c.step(&inputs);
        let first = t0.elapsed().as_secs_f64() * 1000.0;
        let t0 = Instant::now();
        g.step(&inputs);
        let ms = first.min(t0.elapsed().as_secs_f64() * 1000.0);
        let _ = g
            .take_events()
            .iter()
            .filter(|e| matches!(e, Event::Grabbed { .. }))
            .count();
        let awake = g
            .tiles
            .all
            .iter()
            .filter(|t| t.state == State::Loose { awake: true })
            .count();
        nudged += g
            .tiles
            .all
            .iter()
            .zip(&asleep)
            .filter(|(t, a)| **a && t.state == State::Loose { awake: true } && t.rests_on.is_empty())
            .count();
        most_awake = most_awake.max(awake);
        worst = worst.max(ms);
        sum += ms;
        n += 1;
    }
    let moved = g
        .tiles
        .all
        .iter()
        .zip(&start)
        .filter(|(t, (p, l))| *l && t.is_loose() && t.pose != *p)
        .count();
    println!(
        "a 4:00 game of 4 AI crabs: game tick worst {worst:.3} ms, mean {:.4} ms over {n} ticks; {most_awake} tiles awake at most; {nudged} wakings of tiles lying on nothing but sand or rock (nudges, knocks, drops); {moved} loose tiles not where they started",
        sum / n.max(1) as f64
    );
}
