//! Tile sync: the place for it. **Not built yet.**
//!
//! The game's tiles will be the host's (DECISIONS.md, 2026-10-19 online
//! sync; `CLAUDE.md`, "Online tiles are the host's"). The plan, to be
//! written into `PLAN.md` before any of it is built:
//!
//! - A browser asks; the host decides. A grab (`Grab { tile, claw }`) or a
//!   placement (`Place { tile, square }`) is a request sent to the host,
//!   which settles who was first (by its own arrival order) and answers
//!   everyone with a verdict (`Granted` / `Refused`). The asker shows
//!   nothing until the verdict comes; offline, the verdict is immediate,
//!   so one code path serves both.
//! - The host also sends where loose tiles come to rest (a pile that
//!   tumbles), since the piles' physics runs on the host alone.
//! - Every message carries the round (`r`), like the crabs' states, and a
//!   request is repeated until its verdict is heard.
//!
//! When it is built: add the message variants to `msg::Msg` beside the
//! comment there, handle them in `Session::receive`, and give a joiner's
//! request a retry in `Session::update` as the Hello has.
