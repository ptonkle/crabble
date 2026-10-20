//! Touch: playing on a phone or tablet (F15, "make playable on
//! phone/tablet"). The page says whether the reader is using touch
//! (`flag.touch`, from the pointer's type, never the browser's name) and
//! sends each control's touches here; this file lays the controls out
//! (`Ui::touch`, in CSS pixels) and turns them into the same `SeatInput`
//! the keys and the mouse make (`app/play.rs`).
//!
//! The controls, during a game with no menu over it:
//! - a thumbstick on the left half below the overhead view, which walks;
//! - a drag anywhere else on the right half, which turns the camera (no
//!   pointer lock);
//! - the left and right claws' buttons and the jump button by the right
//!   thumb, in the bottom-right corner;
//! - the overhead view itself, moved to the top-left corner: a tap
//!   enlarges it (Tab's view) and another shrinks it;
//! - a small pause button beside the overhead view.
//!
//! Held upright during a game, the screen shows a gentle call to turn it
//! sideways (`touch.turn`).

use serde::Serialize;

use super::*;
use crate::look::TouchLook;

fn is_false(b: &bool) -> bool {
    !*b
}

/// The knob's size, percent of its ring's.
const KNOB_PCT: i32 = 44;

/// One touch control: what it is, where it goes, its action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Control {
    /// the action its touches send (`touch.*`), each as "down,x,y",
    /// "move,x,y" or "up" (the finger's place in CSS pixels)
    pub id: String,
    /// `stick` (a zone: the ring comes to the thumb and the knob follows
    /// it), `look` (a zone: a drag turns the camera), `button` (a tap),
    /// `map` (a tap on the overhead view)
    pub kind: String,
    /// left, top, width, height, in CSS pixels from the view's top left
    pub rect: [i32; 4],
    /// shown on a button; read aloud by a screen reader on any control
    pub label: String,
    /// a stick's ring and its knob, as `rect`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ring: Option<[i32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knob: Option<[i32; 4]>,
    /// a toggle that is on (the overhead view enlarged)
    #[serde(skip_serializing_if = "is_false")]
    pub on: bool,
}

/// The touch controls the page draws over the view, first under last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TouchUi {
    pub controls: Vec<Control>,
}

/// Where the fingers on the stick and the look zone are, CSS pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fingers {
    /// the stick's ring centre while a thumb is on it, and the knob's
    /// lean from there (held within the ring)
    pub stick: Option<[i32; 2]>,
    pub knob: [i32; 2],
    /// the look finger's last place
    pub look: Option<[i32; 2]>,
}

/// A stick's lean (x right, y down, thousandths of its ring's radius) as
/// the walk it asks for: (forward, side), each -1, 0 or 1. It walks a way
/// once it leans `dead_pct` of its radius that way, so a lean between two
/// ways walks both (a diagonal).
pub fn stick_walk(lean: (i32, i32), dead_pct: i32) -> (i8, i8) {
    let dead = dead_pct * 10;
    let way = |v: i32| -> i8 {
        if v > dead {
            1
        } else if v < -dead {
            -1
        } else {
            0
        }
    };
    (-way(lean.1), way(lean.0))
}

/// A finger's report: "down,x,y", "move,x,y" or "up", the place in whole
/// CSS pixels (a fraction is dropped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touch {
    Down([i32; 2]),
    Move([i32; 2]),
    Up,
}

pub fn parse_touch(value: &str) -> Option<Touch> {
    let mut it = value.split(',');
    let what = it.next()?;
    let mut num = || -> Option<i32> {
        let n = it.next()?.trim();
        let whole = n.split('.').next()?;
        whole.parse::<i32>().ok().map(|v| v.clamp(-100_000, 100_000))
    };
    match what {
        "down" => Some(Touch::Down([num()?, num()?])),
        "move" => Some(Touch::Move([num()?, num()?])),
        "up" => Some(Touch::Up),
        _ => None,
    }
}

