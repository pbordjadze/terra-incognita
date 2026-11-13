//! The switches mark (painting-company row 335; chosen on Nov 13 2026 from three mockups): a fourth
//! small mark, left of the others, for a toy whose crowd follows a few simple rules (`Toy::switches`).
//! A tap on it lays those rules out as switches over the live scene, all on; turning one off shows what
//! that rule was holding together (no heading to match, and the starlings smear into strands), and
//! turning it back on brings the order back. The panel stays while the reader plays with it, and touches
//! on the scene still go to the toy; a second tap on the mark closes it, and closing it turns every rule
//! back on, so a toy is never left broken. Opening the demo, the rules or the origins closes it too.
//!
//! The kit draws the mark; the panel is the shell's, native buttons in the system's text (VoiceOver reads
//! them as toggles; `host.switches_text()`, a line a rule: its label, a tab, 1 or 0). A plan taps the mark
//! as `"tap": "switches"` and turns a rule with `"switch": [i, false]` (check.rs), and
//! tools/kit/shell_text.py draws the panel on the shots.

use crate::paint::Painter;
use crate::{Gpu, Host, Phase, PointerEvent, Toy};
use std::ffi::{CStr, CString};

/// One rule as a switch: its label (two or three words: "Match heading") and whether it is on now.
#[derive(Clone, Debug, PartialEq)]
pub struct Switch {
    pub label: String,
    pub on: bool,
}

impl Switch {
    pub fn new(label: &str, on: bool) -> Switch {
        Switch {
            label: label.into(),
            on,
        }
    }
    /// A toy's rules from their labels and whether each is on (`Toy::switches`).
    pub fn all(labels: &[&str], on: &[bool]) -> Vec<Switch> {
        labels
            .iter()
            .zip(on)
            .map(|(l, &o)| Switch::new(l, o))
            .collect()
    }
}

/// The mark and whether its panel shows, kept by the `Host`.
#[derive(Default)]
pub struct Panel {
    open: bool,
    /// Fingers that went down on the mark (never the toy's).
    taken: Vec<u64>,
    /// The panel's lines while it shows (the shell's text).
    text: Option<CString>,
    painter: Option<Painter>,
}

/// A fingertip's reach for the mark, points (as the other marks': half the gap between them).
const REACH: f64 = 22.0;

impl Panel {
    /// Where the mark is, in points: in the corner's row of marks, `slot` places in from the right
    /// (0 is the info mark's place).
    pub fn spot(points: [f64; 2], slot: usize) -> [f64; 2] {
        [points[0] - 26.0 - 44.0 * slot as f64, points[1] - 34.0]
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Routes a real finger: Some(true) when a tap on the mark ends (toggle the panel), Some(false) for
    /// the rest of a finger the mark took, None for anyone else's.
    fn route(&mut self, e: &PointerEvent, spot: Option<[f64; 2]>) -> Option<bool> {
        let near = |s: [f64; 2]| (e.x - s[0]).hypot(e.y - s[1]) <= REACH;
        if self.taken.contains(&e.id) {
            if matches!(e.phase, Phase::Up | Phase::Cancel) {
                self.taken.retain(|&i| i != e.id);
                return Some(e.phase == Phase::Up && spot.is_some_and(near));
            }
            return Some(false);
        }
        if e.phase == Phase::Down && spot.is_some_and(near) {
            self.taken.push(e.id);
            return Some(false);
        }
        None
    }
}

/// The panel's text: a line a rule, its label, a tab, 1 (on) or 0 (off).
pub fn text(switches: &[Switch]) -> String {
    switches
        .iter()
        .map(|s| {
            let label = s.label.replace(['\t', '\n', '\0'], " ");
            format!("{label}\t{}\n", if s.on { 1 } else { 0 })
        })
        .collect()
}

// the switches' side of the host
impl<T: Toy> Host<T> {
    /// The switches mark's place in points, if the toy has rules to switch: left of the other marks.
    pub fn switches_spot(&self) -> Option<[f64; 2]> {
        if self.toy.switches().is_empty() {
            return None;
        }
        let slot = 2 + self.marks.origins as usize;
        Some(Panel::spot(self.ctx.points(), slot))
    }

    /// The panel's lines while it shows (`text`), for the shell; None while it is shut.
    pub fn switches_text(&self) -> Option<&CStr> {
        self.switches.text.as_deref()
    }

    /// Opens the panel (closing the demo, the rules or the origins), or closes it: a tap on the mark, or
    /// VoiceOver's activation of it.
    pub fn open_switches(&mut self) {
        if self.switches.open {
            self.close_switches();
            return;
        }
        if self.toy.switches().is_empty() {
            return;
        }
        self.info.close();
        self.info.show_rules(None);
        self.info.show_origins(None);
        self.switches.open = true;
        self.refresh_switches();
    }

