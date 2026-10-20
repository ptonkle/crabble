//! One browser's online session (Ape Race's `Online`, without its race).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use super::msg::{CrabState, Human, Msg, Refusal};
use super::*;

/// A message for the transport: `to: None` is every connected peer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Outgoing {
    pub to: Option<PeerId>,
    pub text: String,
}

/// What this browser's online session is doing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    /// A joiner waiting for its seat.
    Joining,
    /// Seated in the lobby.
    Lobby,
    /// The host refused this joiner.
    Refused { reason: Refusal },
    /// In a game (or at its end). After a Rematch, back to `Lobby`.
    Playing,
    /// The host closed the lobby.
    Dissolved,
    /// A joiner lost its link to the host before the game.
    Lost,
    /// This browser left.
    Left,
}

/// One seat of a game, in crab order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SeatStart {
    /// empty for an AI crab: the app names it
    pub name: String,
    pub ai: bool,
}

/// Everything this browser needs to set up its game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameStart {
    pub seed: u64,
    pub round: u32,
    pub seats: Vec<SeatStart>,
    /// the crab this browser's player steers
    pub local_index: usize,
}

/// Where another browser's crab should be drawn now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RemoteCrab {
    pub index: usize,
    pub state: CrabState,
}

/// One seat as the lobby draws it, in crab order (its colour by place).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SeatView {
    pub name: String,
    pub ai: bool,
    pub host: bool,
    pub me: bool,
    pub ready: bool,
    /// the round trip to this seat's browser, when measured
    pub ping_ms: Option<i64>,
}

/// The lobby as the screen draws it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LobbyView {
    pub status: Status,
    pub is_host: bool,
    pub seats: Vec<SeatView>,
    pub ai_count: usize,
    /// the most AI crabs the host may set now
    pub max_ai: usize,
    /// this browser's Ready mark, as last asked for
    pub ready: bool,
    /// the host may press Start
    pub can_start: bool,
}

/// Who owns a crab: the person with this client id, or (AI) the host.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Owner {
    Human(String),
    Ai,
}

#[derive(Clone, Debug, Default)]
struct Peer {
    id: Option<String>,
    rtts: VecDeque<i64>,
    /// when anything last came from it
    heard_ms: Option<i64>,
    /// the client ids it hears directly, from its pings
    near: BTreeSet<String>,
}

/// Round trips kept per peer.
const RTT_KEPT: usize = 10;
/// States kept per crab: two seconds at 20 Hz.
const STATES_KEPT: usize = 40;

impl Peer {
    /// The 90th-percentile round trip (nearest rank).
    fn rtt_p90(&self) -> Option<i64> {
        if self.rtts.is_empty() {
            return None;
        }
        let mut sorted: Vec<i64> = self.rtts.iter().copied().collect();
        sorted.sort_unstable();
        let rank = (sorted.len() * 9).div_ceil(10);
        Some(sorted[rank.max(1) - 1])
    }
}

/// One state received for another browser's crab.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    /// the owner's game clock, ms
    pub(crate) sent: i64,
    /// when it arrived here
    pub(crate) got: i64,
    pub(crate) s: CrabState,
}

#[derive(Clone, Debug)]
struct GameState {
    round: u32,
    seed: u64,
    owners: Vec<Owner>,
    /// Host: joiners that have not acknowledged the Start.
    unacked: BTreeSet<PeerId>,
    last_start_ms: i64,
    last_sent: BTreeMap<usize, i64>,
    samples: BTreeMap<usize, VecDeque<Sample>>,
    /// crabs whose owner has left
    gone: BTreeSet<usize>,
    /// the host's clock ran out (on a joiner: its End arrived)
    ended: bool,
    last_end_ms: i64,
    host_gone: bool,
    /// when this browser's game began, ms
    began_ms: i64,
}

/// One browser's online session.
#[derive(Clone, Debug)]
pub struct Session {
    proto: u32,
    client_id: String,
    name: String,
    is_host: bool,
    host_peer: Option<PeerId>,
    status: Status,
    round: u32,
    humans: Vec<Human>,
    ai_count: usize,
    peers: BTreeMap<PeerId, Peer>,
    outbox: Vec<Outgoing>,
    last_hello_ms: i64,
    last_roster_ms: i64,
    last_ping_ms: i64,
    wanted_ready: Option<bool>,
    last_ready_ms: i64,
    wanted_name: Option<String>,
    last_name_ms: i64,
    game: Option<GameState>,
    /// Host: its own start, held until the joiners have theirs.
    host_start_at: Option<i64>,
    start: Option<GameStart>,
}

