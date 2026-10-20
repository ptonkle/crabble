//! The Online game screen and the Lobby through the app, with two apps
//! joined by a stand-in transport (each pump's text delivered at once).

use crate::app::tests::app;
use crate::app::{App, Screen};
use crate::view::Item;

/// Pumps both apps at `now` ms and delivers what each sends to the other.
fn pump(a: &mut App, b: &mut App, now: i64) {
    let pa = a.net_pump(now * 1000);
    let pb = b.net_pump(now * 1000);
    for m in pa.out {
        if m.to.as_deref().is_none_or(|t| t == "pb") {
            b.net_event("message", "pa", &m.text, true);
        }
    }
    for m in pb.out {
        if m.to.as_deref().is_none_or(|t| t == "pa") {
            a.net_event("message", "pb", &m.text, false);
        }
    }
}

fn run(a: &mut App, b: &mut App, from: i64, ms: i64) -> i64 {
    let mut now = from;
    while now < from + ms {
        now += 16;
        pump(a, b, now);
    }
    now
}

/// A hosts by code, B joins by it, and the transport links them.
fn joined() -> (App, App, i64) {
    let mut a = app();
    let mut b = app();
    a.act("flag.client", "ca").unwrap();
    b.act("flag.client", "cb").unwrap();
    a.act("flag.seed", "11").unwrap();
    a.act("online.host", "").unwrap();
    let code = a.online.room.clone();
    b.act("online.code", &code).unwrap();
    b.act("online.join", "").unwrap();
    b.net_event("connected", "pa", "", true);
    a.net_event("connected", "pb", "", false);
    let now = run(&mut a, &mut b, 0, 1200);
    (a, b, now)
}

