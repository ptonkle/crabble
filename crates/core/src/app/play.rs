//! Play: the game (`sim::Match`) as this browser plays it: the keys and
//! the mouse turned into its seat's `SeatInput`, the fixed tick, the HUD
//! (the clock and the 3-2-1), the pause menu on Escape and the end. A
//! Local game pauses under the menu; an online game runs on behind it
//! (Ape Race's pause). The 3D view is `app/view3d.rs`.

use serde::Serialize;

use super::*;
use crate::gfx::fx::TURN;
use crate::rng::mix;
use crate::sim::{Driver, Event, Map, Match, Phase, SeatInput, Setup};
use crate::view::TableRow;

/// How far the local crab must go from its start to count as moved (for
/// the browser check), mm.
pub(crate) const MOVED_MM: i64 = 100;

/// The longest the page's clock may jump in one frame, ms: a tab left in
/// the background does not run minutes of ticks at once.
const MAX_FRAME_MS: i64 = 250;

/// The sounds the core names (`data/sounds.json`).
pub const CUES: [&str; 11] = [
    "countdown",
    "go",
    "end",
    "jump",
    "land",
    "grab",
    "place",
    "drop",
    "word",
    "penalty",
    "idle_tick",
];

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

/// The keys and buttons this browser's player holds or has pressed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keys {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    /// Tab: the overhead view fills the screen while it is held
    pub overhead: bool,
    /// pressed since the last tick took them
    pub jump: bool,
    pub click: [bool; 2],
}

impl Keys {
    /// The key `code` (`KeyboardEvent.code`) held or let go; false for a
    /// key the game does not use.
    pub fn set(&mut self, code: &str, down: bool) -> bool {
        let k = match code {
            "KeyW" => &mut self.forward,
            "KeyS" => &mut self.back,
            "KeyA" => &mut self.left,
            "KeyD" => &mut self.right,
            "Tab" => &mut self.overhead,
            "Space" => {
                if down {
                    self.jump = true;
                }
                return true;
            }
            _ => return false,
        };
        *k = down;
        true
    }

    /// A mouse button pressed (`MouseEvent.button`): 0 the left claw, 2
    /// the right; false for another button.
    pub fn press(&mut self, button: &str) -> bool {
        match button {
            "0" => self.click[crate::sim::tiles::LEFT] = true,
            "2" => self.click[crate::sim::tiles::RIGHT] = true,
            _ => return false,
        }
        true
    }
}

pub struct Play {
    pub game: Match,
    /// the seat this browser plays
    pub local: usize,
    /// the local camera's turn and tilt (angle units; the tilt looks down)
    pub yaw: i32,
    pub pitch: i32,
    pub keys: Keys,
    /// the page's time not yet run as a whole tick, ms
    rest_ms: i64,
    /// the phase last heard, for the sounds of its changes
    heard: Option<Phase>,
    pub paused: bool,
    pub online: bool,
    /// where each crab started (mm)
    pub(crate) starts: Vec<(i32, i32)>,
    turn_per_px: i32,
    /// each AI seat's mind (`crate::ai`), none for the others
    brains: Vec<Option<crate::ai::Brain>>,
    /// this browser's "-2"s: when (the game's clock, ms) and how many points
    pub(crate) popups: Vec<(i64, i32)>,
    /// the idle countdown's second last heard, for its tick
    idle_heard: Option<i64>,
}

impl Play {
    /// The page keeps the keys and the mouse: the game is on, with no menu
    /// over it.
    pub fn captures(&self) -> bool {
        !self.paused && self.game.phase() != Phase::Ended
    }

    /// How far crab `i` is from its start, mm.
    pub(crate) fn moved_mm(&self, i: usize) -> i64 {
        let start = self.starts.get(i).copied().unwrap_or((0, 0));
        self.game.crabs.get(i).map_or(0, |c| {
            let s = c.state();
            let (dx, dz) = ((s.p[0] - start.0) as i64, (s.p[1] - start.1) as i64);
            crate::gfx::fx::isqrt((dx * dx + dz * dz) as u128) as i64
        })
    }

    pub fn local_moved(&self) -> bool {
        self.moved_mm(self.local) >= MOVED_MM
    }

    /// This browser's seat's input for the next tick; its presses are
    /// taken.
    fn local_input(&mut self) -> SeatInput {
        let k = self.keys;
        self.keys.jump = false;
        self.keys.click = [false; 2];
        if !self.captures() {
            return SeatInput {
                yaw: self.yaw,
                pitch: self.pitch,
                ..Default::default()
            };
        }
        SeatInput {
            fwd: k.forward as i8 - k.back as i8,
            side: k.right as i8 - k.left as i8,
            jump: k.jump,
            yaw: self.yaw,
            pitch: self.pitch,
            click: k.click,
        }
    }

