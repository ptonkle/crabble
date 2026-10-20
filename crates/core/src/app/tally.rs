//! The tally at the end of a game (BRIEF.md, Scoring: "points are tallied
//! up one by one, as in Honey Hole's postgame"). Scores were hidden during
//! play; here each player's words are revealed one line at a time, column
//! by column in seat order, in the owner's colour, a word that is not real
//! as a minus, then the idle penalty as a line, each column's total
//! counting up with it, and last the winner. The pacing is `data/look.json`
//! (`tally`); `?fast` shows it all at once.
//!
//! The score itself is `crate::score`; the game hands over its final state
//! through [`final_state`] alone.

use super::online_play::SEAT_COLOURS;
use super::play::Play;
use super::*;
use crate::look::TallyLook;
use crate::score::{self, Board, Tally};
use crate::sim::{Match, Phase};
use crate::view::{TallyColumn, TallyLine};

/// The game's final board and each seat's idle points lost: the one door
/// from the simulation to the tally.
pub fn final_state(game: &Match) -> (Board, Vec<i32>) {
    // the idle penalty is the game's idle timer's (gameplay stage 3); until
    // it charges points, every seat has lost none (SECOND-ORDER R3.11)
    (Board::from_tiles(&game.tiles), vec![0; game.crabs.len()])
}

/// The tally and its timetable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reveal {
    pub tally: Tally,
    /// the app's clock (ms) when it started
    pub start_ms: i64,
    /// when each line of each column shows, ms after the start
    pub times: Vec<Vec<i64>>,
    /// when each column's turn starts
    pub starts: Vec<i64>,
    pub winner_ms: i64,
    /// lines and winner already sounded
    heard: usize,
    fanfare: bool,
}

/// What shows at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// lines shown, by column
    pub lines: Vec<usize>,
    /// the column being revealed
    pub now: Option<usize>,
    pub winner: bool,
}

/// A column's lines: its words in the order claimed, then the idle penalty
/// when there is one.
fn line_count(line: &score::Line) -> usize {
    line.words.len() + (line.idle > 0) as usize
}

/// The points of a column's first `n` lines.
fn total_of(line: &score::Line, n: usize) -> i32 {
    let words: i32 = line.words.iter().take(n).map(score::Scored::signed).sum();
    let idle = if n > line.words.len() { line.idle } else { 0 };
    words - idle
}

impl Reveal {
    /// The timetable: the first line `first_ms` after the start, one every
    /// `line_ms`, `gap_ms` more between columns (a column with no line
    /// still takes one line's time), the winner `winner_ms` after the last.
    /// With no look (or `?fast`), everything shows at once.
    pub fn new(tally: Tally, look: Option<&TallyLook>, start_ms: i64) -> Reveal {
        let (first, line, gap, win) =
            look.map_or((0, 0, 0, 0), |l| (l.first_ms, l.line_ms, l.gap_ms, l.winner_ms));
        let (line, gap) = (line as i64, gap as i64);
        let mut t = first as i64;
        let mut times = Vec::new();
        let mut starts = Vec::new();
        for l in &tally.lines {
            starts.push(t);
            let n = line_count(l);
            times.push((0..n as i64).map(|k| t + k * line).collect());
            t += line * n.max(1) as i64 + gap;
        }
        let winner_ms = (t - gap - line).max(first as i64) + win as i64;
        Reveal {
            tally,
            start_ms,
            times,
            starts,
            winner_ms,
            heard: 0,
            fanfare: false,
        }
    }

    /// What shows `ms` after the start.
    pub fn at(&self, ms: i64) -> Shown {
        let winner = ms >= self.winner_ms;
        let lines = self
            .times
            .iter()
            .map(|ts| {
                if winner {
                    ts.len()
                } else {
                    ts.iter().filter(|t| **t <= ms).count()
                }
            })
            .collect();
        let now = if winner {
            None
        } else {
            self.starts.iter().rposition(|s| *s <= ms)
        };
        Shown { lines, now, winner }
    }

    /// Shows everything now.
    pub fn skip(&mut self, now_ms: i64) {
        self.start_ms = now_ms - self.winner_ms;
    }
}

