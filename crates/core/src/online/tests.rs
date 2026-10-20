//! The protocol between sessions joined by a loopback "transport": every
//! message is delivered in order after a fixed latency, and a link can be
//! cut. Expected values are worked out from the brief and Ape Race's lobby.

use std::collections::VecDeque;

use super::session::{sample_at, Sample};
use super::*;

const LATENCY_MS: i64 = 20;
const STEP_MS: i64 = 10;

struct Node {
    id: String,
    s: Session,
}

struct Loop {
    nodes: Vec<Node>,
    /// (deliver at, from, to, text)
    wire: Vec<(i64, usize, usize, String)>,
    cut: Vec<(usize, usize)>,
    now: i64,
    /// a message lost one time in `loss` hundred, and each delayed up to
    /// `jitter` ms more (so later ones may overtake it)
    loss: u32,
    jitter: u32,
    dice: crate::rng::Dice,
}

impl Loop {
    /// A host and `joiners` joiners, every pair connected.
    fn new(joiners: usize) -> Loop {
        let mut nodes = vec![Node {
            id: "p0".into(),
            s: Session::host("c0", "Host", 0),
        }];
        for k in 1..=joiners {
            nodes.push(Node {
                id: format!("p{k}"),
                s: Session::join(&format!("c{k}"), &format!("Joiner {k}"), 0),
            });
        }
        let mut l = Loop {
            nodes,
            wire: Vec::new(),
            cut: Vec::new(),
            now: 0,
            loss: 0,
            jitter: 0,
            dice: crate::rng::Dice(7),
        };
        l.connect_all();
        l
    }

    fn connect_all(&mut self) {
        let n = self.nodes.len();
        for a in 0..n {
            for b in 0..n {
                if a != b {
                    let (id, host) = (self.nodes[b].id.clone(), b == 0);
                    let now = self.now;
                    self.nodes[a].s.peer_connected(&id, host, now);
                }
            }
        }
    }

    fn cut(&self, a: usize, b: usize) -> bool {
        self.cut.contains(&(a, b)) || self.cut.contains(&(b, a))
    }

    fn send_all(&mut self) {
        for from in 0..self.nodes.len() {
            for out in self.nodes[from].s.take_outbox() {
                let targets: Vec<usize> = match &out.to {
                    None => (0..self.nodes.len()).filter(|k| *k != from).collect(),
                    Some(peer) => self
                        .nodes
                        .iter()
                        .position(|n| n.id == *peer)
                        .into_iter()
                        .collect(),
                };
                for to in targets {
                    if self.cut(from, to) || self.dice.below(100) < self.loss {
                        continue;
                    }
                    let late = self.dice.below(self.jitter + 1) as i64;
                    self.wire
                        .push((self.now + LATENCY_MS + late, from, to, out.text.clone()));
                }
            }
        }
    }

    fn run(&mut self, ms: i64) {
        let end = self.now + ms;
        while self.now < end {
            self.now += STEP_MS;
            let now = self.now;
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.wire)
                .into_iter()
                .partition(|w| w.0 <= now);
            self.wire = later;
            for (_, from, to, text) in due {
                let id = self.nodes[from].id.clone();
                self.nodes[to].s.receive(&id, &text, now);
            }
            for n in &mut self.nodes {
                n.s.update(now);
            }
            self.send_all();
        }
    }
}

#[test]
fn codes_are_six_clear_letters_from_the_seed_and_typing_is_forgiven() {
    let a = room_code(1);
    assert_eq!(a.len(), 6);
    assert!(a.bytes().all(|b| CODE_ALPHABET.contains(&b)));
    for bad in *b"0O1I" {
        assert!(!CODE_ALPHABET.contains(&bad));
    }
    assert_eq!(room_code(1), a);
    assert_ne!(room_code(2), a);
    assert_eq!(clean_code(&format!(" {} ", a.to_lowercase())), Some(a.clone()));
    assert_eq!(clean_code("ABC-DEF"), Some("ABCDEF".to_string()));
    assert_eq!(clean_code("ABCDE0"), None);
    assert_eq!(clean_code("ABCDE"), None);
    assert_eq!(room_name("ABCDEF"), "crabble-p2-ABCDEF");
    assert_eq!(clean_name("  A very long crab name indeed  "), "A very long crab");
}

