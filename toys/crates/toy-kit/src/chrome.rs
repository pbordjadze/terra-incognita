//! The chrome as data (painting-company, Nov 22 2026: Terra "super native on every phone", iPhone, Pixel and
//! Samsung first). The kit says *what* surrounds the toy (its name and place, the mark, the sheet's rules,
//! switches, style and buttons, the demo's line, the postcard's words, the buttons VoiceOver offers, whether
//! there is a way back, which senses it wants); each platform's frame (`toys/ios`, `toys/android`,
//! `toys/terra-web`) decides *how* it looks there, in that platform's own controls. Nothing here names a
//! platform: a frame that can do more (a pen's hover, a folding hinge, a wallpaper) asks for it as a sense or
//! an event, never by asking which phone it is.
//!
//! A frame reads [`Chrome`] as JSON once a frame (`Frame::chrome_json`: only when it changed), drains the
//! [`Out`] events (haptics in intents, a file asked for), and sends [`In`] for what the reader did in the
//! frame's own controls (the sheet's switches and buttons, a back gesture). The pixels and the fingers on
//! them stay on their direct path (`toy_frame`, `toy_pointer`).

use crate::info::{Rules, Style};
use crate::switches::Switch;
use crate::{Host, Label, Toy};
use serde::{Deserialize, Serialize};

/// Everything around the toy, as the frame shows it now.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Chrome {
    /// What is on screen (Terra: the toy whose card is up or open; None on its bare Earth), for the sheet's
    /// heading, a share's words and the screen reader.
    pub about: Option<About>,
    /// The toy's one mark: what it opens, and whether a rule is switched off (its amber dot).
    pub mark: Mark,
    /// The sheet, while it shows.
    pub sheet: Option<Sheet>,
    /// The demo's one line while it plays (`Toy::demo`).
    pub caption: Option<String>,
    /// Words the toy places itself, in its box (Terra's postcard: `Toy::label`).
    pub label: Option<Label>,
    /// One quiet line at the top (`Toy::title`: Screw's song).
    pub title: Option<String>,
    /// What a screen reader should offer as buttons (`Toy::buttons`), by index for `In::Press`.
    pub buttons: Vec<Button>,
    /// The way back, if there is one now (Terra in a toy: "Back to the Earth"), its words for the screen
    /// reader: the frame's back button, the system's back gesture (`In::Back`).
    pub back: Option<String>,
    /// The toy's sound is music the reader asked for (`Toy::plays_music`): the playback category.
    pub music: bool,
    /// The senses the toy reads now (`Toy::senses`): the frame keeps the rest off, for the battery.
    pub senses: Senses,
}

/// What is on screen, in words (`Toy::about`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct About {
    pub name: String,
    pub place: String,
    pub line: String,
}

/// What the mark opens (`what`: `none`, `demo` or `sheet`), whether a rule is off, and where the kit draws
/// it (points), unless the frame draws its own (`In::Frame { native_chrome: true }`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Mark {
    pub what: MarkKind,
    pub off: bool,
    pub at: Option<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkKind {
    #[default]
    None,
    Demo,
    Sheet,
}

/// The sheet: its rules page (the rules, the switches under them, the style's credit, "Show me" and "Where
/// this comes from") or its origins page, which takes the rules' place.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sheet {
    pub page: Page,
    /// Two to four short lines, and a dimmer note (empty: none).
    pub lines: Vec<String>,
    pub note: String,
    /// The rules as switches (rules page only), by index for `In::Switch`.
    pub switches: Vec<Switch>,
    /// The style the toy is drawn in (rules page only), with its credit as one line.
    pub style: Option<Credit>,
    /// Whether the rules page offers "Show me" (the demo) and "Where this comes from" (the origins).
    pub demo: bool,
    pub origins: bool,
    /// The toy asks for it hung from the top, clear of its scene (`Toy::card_at_top`, asked as it opened).
    pub top: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Page {
    Rules,
    Origins,
}

/// A style's credit: the film or process, who, where, when, and the sheet's line.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Credit {
    pub name: String,
    pub maker: String,
    pub place: String,
    pub year: u16,
    pub credit: String,
}

impl From<Style> for Credit {
    fn from(s: Style) -> Credit {
        Credit {
            name: s.name.into(),
            maker: s.maker.into(),
            place: s.place.into(),
            year: s.year,
            credit: s.credit(),
        }
    }
}

/// Something on screen a screen reader offers as a button: its words and where it is (points).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Button {
    pub label: String,
    pub at: [f64; 2],
}

/// The senses a toy reads (`Toy::senses`): a frame turns on only these. `shake` is the device's own motion
/// (toy-kit's shake.rs), `gravity` its tilt, `field` the magnetic field through it, `heading` the compass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Senses {
    pub shake: bool,
    pub gravity: bool,
    pub field: bool,
    pub heading: bool,
}

impl Default for Senses {
    /// A shake, which any toy may answer (`Toy::shaken`); tilt and compass only for the toys that ask.
    fn default() -> Senses {
        Senses {
            shake: true,
            gravity: false,
            field: false,
            heading: false,
        }
    }
}

impl Senses {
    /// Every sense (a host that holds toys asks for whatever the toy it holds asks for).
    pub fn any(&self) -> bool {
        self.shake || self.gravity || self.field || self.heading
    }
}