impl Session {
    fn blank(client_id: &str, name: &str, now_ms: i64) -> Session {
        Session {
            proto: PROTOCOL,
            client_id: client_id.to_string(),
            name: clean_name(name),
            is_host: false,
            host_peer: None,
            status: Status::Joining,
            round: 0,
            humans: Vec::new(),
            ai_count: 0,
            peers: BTreeMap::new(),
            outbox: Vec::new(),
            last_hello_ms: now_ms,
            last_roster_ms: now_ms,
            last_ping_ms: now_ms - PING_EVERY_MS,
            wanted_ready: None,
            last_ready_ms: now_ms,
            wanted_name: None,
            last_name_ms: now_ms,
            game: None,
            host_start_at: None,
            start: None,
        }
    }

    /// Opens a lobby as its host, in the first seat.
    pub fn host(client_id: &str, name: &str, now_ms: i64) -> Session {
        let mut s = Session::blank(client_id, name, now_ms);
        s.is_host = true;
        s.status = Status::Lobby;
        s.humans = vec![Human {
            id: client_id.to_string(),
            name: s.name.clone(),
            ready: false,
        }];
        s
    }

    /// Joins a lobby; the Hello goes when the transport connects the host.
    pub fn join(client_id: &str, name: &str, now_ms: i64) -> Session {
        Session::blank(client_id, name, now_ms)
    }

    /// For tests: speaks another protocol version.
    #[doc(hidden)]
    pub fn with_protocol(mut self, proto: u32) -> Session {
        self.proto = proto;
        self
    }

    fn send(&mut self, to: &PeerId, msg: &Msg) {
        self.outbox.push(Outgoing {
            to: Some(to.clone()),
            text: msg.encode(),
        });
    }

    fn broadcast(&mut self, msg: &Msg) {
        self.outbox.push(Outgoing {
            to: None,
            text: msg.encode(),
        });
    }

    /// Everything to send since the last call.
    pub fn take_outbox(&mut self) -> Vec<Outgoing> {
        std::mem::take(&mut self.outbox)
    }