#[test]
fn joiners_are_seated_in_order_and_see_every_seat() {
    let mut l = Loop::new(2);
    l.run(600);
    for n in &l.nodes {
        assert_eq!(*n.s.status(), Status::Lobby, "{}", n.id);
        let names: Vec<String> = n.s.lobby().seats.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "Host");
    }
    let v = l.nodes[1].s.lobby();
    assert!(v.seats[0].host && !v.is_host);
    assert_eq!(v.seats.iter().filter(|s| s.me).count(), 1);
}

#[test]
fn every_seat_but_ones_own_shows_its_round_trip() {
    let mut l = Loop::new(1);
    l.run(2000);
    let v = l.nodes[0].s.lobby();
    assert_eq!(v.seats[0].ping_ms, None, "the host's own seat");
    // 20 ms each way
    assert_eq!(v.seats[1].ping_ms, Some(2 * LATENCY_MS));
}

#[test]
fn start_waits_for_every_ready_mark_and_two_crabs() {
    let mut l = Loop::new(1);
    l.run(600);
    assert!(!l.nodes[0].s.can_start(), "the joiner is not ready");
    assert!(!l.nodes[0].s.start(5, l.now));
    l.nodes[1].s.set_ready(true, l.now);
    l.run(200);
    assert!(l.nodes[1].s.lobby().ready);
    assert!(l.nodes[0].s.can_start());
    // alone, the host needs an AI crab
    let mut alone = Session::host("c0", "Host", 0);
    assert!(!alone.can_start());
    alone.set_ai_count(1, 0);
    assert!(alone.can_start());
}

#[test]
fn never_more_than_four_crabs_and_a_fifth_person_is_refused() {
    let mut two = Loop::new(1);
    two.run(600);
    let now = two.now;
    two.nodes[0].s.set_ai_count(3, now);
    assert_eq!(
        two.nodes[0].s.lobby().ai_count,
        2,
        "two people leave room for two AI crabs"
    );
    assert_eq!(two.nodes[0].s.lobby().max_ai, 2);
    let mut l = Loop::new(3);
    l.run(600);
    let now = l.now;
    l.nodes[0].s.set_ai_count(3, now);
    assert_eq!(
        l.nodes[0].s.lobby().ai_count,
        0,
        "four people leave no room for an AI crab"
    );
    let mut fifth = Session::join("c9", "Late", 0);
    fifth.peer_connected("p0", true, now);
    let hello = fifth.take_outbox();
    for m in hello {
        l.nodes[0].s.receive("p9", &m.text, now);
    }
    let answer = l.nodes[0]
        .s
        .take_outbox()
        .into_iter()
        .filter(|m| m.to.as_deref() == Some("p9"));
    for m in answer {
        fifth.receive("p0", &m.text, now);
    }
    assert_eq!(
        *fifth.status(),
        Status::Refused {
            reason: Refusal::Full
        }
    );
}

#[test]
fn another_protocol_is_refused_with_both_versions() {
    let mut host = Session::host("c0", "Host", 0);
    let mut old = Session::join("c1", "Old", 0).with_protocol(PROTOCOL + 1);
    host.peer_connected("p1", false, 0);
    old.peer_connected("p0", true, 0);
    for m in old.take_outbox() {
        host.receive("p1", &m.text, 0);
    }
    for m in host.take_outbox() {
        old.receive("p0", &m.text, 0);
    }
    assert_eq!(
        *old.status(),
        Status::Refused {
            reason: Refusal::WrongVersion {
                host: PROTOCOL,
                mine: PROTOCOL + 1
            }
        }
    );
}