    /// Closes the panel and turns every rule back on.
    pub fn close_switches(&mut self) {
        for (i, s) in self.toy.switches().iter().enumerate() {
            if !s.on {
                self.toy.set_switch(&mut self.ctx, i, true);
            }
        }
        self.switches.open = false;
        self.refresh_switches();
    }

    /// The reader turned rule `i` on or off (a button on the shell's panel, or a plan's `"switch"`).
    pub fn set_switch(&mut self, i: usize, on: bool) {
        if self.switches.open && i < self.toy.switches().len() {
            self.toy.set_switch(&mut self.ctx, i, on);
            self.refresh_switches();
        }
    }

    fn refresh_switches(&mut self) {
        self.switches.text = self
            .switches
            .open
            .then(|| CString::new(text(&self.toy.switches())).unwrap_or_default());
    }

    /// A real finger, before the other marks see it: true if it was the switches mark's.
    pub(crate) fn switches_pointer(&mut self, e: &PointerEvent) -> bool {
        let spot = self.switches_spot();
        match self.switches.route(e, spot) {
            Some(true) => {
                self.open_switches();
                true
            }
            Some(false) => true,
            None => false,
        }
    }

    /// Each frame: a card opening closes the panel; a held toy leaving takes its panel with it (Terra).
    pub(crate) fn switches_step(&mut self) {
        if self.switches.open && (self.info.is_open() || self.toy.switches().is_empty()) {
            self.close_switches();
        } else if self.switches.open {
            self.refresh_switches();
        }
    }

    /// Draws the mark: the other marks' dark ground and ring, and three dots joined, a neighbourhood.
    pub(crate) fn draw_switches_mark(&mut self, target: &wgpu::TextureView) {
        let Some(s) = self.switches_spot() else {
            return;
        };
        let gpu: &Gpu = &self.ctx.gpu;
        let painter = self
            .switches
            .painter
            .get_or_insert_with(|| Painter::new(gpu));
        let k = self.ctx.ppp as f32;
        let c = [s[0] as f32 * k, s[1] as f32 * k];
        let lit = if self.switches.open { 0.85 } else { 0.5 };
        let ink = [1.0, 1.0, 1.0, lit];
        painter.disc(c, 9.5 * k, 3.0 * k, [0.0, 0.0, 0.0, 0.42]);
        painter.ring(c, 7.5 * k, 1.1 * k, 0.7, ink);
        let p = [
            [c[0] - 3.0 * k, c[1] + 2.0 * k],
            [c[0] + 3.1 * k, c[1] + 1.5 * k],
            [c[0] + 0.1 * k, c[1] - 3.0 * k],
        ];
        for i in 0..3 {
            painter.line(p[i], p[(i + 1) % 3], 0.5 * k, 0.7, ink);
        }
        for q in p {
            painter.disc(q, 1.05 * k, 0.7, ink);
        }
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let size = [self.ctx.width, self.ctx.height];
        painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: u64, phase: Phase, at: [f64; 2]) -> PointerEvent {
        PointerEvent {
            id,
            phase,
            x: at[0],
            y: at[1],
            t: 0.0,
        }
    }

    #[test]
    fn a_tap_on_the_mark_toggles_and_others_pass() {
        let mut p = Panel::default();
        let at = Panel::spot([390.0, 844.0], 2);
        let spot = Some(at);
        assert_eq!(p.route(&ev(1, Phase::Down, at), spot), Some(false));
        assert_eq!(p.route(&ev(1, Phase::Up, at), spot), Some(true));
        // a finger elsewhere is the toy's
        assert_eq!(p.route(&ev(2, Phase::Down, [100.0, 100.0]), spot), None);
        // one that went down on the mark and slid away does nothing, and is not the toy's
        assert_eq!(p.route(&ev(3, Phase::Down, at), spot), Some(false));
        assert_eq!(
            p.route(&ev(3, Phase::Move, [100.0, 100.0]), spot),
            Some(false)
        );
        assert_eq!(
            p.route(&ev(3, Phase::Up, [100.0, 100.0]), spot),
            Some(false)
        );
        // no mark: nothing is taken
        assert_eq!(p.route(&ev(4, Phase::Down, at), None), None);
    }

    #[test]
    fn the_text_is_a_line_a_rule() {
        let t = text(&[
            Switch::new("Keep close", true),
            Switch::new("Match\theading", false),
        ]);
        assert_eq!(t, "Keep close\t1\nMatch heading\t0\n");
    }
}
