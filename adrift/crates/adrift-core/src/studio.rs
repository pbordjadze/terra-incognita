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
/// The clip's pixels a point (R10): the clip is drawn at its own size, 360 × 640 points, a phone's
/// reading scale, not cropped from the take's screen.
pub const CLIP_PPP: f64 = 3.0;

/// How a clip frames itself (R10: the clip is made, not captured). The take's state is replayed
/// exactly; what the clip shows of it is its own: a camera that follows the take's with a little lag
/// (a pinch or fling eased, whatever the frame rate it was recorded at), a still beat before the
/// take's first moment and an eased one after its last, the sea moving and the date held. These are
/// the first of the studio's framing dials (Motion's to tune, by demand or in the round's brief).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Framing {
    /// Seconds of the take's opening state before its first moment (the drawn motion runs, the date
    /// is held).
    pub hold_in: f64,
    /// Seconds after its last moment: the camera settles, touch feedback fades, the date is held.
    pub hold_out: f64,
    /// The clip's camera follows the take's with this half-life, seconds (0: exactly).
    pub follow: f64,
}

impl Default for Framing {
    fn default() -> Framing {
        Framing { hold_in: 0.5, hold_out: 0.9, follow: 0.12 }
    }
}

/// The clip's camera a step `dt` seconds on toward the take's `target`, with half-life `half`
/// (frame-rate independent: two steps of dt/2 are one of dt). Longitude and roll the short way round,
/// the zoom in its logarithm, so a pinch eases evenly in and out.
pub fn follow(cam: Camera, target: Camera, dt: f64, half: f64) -> Camera {
    if half <= 0.0 {
        return target;
    }
    let k = 1.0 - 0.5f64.powf(dt / half);
    let wrap = |d: f64| (d + 540.0).rem_euclid(360.0) - 180.0;
    Camera {
        lon: wrap(cam.lon + wrap(target.lon - cam.lon) * k),
        lat: cam.lat + (target.lat - cam.lat) * k,
        zoom: (cam.zoom.ln() + (target.zoom.ln() - cam.zoom.ln()) * k).exp(),
        roll: wrap(cam.roll + wrap(target.roll - cam.roll) * k),
    }
}

/// The window of a screen `w` × `h` (any unit) that a 9:16 clip would show of it: as large as the
/// screen allows, centred on `cy` (the globe's centre) vertically, clamped to the screen; x, y, w, h.
/// One function for the app and the look (R10; the clip is now drawn at its own size, so this is for
/// whatever on the live screen wants to show the clip's frame).
pub fn clip_window(w: f64, h: f64, cy: f64) -> [f64; 4] {
    let (cw, ch) = (CLIP.0 as f64, CLIP.1 as f64);
    if h / w >= ch / cw {
        let wh = w * ch / cw;
        [0.0, (cy - wh / 2.0).clamp(0.0, (h - wh).max(0.0)), w, wh]
    } else {
        let ww = h * cw / ch;
        [(w - ww) / 2.0, 0.0, ww, h]
    }
}

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
    /// The clip's speed (R10): 0.5 is a slow-motion clip twice the take's length (the replay's times
    /// scaled; the take itself is unchanged). `Intent::TakeSpeed` sets the newest take's.
    #[serde(default = "one")]
    pub speed: f64,
    /// How the clip frames itself (R10).
    #[serde(default)]
    pub framing: Framing,
}

fn one() -> f64 {
    1.0
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

    /// The clip's length in seconds: the still beat, the span at the take's speed, the eased beat.
    pub fn clip_seconds(&self) -> f64 {
        let (a, b) = self.span();
        self.framing.hold_in + (b - a) / self.speed.max(0.05) + self.framing.hold_out
    }

    /// Where a clip's second `tau` falls: the take's shell time to replay up to, and which part
    /// of the clip it is in (`Before` the take's first moment, `During`, `After` its last), with how
    /// far through that part, 0..1.
    pub fn clip_at(&self, tau: f64) -> (f64, ClipPart, f64) {
        let (a, b) = self.span();
        let f = self.framing;
        let during = (b - a) / self.speed.max(0.05);
        if tau < f.hold_in {
            (self.t0 + a, ClipPart::Before, if f.hold_in > 0.0 { tau / f.hold_in } else { 1.0 })
        } else if tau < f.hold_in + during {
            let u = tau - f.hold_in;
            (self.t0 + a + u * self.speed, ClipPart::During, if during > 0.0 { u / during } else { 1.0 })
        } else {
            let u = tau - f.hold_in - during;
            (self.t0 + b, ClipPart::After, if f.hold_out > 0.0 { (u / f.hold_out).min(1.0) } else { 1.0 })
        }
    }
}

/// The part of a clip a frame is in (R10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipPart {
    Before,
    During,
    After,
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
    /// The clip's own camera, following the take's (R10).
    pub cam: Camera,
    /// The drawn motion's seconds the clip has added in its held beats (the date held, the sea moving).
    pub held: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_is_frame_rate_independent() {
        let a = Camera::new(170.0, 10.0, 1.0);
        let b = Camera::new(-170.0, 20.0, 4.0);
        let one = follow(a, b, 0.1, 0.12);
        let two = follow(follow(a, b, 0.05, 0.12), b, 0.05, 0.12);
        assert!((one.lon - two.lon).abs() < 1e-9 && (one.lat - two.lat).abs() < 1e-9 && (one.zoom - two.zoom).abs() < 1e-9);
        // the short way round the date line
        assert!(one.lon > 170.0 || one.lon < -170.0);
    }

    #[test]
    fn clip_parts() {
        let start = TakeStart { floats: vec![], next_id: 1, clock: Clock { day: 0.0, running: true, rate: 1.0 }, camera: Camera::new(0.0, 0.0, 1.0), chosen: vec![], followed: vec![], riding: None, seconds: 0.0, night: false, reduce_motion: false, credits_open: false };
        let take = Take { start, events: vec![], t0: 10.0, t1: 14.0, width: 100, height: 100, ppp: 1.0, trim: None, end: None, speed: 0.5, framing: Framing::default() };
        assert!((take.clip_seconds() - (0.5 + 8.0 + 0.9)).abs() < 1e-9);
        assert_eq!(take.clip_at(0.2).1, ClipPart::Before);
        let (t, part, _) = take.clip_at(0.5 + 4.0);
        assert_eq!(part, ClipPart::During);
        assert!((t - 12.0).abs() < 1e-9);
        assert_eq!(take.clip_at(9.0).1, ClipPart::After);
        assert!((take.clip_at(9.0).0 - 14.0).abs() < 1e-9);
    }
}