#[test]
fn a_joiner_arriving_after_the_start_is_refused() {
    let mut l = Loop::new(0);
    let now = l.now;
    l.nodes[0].s.set_ai_count(1, now);
    assert!(l.nodes[0].s.start(1, now));
    let mut late = Session::join("c5", "Late", now);
    late.peer_connected("p0", true, now);
    for m in late.take_outbox() {
        l.nodes[0].s.receive("p5", &m.text, now);
    }
    for m in l.nodes[0].s.take_outbox() {
        if m.to.as_deref() == Some("p5") {
            late.receive("p0", &m.text, now);
        }
    }
    assert_eq!(
        *late.status(),
        Status::Refused {
            reason: Refusal::Started
        }
    );
}

#[test]
fn a_name_and_the_ai_count_reach_everyone() {
    let mut l = Loop::new(1);
    l.run(600);
    let now = l.now;
    l.nodes[1].s.set_name("Bea", now);
    l.nodes[0].s.set_ai_count(2, now);
    l.run(200);
    for n in &l.nodes {
        let v = n.s.lobby();
        assert_eq!(v.seats[1].name, "Bea");
        assert_eq!(v.ai_count, 2);
        assert!(v.seats[2].ai && v.seats[3].ai);
    }
}

#[test]
fn a_rename_in_the_lobby_reaches_every_seat_even_when_messages_are_lost() {
    // TRIAGE F5: the host passes a joiner's new name on to every seat
    let mut l = Loop::new(2);
    l.run(600);
    l.loss = 50;
    let now = l.now;
    l.nodes[2].s.set_name("  Sandy Claws  ", now);
    l.nodes[0].s.set_name("Big Red", now);
    l.run(4000);
    for (k, n) in l.nodes.iter().enumerate() {
        let names: Vec<String> = n.s.lobby().seats.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names[..3], ["Big Red", "Joiner 1", "Sandy Claws"], "node {k}");
    }
}

fn started(joiners: usize, ai: usize) -> Loop {
    let mut l = Loop::new(joiners);
    l.run(600);
    let now = l.now;
    for k in 1..=joiners {
        l.nodes[k].s.set_ready(true, now);
    }
    l.nodes[0].s.set_ai_count(ai, now);
    l.run(200);
    let now = l.now;
    assert!(l.nodes[0].s.start(42, now));
    l.run(300);
    l
}

#[test]
fn every_browser_starts_the_same_seats_with_its_own_crab() {
    let mut l = started(2, 1);
    for (k, n) in l.nodes.iter_mut().enumerate() {
        let st = n.s.take_start().expect("a start");
        assert_eq!(st.seed, 42);
        assert_eq!(st.local_index, k);
        let ai: Vec<bool> = st.seats.iter().map(|s| s.ai).collect();
        assert_eq!(ai, vec![false, false, false, true]);
        assert!(n.s.owns(k));
        // the host runs the AI crab
        assert_eq!(n.s.owns(3), k == 0);
        assert!(n.s.take_start().is_none(), "given once");
    }
}

#[test]
fn a_crabs_state_arrives_and_is_drawn_between_its_samples() {
    let mut l = started(1, 0);
    // the joiner walks east at 1 m/s, sending as it goes
    for step in 0..40 {
        let ms = step * 10;
        let s = CrabState {
            p: [ms as i32, 0],
            v: [1000, 0],
            y: 0,
            ..Default::default()
        };
        let now = l.now;
        l.nodes[1].s.send_state(now, ms, 1, s);
        l.run(10);
    }
    let seen = l.nodes[0].s.remote_states(l.now);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].index, 1);
    let x = seen[0].state.p[0];
    // drawn about 100 ms behind its newest state (390 mm): near 290 mm
    assert!((250..=330).contains(&x), "x = {x}");
    // the host sends no state for a crab it does not own
    l.nodes[0].s.send_state(
        l.now,
        0,
        1,
        CrabState {
            p: [0, 0],
            v: [0, 0],
            y: 0,
            ..Default::default()
        },
    );
    assert!(l.nodes[0].s.take_outbox().is_empty());
}

