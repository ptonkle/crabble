//! Play: the stand-in game (`standin`), its HUD (the clock and the 3-2-1),
//! the pause menu on Escape and the end. A Local game pauses under the
//! menu; an online game runs on behind it (Ape Race's pause).

use serde::Serialize;

use super::*;
use crate::rng::mix;
use crate::standin::{Driver, Game, Phase, Setup};

/// How far the local crab must go from its start to count as moved (for
/// the browser check).
const MOVED_MM: i64 = 100;

/// The sounds the stand-in names: each tick of the 3-2-1, the start and
/// the end (`data/sounds.json`).
pub const CUES: [&str; 3] = ["countdown", "go", "end"];

/// A sound for the page to play (`web/audio.js`): its key in
/// `data/sounds.json`, the volume setting, and a seed for its small
/// variation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cue {
    pub key: String,
    pub volume: u32,
    pub seed: u32,
}

/// `data.sounds`: the sounds file holds every sound the core names.
pub fn check_cues(sounds_json: &str) -> Result<(), String> {
    let bank = crate::sound::Bank::parse(sounds_json)?;
    match CUES.iter().find(|k| !bank.cues.iter().any(|c| c.id == **k)) {
        Some(k) => Err(format!("The sounds file has no \"{k}\".")),
        None => Ok(()),
    }
}

pub struct Play {
    pub game: Game,
    /// the phase last heard, for the sounds of its changes
    heard: Option<Phase>,
    pub paused: bool,
    pub online: bool,
    /// where each crab started
    pub(crate) starts: Vec<(i32, i32)>,
}

impl Play {
    /// The page keeps the keys and the mouse: the game is on, with no menu
    /// over it.
    pub fn captures(&self) -> bool {
        !self.paused && self.game.phase() != Phase::Ended
    }

    pub fn local_moved(&self) -> bool {
        let start = self.starts.get(self.game.local).copied().unwrap_or((0, 0));
        self.game.crabs.get(self.game.local).is_some_and(|c| {
            let (dx, dz) = ((c.x - start.0) as i64, (c.z - start.1) as i64);
            dx * dx + dz * dz >= MOVED_MM * MOVED_MM
        })
    }
}