    pub fn is_host(&self) -> bool {
        self.is_host
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// The round: 0, then one more for each Rematch.
    pub fn round(&self) -> u32 {
        self.round
    }

    fn in_lobby(&self) -> bool {
        self.status == Status::Lobby
    }

    fn hello(&self) -> Msg {
        Msg::Hello {
            proto: self.proto,
            id: self.client_id.clone(),
            name: self.name.clone(),
        }
    }

    fn roster(&self) -> Msg {
        Msg::Roster {
            round: self.round,
            humans: self.humans.clone(),
            ai: self.ai_count,
        }
    }

    fn roster_changed(&mut self, now_ms: i64) {
        self.ai_count = self.ai_count.min(self.max_ai());
        self.last_roster_ms = now_ms;
        let roster = self.roster();
        self.broadcast(&roster);
    }

    fn max_ai(&self) -> usize {
        MAX_AI.min(MAX_CRABS.saturating_sub(self.humans.len()))
    }

    fn peer_of_client(&self, id: &str) -> Option<PeerId> {
        self.peers
            .iter()
            .find(|(_, p)| p.id.as_deref() == Some(id))
            .map(|(k, _)| k.clone())
    }

    // ---- transport events ----

    /// The transport connected a peer. `host` is what that peer said of
    /// itself (online.js's `connected` event).
    pub fn peer_connected(&mut self, peer: &str, host: bool, now_ms: i64) {
        self.peers.entry(peer.to_string()).or_default();
        if !self.is_host && host && self.host_peer.is_none() {
            self.host_peer = Some(peer.to_string());
            if self.status == Status::Joining {
                self.last_hello_ms = now_ms;
                let hello = self.hello();
                self.send(&peer.to_string(), &hello);
            }
        }
    }

    /// The transport lost a peer.
    pub fn peer_left(&mut self, peer: &str, now_ms: i64) {
        let client = self.peers.remove(peer).and_then(|p| p.id);
        if self.host_peer.as_deref() == Some(peer) {
            match self.status {
                Status::Joining | Status::Lobby => self.status = Status::Lost,
                Status::Playing => {
                    if let Some(g) = &mut self.game {
                        g.host_gone = true;
                        for (i, o) in g.owners.iter().enumerate() {
                            let host_owned = match o {
                                Owner::Ai => true,
                                Owner::Human(id) => Some(id) == client.as_ref() || i == 0,
                            };
                            if host_owned {
                                g.gone.insert(i);
                            }
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        if let Some(g) = &mut self.game {
            g.unacked.remove(peer);
        }
        let Some(client) = client else { return };
        // the same browser back on a new link: the old link closing is not
        // that player leaving
        if self.peer_of_client(&client).is_some() {
            return;
        }
        self.client_left(&client, now_ms);
    }

    fn client_left(&mut self, client: &str, now_ms: i64) {
        if self.status == Status::Playing {
            if let Some(g) = &mut self.game {
                if let Some(i) = g
                    .owners
                    .iter()
                    .position(|o| *o == Owner::Human(client.to_string()))
                {
                    g.gone.insert(i);
                }
            }
            return;
        }
        if self.is_host && self.in_lobby() {
            let before = self.humans.len();
            let me = self.client_id.clone();
            self.humans.retain(|h| h.id != client || h.id == me);
            if self.humans.len() != before {
                self.roster_changed(now_ms);
            }
        }
    }

    /// Handles one message from a peer. Unknown text is ignored.
    pub fn receive(&mut self, peer: &str, text: &str, now_ms: i64) {
        let Some(msg) = Msg::decode(text) else {
            return;
        };
        let peer = peer.to_string();
        self.peers.entry(peer.clone()).or_default().heard_ms = Some(now_ms);
        let from_host = self.host_peer.as_ref() == Some(&peer);
        if self.is_host {
            self.pass_on(&peer, &msg, text);
        }
        match msg {
            Msg::Ping { id, stamp, near } => {
                if let Some(p) = self.peers.get_mut(&peer) {
                    p.id = Some(id);
                    p.near = near.into_iter().collect();
                }
                self.send(&peer, &Msg::Pong { stamp });
            }
            Msg::Pong { stamp } => {
                if let Some(p) = self.peers.get_mut(&peer) {
                    p.rtts.push_back((now_ms - stamp).max(0));
                    while p.rtts.len() > RTT_KEPT {
                        p.rtts.pop_front();
                    }
                }
            }
            Msg::Hello { proto, id, name } if self.is_host => self.on_hello(&peer, proto, id, &name, now_ms),
            Msg::Ready { ready } if self.is_host => self.on_ready(&peer, ready, now_ms),
            Msg::Name { name } if self.is_host => self.on_name(&peer, &name, now_ms),
            Msg::Leave => {
                if let Some(client) = self.peers.get(&peer).and_then(|p| p.id.clone()) {
                    self.client_left(&client, now_ms);
                }
            }
            Msg::Started { round } if self.is_host => {
                if let Some(g) = &mut self.game {
                    if g.round == round {
                        g.unacked.remove(&peer);
                    }
                }
            }
            Msg::Refuse { reason } if from_host => {
                if self.status == Status::Joining {
                    self.status = Status::Refused { reason };
                }
            }
            Msg::Roster { round, humans, ai } if from_host => self.on_roster(round, humans, ai),
            Msg::Dissolve if from_host => {
                if matches!(self.status, Status::Joining | Status::Lobby) {
                    self.status = Status::Dissolved;
                }
            }
            Msg::Start {
                round,
                seed,
                humans,
                ai,
            } if from_host => {
                let waiting = matches!(self.status, Status::Lobby | Status::Joining);
                if waiting && humans.iter().any(|h| h.id == self.client_id) && round >= self.round {
                    self.round = round;
                    self.humans = humans;
                    self.ai_count = ai;
                    self.begin_game(seed, now_ms);
                }
                if self.game.as_ref().is_some_and(|g| g.round == round) {
                    self.send(&peer, &Msg::Started { round });
                }
            }
            Msg::State { r, i, k, s } => self.on_state(r, i, k, s, now_ms),
            Msg::End { r } if from_host => {
                if let Some(g) = &mut self.game {
                    if g.round == r {
                        g.ended = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// Host, during a game: passes a joiner's crab state on to the joiners
    /// that do not hear that joiner themselves (Ape Race's T71). A repeat
    /// is dropped by its receiver as already seen.
    fn pass_on(&mut self, from: &PeerId, msg: &Msg, text: &str) {
        if self.status != Status::Playing || !matches!(msg, Msg::State { .. }) {
            return;
        }
        let Some(sender) = self.peers.get(from).and_then(|p| p.id.clone()) else {
            return;
        };
        let to: Vec<PeerId> = self
            .peers
            .iter()
            .filter(|(k, p)| {
                *k != from && p.id.as_ref().is_some_and(|id| *id != sender) && !p.near.contains(&sender)
            })
            .map(|(k, _)| k.clone())
            .collect();
        for peer in to {
            self.outbox.push(Outgoing {
                to: Some(peer),
                text: text.to_string(),
            });
        }
    }

    /// The client ids this browser has heard from in the last [`NEAR_MS`].
    fn near(&self, now_ms: i64) -> Vec<String> {
        self.peers
            .values()
            .filter(|p| p.heard_ms.is_some_and(|t| now_ms - t <= NEAR_MS))
            .filter_map(|p| p.id.clone())
            .collect()
    }

    fn on_hello(&mut self, peer: &PeerId, proto: u32, id: String, name: &str, now_ms: i64) {
        if proto != PROTOCOL {
            let reason = Refusal::WrongVersion {
                host: PROTOCOL,
                mine: proto,
            };
            self.send(peer, &Msg::Refuse { reason });
            return;
        }
        // the newest link speaks for this browser
        for (k, p) in self.peers.iter_mut() {
            if k != peer && p.id.as_deref() == Some(id.as_str()) {
                p.id = None;
            }
        }
        if let Some(p) = self.peers.get_mut(peer) {
            p.id = Some(id.clone());
        }
        if self.status == Status::Playing {
            self.send(
                peer,
                &Msg::Refuse {
                    reason: Refusal::Started,
                },
            );
            return;
        }
        if !self.in_lobby() {
            return;
        }
        if self.humans.iter().any(|h| h.id == id) {
            let roster = self.roster();
            self.send(peer, &roster);
            return;
        }
        if self.humans.len() >= MAX_CRABS {
            self.send(
                peer,
                &Msg::Refuse {
                    reason: Refusal::Full,
                },
            );
            return;
        }
        self.humans.push(Human {
            id,
            name: clean_name(name),
            ready: false,
        });
        self.roster_changed(now_ms);
    }

    fn seat_of_peer(&self, peer: &PeerId) -> Option<usize> {
        let id = self.peers.get(peer)?.id.as_deref()?;
        self.humans.iter().position(|h| h.id == id)
    }

    fn on_ready(&mut self, peer: &PeerId, ready: bool, now_ms: i64) {
        if !self.in_lobby() {
            return;
        }
        match self.seat_of_peer(peer) {
            Some(i) if i != 0 && self.humans[i].ready != ready => {
                self.humans[i].ready = ready;
                self.roster_changed(now_ms);
            }
            _ => {}
        }
    }

    fn on_name(&mut self, peer: &PeerId, name: &str, now_ms: i64) {
        if !self.in_lobby() {
            return;
        }
        let name = clean_name(name);
        match self.seat_of_peer(peer) {
            Some(i) if i != 0 && !name.is_empty() && self.humans[i].name != name => {
                self.humans[i].name = name;
                self.roster_changed(now_ms);
            }
            _ => {}
        }
    }

    fn on_roster(&mut self, round: u32, humans: Vec<Human>, ai: usize) {
        if round < self.round {
            return;
        }
        match self.status {
            Status::Joining | Status::Lobby => {}
            // a Rematch: back to the lobby
            Status::Playing if round > self.round => {
                self.game = None;
                self.start = None;
                self.wanted_ready = None;
            }
            _ => return,
        }
        self.round = round;
        self.humans = humans;
        self.ai_count = ai;
        let seated = self.humans.iter().any(|h| h.id == self.client_id);
        self.status = if seated { Status::Lobby } else { Status::Joining };
    }

    fn on_state(&mut self, r: u32, i: usize, k: i64, s: CrabState, now_ms: i64) {
        if self.owns(i) {
            return;
        }
        let Some(g) = &mut self.game else { return };
        if g.round != r || i >= g.owners.len() {
            return;
        }
        let list = g.samples.entry(i).or_default();
        if list.back().is_some_and(|x| x.sent >= k) {
            return;
        }
        list.push_back(Sample {
            sent: k,
            got: now_ms,
            s,
        });
        while list.len() > STATES_KEPT {
            list.pop_front();
        }
    }

    // ---- the lobby ----

    /// Renames this browser's player. A joiner's new name is repeated until
    /// the roster shows it; an empty name is no change (the app gives its
    /// default name instead).
    pub fn set_name(&mut self, name: &str, now_ms: i64) {
        if !matches!(self.status, Status::Lobby | Status::Joining) {
            return;
        }
        let name = clean_name(name);
        if name.is_empty() || name == self.name {
            return;
        }
        self.name = name.clone();
        if self.is_host {
            self.humans[0].name = name;
            self.roster_changed(now_ms);
            return;
        }
        self.wanted_name = Some(name.clone());
        self.last_name_ms = now_ms;
        if let Some(host) = self.host_peer.clone() {
            if self.status == Status::Lobby {
                self.send(&host, &Msg::Name { name });
            }
        }
    }

    /// Joiner: Ready (or not). Repeated until the roster shows it. The host
    /// has no Ready: its Start is its Ready.
    pub fn set_ready(&mut self, ready: bool, now_ms: i64) {
        if self.is_host || self.status != Status::Lobby {
            return;
        }
        self.wanted_ready = Some(ready);
        self.last_ready_ms = now_ms;
        if let Some(host) = self.host_peer.clone() {
            self.send(&host, &Msg::Ready { ready });
        }
    }

    /// Host: the number of AI crabs, at most [`MAX_AI`] and four crabs in
    /// all. Ready marks stay; joiners see the change at once.
    pub fn set_ai_count(&mut self, count: usize, now_ms: i64) {
        if self.is_host && self.in_lobby() {
            self.ai_count = count.min(self.max_ai());
            self.roster_changed(now_ms);
        }
    }

    /// True when every seated person but the host is Ready.
    pub fn all_ready(&self) -> bool {
        self.humans.iter().skip(1).all(|h| h.ready)
    }

    fn crabs(&self) -> usize {
        self.humans.len() + self.ai_count
    }

    /// Whether the host's Start would be taken: every joiner Ready and at
    /// least two crabs.
    pub fn can_start(&self) -> bool {
        self.is_host && self.in_lobby() && self.all_ready() && self.crabs() >= 2
    }

    /// Host: starts the game with a seed the caller chose. Each joiner gets
    /// a Start (repeated until acknowledged); the host's own start follows
    /// once the Start has had time to arrive (see [`MAX_START_LEAD_MS`]).
    /// Refused, with nothing sent, unless [`Session::can_start`].
    pub fn start(&mut self, seed: u64, now_ms: i64) -> bool {
        if !self.can_start() {
            return false;
        }
        self.begin_game(seed, now_ms);
        let lead = self.peers.values().filter_map(|p| p.rtt_p90()).max().unwrap_or(0) / 2;
        self.host_start_at = Some(now_ms + lead.clamp(0, MAX_START_LEAD_MS));
        self.resend_start(now_ms);
        true
    }

    fn begin_game(&mut self, seed: u64, now_ms: i64) {
        let mut owners: Vec<Owner> = self.humans.iter().map(|h| Owner::Human(h.id.clone())).collect();
        let mut seats: Vec<SeatStart> = self
            .humans
            .iter()
            .map(|h| SeatStart {
                name: h.name.clone(),
                ai: false,
            })
            .collect();
        for _ in 0..self.ai_count {
            owners.push(Owner::Ai);
            seats.push(SeatStart {
                name: String::new(),
                ai: true,
            });
        }
        let local_index = self
            .humans
            .iter()
            .position(|h| h.id == self.client_id)
            .unwrap_or(0);
        let unacked: BTreeSet<PeerId> = if self.is_host {
            self.humans
                .iter()
                .skip(1)
                .filter_map(|h| self.peer_of_client(&h.id))
                .collect()
        } else {
            BTreeSet::new()
        };
        self.game = Some(GameState {
            round: self.round,
            seed,
            owners,
            unacked,
            last_start_ms: now_ms - RETRY_MS,
            last_sent: BTreeMap::new(),
            samples: BTreeMap::new(),
            gone: BTreeSet::new(),
            ended: false,
            last_end_ms: now_ms,
            host_gone: false,
            began_ms: now_ms,
        });
        self.start = Some(GameStart {
            seed,
            round: self.round,
            seats,
            local_index,
        });
        self.status = Status::Playing;
        self.wanted_ready = None;
    }

    fn resend_start(&mut self, now_ms: i64) {
        let Some(g) = &mut self.game else { return };
        g.last_start_ms = now_ms;
        let msg = Msg::Start {
            round: g.round,
            seed: g.seed,
            humans: self.humans.clone(),
            ai: self.ai_count,
        };
        let to: Vec<PeerId> = g.unacked.iter().cloned().collect();
        for peer in to {
            self.send(&peer, &msg);
        }
    }

    /// Once the game starts: what this browser needs to set it up. Given
    /// once per game. (On the host it waits for the Start's lead.)
    pub fn take_start(&mut self) -> Option<GameStart> {
        if self.host_start_at.is_some() {
            return None;
        }
        self.start.take()
    }

    /// Leaves. On the host this closes the lobby (or, in a game, ends the
    /// session for everyone). The page then closes the links.
    pub fn leave(&mut self) {
        if self.status == Status::Left {
            return;
        }
        if self.is_host {
            self.broadcast(&Msg::Dissolve);
        } else if let Some(host) = self.host_peer.clone() {
            self.send(&host, &Msg::Leave);
        }
        self.status = Status::Left;
    }

    /// Host, during or after a game: back to the lobby with the same seats
    /// (players who left are gone). Everyone who stayed comes back Ready and
    /// may take it back; a player who joins between rounds starts not ready.
    pub fn rematch(&mut self, now_ms: i64) -> bool {
        if !self.is_host || self.status != Status::Playing {
            return false;
        }
        let connected: BTreeSet<String> = self.peers.values().filter_map(|p| p.id.clone()).collect();
        let me = self.client_id.clone();
        self.humans.retain(|h| h.id == me || connected.contains(&h.id));
        // Ready marks are not cleared by a game, so those who stayed are
        // still ready (SECOND-ORDER R1.46)
        self.round += 1;
        self.game = None;
        self.start = None;
        self.host_start_at = None;
        self.status = Status::Lobby;
        self.roster_changed(now_ms);
        true
    }

    /// Repeats what may have been lost, and pings. Call every frame.
    pub fn update(&mut self, now_ms: i64) {
        if matches!(
            self.status,
            Status::Left | Status::Dissolved | Status::Lost | Status::Refused { .. }
        ) {
            return;
        }
        if self.host_start_at.is_some_and(|at| now_ms >= at) {
            self.host_start_at = None;
        }
        if self.is_host {
            if self.in_lobby() && now_ms - self.last_roster_ms >= ROSTER_EVERY_MS {
                self.last_roster_ms = now_ms;
                let roster = self.roster();
                self.broadcast(&roster);
            }
            let due = self.game.as_ref().map(|g| {
                (
                    !g.unacked.is_empty() && now_ms - g.last_start_ms >= RETRY_MS,
                    g.ended && now_ms - g.last_end_ms >= ROSTER_EVERY_MS,
                )
            });
            if let Some((start, end)) = due {
                if start {
                    self.resend_start(now_ms);
                }
                if end {
                    self.send_end(now_ms);
                }
            }
        } else if let Some(host) = self.host_peer.clone() {
            self.joiner_retries(&host, now_ms);
        }
        if now_ms - self.last_ping_ms >= PING_EVERY_MS {
            self.last_ping_ms = now_ms;
            let ping = Msg::Ping {
                id: self.client_id.clone(),
                stamp: now_ms,
                near: self.near(now_ms),
            };
            self.broadcast(&ping);
        }
    }

    fn joiner_retries(&mut self, host: &PeerId, now_ms: i64) {
        if self.status == Status::Joining && now_ms - self.last_hello_ms >= RETRY_MS {
            self.last_hello_ms = now_ms;
            let hello = self.hello();
            self.send(host, &hello);
        }
        if self.status != Status::Lobby {
            return;
        }
        let mine = self.humans.iter().find(|h| h.id == self.client_id).cloned();
        let Some(me) = mine else { return };
        if let Some(name) = self.wanted_name.clone() {
            if me.name == name {
                self.wanted_name = None;
            } else if now_ms - self.last_name_ms >= RETRY_MS {
                self.last_name_ms = now_ms;
                self.send(host, &Msg::Name { name });
            }
        }
        if let Some(ready) = self.wanted_ready {
            if me.ready == ready {
                self.wanted_ready = None;
            } else if now_ms - self.last_ready_ms >= RETRY_MS {
                self.last_ready_ms = now_ms;
                self.send(host, &Msg::Ready { ready });
            }
        }
    }

    // ---- the lobby view ----

    fn ping_of(&self, id: &str) -> Option<i64> {
        let peer = self.peer_of_client(id)?;
        self.peers.get(&peer)?.rtt_p90()
    }

    /// The lobby as the screen draws it: people in seat order, then the AI
    /// crabs.
    pub fn lobby(&self) -> LobbyView {
        let mut seats: Vec<SeatView> = self
            .humans
            .iter()
            .enumerate()
            .map(|(i, h)| SeatView {
                name: h.name.clone(),
                ai: false,
                host: i == 0,
                me: h.id == self.client_id,
                ready: h.ready,
                ping_ms: if h.id == self.client_id {
                    None
                } else {
                    self.ping_of(&h.id)
                },
            })
            .collect();
        for _ in 0..self.ai_count {
            seats.push(SeatView {
                name: String::new(),
                ai: true,
                host: false,
                me: false,
                ready: true,
                ping_ms: None,
            });
        }
        let ready = self
            .wanted_ready
            .unwrap_or_else(|| self.humans.iter().any(|h| h.id == self.client_id && h.ready));
        LobbyView {
            status: self.status.clone(),
            is_host: self.is_host,
            seats,
            ai_count: self.ai_count,
            max_ai: self.max_ai(),
            ready,
            can_start: self.can_start(),
        }
    }

    /// Whether this browser is seated (a joiner may still be waiting).
    pub fn seated(&self) -> bool {
        self.humans.iter().any(|h| h.id == self.client_id)
    }

    // ---- the game ----

    /// True when this browser moves crab `index`: its own, and on the host
    /// the AI crabs.
    pub fn owns(&self, index: usize) -> bool {
        let Some(g) = &self.game else {
            return false;
        };
        match g.owners.get(index) {
            Some(Owner::Human(id)) => *id == self.client_id,
            Some(Owner::Ai) => self.is_host,
            None => false,
        }
    }

    /// Crabs whose owner has left the game.
    pub fn gone(&self) -> Vec<usize> {
        self.game
            .as_ref()
            .map(|g| g.gone.iter().copied().collect())
            .unwrap_or_default()
    }

    /// The other browsers' crabs whose states have stopped coming for
    /// [`STALL_MS`] (or never came), while the game is on: drawn where
    /// last seen, and the HUD says so (Ape Race's T71) instead of the crab
    /// freezing silently. A crab whose owner left is not stalled, and none
    /// is once the game is over.
    pub fn stalled(&self, now_ms: i64) -> Vec<usize> {
        let Some(g) = &self.game else {
            return Vec::new();
        };
        if g.ended || g.host_gone {
            return Vec::new();
        }
        (0..g.owners.len())
            .filter(|i| !self.owns(*i) && !g.gone.contains(i))
            .filter(|i| {
                let last = g
                    .samples
                    .get(i)
                    .and_then(|l| l.back())
                    .map_or(g.began_ms, |s| s.got);
                now_ms - last > STALL_MS
            })
            .collect()
    }

    /// True on a joiner whose host has left mid-game.
    pub fn host_gone(&self) -> bool {
        self.game.as_ref().is_some_and(|g| g.host_gone)
    }

    /// Sends the state of crab `index` (one this browser owns) at most every
    /// [`STATE_EVERY_MS`]; `game_ms` is how far into the game this browser
    /// is.
    pub fn send_state(&mut self, now_ms: i64, game_ms: i64, index: usize, s: CrabState) {
        if !self.owns(index) {
            return;
        }
        let Some(g) = &mut self.game else { return };
        if g.last_sent
            .get(&index)
            .is_some_and(|t| now_ms - t < STATE_EVERY_MS)
        {
            return;
        }
        g.last_sent.insert(index, now_ms);
        let msg = Msg::State {
            r: g.round,
            i: index,
            k: game_ms,
            s,
        };
        self.broadcast(&msg);
    }

    /// Where each other browser's crab should be drawn now: about
    /// [`INTERP_DELAY_MS`] behind its owner, interpolated between states,
    /// or carried along its velocity for at most [`EXTRAPOLATE_MS`] when
    /// states are late. A crab whose owner left stands where it was last
    /// seen.
    pub fn remote_states(&self, now_ms: i64) -> Vec<RemoteCrab> {
        let Some(g) = &self.game else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (index, list) in &g.samples {
            if self.owns(*index) || list.is_empty() {
                continue;
            }
            let newest = list[list.len() - 1];
            if g.gone.contains(index) {
                let mut s = newest.s;
                s.v = [0, 0];
                out.push(RemoteCrab {
                    index: *index,
                    state: s,
                });
                continue;
            }
            // the quickest delivery seen maps the owner's clock onto ours
            let offset = list.iter().map(|x| x.got - x.sent).min().unwrap_or(0);
            let target = now_ms - offset - INTERP_DELAY_MS;
            out.push(RemoteCrab {
                index: *index,
                state: sample_at(list, target),
            });
        }
        out
    }

    /// Host: its clock ran out. Every joiner hears the End, repeated until
    /// the next round.
    pub fn end_game(&mut self, now_ms: i64) {
        if !self.is_host {
            return;
        }
        let first = self
            .game
            .as_mut()
            .is_some_and(|g| !std::mem::replace(&mut g.ended, true));
        if first {
            self.send_end(now_ms);
        }
    }

    fn send_end(&mut self, now_ms: i64) {
        let Some(g) = &mut self.game else { return };
        g.last_end_ms = now_ms;
        let msg = Msg::End { r: g.round };
        self.broadcast(&msg);
    }

    /// The game is over: the host's clock ran out, or the host left.
    pub fn ended(&self) -> bool {
        self.game.as_ref().is_some_and(|g| g.ended || g.host_gone)
    }
}

/// The angle from `a` to `b` the short way round (angle units).
fn turn_between(a: i32, b: i32) -> i32 {
    (b - a + 32768).rem_euclid(65536) - 32768
}

/// A crab at its owner's moment `target` (ms of its game).
pub(crate) fn sample_at(list: &VecDeque<Sample>, target: i64) -> CrabState {
    let first = list[0];
    if target <= first.sent {
        return first.s;
    }
    for k in 1..list.len() {
        let (a, b) = (list[k - 1], list[k]);
        if target <= b.sent {
            let span = (b.sent - a.sent).max(1);
            let t = target - a.sent;
            let lerp = |x: i32, y: i32| x + ((y - x) as i64 * t / span) as i32;
            // the air mark and the gait from the nearer state
            let near = if 2 * t >= span { b.s } else { a.s };
            return CrabState {
                p: [lerp(a.s.p[0], b.s.p[0]), lerp(a.s.p[1], b.s.p[1])],
                v: [lerp(a.s.v[0], b.s.v[0]), lerp(a.s.v[1], b.s.v[1])],
                y: (a.s.y + (turn_between(a.s.y, b.s.y) as i64 * t / span) as i32).rem_euclid(65536),
                h: lerp(a.s.h, b.s.h),
                vh: lerp(a.s.vh, b.s.vh),
                ..near
            };
        }
    }
    let last = list[list.len() - 1];
    let ahead = (target - last.sent).min(EXTRAPOLATE_MS);
    let step = |p: i32, v: i32| p + (v as i64 * ahead / 1000) as i32;
    CrabState {
        p: [step(last.s.p[0], last.s.v[0]), step(last.s.p[1], last.s.v[1])],
        ..last.s
    }
}