#[test]
fn interpolation_blends_turns_the_short_way_and_extrapolation_stops() {
    let sample = |sent: i64, x: i32, y: i32| Sample {
        sent,
        got: sent,
        s: CrabState {
            p: [x, 0],
            v: [10_000, 0],
            y,
            ..Default::default()
        },
    };
    let list: VecDeque<Sample> = [sample(0, 0, 65000), sample(100, 1000, 500)]
        .into_iter()
        .collect();
    assert_eq!(sample_at(&list, 50).p[0], 500);
    // from 65000 to 500 the short way (1036 units): halfway is 65518
    assert_eq!(sample_at(&list, 50).y, 65518);
    assert_eq!(sample_at(&list, -20).p[0], 0);
    // 100 ms past the newest at 10 m/s: one more metre
    assert_eq!(sample_at(&list, 200).p[0], 2000);
    // but never more than 200 ms ahead
    assert_eq!(sample_at(&list, 5000).p[0], 1000 + 2000);
}

#[test]
fn a_joiner_with_no_link_to_another_hears_it_through_the_host() {
    let mut l = Loop::new(2);
    l.cut.push((1, 2));
    l.run(600);
    let now = l.now;
    l.nodes[1].s.set_ready(true, now);
    l.nodes[2].s.set_ready(true, now);
    l.run(2000);
    let now = l.now;
    assert!(l.nodes[0].s.start(1, now));
    l.run(300);
    for step in 0..20 {
        let now = l.now;
        let s = CrabState {
            p: [step * 50, 0],
            v: [0, 0],
            y: 0,
            ..Default::default()
        };
        l.nodes[1].s.send_state(now, step as i64 * 50, 1, s);
        l.run(50);
    }
    let seen = l.nodes[2].s.remote_states(l.now);
    assert!(seen.iter().any(|r| r.index == 1), "joiner 2 sees joiner 1's crab");
}

#[test]
fn the_hosts_end_reaches_the_joiners_and_rematch_brings_everyone_back_ready() {
    let mut l = started(1, 0);
    assert!(!l.nodes[1].s.ended());
    let now = l.now;
    l.nodes[0].s.end_game(now, Vec::new());
    l.run(100);
    assert!(l.nodes[1].s.ended());
    // only the host may rematch
    assert!(!l.nodes[1].s.rematch(l.now));
    assert!(l.nodes[0].s.rematch(l.now));
    l.run(200);
    for n in &l.nodes {
        assert_eq!(*n.s.status(), Status::Lobby, "{}", n.id);
        assert_eq!(n.s.round(), 1);
    }
    assert!(
        l.nodes[0].s.lobby().seats[1].ready,
        "the joiner who stayed is ready"
    );
    assert!(l.nodes[0].s.can_start());
}

#[test]
fn the_host_leaving_ends_the_lobby_and_a_joiner_leaving_frees_its_seat() {
    let mut l = Loop::new(2);
    l.run(600);
    l.nodes[2].s.leave();
    l.run(100);
    assert_eq!(l.nodes[0].s.lobby().seats.len(), 2);
    l.nodes[0].s.leave();
    l.run(100);
    assert_eq!(*l.nodes[1].s.status(), Status::Dissolved);
}

#[test]
fn a_ready_mark_lost_on_the_way_is_sent_again() {
    let mut l = Loop::new(1);
    l.run(600);
    l.cut.push((0, 1));
    let now = l.now;
    l.nodes[1].s.set_ready(true, now);
    l.run(100);
    l.cut.clear();
    l.run(1000);
    assert!(l.nodes[0].s.lobby().seats[1].ready);
}

