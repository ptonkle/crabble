//! How to play: a handful of short lines and the controls. The person,
//! 2026-10-19: "DO NOT OVEREXPLAIN IN THIS. FOR EXAMPLE YOU DO NOT NEED TO
//! NAME THE EXACT POINT LOSS OR TIMING ON WHEN YOU START LOSING POINTS." So
//! no numbers and no exact rules here, and never the board game's
//! trademarked name (`tests/copy.rs`).

use super::*;

/// The lines, in order: the goal, the claws, claiming a word, words that
/// are not real, and placing often.
const LINES: [&str; 5] = ["how.goal", "how.claws", "how.claim", "how.fake", "how.idle"];

/// The controls, each a (what, which keys) pair of keys.
/// The last is the one touch line (F15).
const CONTROLS: [(&str, &str); 6] = [
    ("how.move", "how.move_keys"),
    ("how.look", "how.look_keys"),
    ("how.claw", "how.claw_keys"),
    ("how.jump", "how.jump_keys"),
    ("how.overhead", "how.overhead_keys"),
    ("how.touch", "how.touch_keys"),
];

impl App {
    pub(super) fn how_panels(&self) -> Vec<Panel> {
        vec![Panel::new(
            "how",
            self.t("menu.how"),
            vec![
                Item::Lines {
                    lines: LINES.iter().map(|k| self.t(k)).collect(),
                },
                Item::List {
                    rows: CONTROLS.iter().map(|(a, b)| [self.t(a), self.t(b)]).collect(),
                },
                self.button("menu.back"),
            ],
        )]
    }
}
