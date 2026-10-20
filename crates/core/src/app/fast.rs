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
//!   placing.

use super::App;

/// A `?short` game's length.
pub const SHORT_GAME_MS: i64 = 3000;

impl App {
    pub(super) fn test_flag(&mut self, name: &str, value: &str) -> Result<(), String> {
        let on = value != "off";
        match name {
            "flag.fast" => self.fast = on,
            "flag.spot" => {
                let v: Vec<i32> = value.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                self.spot = match v[..] {
                    [x, z, yaw] => Some((x, z, yaw)),
                    _ => return Err(format!("The spot \"{value}\" is not x,z,degrees.")),
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