impl App {
    /// Starts the tally when the game has ended, and sounds each line as it
    /// shows and the winner.
    pub(super) fn tally_advance(&mut self) {
        let now = self.now_ms();
        let fast = self.fast;
        let look = self.view.look.as_ref().map(|l| l.tally.clone());
        let Some(p) = self.play.as_mut() else {
            return;
        };
        if p.game.phase() != Phase::Ended {
            return;
        }
        let r = p.tally.get_or_insert_with(|| {
            let (board, idle) = final_state(&p.game);
            let tally = score::tally(&board, p.game.crabs.len(), &idle);
            Reveal::new(tally, look.as_ref().filter(|_| !fast), now)
        });
        let shown = r.at(now - r.start_ms);
        let lines: usize = shown.lines.iter().sum();
        let mut keys = Vec::new();
        if lines > r.heard {
            r.heard = lines;
            if !shown.winner {
                keys.push("tally");
            }
        }
        if shown.winner && !r.fanfare {
            r.fanfare = true;
            keys.push("fanfare");
        }
        for k in keys {
            self.cue(k);
        }
    }

    pub(super) fn tally_skip(&mut self) {
        let now = self.now_ms();
        if let Some(r) = self.play.as_mut().and_then(|p| p.tally.as_mut()) {
            r.skip(now);
        }
    }

    /// Each seat's name at the tally: an online player's name, or the
    /// crab's colour; this browser's own marked as yours.
    fn tally_names(&self, p: &Play) -> Vec<String> {
        let seats = self
            .online
            .session
            .as_ref()
            .filter(|_| p.online)
            .map(|s| s.lobby().seats);
        (0..p.game.crabs.len())
            .map(|k| {
                let colour = self.t(&format!("colour.{}", SEAT_COLOURS[k % 4]));
                let name = seats
                    .as_ref()
                    .and_then(|s| s.get(k))
                    .filter(|s| !s.ai && !s.name.is_empty())
                    .map_or(colour, |s| s.name.clone());
                if k == p.local {
                    self.t("lobby.you").replace("{name}", &name)
                } else {
                    name
                }
            })
            .collect()
    }

    /// The end's panel: the tally so far, then the winner and the way on.
    pub(super) fn tally_panels(&self, p: &Play) -> Vec<Panel> {
        let mut items = Vec::new();
        let names = self.tally_names(p);
        let winner_line = p.tally.as_ref().map(|r| {
            let shown = r.at(self.now_ms() - r.start_ms);
            let columns = r
                .tally
                .lines
                .iter()
                .zip(&shown.lines)
                .map(|(l, n)| self.tally_column(l, *n, &names, &shown, &r.tally))
                .collect();
            items.push(Item::Tally { columns });
            shown.winner.then(|| match r.tally.winners.as_slice() {
                [one] => self.t("tally.wins").replace("{player}", &names[*one]),
                _ => self.t("tally.tie"),
            })
        });
        match winner_line.flatten() {
            Some(line) => {
                items.push(Item::Lines { lines: vec![line] });
                if p.online && self.online.hosting() {
                    items.push(self.button("end.rematch"));
                }
                items.push(self.button("end.title"));
            }
            None => items.push(Item::quiet_button("end.skip", self.t("end.skip"))),
        }
        vec![Panel::new("end", self.t("end.heading"), items)]
    }

    fn tally_column(
        &self,
        line: &score::Line,
        n: usize,
        names: &[String],
        shown: &Shown,
        tally: &Tally,
    ) -> TallyColumn {
        let mut lines: Vec<TallyLine> = line
            .words
            .iter()
            .take(n)
            .map(|w| TallyLine {
                text: w.letters.clone(),
                points: format!("{:+}", w.signed()),
                minus: w.signed() < 0,
                idle: false,
            })
            .collect();
        if n > line.words.len() {
            lines.push(TallyLine {
                text: self.t("tally.idle"),
                points: format!("{}", -line.idle),
                minus: true,
                idle: true,
            });
        }
        TallyColumn {
            name: names.get(line.seat).cloned().unwrap_or_default(),
            tone: SEAT_COLOURS[line.seat % 4].into(),
            lines,
            total: total_of(line, n).to_string(),
            now: shown.now == Some(line.seat),
            winner: shown.winner && tally.winners.contains(&line.seat),
        }
    }
}

#[cfg(test)]
mod tests;
