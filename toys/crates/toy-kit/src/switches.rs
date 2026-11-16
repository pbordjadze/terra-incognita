//! The rules as switches (painting-company row 335, chosen on Nov 13 2026 from three mockups; row 355, Nov 15:
//! part of the one sheet, and they stay set). A toy whose crowd follows a few simple rules lists them
//! (`Toy::switches`), and the sheet (info.rs) lays them out under its rules, all on at first; turning one off
//! shows what that rule was holding together (no heading to match, and the starlings smear into strands), and
//! turning it back on brings the order back. Closing the sheet leaves them as they are: the reader goes on
//! playing with the toy so changed, and the mark wears an amber dot while any rule is off. They last as long as
//! the toy is held: in Terra, through its card and back in, until the card is put away and the toy let go; a
//! toy made afresh starts with every rule on.
//!
//! The switches are the shell's, native toggles in the system's text (VoiceOver reads them as switches;
//! `host.switches_text()`, a line a rule: its label, a tab, 1 or 0). A plan opens the sheet as `"tap":
//! "switches"` and turns a rule with `"switch": [i, false]` (check.rs), and tools/kit/shell_text.py draws
//! them on the shots.

use crate::{Host, Toy};
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

/// The switches' lines while the sheet shows them, kept by the `Host`.
#[derive(Default)]
pub struct Panel {
    text: Option<CString>,
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
    /// The switches' lines while the sheet shows them (`text`), for the shell; None otherwise.
    pub fn switches_text(&self) -> Option<&CStr> {
        self.switches.text.as_deref()
    }

    /// The reader turned rule `i` on or off (a switch on the sheet, or a plan's `"switch"`). It stays so
    /// when the sheet closes.
    pub fn set_switch(&mut self, i: usize, on: bool) {
        if self.switches.text.is_some() && i < self.toy.switches().len() {
            self.toy.set_switch(&mut self.ctx, i, on);
            self.switches_step();
        }
    }

    /// Each frame: the switches show while the sheet's rules page does (a held toy's own, in Terra).
    pub(crate) fn switches_step(&mut self) {
        let want = if self.info.rules().is_some() {
            let sw = self.toy.switches();
            (!sw.is_empty()).then(|| text(&sw))
        } else {
            None
        };
        let same = match (&want, &self.switches.text) {
            (Some(w), Some(t)) => w.as_bytes() == t.as_bytes(),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.switches.text = want.map(|w| CString::new(w).unwrap_or_default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_a_line_a_rule() {
        let t = text(&[
            Switch::new("Keep close", true),
            Switch::new("Match\theading", false),
        ]);
        assert_eq!(t, "Keep close\t1\nMatch heading\t0\n");
    }
}
