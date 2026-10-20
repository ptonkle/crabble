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
//!   grabs; [`face_a_tile`]).
//! - **`?cam=deg,pitch,mm`** (`flag.cam`): the stills' camera. The main
//!   view looks at the local crab from `mm` away, `pitch` degrees above it,
//!   `deg` degrees round from behind it toward its left (90: its right
//!   side; 180: its front), so a still shows a crab's legs from the side
//!   and 3/4 (TRIAGE F1, F12). The crosshair still aims as the game does.

use super::App;
use crate::gfx::geom::{v3, yaw_of};
use crate::model::mm1;
use crate::sim::{Match, Target};

/// `?spot=tile`: stands crab `i` beside the loose tile nearest it, facing
/// it with the crosshair on a tile in reach. False when no place found.
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

/// A `?short` game's length.
pub const SHORT_GAME_MS: i64 = 3000;

impl App {
    pub(super) fn test_flag(&mut self, name: &str, value: &str) -> Result<(), String> {
        let on = value != "off";
        match name {
            "flag.fast" => self.fast = on,
            "flag.spot" if value == "tile" => self.spot_tile = on,
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
