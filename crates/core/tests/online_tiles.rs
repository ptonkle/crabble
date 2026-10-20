//! An online game on the shipped map between two apps over a stand-in link
//! with a delay, as `tests/browser/online_game.py` plays it (SECOND-ORDER
//! R17.*): an AI crab in the game, each crab started by a tile
//! (`?spot=tile`), then A grabs, B grabs, A drops, B drops. After each,
//! both apps show the same crab holding the same tile, the AI's grabs
//! aside; at the end every tile is in the same state and at the same pose
//! on both.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

use tool_core::app::App;
use tool_core::sim::{Match, State};

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

/// A page as the online check opens it: the shipped map, no 3-2-1, the
/// crab started by a tile.
fn page(client: &str, seed: &str) -> App {
    let mut a = App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    a.act("data.tuning", &data("tuning.json")).unwrap();
    a.act("data.models", &bundle("models")).unwrap();
    a.act("data.map", &data("scenes/seafloor.json")).unwrap();
    a.act("flag.client", client).unwrap();
    a.act("flag.seed", seed).unwrap();
    a.act("flag.fast", "on").unwrap();
    a.act("flag.spot", "tile").unwrap();
    a
}

/// One way's delay on the link, ms.
const DELAY_MS: i64 = 40;

/// Two apps and the link between them: each message arrives `DELAY_MS`
/// after it was sent, in order.
struct Link {
    a: App,
    b: App,
    now: i64,
    to_a: VecDeque<(i64, String)>,
    to_b: VecDeque<(i64, String)>,
}

impl Link {
    fn frame(&mut self) {
        self.now += 16;
        let now = self.now;
        for m in self.a.net_pump(now * 1000).out {
            if m.to.as_deref().is_none_or(|t| t == "pb") {
                self.to_b.push_back((now + DELAY_MS, m.text));
            }
        }
        for m in self.b.net_pump(now * 1000).out {
            if m.to.as_deref().is_none_or(|t| t == "pa") {
                self.to_a.push_back((now + DELAY_MS, m.text));
            }
        }
        while self.to_b.front().is_some_and(|m| m.0 <= now) {
            let (_, text) = self.to_b.pop_front().unwrap();
            self.b.net_event("message", "pa", &text, true);
        }
        while self.to_a.front().is_some_and(|m| m.0 <= now) {
            let (_, text) = self.to_a.pop_front().unwrap();
            self.a.net_event("message", "pb", &text, false);
        }
    }

    fn run(&mut self, ms: i64) {
        let end = self.now + ms;
        while self.now < end {
            self.frame();
        }
    }

    /// Runs until `done` holds of both games, or `ms` pass; whether it did.
    fn until(&mut self, ms: i64, done: impl Fn(&Match, &Match) -> bool) -> bool {
        let end = self.now + ms;
        while self.now < end {
            self.frame();
            if done(game(&self.a), game(&self.b)) {
                return true;
            }
        }
        false
    }
}

fn game(x: &App) -> &Match {
    &x.play.as_ref().expect("in a game").game
}

/// What the two players' crabs (seats 0 and 1) hold.
fn players(g: &Match) -> Vec<[Option<u16>; 2]> {
    g.crabs[..2].iter().map(|c| c.claws).collect()
}

#[test]
fn on_the_shipped_map_with_an_ai_crab_each_players_grab_and_drop_shows_the_same_on_both() {
    let mut l = Link {
        a: page("ca", "11"),
        b: page("cb", "12"),
        now: 0,
        to_a: VecDeque::new(),
        to_b: VecDeque::new(),
    };
    l.a.act("online.host", "").unwrap();
    let code = l.a.online.room.clone();
    l.b.act("online.code", &code).unwrap();
    l.b.act("online.join", "").unwrap();
    l.b.net_event("connected", "pa", "", true);
    l.a.net_event("connected", "pb", "", false);
    l.run(1200);
    l.a.act("online.ai", "1").unwrap();
    l.run(300);
    l.b.act("online.ready", "").unwrap();
    l.run(300);
    l.a.act("online.start", "").unwrap();
    l.run(300);
    while l.a.ui().phase != "running" || l.b.ui().phase != "running" {
        l.frame();
    }
    assert_eq!(game(&l.a).crabs.len(), 3, "two players and the AI crab");
    // the online check's clicks, each on the left claw; A is seat 0, B 1
    let mut held: Vec<[Option<u16>; 2]> = vec![[None, None]; 2];
    let mut dropped = Vec::new();
    for (seat, what) in [(0, "A's grab"), (1, "B's grab"), (0, "A's drop"), (1, "B's drop")] {
        let me = if seat == 0 { &mut l.a } else { &mut l.b };
        let before = game(me).crabs[seat].claws[0];
        me.act("mouse.down", "0").unwrap();
        let ok = l.until(1500, |ga, gb| {
            let now = ga.crabs[seat].claws[0];
            now != before && players(ga) == players(gb)
        });
        let ga = game(&l.a);
        assert!(
            ok,
            "{what}: A shows {:?}, B {:?}",
            players(ga),
            players(game(&l.b))
        );
        match before {
            None => held[seat][0] = ga.crabs[seat].claws[0],
            Some(t) => {
                held[seat][0] = None;
                dropped.push(t);
            }
        }
        assert_eq!(players(ga), held, "{what}: only that claw changed");
    }
    // the dropped tiles land loose (a drop is never a placement), and once
    // the host's last changes are in, every tile is the same on both, a
    // tile at rest where it lies too, and every claw (the AI's as well)
    l.run(2000);
    let same = |ga: &Match, gb: &Match| {
        ga.placements == gb.placements
            && ga.crabs.iter().zip(&gb.crabs).all(|(x, y)| x.claws == y.claws)
            && ga.tiles.all.iter().zip(&gb.tiles.all).all(|(x, y)| {
                x.state == y.state && (x.state != (State::Loose { awake: false }) || x.pose == y.pose)
            })
    };
    let ok = l.until(2000, same);
    let (ga, gb) = (game(&l.a), game(&l.b));
    let differ: Vec<_> = (ga.tiles.all.iter().zip(&gb.tiles.all))
        .filter(|(x, y)| x.state != y.state || x.pose != y.pose)
        .map(|(x, y)| (x.id, x.state, y.state))
        .collect();
    assert!(ok, "the tiles differ: {differ:?}");
    for t in dropped {
        assert!(
            matches!(
                ga.tiles.all[t as usize].state,
                State::Loose { .. } | State::Held { seat: 2, .. }
            ),
            "tile {t} lies loose (or the AI took it): {:?}",
            ga.tiles.all[t as usize].state
        );
    }
}
