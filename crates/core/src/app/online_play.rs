//! Online play in the app: the Online game screen (Host, or Join by code),
//! the Lobby, the start of an online game and its sync, and the wiring
//! between the page's transport (`web/online.js`, through `web/net.js`) and
//! the protocol (`crate::online`). The lobby's conveniences are Ape Race's:
//! the code shown big with Copy code and Copy link, joining straight from
//! an invite link, the pasted-code path, names, seat colours, Ready marks,
//! AI crabs, each player's round trip, and Rematch.
//!
//! - **The page's side.** `net_event(kind, peer, text, host)` for each
//!   transport event; `net_code(which, code)` for a pasted code the
//!   transport made; and every frame `net_pump(now)`, whose answer holds the
//!   text to send (`out`, each `{to, text}`, `to: null` for everyone) and
//!   what to ask of the transport (`requests`, each `{op, ...}`: host_room,
//!   join_room, host_code, invite_code, join_code, accept_reply, leave).
//!   The page sends `out` before it carries out `requests`, so a parting
//!   word goes out before the links close (Ape Race's N14).
//! - **Pasted codes.** With `flag.codes` (the page's `?codes`, for the
//!   automated online check) the Online game screen and the Lobby also offer
//!   the pasted-code path, which needs no relay.
//! - **Invite links.** The page's address with `#room=CODE`: opening it goes
//!   straight to that lobby (`flag.room_link`). A joiner's address keeps it
//!   while it is in the lobby, so a refresh joins again (Honey Hole's way,
//!   with no browser storage).

use serde::Serialize;
use serde_json::{json, Value};

use super::play::Play;
use super::*;
use crate::online::{self, Refusal, Session, Status};
use crate::rng::mix;
use crate::sim::Driver;
use crate::view::TableRow;

#[cfg(test)]
mod tests;

/// The seats' colours by place, for the strings (`colour.*`) and the
/// stylesheet's row tints; `data/standin.json` draws them.
pub const SEAT_COLOURS: [&str; 4] = ["red", "yellow", "blue", "purple"];

/// How long a Copy button reads Copied after a press, in milliseconds.
const COPIED_MS: i64 = 1500;

/// The longest pasted code a field takes (a compressed session
/// description is a few hundred characters).
const CODE_FIELD_MAX: u32 = 8000;

/// How far another browser's crab must go from its start to count as moved
/// (for the online check).
const MOVED_MM: i64 = 300;

/// The online part of the app.
#[derive(Default)]
pub struct OnlinePlay {
    /// this browser's id, from the page (`crypto.randomUUID()`)
    pub client_id: String,
    pub session: Option<Session>,
    /// the code, typed or made by Host
    pub room: String,
    /// the pasted-code path is offered (`flag.codes`)
    pub codes: bool,
    /// this browser's own pasted code to give (the host's invite, a
    /// joiner's reply)
    pub my_code: String,
    /// a pasted code typed in (a joiner's invite, the host's reply)
    pub their_code: String,
    /// a problem to show: a strings key and its placeholders
    pub problem: Option<(String, Vec<(String, String)>)>,
    /// what the transport is asked to do next
    pub requests: Vec<Value>,
    /// the last messages of a session that has been left, still to send
    parting: Vec<online::Outgoing>,
    /// the page's own address, no query or hash (`flag.page_url`)
    pub page_url: String,
    /// the Copy button last pressed and when, in ms
    pub copied: Option<(String, i64)>,
    /// each seat's name in the game under way, in crab order (an AI crab's
    /// is empty), for the HUD's stalled notice
    pub seat_names: Vec<String>,
}

impl OnlinePlay {
    pub fn hosting(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.is_host())
    }
}

/// What `net_pump` hands the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pump {
    pub out: Vec<online::Outgoing>,
    pub requests: Vec<Value>,
    /// the sounds to play now (`web/audio.js`)
    pub sounds: Vec<super::play::Cue>,
    /// the screen may have changed: draw `ui` again
    pub redraw: bool,
    /// in an online game, whether another browser's crab has left its
    /// start (the online check reads it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_moved: Option<bool>,
    /// the address's hash: `#room=CODE` while this browser is a joiner in
    /// a lobby by its code, so a refresh joins it again; empty otherwise
    pub hash: String,
}

