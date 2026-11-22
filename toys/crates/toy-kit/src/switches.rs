//! The rules as switches (painting-company row 335, chosen on Nov 13 2026 from three mockups; row 355, Nov 15:
//! part of the one sheet, and they stay set). A toy whose crowd follows a few simple rules lists them
//! (`Toy::switches`), and the sheet (info.rs) lays them out under its rules, all on at first; turning one off
//! shows what that rule was holding together (no heading to match, and the starlings smear into strands), and
//! turning it back on brings the order back. Closing the sheet leaves them as they are: the reader goes on
//! playing with the toy so changed, and the mark wears an amber dot while any rule is off. They last as long as
//! the toy is held: in Terra, through its card and back in, until the card is put away and the toy let go; a
//! toy made afresh starts with every rule on.
//!
//! The switches are the frame's, native toggles in the system's text (VoiceOver reads them as switches;
//! `chrome::Sheet::switches`). A plan opens the sheet as `"tap": "switches"` and turns a rule with
//! `"switch": [i, false]` (check.rs), and tools/kit/shell_text.py draws them on the shots.

use crate::{Host, Toy};

/// One rule as a switch: its label (two or three words: "Match heading") and whether it is on now.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
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

// the switches' side of the host
impl<T: Toy> Host<T> {
    /// The reader turned rule `i` on or off (a switch on the frame's sheet, or a plan's `"switch"`). It stays
    /// so when the sheet closes.
    pub fn set_switch(&mut self, i: usize, on: bool) {
        if self.info.rules().is_some() && i < self.toy.switches().len() {
            self.toy.set_switch(&mut self.ctx, i, on);
        }
    }
}
