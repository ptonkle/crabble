//! Every message between browsers, as small JSON (`t` names the kind).

use serde::{Deserialize, Serialize};

use super::tiles::{Entry, WireClaim};

/// A person's seat in the lobby.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Human {
    /// the browser's client id (stable for its tab)
    pub id: String,
    pub name: String,
    /// Ready. The host's own seat is never marked: its Start is its Ready.
    pub ready: bool,
}

/// Why the host refused a joiner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Refusal {
    /// The host runs another protocol version.
    WrongVersion { host: u32, mine: u32 },
    /// Four people are seated already.
    Full,
    /// The game has started.
    Started,
}

/// One crab's state as its owner sends it: where it is on the sand (x, z
/// in millimetres), how fast it is going (millimetres a second), which
/// way it faces (angle units, 65536 a turn), its feet's height and up
/// speed (mm, mm a second), whether it is in the air, and its gait for its
/// legs ([`crate::sim::Gait::code`]). The height and the air mark make
/// another browser's jump show (SECOND-ORDER R5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrabState {
    pub p: [i32; 2],
    pub v: [i32; 2],
    pub y: i32,
    #[serde(default)]
    pub h: i32,
    #[serde(default)]
    pub vh: i32,
    #[serde(default)]
    pub a: bool,
    #[serde(default)]
    pub g: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub(crate) enum Msg {
    Hello {
        proto: u32,
        id: String,
        name: String,
    },
    Refuse {
        reason: Refusal,
    },
    Roster {
        round: u32,
        humans: Vec<Human>,
        ai: usize,
    },
    Ready {
        ready: bool,
    },
    Name {
        name: String,
    },
    Leave,
    Dissolve,
    Ping {
        id: String,
        stamp: i64,
        /// the client ids this browser hears directly: the host passes game
        /// messages on to a joiner that does not hear their sender
        #[serde(default)]
        near: Vec<String>,
    },
    Pong {
        stamp: i64,
    },
    Start {
        round: u32,
        seed: u64,
        humans: Vec<Human>,
        ai: usize,
    },
    Started {
        round: u32,
    },
    /// Crab `i`'s state in round `r`, `k` ms into its owner's game.
    State {
        r: u32,
        i: usize,
        k: i64,
        s: CrabState,
    },
    /// The host's clock ran out in round `r`, after `n` log entries; `pen`
    /// is each seat's idle penalty (`tiles.rs`).
    End {
        r: u32,
        #[serde(default)]
        n: u64,
        #[serde(default)]
        pen: Vec<i32>,
    },
    // ---- tiles (`tiles.rs`) ----
    /// A joiner's claim, its `n`th, for the host's referee.
    Claim {
        r: u32,
        n: u32,
        c: WireClaim,
    },
    /// The host's log entries from number `from` on.
    Log {
        r: u32,
        from: u64,
        e: Vec<Entry>,
    },
    /// A joiner needs log entry `n` next.
    Have {
        r: u32,
        n: u64,
    },
}

impl Msg {
    pub(crate) fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub(crate) fn decode(text: &str) -> Option<Msg> {
        serde_json::from_str(text).ok()
    }
}