impl App {
    fn request(&mut self, op: &str, extra: Value) {
        let mut r = json!({ "op": op });
        if let (Some(r), Value::Object(e)) = (r.as_object_mut(), extra) {
            r.extend(e);
        }
        self.online.requests.push(r);
    }

    fn problem(&mut self, key: &str) {
        self.online.problem = Some((key.to_string(), Vec::new()));
    }

    /// The player's name: the one typed, or the default.
    fn player_name(&self) -> String {
        let name = online::clean_name(&self.settings.name);
        if name.is_empty() {
            self.t("name.default")
        } else {
            name
        }
    }

    fn open_session(&mut self, host: bool) {
        let now = self.now_ms();
        let name = self.player_name();
        if self.online.client_id.is_empty() {
            self.online.client_id = format!("c{}", mix(self.seed ^ 0x6964));
        }
        let id = self.online.client_id.clone();
        self.online.session = Some(if host {
            Session::host(&id, &name, now)
        } else {
            Session::join(&id, &name, now)
        });
        self.online.problem = None;
        self.online.my_code.clear();
        self.go(Screen::Lobby);
    }

    /// The room as a link: opening it joins the lobby.
    pub(crate) fn room_link(&self) -> String {
        format!("{}#room={}", self.online.page_url, self.online.room)
    }

