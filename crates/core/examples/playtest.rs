//! The headless playtest (Ape Race's reports, for Crabble): four AI crabs
//! play whole games on the shipped map, and the balance numbers come out:
//! tiles placed per crab, words made, fake words, idle penalty paid,
//! steals, final scores, and the tiles still loose at the end.
//!
//! `cargo run --release -p tool_core --example playtest [-- GAMES [SEED]]`
//! (default 8 games, run in parallel). Each game is its own seed.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::ai::{self, plan, Brain, Thought};
use tool_core::app::App;
use tool_core::score;
use tool_core::sim::{Driver, Event, Match, Phase, Setup};

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
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|p| {
            let id = p.file_stem().unwrap().to_str().unwrap();
            format!("\"{id}\":{}", fs::read_to_string(p).unwrap())
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

#[derive(Default, Clone)]
struct Crab {
    grabs: u32,
    placed: u32,
    /// placements whose every word was real, and those making a fake one
    real_placements: u32,
    fake_placements: u32,
    steals: u32,
    drops: u32,
    give_ups: u32,
    /// longest stretch without placing, ms
    longest_idle: i64,
    idle_paid: i32,
    real_words: u32,
    fake_words: u32,
    total: i32,
    won: bool,
}

struct Game {
    crabs: Vec<Crab>,
    loose: usize,
    held: usize,
    on_board: usize,
    words_on_board: usize,
    longest_word: String,
}

const SEATS: usize = 4;

fn play(app: &App, seed: u64) -> Game {
    let t = app.tuning.as_ref().unwrap();
    let map = app.view.map.as_ref().unwrap();
    let mut g = Match::new(
        t,
        map,
        Setup {
            drivers: vec![Driver::Ai; SEATS],
            seed,
            fast: true,
            length_ms: None,
        },
    );
    let mut brains: Vec<Brain> = (0..SEATS)
        .map(|k| Brain::new(t.ai.clone(), tool_core::rng::mix(seed ^ k as u64)))
        .collect();
    let mut crabs = vec![Crab::default(); SEATS];
    let mut last_place = [0i64; SEATS];
    // PLAYTEST_TRACE=<seat>: that crab's steps as they change
    let trace: Option<usize> = std::env::var("PLAYTEST_TRACE").ok().and_then(|v| v.parse().ok());
    let mut last_doing = String::new();
    while g.phase() != Phase::Ended {
        let before = plan::Board::from_tiles(&g.tiles);
        let inputs: Vec<_> = (0..SEATS).map(|k| ai::think(&g, k, &mut brains[k])).collect();
        if let Some(k) = trace {
            let d = format!("{:?}", brains[k].doing);
            let goal = match brains[k].doing {
                ai::Doing::Fetch { tile, .. } => Some(g.tiles.all[tile as usize].pose.t),
                ai::Doing::Place { square, .. } | ai::Doing::ToBoard { square } => {
                    Some(ai::square_point(&g, square))
                }
                _ => None,
            };
            if let Some(q) = goal {
                if g.play_ms() % 500 == 0 {
                    let a = g.aim(k);
                    let mm = |q: i32| q as i64 * 1000 / 65536;
                    let p = g.crabs[k].pos;
                    println!(
                        "    aim {:?} reach {} | crab ({}, {}, {}) goal ({}, {}, {}) yaw {} pitch {}",
                        a.target,
                        a.in_reach,
                        mm(p.x),
                        mm(p.y),
                        mm(p.z),
                        mm(q.x),
                        mm(q.y),
                        mm(q.z),
                        g.crabs[k].yaw,
                        g.crabs[k].pitch
                    );
                }
            }
            if d != last_doing {
                let p = g.crabs[k].pos;
                let mm = |q: i32| q as i64 * 1000 / 65536;
                println!(
                    "{:>6} ms  at ({}, {}, {})  {d}",
                    g.play_ms(),
                    mm(p.x),
                    mm(p.y),
                    mm(p.z)
                );
                last_doing = d;
            }
        }
        g.step(&inputs);
        for e in g.take_events() {
            match e {
                Event::Grabbed { seat, .. } => crabs[seat].grabs += 1,
                Event::Dropped { seat, .. } => crabs[seat].drops += 1,
                Event::Placed {
                    seat, tile, square, ..
                } => {
                    let c = &mut crabs[seat];
                    c.placed += 1;
                    let letter = g.tiles.all[tile as usize].letter;
                    let p = plan::rate(&before, square, letter, seat, 0);
                    if p.real {
                        c.real_placements += 1;
                    } else {
                        c.fake_placements += 1;
                    }
                    c.steals += p.steal as u32;
                    let now = g.play_ms();
                    c.longest_idle = c.longest_idle.max(now - last_place[seat]);
                    last_place[seat] = now;
                }
                _ => {}
            }
        }
    }
    if std::env::var("PLAYTEST_GIVEUPS").is_ok() {
        for (k, b) in brains.iter().enumerate() {
            for t in &b.thoughts {
                if let Thought::GaveUp { ms, what } = t {
                    println!("seed {seed} crab {k} gave up {what} at {ms}");
                }
            }
        }
    }
    let end = g.play_ms();
    let board = score::Board::from_tiles(&g.tiles);
    let idle: Vec<i32> = g.crabs.iter().map(|c| -c.penalty).collect();
    let tally = score::tally(&board, SEATS, &idle);
    let words = score::words(&board);
    for (k, c) in crabs.iter_mut().enumerate() {
        c.longest_idle = c.longest_idle.max(end - last_place[k]);
        c.give_ups = brains[k]
            .thoughts
            .iter()
            .filter(|t| matches!(t, Thought::GaveUp { .. }))
            .count() as u32;
        c.idle_paid = idle[k];
        let line = &tally.lines[k];
        c.real_words = line.words.iter().filter(|w| w.word.real).count() as u32;
        c.fake_words = line.words.iter().filter(|w| !w.word.real).count() as u32;
        c.total = line.total;
        c.won = tally.winners.contains(&k);
    }
    let count = |f: &dyn Fn(&tool_core::sim::Tile) -> bool| g.tiles.all.iter().filter(|t| f(t)).count();
    Game {
        crabs,
        loose: count(&|t| t.is_loose()),
        held: count(&|t| t.is_held()),
        on_board: count(&|t| matches!(t.state, tool_core::sim::State::Placed { .. })),
        words_on_board: words.len(),
        longest_word: words
            .iter()
            .filter(|w| w.word.real)
            .map(|w| w.letters.clone())
            .max_by_key(|w| w.len())
            .unwrap_or_default(),
    }
}

fn mean(xs: impl Iterator<Item = f64>) -> f64 {
    let v: Vec<f64> = xs.collect();
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(8);
    let base: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1);
    let mut app = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.glyphs", &data("glyphs.json")).unwrap();
    app.act("data.look", &data("look.json")).unwrap();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    let app = &app;
    let started = std::time::Instant::now();
    let results: Vec<(u64, Game)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..games)
            .map(|k| {
                let seed = base + k;
                s.spawn(move || (seed, play(app, seed)))
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    println!(
        "{games} games of 4 AI crabs, 4:00 each, on the shipped map ({:.1} s)\n",
        started.elapsed().as_secs_f64()
    );
    println!("seed | crab | grabs | placed | real/fake placements | steals | drops | gave up | longest idle s | idle paid | real/fake words | total");
    println!("---|---|---|---|---|---|---|---|---|---|---|---");
    for (seed, g) in &results {
        for (k, c) in g.crabs.iter().enumerate() {
            println!(
                "{seed} | {k}{} | {} | {} | {}/{} | {} | {} | {} | {:.0} | {} | {}/{} | {}",
                if c.won { " (won)" } else { "" },
                c.grabs,
                c.placed,
                c.real_placements,
                c.fake_placements,
                c.steals,
                c.drops,
                c.give_ups,
                c.longest_idle as f64 / 1000.0,
                c.idle_paid,
                c.real_words,
                c.fake_words,
                c.total
            );
        }
    }
    println!("\nseed | on board | loose at end | held at end | words on board | longest real word");
    println!("---|---|---|---|---|---");
    for (seed, g) in &results {
        println!(
            "{seed} | {} | {} | {} | {} | {}",
            g.on_board, g.loose, g.held, g.words_on_board, g.longest_word
        );
    }
    let all: Vec<&Crab> = results.iter().flat_map(|(_, g)| g.crabs.iter()).collect();
    let m = |f: &dyn Fn(&Crab) -> f64| mean(all.iter().map(|c| f(c)));
    let mg = |f: &dyn Fn(&Game) -> f64| mean(results.iter().map(|(_, g)| f(g)));
    println!("\nMeans per crab per game:");
    println!("- grabs {:.1}, tiles placed {:.1} (real {:.1}, fake {:.1}), steals {:.1}, drops {:.1}, gave up {:.1}",
        m(&|c| c.grabs as f64), m(&|c| c.placed as f64), m(&|c| c.real_placements as f64),
        m(&|c| c.fake_placements as f64), m(&|c| c.steals as f64), m(&|c| c.drops as f64), m(&|c| c.give_ups as f64));
    println!("- words owned at the end: real {:.1}, fake {:.1}; idle penalty paid {:.1} (crabs that paid any: {:.0}%)",
        m(&|c| c.real_words as f64), m(&|c| c.fake_words as f64), m(&|c| c.idle_paid as f64),
        100.0 * m(&|c| (c.idle_paid > 0) as u32 as f64));
    println!(
        "- final score {:.1} (lowest {}, highest {}); longest idle {:.1} s",
        m(&|c| c.total as f64),
        all.iter().map(|c| c.total).min().unwrap_or(0),
        all.iter().map(|c| c.total).max().unwrap_or(0),
        m(&|c| c.longest_idle as f64 / 1000.0)
    );
    println!("Means per game: {:.1} tiles on the board (the centre one counted), {:.1} still loose, {:.1} held, {:.1} words",
        mg(&|g| g.on_board as f64), mg(&|g| g.loose as f64), mg(&|g| g.held as f64), mg(&|g| g.words_on_board as f64));
}