    /// The mouse moved by (dx, dy) pixels: the camera turns, at
    /// `sensitivity_pct` percent, and tilts (the other way with `invert_y`).
    pub fn look(&mut self, dx: i32, dy: i32, sensitivity_pct: u32, invert_y: bool) {
        let per = self.turn_per_px as i64 * sensitivity_pct as i64;
        // the mouse to the right turns the view right: the yaw goes down
        self.yaw = (self.yaw as i64 - dx as i64 * per / 100).rem_euclid(TURN as i64) as i32;
        let dy = if invert_y { -dy } else { dy };
        let (lo, hi) = self.game.rules.pitch_range;
        self.pitch = ((self.pitch as i64 + dy as i64 * per / 100) as i32).clamp(lo, hi);
    }

    /// Runs the page's `ms` as whole ticks.
    pub fn advance(&mut self, ms: i64) {
        let tick = self.game.rules.tick_ms;
        self.rest_ms += ms.clamp(0, MAX_FRAME_MS);
        while self.rest_ms >= tick {
            self.rest_ms -= tick;
            let mut inputs: Vec<SeatInput> = self
                .game
                .crabs
                .iter()
                .map(|c| SeatInput {
                    yaw: c.yaw,
                    pitch: c.pitch,
                    ..Default::default()
                })
                .collect();
            if let Some(i) = inputs.get_mut(self.local) {
                *i = self.local_input();
            }
            for (k, brain) in self.brains.iter_mut().enumerate() {
                if let (Some(b), Some(i)) = (brain.as_mut(), inputs.get_mut(k)) {
                    *i = crate::ai::think(&self.game, k, b);
                }
            }
            self.game.step(&inputs);
        }
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
        // until the map's files have come (or in a test), flat sand
        let flat;
        let map = match self.view.map.as_ref() {
            Some(m) => m,
            None => {
                flat = Map::flat(t.board_mm() / 2 + t.crab.roam_mm + 200);
                &flat
            }
        };
        let mut game = Match::new(
            t,
            map,
            Setup {
                drivers,
                seed,
                fast: self.fast,
                length_ms: self.short,
            },
        );
        // the checks' and stills' `?spot`
        if let (Some((x, z, d)), true) = (self.spot, local < game.crabs.len()) {
            let at = crate::gfx::geom::v3(crate::model::mm1(x), 0, crate::model::mm1(z));
            let y = game.support(at, None);
            game.crabs[local].pos = crate::gfx::geom::v3(at.x, y, at.z);
            game.crabs[local].yaw = crate::gfx::fx::deg(d);
        }
        // a joiner's game ends when the host's does
        game.hold_at_zero = online && !self.online.hosting();
        let starts = game
            .crabs
            .iter()
            .map(|c| (c.state().p[0], c.state().p[1]))
            .collect();
        // the AI crabs this browser runs (a local game's, or a host's)
        let brains = game
            .crabs
            .iter()
            .enumerate()
            .map(|(k, c)| {
                (c.driver == Driver::Ai).then(|| crate::ai::Brain::new(t.ai.clone(), mix(seed ^ k as u64)))
            })
            .collect();
        let (yaw, pitch) = game.crabs.get(local).map_or((0, 0), |c| (c.yaw, c.pitch));
        self.play = Some(Play {
            game,
            local,
            yaw,
            pitch,
            keys: Keys::default(),
            rest_ms: 0,
            heard: None,
            paused: false,
            online,
            starts,
            turn_per_px: t.look.turn_per_px,
            brains,
            popups: Vec::new(),
            idle_heard: None,
        });
        self.view.new_game();
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
            "key" if on => p.keys.set(value, true),
            "key.up" => p.keys.set(value, false),
            "mouse.down" if on => p.keys.press(value),
            "keys.release" => {
                p.keys = Default::default();
                true
            }
            "mouse.move" if on => {
                let mut it = value.split(',').map(|n| n.trim().parse::<i32>().unwrap_or(0));
                let (dx, dy) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
                p.look(dx, dy, sensitivity, invert);
                true
            }
            // the browser let the pointer go (its own Escape): the menu
            "pointer.released" if on => {
                p.paused = true;
                p.keys = Default::default();
                true
            }
            _ => false,
        }
    }

    pub(super) fn toggle_pause(&mut self) {
        if let Some(p) = self.play.as_mut() {
            if p.game.phase() != Phase::Ended {
                p.paused = !p.paused;
                p.keys = Default::default();
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
                p.advance(ms);
            }
            if p.game.phase() == Phase::Ended {
                p.keys = Default::default();
            }
        }
        self.phase_sounds();
        self.event_sounds();
        self.online_game_sync();
    }

    /// The sounds of what happened to this browser's crab.
    fn event_sounds(&mut self) {
        let Some(p) = self.play.as_mut() else {
            return;
        };
        let local = p.local;
        let mut keys = Vec::new();
        for e in p.game.take_events() {
            match e {
                Event::Jumped { seat } if seat == local => keys.push("jump"),
                Event::Landed { seat } if seat == local => keys.push("land"),
                Event::Grabbed { seat, .. } if seat == local => keys.push("grab"),
                Event::Placed { seat, .. } if seat == local => keys.push("place"),
                Event::Dropped { seat, .. } if seat == local => keys.push("drop"),
                Event::Placed { seat, square, .. } if seat == local => {
                    if p.game.grid().words_through(square).iter().any(|w| w.real) {
                        keys.push("word");
                    }
                }
                Event::IdlePenalty { seat, points } if seat == local => {
                    keys.push("penalty");
                    let now = p.game.play_ms();
                    p.popups.push((now, points));
                    p.popups.retain(|(at, _)| now - at < 10_000);
                }
                _ => {}
            }
        }
        // a tick for each second of the idle countdown
        let r = &p.game.rules;
        let left = (r.idle_grace_ms - p.game.idle_ms(local) + 999) / 1000;
        if (1..=r.idle_warn_ms / 1000).contains(&left) && p.idle_heard != Some(left) {
            keys.push("idle_tick");
        }
        p.idle_heard = Some(left);
        for k in keys {
            self.cue(k);
        }
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

    /// Builds the 3D view's frame (`app/view3d.rs`): the title card on the
    /// Title (`app/title.rs`), none off Play.
    pub fn build_frame(&mut self) {
        if self.title_card() {
            self.view.build_title(self.size, self.clock / 1000);
            return;
        }
        let on = self.screen == Screen::Play;
        let play = self.play.as_ref().filter(|_| on);
        let local = play.map(|p| (p.local, p.yaw, p.pitch));
        let big = play.is_some_and(|p| p.keys.overhead && p.captures());
        let clock = self.clock / 1000;
        self.view
            .build(play.map(|p| &p.game), local, big, self.size, clock);
    }

    /// Static layer `k`'s triangles, as the game now is.
    pub fn layer(&mut self, k: usize) -> Vec<i32> {
        self.view.layer(self.play.as_ref().map(|p| &p.game), k)
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

    /// The HUD during play: the idle call under the clock (with the seconds
    /// left in its warning window), each "-2" while it shows, and the word
    /// preview while a ghost tile is up. No scores.
    fn hud_panels(&self, p: &Play) -> Vec<Panel> {
        let mut out = Vec::new();
        let g = &p.game;
        let r = &g.rules;
        let idle = g.idle_ms(p.local);
        if idle >= r.idle_grace_ms - r.idle_warn_ms {
            let left = (r.idle_grace_ms - idle + 999) / 1000;
            let mut call = Panel::new("idle", self.t("idle.call"), Vec::new());
            if left > 0 {
                call.sub = left.to_string();
            }
            out.push(call);
        }
        let now = g.play_ms();
        let show = self.view.look.as_ref().map_or(0, |l| l.play.popup_ms as i64);
        for (at, points) in &p.popups {
            if now - at < show {
                out.push(Panel::new("penalty", points.to_string(), Vec::new()));
            }
        }
        if let Some((_, tile, sq)) = g.ghost(p.local) {
            let rows: Vec<TableRow> = g
                .preview(p.local, tile, sq)
                .into_iter()
                .map(|w| TableRow {
                    cells: vec![
                        w.text,
                        self.t(if w.real { "preview.real" } else { "preview.fake" }),
                    ],
                    marked: false,
                    tone: if w.real { "real" } else { "fake" }.into(),
                })
                .collect();
            if !rows.is_empty() {
                out.push(Panel::new(
                    "preview",
                    String::new(),
                    vec![Item::Table {
                        head: Vec::new(),
                        rows,
                    }],
                ));
            }
        }
        out
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
                let mut end = Panel::new("end", self.t("end.heading"), items);
                end.sub = self.online_notice(p).unwrap_or_default();
                vec![end]
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
                if let Phase::Running { .. } = phase {
                    out.extend(self.hud_panels(p));
                }
                if let Some(text) = self.online_notice(p) {
                    out.push(Panel::new("notice", text, Vec::new()));
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
}