/// `d` held within `r` of the centre (its length cut to `r`).
pub fn within(d: [i32; 2], r: i32) -> [i32; 2] {
    let len2 = d[0] as i64 * d[0] as i64 + d[1] as i64 * d[1] as i64;
    let r = r.max(1) as i64;
    if len2 <= r * r {
        return d;
    }
    let len = crate::gfx::fx::isqrt(len2 as u128) as i64;
    [(d[0] as i64 * r / len) as i32, (d[1] as i64 * r / len) as i32]
}

/// Lays the controls out on a view `css` pixels across, the overhead view
/// enlarged or not, the stick's ring where `fingers` put it.
pub fn layout(
    t: &TouchLook,
    o: &crate::look::OverheadLook,
    css: [i32; 2],
    big: bool,
    fingers: &Fingers,
    label: impl Fn(&str) -> String,
) -> TouchUi {
    let [w, h] = [css[0].max(1), css[1].max(1)];
    // a bigger screen gets bigger controls, up to scale_max_pct
    let scale = (w.min(h) * 100 / t.short_px.max(1)).clamp(100, t.scale_max_pct);
    let px = |v: i32| v * scale / 100;
    let (m, gap) = (px(t.margin_px), px(t.gap_px));
    let (claw, jump, small) = (px(t.claw_px), px(t.jump_px), px(t.small_px));
    // the overhead view's square, in CSS pixels from the top left
    let (side, at) = super::view3d::overhead_square(o, css, big, true);
    let map = [at[0], h - at[1] - side, side, side];
    let small_map = super::view3d::overhead_square(o, css, false, true);
    let below_map = h - small_map.1[1] + m;
    // the right thumb's corner: the jump button in it, the left claw to
    // its left and the right claw above it, each centred on its line
    let jump_at = [w - m - jump, h - m - jump];
    let centred = jump / 2 - claw / 2;
    let left_claw = [jump_at[0] - gap - claw, jump_at[1] + centred];
    let right_claw = [jump_at[0] + centred, jump_at[1] - gap - claw];
    // the stick's ring: at rest by the left thumb, or under it
    let r = px(t.stick_px) / 2;
    let centre = fingers.stick.unwrap_or([m + r + gap, h - m - r]);
    let k = r * KNOB_PCT / 100;
    let knob = [
        centre[0] + fingers.knob[0] - k,
        centre[1] + fingers.knob[1] - k,
        2 * k,
        2 * k,
    ];
    let control = |id: &str, kind: &str, rect: [i32; 4]| Control {
        id: id.into(),
        kind: kind.into(),
        rect,
        label: label(id),
        ring: None,
        knob: None,
        on: false,
    };
    let controls = vec![
        control("touch.look", "look", [w / 2, 0, w - w / 2, h]),
        Control {
            ring: Some([centre[0] - r, centre[1] - r, 2 * r, 2 * r]),
            knob: Some(knob),
            ..control(
                "touch.stick",
                "stick",
                [0, below_map, w / 2, (h - below_map).max(0)],
            )
        },
        control("touch.jump", "button", [jump_at[0], jump_at[1], jump, jump]),
        control("touch.left", "button", [left_claw[0], left_claw[1], claw, claw]),
        control(
            "touch.right",
            "button",
            [right_claw[0], right_claw[1], claw, claw],
        ),
        control(
            "touch.pause",
            "button",
            [small_map.1[0] + small_map.0 + m, m, small, small],
        ),
        Control {
            on: big,
            ..control("touch.overhead", "map", map)
        },
    ];
    TouchUi { controls }
}

impl App {
    fn touch_layout(&self, p: &play::Play, label: impl Fn(&str) -> String) -> Option<TouchUi> {
        let look = self.view.look.as_ref()?;
        Some(layout(
            &look.touch,
            &look.play.overhead,
            self.css,
            p.keys.overhead,
            &p.fingers,
            label,
        ))
    }

    /// The touch controls while a game is on with no menu over it, on a
    /// touch screen.
    pub(super) fn touch_ui(&self) -> Option<TouchUi> {
        let p = self
            .play
            .as_ref()
            .filter(|p| self.touch && self.screen == Screen::Play && p.captures())?;
        self.touch_layout(p, |id| self.t(id))
    }

