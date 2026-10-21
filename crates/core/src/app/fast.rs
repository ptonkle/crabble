//! The page's test flags, never set for players:
//!
//! - **`?fast`** (`flag.fast`): skips the waits that only make a person
//!   wait, and leaves every rule's timing alone. Today that is the 3-2-1
//!   countdown before a game (`standin::Setup::fast`); a later wait (the
//!   tally's reveal, say) reads `App::fast` too. The browser checks open the
//!   page with it (`tests/browser/interactions.py`, `online_game.py`).
//! - **`?short`** (`flag.short`): a game of [`SHORT_GAME_MS`] instead of
//!   `game.minutes` (or `?short=N`, N seconds), so the browser checks reach
//!   the end screen.
//! - **`flag.seed`**: the page's seed (from `crypto.getRandomValues`),
//!   mixed into each game's and each code's.
//! - **`?spot=x,z,deg`** (`flag.spot`): the local crab starts at (x, z) mm
//!   facing `deg` degrees, for the stills and checks of grabbing and
//!   placing. `?spot=tile` instead stands it beside the loose tile nearest
//!   its corner, the crosshair on it and in reach (the online check's
//!   grabs; [`face_a_tile`]). `?spot=board` hands it a loose tile and
//!   stands it by the board, the crosshair on the square beside the centre
//!   tile, in reach (the check that placing leaves the centre letter be,
//!   SECOND-ORDER R21.1; [`face_the_centre`]).
//! - **`?cam=deg,pitch,mm`** (`flag.cam`): the stills' camera. The main
//!   view looks at the local crab from `mm` away, `pitch` degrees above it,
//!   `deg` degrees round from behind it toward its left (90: its right
//!   side; 180: its front), so a still shows a crab's legs from the side
//!   and 3/4 (TRIAGE F1, F12). The crosshair still aims as the game does.

use super::App;
use crate::gfx::geom::{v3, yaw_of};
use crate::model::mm1;
use crate::sim::{Match, Target};

/// `?spot=tile`: stands crab `i` beside the loose tile nearest it, clear
/// of every tile (so it nudges none away), facing it with the crosshair on
/// a tile in reach. False when no place found.
pub(crate) fn face_a_tile(g: &mut Match, i: usize) -> bool {
    let start = g.crabs[i].pos;
    let mut tiles: Vec<_> = g
        .tiles
        .all
        .iter()
        .filter(|t| t.is_loose())
        .map(|t| (t.pose.t.sub(start).flat().len(), t.pose.t))
        .collect();
    tiles.sort_by_key(|t| t.0);
    let (lo, hi) = g.rules.pitch_range;
    for (_, at) in tiles.into_iter().take(12) {
        for side in 0..8 {
            let dir = crate::gfx::geom::facing(side * 8192, 0);
            for back in (150..=450).step_by(25) {
                let p = v3(at.x, 0, at.z).sub(dir.scale(mm1(back)));
                let p = v3(p.x, g.support(p, None), p.z);
                // a crab standing into a tile pushes it away (R8.9, R16.7),
                // and the crosshair with it: stand clear of every tile
                let r = &g.rules;
                let mut tiles = g.tiles.clone();
                if !tiles
                    .nudge(p, r.radius, r.step, r.nudge_pct, &r.physics, r.tick_ms as i32)
                    .is_empty()
                {
                    continue;
                }
                for k in 0..=16 {
                    let c = &mut g.crabs[i];
                    c.pos = p;
                    c.yaw = yaw_of(at.sub(p).flat());
                    c.pitch = lo + (hi - lo) * k / 16;
                    let aim = g.aim(i);
                    if matches!(aim.target, Target::Tile(_)) && aim.in_reach {
                        return true;
                    }
                }
            }
        }
    }
    g.crabs[i].pos = start;
    false
}

/// `?spot=board`: hands crab `i` the first loose tile (a grab claim the
/// first tick grants) and stands it by the board, the crosshair on the
/// square on the centre's +column side, in reach and legal. False when no
/// place found.
pub(crate) fn face_the_centre(g: &mut Match, i: usize) -> bool {
    let Some(tile) = g.tiles.all.iter().find(|t| t.is_loose()).map(|t| t.id) else {
        return false;
    };
    g.submit_claim(crate::sim::Claim::Grab {
        seat: i,
        claw: crate::sim::tiles::LEFT,
        tile,
    });
    let n = g.rules.board_size;
    let sq = (n / 2, n / 2 + 1);
    let (lo, hi) = g.rules.pitch_range;
    let start = (g.crabs[i].pos, g.crabs[i].yaw, g.crabs[i].pitch);
    for z in (-900..=900).step_by(50) {
        for x in (-300..=300).step_by(100) {
            let at = v3(mm1(x), 0, mm1(z));
            let p = v3(at.x, g.support(at, None), at.z);
            for yaw in (0..crate::gfx::fx::TURN).step_by(2048) {
                for k in 0..=32 {
                    let c = &mut g.crabs[i];
                    c.pos = p;
                    c.yaw = yaw;
                    c.pitch = lo + (hi - lo) * k / 32;
                    let aim = g.aim(i);
                    if aim.target == Target::Square(sq) && aim.in_reach && g.legal_square(sq) {
                        return true;
                    }
                }
            }
        }
    }
    let c = &mut g.crabs[i];
    (c.pos, c.yaw, c.pitch) = start;
    false
}

/// A `?short` game's length.
pub const SHORT_GAME_MS: i64 = 3000;

impl App {
    pub(super) fn test_flag(&mut self, name: &str, value: &str) -> Result<(), String> {
        let on = value != "off";
        match name {
            "flag.fast" => self.fast = on,
            "flag.spot" if value == "tile" => self.spot_tile = on,
            "flag.spot" if value == "board" => self.spot_board = on,
            "flag.spot" => {
                let v: Vec<i32> = value.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                self.spot = match v[..] {
                    [x, z, yaw] => Some((x, z, yaw)),
                    _ => return Err(format!("The spot \"{value}\" is not x,z,degrees.")),
                }
            }
            "flag.cam" => {
                let v: Vec<i32> = value.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                self.view.still_cam = match v[..] {
                    [yaw, pitch, mm] if mm > 0 => Some((yaw, pitch, mm)),
                    _ => return Err(format!("The camera \"{value}\" is not degrees,degrees,mm.")),
                }
            }
            "flag.short" => {
                self.short = match value.parse::<i64>() {
                    Ok(s) if s > 0 => Some(s * 1000),
                    _ if on => Some(SHORT_GAME_MS),
                    _ => None,
                }
            }
            _ => {
                self.seed = value
                    .parse()
                    .map_err(|_| format!("The seed \"{value}\" is not a whole number."))?
            }
        }
        Ok(())
    }
}
