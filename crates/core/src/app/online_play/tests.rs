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