    /// The call to turn the screen sideways: a touch screen held upright
    /// during a game.
    pub(super) fn turn_panel(&self) -> Option<Panel> {
        let upright = self.css[1] > self.css[0];
        (self.touch && upright && self.play.as_ref().is_some_and(|p| p.captures()))
            .then(|| Panel::new("turn", self.t("touch.turn"), Vec::new()))
    }

    /// A touch control's report during a game. True when it was one.
    pub(super) fn touch_input(&mut self, name: &str, value: &str) -> bool {
        if !name.starts_with("touch.") {
            return false;
        }
        let Some(touch) = parse_touch(value) else {
            return true;
        };
        if self.screen != Screen::Play {
            return true;
        }
        let (sensitivity, invert) = (self.settings.mouse_pct, self.settings.invert_y);
        let look = self
            .view
            .look
            .as_ref()
            .map(|l| (l.touch.dead_pct, l.touch.look_pct));
        let (dead_pct, look_pct) = look.unwrap_or((35, 100));
        // the stick's ring radius, from its layout
        let radius = self
            .play
            .as_ref()
            .and_then(|p| self.touch_layout(p, |_| String::new()))
            .and_then(|ui| ui.controls.into_iter().find_map(|c| c.ring))
            .map_or(1, |r| r[2] / 2);
        if name == "touch.pause" {
            if let Touch::Down(_) = touch {
                self.toggle_pause();
            }
            return true;
        }
        let Some(p) = self.play.as_mut() else {
            return true;
        };
        if !p.captures() {
            p.let_go();
            return true;
        }
        let down = matches!(touch, Touch::Down(_));
        match (name, touch) {
            ("touch.stick", Touch::Down(at)) => {
                p.fingers.stick = Some(at);
                p.fingers.knob = [0, 0];
                p.stick = (0, 0);
            }
            ("touch.stick", Touch::Move(at)) => {
                if let Some(c) = p.fingers.stick {
                    let d = within([at[0] - c[0], at[1] - c[1]], radius);
                    p.fingers.knob = d;
                    let lean = (d[0] * 1000 / radius.max(1), d[1] * 1000 / radius.max(1));
                    p.stick = stick_walk(lean, dead_pct);
                }
            }
            ("touch.stick", Touch::Up) => {
                p.fingers.stick = None;
                p.fingers.knob = [0, 0];
                p.stick = (0, 0);
            }
            ("touch.look", Touch::Down(at)) => p.fingers.look = Some(at),
            ("touch.look", Touch::Move(at)) => {
                if let Some(last) = p.fingers.look {
                    p.look(
                        at[0] - last[0],
                        at[1] - last[1],
                        sensitivity * look_pct as u32 / 100,
                        invert,
                    );
                }
                p.fingers.look = Some(at);
            }
            ("touch.look", Touch::Up) => p.fingers.look = None,
            ("touch.left", _) if down => {
                p.keys.press("0");
            }
            ("touch.right", _) if down => {
                p.keys.press("2");
            }
            ("touch.jump", _) if down => {
                p.keys.set("Space", true);
            }
            ("touch.overhead", _) if down => p.keys.overhead = !p.keys.overhead,
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stick_walks_once_it_leans_past_its_dead_zone() {
        // 35%: a lean of 350 thousandths walks nowhere, 351 walks; up is
        // forward (y is down on the screen)
        assert_eq!(stick_walk((0, 0), 35), (0, 0));
        assert_eq!(stick_walk((350, -350), 35), (0, 0));
        assert_eq!(stick_walk((0, -351), 35), (1, 0));
        assert_eq!(stick_walk((0, 900), 35), (-1, 0));
        assert_eq!(stick_walk((-900, 100), 35), (0, -1));
        // a diagonal (45 degrees, 707 each way) walks both ways
        assert_eq!(stick_walk((707, -707), 35), (1, 1));
    }

    const LOOK: &str = include_str!("../../../../data/look.json");

    /// A Local game on a touch screen of `css` pixels, the look loaded.
    fn touch_game(css: [i32; 2]) -> App {
        let mut a = super::super::tests::app();
        a.act("data.look", LOOK).unwrap();
        a.act("flag.fast", "on").unwrap();
        a.act("flag.touch", "on").unwrap();
        a.resize(css[0], css[1], 300);
        a.act("local.start", "").unwrap();
        a
    }

    #[test]
    fn the_stick_walks_the_crab_and_a_drag_turns_the_camera() {
        let mut a = touch_game([844, 390]);
        a.act("touch.stick", "down,100,300").unwrap();
        a.act("touch.stick", "move,100.6,250").unwrap();
        super::super::tests::wait(&mut a, 500);
        assert!(a.ui().moved, "the stick pushed up walks the crab");
        assert_eq!(a.play.as_ref().unwrap().stick, (1, 0));
        a.act("touch.stick", "up").unwrap();
        assert_eq!(a.play.as_ref().unwrap().stick, (0, 0));
        let yaw = a.play.as_ref().unwrap().yaw;
        // 27 units a pixel for the mouse at 100%; a drag turns 250% of it
        a.act("touch.look", "down,600,200").unwrap();
        a.act("touch.look", "move,610,200").unwrap();
        assert_eq!(a.play.as_ref().unwrap().yaw, yaw - 675);
        a.act("touch.look", "move,620,200").unwrap();
        assert_eq!(
            a.play.as_ref().unwrap().yaw,
            yaw - 1350,
            "each move turns by the step since the last"
        );
    }

    #[test]
    fn the_claw_and_jump_buttons_press_and_the_map_toggles() {
        let mut a = touch_game([844, 390]);
        a.act("touch.left", "down,700,350").unwrap();
        a.act("touch.right", "down,790,260").unwrap();
        a.act("touch.jump", "down,790,350").unwrap();
        a.act("touch.left", "up").unwrap();
        let k = a.play.as_ref().unwrap().keys;
        assert_eq!(k.click, [true, true]);
        assert!(k.jump);
        a.act("touch.overhead", "down,50,50").unwrap();
        a.act("touch.overhead", "up").unwrap();
        let ui = a.ui();
        assert!(!ui.crosshair, "the enlarged overhead view hides the crosshair");
        let map = ui
            .touch
            .unwrap()
            .controls
            .into_iter()
            .find(|c| c.id == "touch.overhead")
            .unwrap();
        assert!(map.on);
        // enlarged: 90% of the short side (390), 351, in the middle:
        // (844 - 351) / 2 = 246 in, (390 - 351) / 2 = 19 down from the
        // bottom, so 390 - 19 - 351 = 20 from the top
        assert_eq!(map.rect, [246, 20, 351, 351]);
        a.act("touch.overhead", "down,300,100").unwrap();
        assert!(!a.play.as_ref().unwrap().keys.overhead);
    }

    #[test]
    fn the_controls_show_only_on_touch_during_a_game_with_no_menu() {
        let mut a = touch_game([844, 390]);
        assert!(a.ui().touch.is_some());
        a.act("touch.stick", "down,100,300").unwrap();
        a.act("touch.stick", "move,100,200").unwrap();
        a.act("touch.pause", "down,180,20").unwrap();
        assert_eq!(
            a.play.as_ref().unwrap().stick,
            (0, 0),
            "the pause lets the stick go"
        );
        let ui = a.ui();
        assert!(ui.touch.is_none(), "no controls under the pause menu");
        assert!(ui.panels.iter().any(|p| p.kind == "pause"));
        a.act("menu.resume", "").unwrap();
        a.act("flag.touch", "off").unwrap();
        assert!(a.ui().touch.is_none(), "none for a mouse");
        assert!(!a.ui().touch_on);
    }

    #[test]
    fn an_upright_touch_screen_is_asked_to_turn_during_a_game() {
        let a = touch_game([390, 844]);
        assert!(a
            .ui()
            .panels
            .iter()
            .any(|p| p.kind == "turn" && p.heading == "TOUCH.TURN"));
        let a = touch_game([844, 390]);
        assert!(!a.ui().panels.iter().any(|p| p.kind == "turn"));
    }

    #[test]
    fn a_finger_report_is_read_whole_or_not_at_all() {
        assert_eq!(parse_touch("down,12,34"), Some(Touch::Down([12, 34])));
        assert_eq!(parse_touch("move,12.7,-3.2"), Some(Touch::Move([12, -3])));
        assert_eq!(parse_touch("up"), Some(Touch::Up));
        assert_eq!(parse_touch("down,12"), None);
        assert_eq!(parse_touch(""), None);
        assert_eq!(parse_touch("tap,1,2"), None);
    }

    #[test]
    fn the_knob_stays_within_its_ring() {
        assert_eq!(within([30, 40], 65), [30, 40]);
        // 300, 400 is 500 long: cut to 65 it is 39, 52
        assert_eq!(within([300, 400], 65), [39, 52]);
        assert_eq!(within([0, -200], 65), [0, -65]);
    }

    #[test]
    fn the_title_card_backs_off_on_a_narrow_screen() {
        use crate::app::title::fit_narrow;
        // 16:9 and wider keep the distance
        assert_eq!(fit_narrow(1000, [1920, 1080]), 1000);
        assert_eq!(fit_narrow(1000, [844, 390]), 1000);
        // 4:3: 1024 x 9 = 9216 against 768 x 16 = 12288, so 12288 / 9216
        assert_eq!(fit_narrow(900, [1024, 768]), 1200);
        // a phone upright (390 x 844) would need 3.85 times: at most 4
        assert_eq!(fit_narrow(1000, [390, 844]), 1000 * 13504 / 3510);
        assert_eq!(fit_narrow(1000, [100, 1000]), 4000);
    }

    fn touch_look() -> TouchLook {
        crate::testkit::look().touch
    }

    #[test]
    fn the_touch_controls_fit_a_landscape_phone_by_the_right_thumb() {
        let t = touch_look();
        let o = crate::testkit::look().play.overhead;
        let ui = layout(&t, &o, [844, 390], false, &Fingers::default(), |id| {
            id.to_string()
        });
        let rect = |id: &str| ui.controls.iter().find(|c| c.id == id).unwrap().rect;
        // every control lies on the screen
        for c in &ui.controls {
            let [x, y, w, h] = c.rect;
            assert!(
                x >= 0 && y >= 0 && x + w <= 844 && y + h <= 390,
                "{} {:?}",
                c.id,
                c.rect
            );
        }
        // the claws and the jump are on the right half, near the bottom
        for id in ["touch.left", "touch.right", "touch.jump"] {
            let [x, y, _, _] = rect(id);
            assert!(x > 844 / 2 && y > 390 / 2, "{id}");
        }
        // the left claw is left of the right claw
        assert!(rect("touch.left")[0] < rect("touch.right")[0]);
        // the stick's zone is the left half, under the overhead view (34%
        // of 390 = 132, 2% margin = 7: it ends at 139)
        let [sx, sy, sw, _] = rect("touch.stick");
        assert_eq!((sx, sw), (0, 422));
        assert!(sy > 139);
        // the overhead view is in the top-left corner: 132 square, 7 in
        assert_eq!(rect("touch.overhead"), [7, 7, 132, 132]);
    }

    #[test]
    fn no_two_buttons_overlap() {
        let t = touch_look();
        let o = crate::testkit::look().play.overhead;
        for size in [[844, 390], [667, 375], [1180, 820], [390, 844]] {
            let ui = layout(&t, &o, size, false, &Fingers::default(), |id| id.to_string());
            let buttons: Vec<_> = ui
                .controls
                .iter()
                .filter(|c| c.kind == "button" || c.kind == "map")
                .collect();
            for (i, a) in buttons.iter().enumerate() {
                for b in &buttons[i + 1..] {
                    let [ax, ay, aw, ah] = a.rect;
                    let [bx, by, bw, bh] = b.rect;
                    let apart = ax + aw <= bx || bx + bw <= ax || ay + ah <= by || by + bh <= ay;
                    assert!(apart, "{} and {} overlap at {size:?}", a.id, b.id);
                }
            }
        }
    }
}
