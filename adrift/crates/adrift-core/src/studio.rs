//! The studio's first tool (R9, Engineering; painting-company row 199): a take, recorded as data and
//! rendered to video offline.
//!
//! A take is the state when recording began (`TakeStart`) and everything the shell fed the app while
//! it ran: each frame's time, each touch, wheel, key and shake, and any intent applied from outside
//! (`App::command`). The app is deterministic for its inputs (the sea's rules are pure; the stage's
//! budget is locked while exporting), so replaying a take from its start gives the same clip, which is
//! then rendered at a fixed rate into an offscreen target, frame by frame, at full quality, whatever
//! the phone managed live. Beginning a take resets the live app to the same start (camera inertia,
//! gestures, ripples), so the live view and the replay agree from the first frame.
//!
//! The ocean is the first preset: a take of it replays the ocean. What a later preset changes, a take
//! of it carries the same way.

use crate::model::{Camera, Clock, Float, Intent, PointerEvent};
use crate::touch::Key;
use serde::{Deserialize, Serialize};

/// The longest take, seconds of the shell's clock (a take ends itself there).
pub const TAKE_MAX: f64 = 60.0;
/// The exported clip's size: 9:16, the short-form platforms' frame (a take's screen is cropped to
/// it about the globe's centre and scaled).
pub const CLIP: (u32, u32) = (1080, 1920);
/// How many takes are kept (the newest last).
pub const TAKES_KEPT: usize = 5;

/// The state a take starts from: what the sea and the reader's choices were.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TakeStart {
    pub floats: Vec<Float>,
    pub next_id: u32,
    pub clock: Clock,
    pub camera: Camera,
    pub chosen: Vec<u32>,
    pub followed: Vec<u32>,
    pub riding: Option<u32>,
    /// The drawn motion's seconds.
    pub seconds: f64,
    pub night: bool,
    pub reduce_motion: bool,
    pub credits_open: bool,
}

/// One thing the shell fed the app during a take, at the shell's time `t`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TakeEvent {
    Frame { t: f64 },
    Pointer(PointerEvent),
    Wheel { t: f64, x: f64, y: f64, factor: f64 },
    Key { t: f64, key: Key },
    Shake { t: f64 },
    Intent { t: f64, intent: Intent },
}

impl TakeEvent {
    pub fn t(&self) -> f64 {
        match self {
            TakeEvent::Frame { t } | TakeEvent::Wheel { t, .. } | TakeEvent::Key { t, .. } | TakeEvent::Shake { t } | TakeEvent::Intent { t, .. } => *t,
            TakeEvent::Pointer(p) => p.t,
        }
    }
}

/// A recorded take.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Take {
    pub start: TakeStart,
    pub events: Vec<TakeEvent>,
    /// The shell's time when it began and ended.
    pub t0: f64,
    pub t1: f64,
    /// The screen it was recorded on: pixels and pixels a point (a replay needs the same layout, as
    /// touches are in points).
    pub width: u32,
    pub height: u32,
    pub ppp: f64,
    /// The part to export, seconds from `t0` (the whole take by default): the first timing tool.
    #[serde(default)]
    pub trim: Option<(f64, f64)>,
    /// The state when it ended (for the check: a replay must end where the take did).
    #[serde(default)]
    pub end: Option<TakeStart>,
}

impl Take {
    pub fn duration(&self) -> f64 {
        (self.t1 - self.t0).max(0.0)
    }

    /// The exported span, seconds from `t0`.
    pub fn span(&self) -> (f64, f64) {
        let d = self.duration();
        match self.trim {
            Some((a, b)) => (a.clamp(0.0, d), b.clamp(0.0, d).max(a.clamp(0.0, d))),
            None => (0.0, d),
        }
    }
}

/// The studio's state in the app: the take being recorded, the takes kept, an export under way.
#[derive(Default)]
pub struct Studio {
    pub recording: Option<Take>,
    pub takes: Vec<Take>,
    /// The reader asked to export the newest take (`Intent::Export`): the shell polls and clears it.
    pub export_wanted: bool,
    pub(crate) replay: Option<Replay>,
}

/// An export in progress: the take, where its events have been fed to, the next output frame, and
/// what the live app was, to put back.
pub(crate) struct Replay {
    pub take: Take,
    pub next_event: usize,
    pub frame: u32,
    pub frames: u32,
    pub fps: f64,
    pub live: crate::app::Stash,
}