#[test]
fn a_crab_that_stops_sending_is_stalled_and_one_that_left_is_not() {
    let mut l = started(2, 0);
    let send = |l: &mut Loop, k: usize| {
        let now = l.now;
        l.nodes[k].s.send_state(now, now, k, CrabState::default());
    };
    for _ in 0..10 {
        send(&mut l, 1);
        send(&mut l, 2);
        l.run(100);
    }
    assert!(l.nodes[0].s.stalled(l.now).is_empty());
    // joiner 1 falls silent for 1.6 s (more than STALL_MS, 1.5 s)
    for _ in 0..16 {
        send(&mut l, 2);
        l.run(100);
    }
    assert_eq!(l.nodes[0].s.stalled(l.now), vec![1]);
    assert_eq!(
        l.nodes[2].s.stalled(l.now),
        vec![0, 1],
        "the host sent none either"
    );
    // a crab whose player left is gone, not stalled
    let now = l.now;
    l.nodes[0].s.peer_left("p1", now);
    assert!(l.nodes[0].s.stalled(l.now).is_empty());
    // and none is stalled once the game is over
    l.nodes[0].s.end_game(now, Vec::new());
    l.run(100);
    assert!(l.nodes[2].s.stalled(l.now).is_empty());
}

#[test]
fn height_is_blended_and_the_air_mark_and_gait_come_from_the_nearer_state() {
    let sample = |sent: i64, h: i32, a: bool, g: u8| Sample {
        sent,
        got: sent,
        s: CrabState {
            h,
            a,
            g,
            ..Default::default()
        },
    };
    let list: VecDeque<Sample> = [sample(0, 0, false, 3), sample(100, 60, true, 1)]
        .into_iter()
        .collect();
    let early = sample_at(&list, 40);
    assert_eq!((early.h, early.a, early.g), (24, false, 3));
    let late = sample_at(&list, 60);
    assert_eq!((late.h, late.a, late.g), (36, true, 1));
    // a state sent as JSON and read back keeps every field
    let s = CrabState {
        p: [1, -2],
        v: [3, 4],
        y: 5,
        h: 6,
        vh: -7,
        a: true,
        g: 4,
    };
    let text = msg::Msg::State { r: 0, i: 1, k: 2, s }.encode();
    assert_eq!(
        msg::Msg::decode(&text),
        Some(msg::Msg::State { r: 0, i: 1, k: 2, s })
    );
}

/// A stand-in referee on the host: a grab is granted when its tile is
/// still free, first come first served (the real one is `sim::Match`).
fn referee(taken: &mut Vec<u16>, c: tiles::WireClaim, at: i64) -> tiles::Entry {
    let ok = !taken.contains(&c.t);
    if ok {
        taken.push(c.t);
    }
    tiles::Entry::Verdict { ok, c, n: 0, at }
}

fn grab(seat: usize, tile: u16) -> tiles::WireClaim {
    tiles::WireClaim {
        k: tiles::Kind::Grab,
        s: seat,
        c: 0,
        t: tile,
        q: None,
        at: None,
    }
}

