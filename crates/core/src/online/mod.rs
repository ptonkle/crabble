//! Online play for up to four browsers (DECISIONS.md, 2026-10-19 online
//! sync): the lobby, the start, each crab's state and the end. A pure state
//! machine after Ape Race's `online.rs`: text comes in with
//! [`Session::receive`], text goes out through [`Session::take_outbox`], and
//! time comes in as `now_ms` arguments. The transport is `web/online.js`
//! (WebRTC introduced by a code over pinned Nostr relays, or by pasted
//! codes; copied from Ape Race and Honey Hole): it only carries the text.
//!
//! # Crabs: each browser moves its own
//!
//! Each browser moves its own crab with no delay and sends that crab's
//! state ([`CrabState`]) about 20 times a second ([`STATE_EVERY_MS`]). The
//! host also runs the AI crabs and sends theirs. Every other crab is drawn
//! where its owner said it was about [`INTERP_DELAY_MS`] ago, interpolated
//! between two states, or carried on along its velocity for at most
//! [`EXTRAPOLATE_MS`] when states are late. The host passes a joiner's
//! states on to any joiner that does not hear that joiner directly (Ape
//! Race's T71: with STUN only, two joiners may find no path to each other
//! while both reach the host).
//!
//! # Tiles: the host's (not built yet)
//!
//! Tiles will be host-authoritative: a grab or a placement is a request
//! the host settles, first come first served, and every browser shows the
//! host's answer (DECISIONS.md, 2026-10-19 online sync). Their messages
//! and the session's hooks go in [`tiles`]; nothing there is built yet.
//!
//! # Roles
//!
//! The **host** owns the lobby: who sits where, the AI count (never more
//! than [`MAX_CRABS`] crabs in all) and each joiner's Ready mark. It sends
//! the roster every second, since a lobby message may be lost. A
//! **joiner** repeats its Hello until seated (or refused: a full lobby, a
//! game under way, or another [`PROTOCOL`]), and repeats a new name or a
//! Ready mark until the roster shows it. Everyone pings everyone every half
//! second; the round trips appear in the lobby.
//!
//! The host's Start is repeated to each joiner until it is acknowledged.
//! Every browser builds the same seats (humans in lobby order, then the AI
//! crabs), so crab `i` is the same crab everywhere and takes the `i`th
//! colour (red, yellow, blue, purple). The host's clock ends the game: its
//! End is repeated until the next round. A **Rematch** (host only) goes
//! back to the lobby with the same seats; everyone who stayed comes back
//! ready. Each Rematch is a new *round*, and a game message of an old round
//! is ignored.
//!
//! # For the app (`app/online_play.rs`)
//!
//! 1. **Open:** [`Session::host`] or [`Session::join`]; the page gets the
//!    transport room ([`room_name`]) or the pasted-code requests.
//! 2. **Transport events:** `connected` → [`Session::peer_connected`],
//!    `message` → [`Session::receive`], `disconnected` →
//!    [`Session::peer_left`].
//! 3. **Every frame:** [`Session::update`], then send [`Session::take_outbox`].
//! 4. **Lobby:** [`Session::lobby`] to draw; [`Session::set_name`],
//!    [`Session::set_ready`], and on the host [`Session::set_ai_count`] and
//!    [`Session::start`].
//! 5. **Start:** [`Session::take_start`] gives the [`GameStart`] once.
//! 6. **In the game, every frame:** [`Session::send_state`] for each crab
//!    this browser [`Session::owns`], then place the others from
//!    [`Session::remote_states`]. The host calls [`Session::end_game`] when
//!    its clock runs out; a joiner's game ends when [`Session::ended`].
//! 7. **Rematch / leave:** the host's [`Session::rematch`]; joiners follow
//!    the roster back to the lobby. [`Session::leave`], send the outbox,
//!    then close the links.

mod msg;
mod session;
pub mod tiles;

#[cfg(test)]
mod tests;

pub use msg::{CrabState, Human, Refusal};
pub use session::{GameStart, LobbyView, Outgoing, RemoteCrab, SeatStart, SeatView, Session, Status};

use crate::rng::Dice;

/// The protocol version. Different versions never meet: the transport room
/// and the Hello carry it.
pub const PROTOCOL: u32 = 1;
/// The most crabs in a game, people and AI crabs together.
pub const MAX_CRABS: usize = 4;
/// The most AI crabs the host may add.
pub const MAX_AI: usize = 3;
/// How often a joiner repeats a Hello, a name or a Ready mark, and the host
/// a Start, until it is seen.
pub const RETRY_MS: i64 = 500;
/// How often the host sends the roster, and its End.
pub const ROSTER_EVERY_MS: i64 = 1000;
/// How often everyone pings everyone.
pub const PING_EVERY_MS: i64 = 500;
/// A peer heard from this recently counts as reachable: it goes in this
/// browser's `near` list. Three pings.
pub const NEAR_MS: i64 = 1500;
/// How often a crab's state is sent: 20 times a second.
pub const STATE_EVERY_MS: i64 = 50;
/// How far behind its owner another browser's crab is drawn.
pub const INTERP_DELAY_MS: i64 = 100;
/// How long a late crab is carried on along its velocity.
pub const EXTRAPOLATE_MS: i64 = 200;
/// A crab whose states have not come for this long is stalled: the HUD
/// names its player (Ape Race's).
pub const STALL_MS: i64 = 1500;
/// The most the host waits for its Start to reach the joiners before its own
/// game begins (half the slowest round trip, at most this).
pub const MAX_START_LEAD_MS: i64 = 250;
/// The longest player name, in characters.
pub const MAX_NAME_CHARS: usize = 16;
/// A code's length and letters: no 0, O, 1 or I.
pub const CODE_LEN: usize = 6;
pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The transport's name for another browser (online.js's peer id).
pub type PeerId = String;

/// A code from a seed the caller chose: six letters and digits, with none of
/// 0, O, 1 or I.
pub fn room_code(seed: u64) -> String {
    let mut dice = Dice(seed);
    (0..CODE_LEN)
        .map(|_| CODE_ALPHABET[dice.below(CODE_ALPHABET.len() as u32) as usize] as char)
        .collect()
}

/// A code as a player typed it: case, spaces and dashes are forgiven.
/// `None` when it is not six code letters.
pub fn clean_code(text: &str) -> Option<String> {
    let code: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let ok = code.len() == CODE_LEN && code.bytes().all(|b| CODE_ALPHABET.contains(&b));
    ok.then_some(code)
}

/// The transport room for a code: the protocol version keeps browsers of
/// different versions apart.
pub fn room_name(code: &str) -> String {
    format!("crabble-p{PROTOCOL}-{code}")
}

/// A player name as the lobby shows it: printable, trimmed, at most
/// [`MAX_NAME_CHARS`] characters.
pub fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_NAME_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}