/// A clock as minutes and seconds, rounded up: "4:00", "0:07".
pub fn clock_text(ms_left: i64) -> String {
    let s = (ms_left.max(0) + 999) / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

impl App {
    /// Sets up a game and opens Play.
    pub(crate) fn begin_game(
        &mut self,
        drivers: Vec<Driver>,
        local: usize,
        seed: u64,
        online: bool,
    ) -> Result<(), String> {
        let t = self.tuning.as_ref().ok_or("The tuning file is not loaded yet.")?;
        let mut game = Game::new(
            t,
            Setup {
                drivers,
                local,
                seed,
                fast: self.fast,
                length_ms: self.short,
            },
        );
        // a joiner's game ends when the host's does
        game.hold_at_zero = online && !self.online.hosting();
        let starts = game.crabs.iter().map(|c| (c.x, c.z)).collect();
        self.play = Some(Play {
            game,
            heard: None,
            paused: false,
            online,
            starts,
        });
        self.go(Screen::Play);
        Ok(())
    }

    pub(super) fn start_local(&mut self) -> Result<(), String> {
        let mut drivers = vec![Driver::Local];
        drivers.extend(std::iter::repeat_n(Driver::Ai, self.local_ai));
        let seed = mix(self.seed ^ self.clock as u64);
        self.begin_game(drivers, 0, seed, false)
    }

    /// Keys and the mouse during a game. True when the input was the
    /// game's.
    pub(super) fn play_input(&mut self, name: &str, value: &str) -> bool {
        let (sensitivity, invert) = (self.settings.mouse_pct, self.settings.invert_y);
        if self.screen != Screen::Play {
            return false;
        }
        let Some(p) = self.play.as_mut() else {
            return false;
        };
        let on = p.captures();
        match name {
            "key" if on => p.game.keys.set(value, true),
            "key.up" => p.game.keys.set(value, false),
            "keys.release" => {
                p.game.keys = Default::default();
                true
            }
            "mouse.move" if on => {
                let mut it = value.split(',').map(|n| n.trim().parse::<i32>().unwrap_or(0));
                let (dx, dy) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
                p.game.look(dx, dy, sensitivity, invert);
                true
            }
            // the browser let the pointer go (its own Escape): the menu
            "pointer.released" if on => {
                p.paused = true;
                p.game.keys = Default::default();
                true
            }
            _ => false,
        }
    }

    pub(super) fn toggle_pause(&mut self) {
        if let Some(p) = self.play.as_mut() {
            if p.game.phase() != Phase::Ended {
                p.paused = !p.paused;
                p.game.keys = Default::default();
            }
        }
    }

    /// The pause menu's and the end's buttons.
    pub(super) fn play_act(&mut self, name: &str) -> Result<(), String> {
        match name {
            "menu.resume" => {
                if let Some(p) = self.play.as_mut() {
                    p.paused = false;
                }
            }
            "menu.quit" | "end.title" => {
                self.leave_online();
                self.play = None;
                self.go(Screen::Title);
            }
            "end.rematch" => self.online_rematch(),
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The clock moved on `ms`: the game with it, unless a Local game is
    /// paused; then the online sync.
    pub(super) fn play_advance(&mut self, ms: i64) {
        if let Some(p) = self.play.as_mut() {
            if !(p.paused && !p.online) {
                p.game.advance(ms);
            }
            if p.game.phase() == Phase::Ended {
                p.game.keys = Default::default();
            }
        }
        self.phase_sounds();
        self.online_game_sync();
    }

    /// The sound of a phase changing: a tick for each second of the 3-2-1,
    /// "go" when the clock starts, "end" at 0:00.
    fn phase_sounds(&mut self) {
        let Some(p) = self.play.as_mut() else {
            return;
        };
        let now = p.game.phase();
        if p.heard == Some(now) {
            return;
        }
        let before = p.heard.replace(now);
        let key = match now {
            Phase::Countdown { .. } => "countdown",
            Phase::Running { .. } if matches!(before, None | Some(Phase::Countdown { .. })) => "go",
            Phase::Running { .. } => return,
            Phase::Ended => "end",
        };
        self.cue(key);
    }

    /// Names a sound for the page to play next.
    pub(crate) fn cue(&mut self, key: &str) {
        self.cue_count += 1;
        self.cues.push(Cue {
            key: key.into(),
            volume: self.settings.volume,
            seed: (mix(self.seed ^ self.cue_count) & 0xffff) as u32,
        });
    }

    pub(super) fn phase_key(&self) -> String {
        match self.play.as_ref().map(|p| p.game.phase()) {
            Some(Phase::Countdown { .. }) => "countdown",
            Some(Phase::Running { .. }) => "running",
            Some(Phase::Ended) => "ended",
            None => "",
        }
        .into()
    }

    pub(super) fn play_panels(&self) -> Vec<Panel> {
        let Some(p) = self.play.as_ref() else {
            return Vec::new();
        };
        match p.game.phase() {
            Phase::Ended => {
                let mut items = Vec::new();
                if p.online && self.online.hosting() {
                    items.push(self.button("end.rematch"));
                }
                items.push(self.button("end.title"));
                vec![Panel::new("end", self.t("end.heading"), items)]
            }
            phase => {
                let ms_left = match phase {
                    Phase::Running { ms_left } => ms_left,
                    _ => p.game.length_ms(),
                };
                let mut out = vec![Panel::new("clock", clock_text(ms_left), Vec::new())];
                if let Phase::Countdown { seconds } = phase {
                    out.push(Panel::new("countdown", seconds.to_string(), Vec::new()));
                }
                if p.paused {
                    out.push(Panel::new(
                        "pause",
                        self.t("pause.heading"),
                        vec![self.button("menu.resume"), self.button("menu.quit")],
                    ));
                }
                out
            }
        }
    }

    /// The stand-in's picture for the drawing buffer's size, or nothing off
    /// Play.
    pub fn scene(&self) -> crate::standin::draw::Scene {
        match (&self.play, &self.tuning, &self.look) {
            (Some(p), Some(t), Some(l)) if self.screen == Screen::Play => {
                crate::standin::draw::scene(&p.game, t, l, self.size[0], self.size[1])
            }
            _ => Default::default(),
        }
    }
}