fn button_ids(a: &App) -> Vec<String> {
    a.ui()
        .panels
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|i| match i {
            Item::Button { id, .. } | Item::Copy { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn host_asks_the_transport_for_its_room_and_shows_the_code_and_link() {
    let mut a = app();
    a.act("flag.page_url", "https://x.example/crabble/").unwrap();
    a.act("online.host", "").unwrap();
    assert_eq!(a.screen, Screen::Lobby);
    let code = a.online.room.clone();
    assert_eq!(code.len(), 6);
    let p = a.net_pump(1000);
    assert_eq!(p.requests[0]["op"], "host_room");
    assert_eq!(p.requests[0]["room"], format!("crabble-p2-{code}"));
    assert_eq!(p.hash, "", "the host's address keeps no code");
    let items = &a.ui().panels[0].items;
    assert!(
        matches!(&items[0], Item::Copy { id, value, copy, .. } if id == "online.copy" && *value == code && *copy == code)
    );
    let link = format!("https://x.example/crabble/#room={code}");
    assert!(matches!(&items[1], Item::Copy { id, copy, .. } if id == "online.copy_link" && *copy == link));
    a.act("online.copy", "").unwrap();
    assert!(matches!(&a.ui().panels[0].items[0], Item::Copy { text, .. } if text == "ONLINE.COPIED"));
}

#[test]
fn an_invite_link_goes_straight_to_the_lobby_and_keeps_the_code_in_the_address() {
    let mut a = app();
    a.act("flag.room_link", "abc-def").unwrap();
    assert_eq!(a.screen, Screen::Lobby);
    let p = a.net_pump(1000);
    assert_eq!(p.requests[0]["op"], "join_room");
    assert_eq!(p.hash, "#room=ABCDEF");
    // a link with no proper code changes nothing
    let mut b = app();
    b.act("flag.room_link", "xyz").unwrap();
    assert_eq!(b.screen, Screen::Title);
}

#[test]
fn a_short_code_is_refused_with_its_length() {
    let mut a = app();
    a.act("menu.online", "").unwrap();
    a.act("online.code", "AB").unwrap();
    a.act("online.join", "").unwrap();
    assert_eq!(a.screen, Screen::Online);
    // the strings file here holds each key in capitals; {n} is filled
    assert_eq!(a.ui().panels[0].sub, "ONLINE.CODE_SHAPE");
    assert_eq!(a.online.problem.as_ref().unwrap().1[0].1, "6");
}

#[test]
fn the_pasted_code_path_shows_only_with_its_flag() {
    let mut a = app();
    a.act("menu.online", "").unwrap();
    assert!(!button_ids(&a).contains(&"online.host_code".to_string()));
    a.act("flag.codes", "on").unwrap();
    assert!(button_ids(&a).contains(&"online.host_code".to_string()));
    a.act("online.host_code", "").unwrap();
    assert_eq!(a.net_pump(1000).requests[0]["op"], "host_code");
    a.net_code("invite", "Oabc");
    assert!(a.ui().panels[0]
        .items
        .iter()
        .any(|i| matches!(i, Item::Text { id, value, .. } if id == "online.invite" && value == "Oabc")));
}

#[test]
fn the_lobby_seats_both_with_colours_and_start_waits_for_ready() {
    let (mut a, mut b, now) = joined();
    assert_eq!(b.screen, Screen::Lobby);
    let rows = a
        .ui()
        .panels
        .iter()
        .flat_map(|p| p.items.clone())
        .find_map(|i| match i {
            Item::Table { rows, .. } => Some(rows),
            _ => None,
        })
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].tone, "red");
    assert_eq!(rows[1].tone, "yellow");
    assert_eq!(rows[0].cells[0], "COLOUR.RED");
    assert!(rows[0].marked, "the host's own seat on its own screen");
    assert!(
        !button_ids(&b).contains(&"online.start".to_string()),
        "a joiner has no Start"
    );
    a.act("online.start", "").unwrap();
    assert_eq!(a.screen, Screen::Lobby, "Start waits for B's Ready");
    b.act("online.ready", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    a.act("online.start", "").unwrap();
    run(&mut a, &mut b, now, 500);
    assert_eq!(a.screen, Screen::Play);
    assert_eq!(b.screen, Screen::Play);
}

fn has_name_field(a: &App) -> bool {
    a.ui()
        .panels
        .iter()
        .flat_map(|p| p.items.iter())
        .any(|i| matches!(i, Item::Text { id, .. } if id == "online.name"))
}

fn lobby_names(a: &App) -> Vec<String> {
    let seats = a.online.session.as_ref().unwrap().lobby().seats;
    seats.into_iter().map(|s| s.name).collect()
}

#[test]
fn a_name_changed_in_the_lobby_reaches_the_other_app_and_is_kept() {
    // TRIAGE F5
    let (mut a, mut b, now) = joined();
    assert!(has_name_field(&a) && has_name_field(&b));
    b.act("online.name", "Bea Moss").unwrap();
    a.act("online.name", "Hana").unwrap();
    let now = run(&mut a, &mut b, now, 500);
    assert_eq!(lobby_names(&a), ["Hana", "Bea Moss"]);
    assert_eq!(lobby_names(&b), ["Hana", "Bea Moss"]);
    assert!(
        b.stored_settings().contains("\"Bea Moss\""),
        "kept for the next visit"
    );
    // a cleared field gives the default name
    b.act("online.name", "").unwrap();
    run(&mut a, &mut b, now, 500);
    assert_eq!(lobby_names(&a)[1], "NAME.DEFAULT");
}

#[test]
fn a_joiner_waiting_for_its_seat_can_already_change_its_name() {
    let mut b = app();
    b.act("flag.room_link", "abcdef").unwrap();
    assert_eq!(b.screen, Screen::Lobby);
    assert!(has_name_field(&b));
}

#[test]
fn an_online_game_moves_the_other_crab_ends_with_the_host_and_rematches() {
    let (mut a, mut b, now) = joined();
    a.act("flag.fast", "on").unwrap();
    b.act("flag.fast", "on").unwrap();
    a.act("flag.short", "2").unwrap();
    b.act("flag.short", "2").unwrap();
    a.act("online.ai", "1").unwrap();
    b.act("online.ready", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    a.act("online.start", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    assert_eq!(b.screen, Screen::Play);
    assert_eq!(b.play.as_ref().unwrap().game.crabs.len(), 3);
    b.act("key", "KeyW").unwrap();
    let now = run(&mut a, &mut b, now, 1200);
    b.act("key.up", "KeyW").unwrap();
    assert_eq!(
        a.net_pump(now * 1000).remote_moved,
        Some(true),
        "A sees B's crab move"
    );
    let now = run(&mut a, &mut b, now, 2500);
    assert_eq!(a.ui().phase, "ended");
    assert_eq!(b.ui().phase, "ended", "the joiner's game ends with the host's");
    assert!(button_ids(&a).contains(&"end.rematch".to_string()));
    assert!(!button_ids(&b).contains(&"end.rematch".to_string()));
    a.act("end.rematch", "").unwrap();
    run(&mut a, &mut b, now, 300);
    assert_eq!(a.screen, Screen::Lobby);
    assert_eq!(b.screen, Screen::Lobby);
}

#[test]
fn leaving_sends_a_parting_word_and_the_host_leaving_sends_the_joiner_back() {
    let (mut a, mut b, now) = joined();
    a.act("online.leave", "").unwrap();
    assert_eq!(a.screen, Screen::Online);
    let p = a.net_pump((now + 16) * 1000);
    assert!(
        p.out.iter().any(|m| m.text.contains("dissolve")),
        "the host says it is going"
    );
    assert_eq!(p.requests.last().unwrap()["op"], "leave");
    for m in p.out {
        b.net_event("message", "pa", &m.text, true);
    }
    b.net_pump((now + 32) * 1000);
    assert_eq!(b.screen, Screen::Online);
    assert_eq!(b.ui().panels[0].sub, "ONLINE.HOST_LEFT");
}

/// A and B in an online game with the 3-2-1 (no `?fast`), started.
fn playing() -> (App, App, i64) {
    let (mut a, mut b, now) = joined();
    b.act("online.ready", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    a.act("online.start", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    assert_eq!((a.screen, b.screen), (Screen::Play, Screen::Play));
    (a, b, now)
}

#[test]
fn two_browsers_agree_on_the_seats_the_tiles_the_clock_and_a_jump() {
    let (mut a, mut b, now) = playing();
    {
        let (ga, gb) = (&a.play.as_ref().unwrap().game, &b.play.as_ref().unwrap().game);
        assert_eq!(ga.crabs.len(), 2);
        assert_eq!(gb.crabs.len(), 2);
        // the same seed: the same letters in the same slots
        let letters = |g: &crate::sim::Match| g.tiles.all.iter().map(|t| t.letter).collect::<Vec<_>>();
        assert_eq!(letters(ga), letters(gb));
        // the clocks (a 3-2-1, then 4:00) agree within a frame and a tick
        assert!(
            (ga.elapsed_ms - gb.elapsed_ms).abs() <= 40,
            "{} vs {}",
            ga.elapsed_ms,
            gb.elapsed_ms
        );
        assert_eq!(ga.length_ms(), 4 * 60 * 1000);
    }
    assert_eq!(a.ui().phase, "countdown");
    assert_eq!(b.ui().phase, "countdown");
    // past the 3-2-1, B jumps: A sees B's crab leave the ground
    let mut now = run(&mut a, &mut b, now, 3000);
    assert_eq!(b.ui().phase, "running");
    b.act("key", "Space").unwrap();
    let mut highest = 0;
    let mut in_air = false;
    for _ in 0..80 {
        now = run(&mut a, &mut b, now, 16);
        let c = &a.play.as_ref().unwrap().game.crabs[1];
        highest = highest.max(c.pos.y);
        in_air |= !c.on_ground;
    }
    assert!(in_air, "A draws B's crab in the air");
    // the jump's apex is 65 mm; drawn between states, most of it shows
    assert!(highest > crate::model::mm1(40), "highest {highest}");
    let c = &a.play.as_ref().unwrap().game.crabs[1];
    assert!(c.on_ground && c.pos.y == 0, "and back on the sand");
}

#[test]
fn a_crab_whose_states_stop_is_named_and_the_host_leaving_ends_the_game() {
    let (mut a, mut b, mut now) = playing();
    let notice = |x: &App| {
        x.ui()
            .panels
            .iter()
            .find(|p| p.kind == "notice")
            .map(|p| p.heading.clone())
    };
    assert_eq!(notice(&a), None);
    // B's messages stop reaching A for two seconds
    for _ in 0..125 {
        now += 16;
        a.net_pump(now * 1000);
        b.net_pump(now * 1000);
    }
    assert_eq!(notice(&a).as_deref(), Some("HUD.STALLED"));
    let now = run(&mut a, &mut b, now, 200);
    assert_eq!(notice(&a), None, "it clears when B's states come again");
    // the host's link drops mid-game: B's game ends, saying why
    b.net_event("disconnected", "pa", "", false);
    b.net_pump((now + 16) * 1000);
    assert_eq!(b.ui().phase, "ended");
    let end = b.ui().panels.into_iter().find(|p| p.kind == "end").unwrap();
    assert_eq!(end.sub, "END.HOST_LEFT");
}

/// Two apps joined by a bad network: each message lost `loss` times in a
/// hundred and delayed 10 ms plus up to `jitter` ms, so later ones often
/// overtake it.
struct Link {
    apps: [App; 2],
    /// (deliver at, to, text)
    wire: Vec<(i64, usize, String)>,
    dice: crate::rng::Dice,
    loss: u32,
    jitter: u32,
    now: i64,
}

impl Link {
    fn frame(&mut self) {
        self.now += 16;
        let now = self.now;
        for from in 0..2 {
            let out = self.apps[from].net_pump(now * 1000).out;
            let peer = if from == 0 { "pb" } else { "pa" };
            for m in out {
                if m.to.as_deref().is_some_and(|t| t != peer) {
                    continue;
                }
                if self.dice.below(100) < self.loss {
                    continue;
                }
                let late = 10 + self.dice.below(self.jitter + 1) as i64;
                self.wire.push((now + late, 1 - from, m.text));
            }
        }
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.wire)
            .into_iter()
            .partition(|w| w.0 <= now);
        self.wire = later;
        for (_, to, text) in due {
            let (from, host) = if to == 0 { ("pb", false) } else { ("pa", true) };
            self.apps[to].net_event("message", from, &text, host);
        }
    }

    fn game(&mut self, k: usize) -> &mut crate::sim::Match {
        &mut self.apps[k].play.as_mut().unwrap().game
    }
}

/// Seat `k`'s next claim, as its own browser sees the game: grab one of
/// the first few loose tiles (so the two crabs often want the same one),
/// place a held tile on one of the first few legal squares, or drop it.
fn a_move(g: &crate::sim::Match, k: usize, dice: &mut crate::rng::Dice) -> Option<crate::sim::Claim> {
    use crate::sim::Claim;
    let claw = dice.below(2) as usize;
    match g.crabs[k].claws[claw] {
        None => {
            let loose: Vec<u16> = g
                .tiles
                .all
                .iter()
                .filter(|t| t.is_loose())
                .map(|t| t.id)
                .take(4)
                .collect();
            let tile = *loose.get(dice.below(loose.len() as u32) as usize)?;
            Some(Claim::Grab { seat: k, claw, tile })
        }
        Some(tile) => {
            let n = g.rules.board_size;
            let legal: Vec<(i32, i32)> = (0..n)
                .flat_map(|r| (0..n).map(move |c| (r, c)))
                .filter(|sq| g.legal_square(*sq))
                .take(3)
                .collect();
            if dice.below(10) < 7 && !legal.is_empty() {
                let square = legal[dice.below(legal.len() as u32) as usize];
                Some(Claim::Place {
                    seat: k,
                    claw,
                    tile,
                    square,
                })
            } else {
                Some(Claim::Drop { seat: k, claw, tile })
            }
        }
    }
}

#[test]
fn over_a_lossy_jittery_link_both_browsers_end_with_the_same_tiles_and_one_winner_each_time() {
    let (mut a, mut b, now) = joined();
    // twenty messy piles of five, which tumble when a tile under is pulled
    let mut map = crate::sim::Map::flat(6000);
    for p in 0..20 {
        for k in 0..5 {
            let at = [
                1800 + (p % 5) * 320 + k * 37,
                30 + k * 50,
                -1200 + (p / 5) * 400 - k * 23,
            ];
            map.slots.push((
                crate::gfx::geom::Affine::translate(crate::model::mm(at)),
                k % 3 == 2,
            ));
        }
    }
    map.slots.push((crate::gfx::geom::Affine::IDENTITY, false));
    for x in [&mut a, &mut b] {
        x.act("flag.fast", "on").unwrap();
        x.act("flag.short", "12").unwrap();
        x.view.map = Some(map.clone());
    }
    b.act("online.ready", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    a.act("online.start", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    let mut link = Link {
        apps: [a, b],
        wire: Vec::new(),
        dice: crate::rng::Dice(99),
        loss: 20,
        jitter: 120,
        now,
    };
    // the host referees; the joiner asks it (its clicks go to claims_out)
    assert_eq!(link.game(1).referee, crate::sim::Referee::Remote);
    assert!(link.game(0).record_verdicts && link.game(0).referee == crate::sim::Referee::Local);
    // both crabs grab the same tile in the same frame
    let first = link.game(0).tiles.all.iter().find(|t| t.is_loose()).unwrap().id;
    link.game(0).submit_claim(crate::sim::Claim::Grab {
        seat: 0,
        claw: 0,
        tile: first,
    });
    link.game(1).claims_out.push(crate::sim::Claim::Grab {
        seat: 1,
        claw: 0,
        tile: first,
    });
    for _ in 0..60 {
        link.frame();
    }
    let holder = |g: &crate::sim::Match| g.tiles.get(first).unwrap().state;
    let (ha, hb) = (holder(link.game(0)), holder(link.game(1)));
    assert_eq!(ha, hb, "both browsers agree who has it");
    assert!(matches!(ha, crate::sim::State::Held { claw: 0, .. }), "{ha:?}");
    let holding = |g: &crate::sim::Match| g.crabs.iter().filter(|c| c.claws[0] == Some(first)).count();
    assert_eq!(
        (holding(link.game(0)), holding(link.game(1))),
        (1, 1),
        "one winner"
    );
    let mut moves = crate::rng::Dice(5);
    let mut frames = 0;
    while !(link.apps[0].ui().phase == "ended" && link.apps[1].ui().phase == "ended") {
        link.frame();
        frames += 1;
        assert!(frames < 2000, "both games end");
        if frames % 6 == 0 && link.apps[0].ui().phase == "running" {
            if let Some(c) = a_move(link.game(0), 0, &mut moves) {
                link.game(0).submit_claim(c);
            }
            if let Some(c) = a_move(link.game(1), 1, &mut moves) {
                link.game(1).claims_out.push(c);
            }
        }
    }
    let (ga, gb) = (
        &link.apps[0].play.as_ref().unwrap().game,
        &link.apps[1].play.as_ref().unwrap().game,
    );
    // the first tile: one winner, the same on both
    let held = |g: &crate::sim::Match| g.crabs.iter().map(|c| c.claws).collect::<Vec<_>>();
    let placed = |g: &crate::sim::Match| {
        g.tiles
            .all
            .iter()
            .filter_map(|t| match t.state {
                crate::sim::State::Placed { square, seat, order } => {
                    Some((t.id, square, seat, order, t.since_ms))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let states = |g: &crate::sim::Match| g.tiles.all.iter().map(|t| t.state).collect::<Vec<_>>();
    assert!(
        placed(ga).len() > 5,
        "a real game was played: {} placed",
        placed(ga).len()
    );
    assert_eq!(
        placed(ga),
        placed(gb),
        "the same tiles on the same squares, by the same crabs, in the same order and at the same times"
    );
    assert_eq!(held(ga), held(gb), "the same tiles in the same claws");
    let loose = |s: crate::sim::State| matches!(s, crate::sim::State::Loose { .. });
    assert_eq!(
        states(ga)
            .into_iter()
            .map(|s| if loose(s) { None } else { Some(s) })
            .collect::<Vec<_>>(),
        states(gb)
            .into_iter()
            .map(|s| if loose(s) { None } else { Some(s) })
            .collect::<Vec<_>>()
    );
    assert_eq!(ga.placements, gb.placements);
    let idle = |g: &crate::sim::Match| {
        g.crabs
            .iter()
            .map(|c| (c.placed_ms, c.penalty))
            .collect::<Vec<_>>()
    };
    assert_eq!(idle(ga), idle(gb), "the same idle timers and penalties");
    // and the tiles at rest lie where the host's lie
    let rest = |g: &crate::sim::Match| {
        g.tiles
            .all
            .iter()
            .filter(|t| t.state == (crate::sim::State::Loose { awake: false }))
            .map(|t| (t.id, t.pose, t.face_down))
            .collect::<Vec<_>>()
    };
    let (ra, rb) = (rest(ga), rest(gb));
    let differ = ra.iter().filter(|x| !rb.contains(x)).count();
    assert_eq!(differ, 0, "loose tiles lying differently");
}

#[test]
fn a_tile_held_by_a_crab_whose_player_left_is_let_go_on_the_host() {
    let (mut a, mut b, now) = joined();
    let mut map = crate::sim::Map::flat(6000);
    map.slots.push((
        crate::gfx::geom::Affine::translate(crate::model::mm([2000, 0, 0])),
        false,
    ));
    map.slots.push((crate::gfx::geom::Affine::IDENTITY, false));
    for x in [&mut a, &mut b] {
        x.act("flag.fast", "on").unwrap();
        x.view.map = Some(map.clone());
    }
    b.act("online.ready", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    a.act("online.start", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    b.play
        .as_mut()
        .unwrap()
        .game
        .claims_out
        .push(crate::sim::Claim::Grab {
            seat: 1,
            claw: 1,
            tile: 0,
        });
    let mut now = run(&mut a, &mut b, now, 300);
    let held = |x: &App| x.play.as_ref().unwrap().game.crabs[1].claws;
    assert_eq!(held(&a), [None, Some(0)]);
    assert_eq!(held(&b), [None, Some(0)], "B heard the host's verdict");
    a.net_event("disconnected", "pb", "", false);
    for _ in 0..20 {
        now += 16;
        a.net_pump(now * 1000);
    }
    assert_eq!(held(&a), [None, None]);
    assert!(a.play.as_ref().unwrap().game.tiles.all[0].is_loose());
}

#[test]
fn clicks_grab_and_drop_through_the_host() {
    let (mut a, mut b, now) = joined();
    let mut map = crate::sim::Map::flat(6000);
    for (x, z) in [(-1800, 1800), (-1800, -1800)] {
        map.slots.push((
            crate::gfx::geom::Affine::translate(crate::model::mm([x, 0, z])),
            false,
        ));
    }
    map.slots.push((crate::gfx::geom::Affine::IDENTITY, false));
    for x in [&mut a, &mut b] {
        x.act("flag.fast", "on").unwrap();
        x.act("flag.spot", "tile").unwrap();
        x.view.map = Some(map.clone());
    }
    b.act("online.ready", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    a.act("online.start", "").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    let held = |x: &App| {
        x.play
            .as_ref()
            .unwrap()
            .game
            .crabs
            .iter()
            .map(|c| c.claws)
            .collect::<Vec<_>>()
    };
    a.act("mouse.down", "0").unwrap();
    b.act("mouse.down", "0").unwrap();
    let now = run(&mut a, &mut b, now, 300);
    assert_eq!(held(&a), held(&b));
    assert_eq!(held(&a), vec![[Some(0), None], [Some(1), None]]);
    a.act("mouse.down", "0").unwrap();
    b.act("mouse.down", "0").unwrap();
    run(&mut a, &mut b, now, 300);
    assert_eq!(held(&a), vec![[None, None], [None, None]]);
    assert_eq!(held(&b), held(&a));
}