#[test]
fn claims_and_verdicts_survive_loss_and_reordering_and_every_browser_applies_the_same_log() {
    let mut l = started(2, 0);
    l.loss = 25;
    l.jitter = 80;
    let mut taken = Vec::new();
    let mut sent = [0u32, 1, 1];
    let mut applied: Vec<Vec<tiles::Entry>> = vec![Vec::new(); 3];
    // all three grab tile 5 at the same moment; the joiners then grab one
    // tile each every 100 ms, some of them the same
    let now = l.now;
    let mine = referee(&mut taken, grab(0, 5), now);
    assert!(l.nodes[1].s.claim(grab(1, 5), now));
    assert!(l.nodes[2].s.claim(grab(2, 5), now));
    assert!(!l.nodes[1].s.claim(grab(1, 6), now), "one claim a claw at a time");
    assert!(!l.nodes[1].s.claim(grab(2, 6), now), "only for its own crab");
    l.nodes[0].s.record(vec![mine], now);
    for step in 0..60 {
        let now = l.now;
        for k in [1, 2] {
            let claw = step % 2;
            let c = tiles::WireClaim {
                c: claw,
                ..grab(k, (10 + step / 3) as u16)
            };
            sent[k] += l.nodes[k].s.claim(c, now) as u32;
        }
        let claims = l.nodes[0].s.take_claims();
        let entries: Vec<_> = claims.into_iter().map(|c| referee(&mut taken, c, now)).collect();
        l.nodes[0].s.record(entries, now);
        for (k, n) in l.nodes.iter_mut().enumerate().skip(1) {
            applied[k].extend(n.s.take_entries());
        }
        l.run(50);
    }
    // the network heals; the host's game ends
    l.loss = 0;
    l.run(2000);
    let claims = l.nodes[0].s.take_claims();
    let entries: Vec<_> = claims.into_iter().map(|c| referee(&mut taken, c, 0)).collect();
    l.nodes[0].s.record(entries, l.now);
    let now = l.now;
    l.nodes[0].s.end_game(now, vec![0, -4, -2]);
    let mut waited = 0;
    while !(l.nodes[1].s.ended() && l.nodes[2].s.ended()) {
        for (k, n) in l.nodes.iter_mut().enumerate().skip(1) {
            applied[k].extend(n.s.take_entries());
        }
        l.run(50);
        waited += 50;
        assert!(waited < 5000, "the joiners' games end");
    }
    for (k, n) in l.nodes.iter_mut().enumerate().skip(1) {
        applied[k].extend(n.s.take_entries());
    }
    let log = l.nodes[0].s.tile_log();
    // every claim sent was judged once, the host's one too
    assert_eq!(log.len() as u32, 1 + sent[1] + sent[2]);
    assert!(sent[1] >= 5 && sent[2] >= 5, "{sent:?}");
    assert_eq!(applied[1], log, "joiner 1 applied the host's log in its order");
    assert_eq!(applied[2], log, "joiner 2 too");
    let granted_5: Vec<usize> = log
        .iter()
        .filter_map(|e| match e {
            tiles::Entry::Verdict { ok: true, c, .. } if c.t == 5 => Some(c.s),
            _ => None,
        })
        .collect();
    assert_eq!(
        granted_5,
        vec![0],
        "one winner for tile 5: the host, first to its referee"
    );
    // each joiner's claims were taken in its own order, once each
    for k in [1usize, 2] {
        let ns: Vec<u32> = log
            .iter()
            .filter_map(|e| match e {
                tiles::Entry::Verdict { c, n, .. } if c.s == k => Some(n.to_owned()),
                _ => None,
            })
            .collect();
        assert_eq!(ns, (1..=sent[k]).collect::<Vec<u32>>());
        assert!(!l.nodes[k].s.in_flight(0) && !l.nodes[k].s.in_flight(1));
        assert_eq!(l.nodes[k].s.final_penalties(), Some(vec![0, -4, -2]));
    }
}

#[test]
fn a_joiners_game_waits_at_the_end_for_the_last_tile_changes() {
    let mut l = started(1, 0);
    let now = l.now;
    // the host's End (after one entry) reaches the joiner before the entry
    let end = msg::Msg::End {
        r: 0,
        n: 1,
        pen: vec![0, -2],
    };
    l.nodes[1].s.receive("p0", &end.encode(), now);
    assert!(!l.nodes[1].s.ended(), "the End alone is not the end");
    let entry = referee(&mut Vec::new(), grab(0, 3), now);
    let log = msg::Msg::Log {
        r: 0,
        from: 0,
        e: vec![entry],
    };
    l.nodes[1].s.receive("p0", &log.encode(), now);
    assert!(l.nodes[1].s.ended());
    assert_eq!(l.nodes[1].s.take_entries().len(), 1);
    assert_eq!(l.nodes[1].s.final_penalties(), Some(vec![0, -2]));
}