    /// The Online game screen's and the Lobby's controls (`online.*`), and
    /// the page's online flags (`flag.*`).
    pub(super) fn online_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        let now = self.now_ms();
        match name {
            "flag.client" => self.online.client_id = value.to_string(),
            "flag.codes" => self.online.codes = value != "off",
            "flag.page_url" => self.online.page_url = value.to_string(),
            // opened from an invite link: straight to that lobby
            "flag.room_link" => {
                if online::clean_code(value).is_some() {
                    self.go(Screen::Online);
                    self.online.room = value.to_string();
                    self.online_act("online.join", "")?;
                }
            }
            "online.name" => {
                self.settings.name = value
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(online::MAX_NAME_CHARS)
                    .collect();
                let n = self.player_name();
                if let Some(s) = self.online.session.as_mut() {
                    s.set_name(&n, now);
                }
            }
            "online.code" => self.online.room = value.trim().to_string(),
            "online.host" => {
                let code = online::room_code(mix(self.seed ^ self.clock as u64 ^ 0x726f_6f6d));
                self.open_session(true);
                self.online.room = code.clone();
                self.request("host_room", json!({ "room": online::room_name(&code) }));
            }
            "online.join" => match online::clean_code(&self.online.room) {
                Some(code) => {
                    self.open_session(false);
                    self.online.room = code.clone();
                    self.request("join_room", json!({ "room": online::room_name(&code) }));
                }
                None => {
                    self.online.problem = Some((
                        "online.code_shape".into(),
                        vec![("{n}".into(), online::CODE_LEN.to_string())],
                    ))
                }
            },
            "online.host_code" => {
                self.open_session(true);
                self.online.room.clear();
                self.request("host_code", json!({}));
            }
            "online.invite" | "online.reply" => self.online.their_code = value.trim().to_string(),
            "online.join_code" => {
                let invite = std::mem::take(&mut self.online.their_code);
                self.open_session(false);
                self.online.room.clear();
                self.request("join_code", json!({ "invite": invite }));
            }
            "online.connect" => {
                let reply = std::mem::take(&mut self.online.their_code);
                self.request("accept_reply", json!({ "reply": reply }));
            }
            "online.next_invite" => {
                self.online.my_code.clear();
                self.request("invite_code", json!({}));
            }
            "online.ready" => {
                if let Some(s) = self.online.session.as_mut() {
                    let r = !s.lobby().ready;
                    s.set_ready(r, now);
                }
            }
            "online.ai" => {
                let n: usize = value
                    .parse()
                    .map_err(|_| format!("The AI crab count \"{value}\" is not a whole number."))?;
                if let Some(s) = self.online.session.as_mut() {
                    s.set_ai_count(n, now);
                }
            }
            "online.copy" | "online.copy_link" => self.online.copied = Some((name.to_string(), now)),
            "online.start" => {
                let seed = mix(self.seed ^ (self.clock as u64).rotate_left(29));
                if let Some(s) = self.online.session.as_mut() {
                    s.start(seed, now);
                }
            }
            "online.leave" => {
                self.leave_online();
                self.go(Screen::Online);
            }
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// Leaves the online session: its parting word goes out before the
    /// transport closes (the page sends `out` before the requests).
    pub(crate) fn leave_online(&mut self) {
        if let Some(mut s) = self.online.session.take() {
            s.leave();
            self.online.parting.extend(s.take_outbox());
            self.request("leave", json!({}));
        }
        self.online.my_code.clear();
        self.online.their_code.clear();
    }

    /// The host's Rematch: everyone back to the Lobby with the same seats.
    pub(super) fn online_rematch(&mut self) {
        let now = self.now_ms();
        if let Some(s) = self.online.session.as_mut() {
            if s.rematch(now) {
                self.play = None;
                self.go(Screen::Lobby);
            }
        }
    }

    // ---- the page's side ----

    /// A transport event: connected, message, disconnected or error (then
    /// `text` is a strings key).
    pub fn net_event(&mut self, kind: &str, peer: &str, text: &str, host: bool) {
        let now = self.now_ms();
        if kind == "error" {
            self.problem(text);
            return;
        }
        let Some(s) = self.online.session.as_mut() else {
            return;
        };
        match kind {
            "connected" => s.peer_connected(peer, host, now),
            "message" => s.receive(peer, text, now),
            "disconnected" => s.peer_left(peer, now),
            _ => {}
        }
    }

    /// A pasted code the transport made: the host's invite or a joiner's
    /// reply, to show for copying.
    pub fn net_code(&mut self, _which: &str, code: &str) {
        self.online.my_code = code.to_string();
    }

    /// Every frame: the clock, the session's repeats and its start, and
    /// what the page sends and asks of the transport.
    pub fn net_pump(&mut self, now_us: i64) -> Pump {
        self.advance(now_us);
        let now = self.now_ms();
        let mut out = std::mem::take(&mut self.online.parting);
        let mut start = None;
        if let Some(s) = self.online.session.as_mut() {
            s.update(now);
            start = s.take_start();
            out.extend(s.take_outbox());
        }
        if let Some(st) = start {
            if let Err(e) = self.begin_online_game(st) {
                self.online.problem = Some((e, Vec::new()));
            }
        }
        self.session_moved_on();
        let redraw = self.online.session.is_some() || matches!(self.screen, Screen::Online | Screen::Lobby);
        Pump {
            out,
            requests: std::mem::take(&mut self.online.requests),
            sounds: std::mem::take(&mut self.cues),
            redraw,
            remote_moved: self.remote_moved(),
            hash: self.room_hash(),
        }
    }

    /// `#room=CODE` while this browser is a joiner in a lobby by its code.
    fn room_hash(&self) -> String {
        match &self.online.session {
            Some(s) if !s.is_host() && !self.online.room.is_empty() => format!("#room={}", self.online.room),
            _ => String::new(),
        }
    }

    /// The session's status took this browser elsewhere: a lobby that
    /// ended (refused, closed, the host lost) goes back to the Online game
    /// screen, which says why, with the code kept for another try; the
    /// host's Rematch brings a joiner back to the Lobby.
    fn session_moved_on(&mut self) {
        let Some(s) = self.online.session.as_ref() else {
            return;
        };
        let why = match s.status() {
            Status::Refused { reason } => Some(match reason {
                Refusal::Full => "online.refused.full",
                Refusal::Started => "online.refused.underway",
                Refusal::WrongVersion { .. } => "online.refused.version",
            }),
            Status::Dissolved => Some("online.host_left"),
            Status::Lost => Some("online.lost"),
            _ => None,
        };
        if let Some(key) = why {
            if self.screen == Screen::Lobby {
                self.problem(key);
                self.online.session = None;
                self.request("leave", json!({}));
                self.go(Screen::Online);
            }
            return;
        }
        if *s.status() == Status::Lobby && self.screen == Screen::Play {
            self.play = None;
            self.go(Screen::Lobby);
        }
    }

    fn begin_online_game(&mut self, st: online::GameStart) -> Result<(), String> {
        let host = self.online.hosting();
        self.online.seat_names = st.seats.iter().map(|s| s.name.clone()).collect();
        let drivers = st
            .seats
            .iter()
            .enumerate()
            .map(|(i, seat)| {
                if i == st.local_index {
                    Driver::Local
                } else if seat.ai && host {
                    Driver::Ai
                } else {
                    Driver::Remote
                }
            })
            .collect();
        self.begin_game(drivers, st.local_index, st.seed, true)
    }

    /// After the game moves on, in an online game: this browser's crabs'
    /// states out, the others' in; the host's clock ends the game for
    /// everyone, and a joiner's ends when the host's End arrives.
    pub(super) fn online_game_sync(&mut self) {
        let now = self.now_ms();
        let (Some(p), Some(s)) = (self.play.as_mut(), self.online.session.as_mut()) else {
            return;
        };
        if !p.online {
            return;
        }
        let g = &mut p.game;
        for (i, c) in g.crabs.iter().enumerate() {
            s.send_state(now, g.elapsed_ms, i, c.state());
        }
        for r in s.remote_states(now) {
            g.set_remote(r.index, r.state);
        }
        if s.is_host() {
            if g.out_of_time() {
                s.end_game(now, Vec::new());
            }
        } else if s.ended() {
            g.finish();
        }
    }

    /// Whether a crab another browser moves is away from its start, in an
    /// online game.
    fn remote_moved(&self) -> Option<bool> {
        self.online.session.as_ref()?;
        let p: &Play = self.play.as_ref()?;
        Some(
            p.game
                .crabs
                .iter()
                .enumerate()
                .any(|(i, c)| c.driver == Driver::Remote && p.moved_mm(i) >= MOVED_MM),
        )
    }

    // ---- panels ----

    /// During an online game: the players whose crabs' states have stopped
    /// coming ("No updates from Ana"), or the host gone at the end. None
    /// in a Local game or when all is well.
    pub(super) fn online_notice(&self, p: &Play) -> Option<String> {
        if !p.online {
            return None;
        }
        let s = self.online.session.as_ref()?;
        if s.host_gone() {
            return Some(self.t("end.host_left"));
        }
        let names: Vec<String> = s
            .stalled(self.now_ms())
            .into_iter()
            .map(|i| match self.online.seat_names.get(i) {
                Some(n) if !n.is_empty() => n.clone(),
                _ => self.t("lobby.ai_crab"),
            })
            .collect();
        (!names.is_empty()).then(|| self.t("hud.stalled").replace("{names}", &names.join(", ")))
    }

    fn problem_text(&self) -> String {
        match &self.online.problem {
            Some((key, args)) => args.iter().fold(self.t(key), |t, (k, v)| t.replace(k, v)),
            None => String::new(),
        }
    }

    fn code_field(&self, id: &str, value: &str) -> Item {
        Item::Text {
            id: id.into(),
            label: self.t(id),
            value: value.to_string(),
            max: CODE_FIELD_MAX,
            placeholder: String::new(),
        }
    }

    fn name_field(&self) -> Item {
        Item::Text {
            id: "online.name".into(),
            label: self.t("online.name"),
            value: self.settings.name.clone(),
            max: online::MAX_NAME_CHARS as u32,
            placeholder: self.t("name.default"),
        }
    }

    pub(super) fn online_panels(&self) -> Vec<Panel> {
        let mut items = vec![
            self.name_field(),
            self.button("online.host"),
            Item::Text {
                id: "online.code".into(),
                label: self.t("online.code"),
                value: self.online.room.clone(),
                max: online::CODE_LEN as u32 + 2,
                placeholder: String::new(),
            },
            self.button("online.join"),
        ];
        if self.online.codes {
            items.push(self.button("online.host_code"));
            items.push(self.code_field("online.invite", &self.online.their_code));
            items.push(self.button("online.join_code"));
        }
        items.push(self.button("menu.back"));
        let mut p = Panel::new("online", self.t("menu.online"), items);
        p.sub = self.problem_text();
        vec![p]
    }

    /// A Copy button, reading Copied for a moment after a press.
    fn copy_item(&self, id: &str, value: &str, copy: String) -> Item {
        let copied = self
            .online
            .copied
            .as_ref()
            .is_some_and(|(k, at)| k == id && self.now_ms() - at < COPIED_MS);
        Item::Copy {
            id: id.into(),
            label: if value.is_empty() {
                String::new()
            } else {
                self.t("online.code")
            },
            value: value.to_string(),
            copy,
            text: self.t(if copied { "online.copied" } else { id }),
            disabled: false,
        }
    }

    /// The code shown big with Copy code, and Copy link, while the lobby
    /// has a code (a pasted-code lobby has none).
    fn room_items(&self) -> Vec<Item> {
        let room = &self.online.room;
        if room.is_empty() {
            return Vec::new();
        }
        let mut items = vec![self.copy_item("online.copy", room, room.clone())];
        if !self.online.page_url.is_empty() {
            items.push(self.copy_item("online.copy_link", "", self.room_link()));
        }
        items
    }

    fn status_text(&self, s: &Session) -> String {
        let problem = self.problem_text();
        if !problem.is_empty() {
            return problem;
        }
        match s.status() {
            Status::Joining => self.t("online.joining"),
            _ => String::new(),
        }
    }

    pub(super) fn lobby_panels(&self) -> Vec<Panel> {
        let Some(s) = self.online.session.as_ref() else {
            return vec![Panel::new(
                "lobby",
                self.t("lobby.heading"),
                vec![self.button("online.leave")],
            )];
        };
        let view = s.lobby();
        let mut items = self.room_items();
        if !view.is_host && !s.seated() {
            if self.online.codes {
                items.push(self.code_field("online.reply", &self.online.my_code));
            }
            items.push(self.button("online.leave"));
            let mut p = Panel::new("lobby", self.t("lobby.heading"), items);
            p.sub = self.status_text(s);
            return vec![p];
        }
        let rows = view
            .seats
            .iter()
            .enumerate()
            .map(|(k, seat)| {
                let colour = SEAT_COLOURS[k % 4];
                let name = if seat.ai {
                    self.t("lobby.ai_crab")
                } else if seat.me {
                    self.t("lobby.you").replace("{name}", &seat.name)
                } else {
                    seat.name.clone()
                };
                let state = if seat.ai {
                    String::new()
                } else if seat.host {
                    self.t("lobby.host")
                } else if seat.ready {
                    self.t("lobby.ready")
                } else {
                    self.t("lobby.not_ready")
                };
                let ping = seat
                    .ping_ms
                    .map(|ms| self.t("lobby.ping").replace("{ms}", &ms.to_string()))
                    .unwrap_or_default();
                TableRow {
                    cells: vec![self.t(&format!("colour.{colour}")), name, state, ping],
                    marked: seat.me,
                    tone: colour.into(),
                }
            })
            .collect();
        items.push(self.name_field());
        items.push(Item::Table {
            head: vec![
                self.t("lobby.colour"),
                self.t("online.name"),
                self.t("lobby.state"),
                self.t("lobby.ping_head"),
            ],
            rows,
        });
        if view.is_host {
            items.push(Item::Select {
                id: "online.ai".into(),
                label: self.t("setup.ai"),
                value: view.ai_count.to_string(),
                options: self.ai_choices(0, view.max_ai),
            });
            items.push(Item::Button {
                id: "online.start".into(),
                text: self.t("local.start"),
                quiet: false,
                badge: None,
                disabled: !view.can_start,
            });
        } else {
            let key = if view.ready {
                "online.unready"
            } else {
                "online.ready"
            };
            items.push(Item::Button {
                id: "online.ready".into(),
                text: self.t(key),
                quiet: false,
                badge: None,
                disabled: view.status != Status::Lobby,
            });
        }
        if self.online.codes {
            if view.is_host {
                items.push(self.code_field("online.invite", &self.online.my_code));
                items.push(self.code_field("online.reply", &self.online.their_code));
                items.push(self.button("online.connect"));
                items.push(self.button("online.next_invite"));
            } else {
                items.push(self.code_field("online.reply", &self.online.my_code));
            }
        }
        items.push(self.button("online.leave"));
        let mut p = Panel::new("lobby", self.t("lobby.heading"), items);
        p.sub = self.status_text(s);
        vec![p]
    }
}