/// A haptic, as an intent: each frame plays it in its platform's own (Core Haptics' impacts on an iPhone,
/// Android's composed primitives where the phone has them, else its plain clicks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Haptic {
    /// The lightest: a light landing, a click of a dial.
    #[default]
    Tick,
    /// Soft and round: something giving under a finger.
    Soft,
    /// Firm: a thing struck.
    Firm,
    /// Heavy and low: a wave breaking, thunder.
    Thud,
}

/// What the toy asks of the frame, once (`Frame::events_json`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "event")]
pub enum Out {
    /// Play a haptic.
    Haptic { haptic: Haptic },
    /// Show the platform's file picker for a sound file (`Toy::wants_file`).
    File,
}

/// What the reader did in the frame's own controls, or what the frame says of itself.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum In {
    /// The frame says what it draws itself: with `native_chrome`, the mark (and Terra its Earth mark) are the
    /// frame's controls, and the kit draws neither.
    Frame { native_chrome: bool },
    /// The mark was pressed (`t`: the frame's seconds): the sheet opened or closed, or the demo.
    Mark { t: f64 },
    /// The sheet on its rules page (from closed, or back from the origins).
    Rules,
    /// The origins page in the sheet's place ("Where this comes from").
    Origins,
    /// The demo ("Show me").
    Demo { t: f64 },
    /// The sheet was dismissed in the frame (swiped down, or the system's back).
    Close,
    /// Rule `i` switched on or off.
    Switch { i: usize, on: bool },
    /// A screen reader pressed `buttons[i]`.
    Press { i: usize, t: f64 },
    /// The way back (`Chrome::back`): the frame's back button or the system's back gesture.
    Back { t: f64 },
}

// the chrome's side of the host
impl<T: Toy> Host<T> {
    /// The chrome now.
    pub fn chrome(&self) -> Chrome {
        let m = self.marks;
        let what = if !m.any() {
            MarkKind::None
        } else if m.sheet() {
            MarkKind::Sheet
        } else {
            MarkKind::Demo
        };
        let page = |page: Page, r: &Rules| Sheet {
            page,
            lines: r.lines.clone(),
            note: r.note.clone(),
            switches: Vec::new(),
            style: None,
            demo: false,
            origins: false,
            top: self.sheet_top,
        };
        let sheet = if let Some(r) = self.info.rules() {
            Some(Sheet {
                switches: self.toy.switches(),
                style: self.toy.style().map(Credit::from),
                demo: m.demo,
                origins: m.origins,
                ..page(Page::Rules, r)
            })
        } else {
            self.info.origins().map(|r| page(Page::Origins, r))
        };
        Chrome {
            about: self.toy.about(&self.ctx),
            mark: Mark {
                what,
                off: m.off,
                at: (!self.ctx.native_chrome)
                    .then(|| self.info_spot())
                    .flatten(),
            },
            sheet,
            caption: self.info.line().map(str::to_string),
            label: self.toy.label(&self.ctx),
            title: self.toy.title(),
            buttons: self
                .toy
                .buttons(&self.ctx)
                .into_iter()
                .map(|(label, at)| Button { label, at })
                .collect(),
            back: self.toy.back(&self.ctx),
            music: self.toy.plays_music(),
            senses: self.toy.senses(),
        }
    }

    /// What the reader did in the frame's controls.
    pub fn send(&mut self, e: In) {
        match e {
            In::Frame { native_chrome } => {
                self.ctx.native_chrome = native_chrome;
                self.info.native = native_chrome;
                self.toy.settings(&mut self.ctx);
            }
            In::Mark { t } => self.open_mark(t),
            In::Rules => {
                if self.info.rules().is_none() {
                    // (from the origins: closed first, so the rules open rather than toggle shut)
                    self.info.close();
                    self.open_rules();
                }
            }
            In::Origins => {
                if self.info.origins().is_none() && self.marks.origins {
                    self.open_origins();
                }
            }
            In::Demo { t } => self.open_info(t),
            In::Close => {
                if self.info.sheet_open() {
                    self.info.close();
                }
            }
            In::Switch { i, on } => self.set_switch(i, on),
            In::Press { i, t } => self.press(i, t),
            In::Back { t } => {
                if self.info.is_open() {
                    self.info.close();
                }
                self.toy.go_back(&mut self.ctx, t);
            }
        }
    }

    /// What the toy asked of the frame since the last call (its haptics, a file), forgetting them.
    pub fn take_events(&mut self) -> Vec<Out> {
        let mut out: Vec<Out> = self
            .ctx
            .take_haptics()
            .into_iter()
            .map(|haptic| Out::Haptic { haptic })
            .collect();
        if self.toy.wants_file() {
            out.push(Out::File);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_frame_sends_reads_as_json() {
        let e = |s: &str| serde_json::from_str::<In>(s).unwrap();
        assert_eq!(
            e(r#"{"frame": {"native_chrome": true}}"#),
            In::Frame {
                native_chrome: true
            }
        );
        assert_eq!(e(r#""rules""#), In::Rules);
        assert_eq!(
            e(r#"{"switch": {"i": 1, "on": false}}"#),
            In::Switch { i: 1, on: false }
        );
        assert_eq!(e(r#"{"back": {"t": 2.5}}"#), In::Back { t: 2.5 });
    }

    #[test]
    fn events_are_tagged() {
        let j = serde_json::to_string(&Out::Haptic {
            haptic: Haptic::Thud,
        })
        .unwrap();
        assert_eq!(j, r#"{"event":"haptic","haptic":"thud"}"#);
        assert_eq!(
            serde_json::to_string(&Out::File).unwrap(),
            r#"{"event":"file"}"#
        );
    }
}
