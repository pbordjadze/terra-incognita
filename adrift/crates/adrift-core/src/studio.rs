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
/// exactly; what the clip shows of it is its own (`ClipCamera`): a camera that follows the take's on a
/// critically damped spring (a pinch or fling eased, whatever the frame rate it was recorded at, and no
/// kick where a gesture starts or stops), a still beat before the take's first moment in which the
/// camera eases in from a touch wider, and a beat after its last in which whatever drift the camera
/// had is carried on and settles, the sea moving and the date held. The studio's framing dials
/// (Motion's).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Framing {
    /// Seconds of the take's opening state before its first moment (the drawn motion runs, the date
    /// is held, the camera eases in by `open`).
    pub hold_in: f64,
    /// Seconds after its last moment: the camera's drift settles, touch feedback fades, the date is
    /// held.
    pub hold_out: f64,
    /// The clip's camera follows the take's on a critically damped spring whose answer to a step is
    /// half-way after this many seconds (0: exactly).
    pub follow: f64,
    /// How much wider the opening beat begins, in the zoom's logarithm (0.1: a tenth further out),
    /// eased in to the take's first framing by the beat's end (0: a still opening).
    pub open: f64,
    /// The time constant, seconds, over which the camera's drift at the take's last moment (a fling,
    /// a ride, the idle turn, a pinch still easing) is carried on and dies away in the end beat.
    pub settle: f64,
}

impl Default for Framing {
    fn default() -> Framing {
        Framing { hold_in: 0.8, hold_out: 1.5, follow: 0.3, open: 0.1, settle: 0.3 }
    }
}

/// The clip's camera a step `dt` seconds on toward the take's `target`, with half-life `half`
/// (frame-rate independent: two steps of dt/2 are one of dt). Longitude and roll the short way round,
/// the zoom in its logarithm. The first-order follow (R10's seam); the export uses `ClipCamera`.
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

/// A critically damped spring's step response is half-way at 1.678 time constants.
const HALF_TO_TAU: f64 = 1.678_346_99;

/// The clip's own camera (R10, Motion): it follows the take's camera in rotation (so a turn over a pole
/// is as smooth as anywhere) and the zoom in its logarithm, each on a critically damped spring, so its
/// speed never jumps: a pinch recorded at 20 events a second comes out as one gesture, not steps, and a
/// fling's start and a ride's catch are eased. It also makes the clip's two beats: the opening eases in
/// from `Framing::open` wider; the end carries the take camera's last drift on, dying away over
/// `Framing::settle`, so the clip comes to rest rather than stopping.
#[derive(Clone, Copy, Debug)]
pub struct ClipCamera {
    pub cam: Camera,
    /// The spring's angular velocity (world frame, radians a second) and the zoom's (log a second).
    w: [f64; 3],
    vz: f64,
    /// The take camera last frame and its smoothed velocity (world frame, radians a second; log zoom a
    /// second), measured in the clip's seconds, for the end beat.
    seen: Option<Camera>,
    take_w: [f64; 3],
    take_vz: f64,
    /// The take's first zoom: its pinches are read against it (R12).
    zoom0: f64,
}

impl ClipCamera {
    pub fn new(start: Camera) -> ClipCamera {
        ClipCamera { cam: start, w: [0.0; 3], vz: 0.0, seen: None, take_w: [0.0; 3], take_vz: 0.0, zoom0: start.zoom }
    }

    /// One frame of the clip, `dt` seconds after the last: `take` is the replay's camera at this
    /// frame, `part` and `through` where the frame falls (`Take::clip_at`). Returns the camera to draw.
    pub fn step(&mut self, take: Camera, part: ClipPart, through: f64, dt: f64, f: &Framing, lamps: &Lean) -> Camera {
        let take = lean(take, lamps, self.zoom0);
        match part {
            ClipPart::Before => {
                // the opening: the take's first framing, from a touch wider, eased in (still at both ends)
                let e = crate::touch::motion::ease_in_out(through);
                self.cam = Camera { zoom: take.zoom * (-f.open * (1.0 - e)).exp(), ..take };
                self.w = [0.0; 3];
                self.vz = 0.0;
                self.seen = Some(take);
                self.cam
            }
            ClipPart::During => {
                if let Some(prev) = self.seen.filter(|_| dt > 0.0) {
                    // the take camera's own velocity, smoothed over a tenth of a second
                    let r = rot_log(qmul(cam_quat(take), qconj(cam_quat(prev))));
                    let k = 1.0 - (-dt / 0.1).exp();
                    for i in 0..3 {
                        self.take_w[i] += (r[i] / dt - self.take_w[i]) * k;
                    }
                    self.take_vz += ((take.zoom.ln() - prev.zoom.ln()) / dt - self.take_vz) * k;
                }
                self.seen = Some(take);
                self.spring(take, dt, f)
            }
            ClipPart::After => {
                // the end: the take's last framing, carried on by its drift as that dies away
                let end = self.seen.unwrap_or(take);
                let u = through * f.hold_out;
                let carry = if f.settle > 0.0 { f.settle * (1.0 - (-u / f.settle).exp()) } else { 0.0 };
                let turn = [self.take_w[0] * carry, self.take_w[1] * carry, self.take_w[2] * carry];
                let q = qmul(rot_exp(turn), cam_quat(end));
                let target = Camera::from_rotation(crate::touch::camera::quat_mat(q), end.zoom * (self.take_vz * carry).exp());
                self.spring(target, dt, f)
            }
        }
    }

    fn spring(&mut self, target: Camera, dt: f64, f: &Framing) -> Camera {
        if f.follow <= 0.0 {
            self.cam = target;
            return target;
        }
        let tau = f.follow / HALF_TO_TAU;
        let qt = cam_quat(target);
        // where the camera lies from the target, as a turn (world frame): cam = exp(x) · target
        let x = rot_log(qmul(cam_quat(self.cam), qconj(qt)));
        let mut nx = [0.0; 3];
        for i in 0..3 {
            let (a, v) = crate::touch::motion::spring(x[i], self.w[i], 0.0, tau, dt);
            nx[i] = a;
            self.w[i] = v;
        }
        let (lz, vz) = crate::touch::motion::spring(self.cam.zoom.ln(), self.vz, target.zoom.ln(), tau, dt);
        self.vz = vz;
        self.cam = Camera::from_rotation(crate::touch::camera::quat_mat(qmul(rot_exp(nx), qt)), lz.exp());
        self.cam
    }
}

/// What the camera frames while the hand conducts (R11, R12): the lamps that matter, the last
/// `FRAME_TRAIL` days of their lines, and the room, points: the disc's radius at zoom 1 and the
/// half-width of the view.
#[derive(Clone, Debug, Default)]
pub struct Lean {
    pub lamps: Vec<crate::model::LonLat>,
    pub trail: Vec<crate::model::LonLat>,
    pub r1: f64,
    pub half: f64,
    /// A lamp just cued alone (R12): its place, its line, and how far the frame leans in on it, 0..1.
    pub focus: Option<(Vec<crate::model::LonLat>, Vec<crate::model::LonLat>, f64)>,
}

/// How far the clip leans in on a lamp cued alone, `age` seconds after the cue (R12): in over half a
/// second, held to 1.5 s, back out to every lamp by 4 s; at most `FOCUS_MAX` of the way.
pub fn focus_weight(age: f64) -> f64 {
    let ss = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    if age < 0.0 {
        return 0.0;
    }
    FOCUS_MAX * ss(age / 0.5) * (1.0 - ss((age - 1.5) / 2.5))
}
pub const FOCUS_MAX: f64 = 0.6;
/// The lean on a cued lamp stops where any lamp out would pass this share of the half-width.
pub const FOCUS_HOLD: f64 = 0.75;

/// The lamps' span, their recent lines and a margin, fills this share of the view's width (R12: the
/// round-11 critic; at zoom 3 a whole take's conducting was a hook a few millimetres long).
pub const FRAME_FILL: f64 = 0.6;
/// Days of each lamp's line the frame holds.
pub const FRAME_TRAIL: f64 = 6.0;
/// Km of margin round the span (the cue's fan, the lamp's glow).
pub const FRAME_MARGIN_KM: f64 = 25.0;
/// The least half-span, km: a lone lamp with a short line is framed so a hundred km fill 60% of the
/// width's half, where a stroke's tens of km are tens of points.
pub const FRAME_SPAN_MIN_KM: f64 = 100.0;
/// The deepest the frame comes in (the reader's own pinch stops at `motion::ZOOM_MAX` otherwise).
pub const FRAME_ZOOM_MAX: f64 = 40.0;
/// How much of the take's own zoom (against the take's first) the clip keeps on top of the frame, in
/// the zoom's log. 0 since R12: while conducting, the take's camera itself comes in to the lamps (the
/// live hold), so its zoom is mostly the frame's, not the reader's pinch, and counting it doubled
/// the push-in.
pub const FRAME_PINCH: f64 = 0.0;
const EARTH_KM: f64 = 6371.0;

/// The centre and zoom that frame the lamps (R12): the centre of the smallest circle round the lamps
/// and their lines (Bădoiu–Clarkson, on the sphere), the zoom at which that circle and its margin fill
/// `FRAME_FILL` of the width. None without lamps.
pub fn frame(l: &Lean) -> Option<(crate::model::LonLat, f64)> {
    let all = frame_of(&l.lamps, &l.trail, l.r1, l.half)?;
    match &l.focus {
        Some((lamps, trail, w)) if *w > 0.0 => {
            let one = frame_of(lamps, trail, l.r1, l.half)?;
            // the zoom in its log; the centre at least as far toward the lamp as keeps it no further
            // out on the screen than it lay in the frame of them all (the zoom grows faster than a
            // plain blend of centres would follow)
            let at = |w: f64| {
                let zoom = (all.1.ln() + (one.1.ln() - all.1.ln()) * w).exp();
                let wc = w.max(1.0 - all.1 / zoom);
                let (a, b) = (all.0.to_xyz(), one.0.to_xyz());
                let v = [a[0] + (b[0] - a[0]) * wc, a[1] + (b[1] - a[1]) * wc, a[2] + (b[2] - a[2]) * wc];
                (crate::model::LonLat::from_xyz(v), zoom)
            };
            // never so far in that another lamp leaves the frame (the lean is as much as keeps them all)
            let holds = |(c, z): (crate::model::LonLat, f64)| {
                let cam = Camera::new(c.lon, c.lat, z);
                l.lamps.iter().all(|p| {
                    let v = cam.view(p.to_xyz());
                    v[2] > 0.0 && (v[0] * v[0] + v[1] * v[1]).sqrt() * l.r1 * z <= FOCUS_HOLD * l.half
                })
            };
            if holds(at(*w)) {
                return Some(at(*w));
            }
            let (mut lo, mut hi) = (0.0, *w);
            for _ in 0..14 {
                let mid = (lo + hi) / 2.0;
                if holds(at(mid)) { lo = mid } else { hi = mid }
            }
            Some(at(lo))
        }
        _ => Some(all),
    }
}

fn frame_of(lamps: &[crate::model::LonLat], trail: &[crate::model::LonLat], r1: f64, half: f64) -> Option<(crate::model::LonLat, f64)> {
    if lamps.is_empty() || r1 <= 0.0 || half <= 0.0 {
        return None;
    }
    let pts: Vec<[f64; 3]> = lamps.iter().chain(trail).map(|p| p.to_xyz()).collect();
    let dot = |a: [f64; 3], b: [f64; 3]| (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let norm = |v: [f64; 3]| {
        let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
        [v[0] / m, v[1] / m, v[2] / m]
    };
    let mut c = norm(pts.iter().fold([0.0; 3], |a, p| [a[0] + p[0], a[1] + p[1], a[2] + p[2]]));
    for i in 1..=40 {
        let far = *pts.iter().min_by(|a, b| dot(**a, c).total_cmp(&dot(**b, c))).unwrap();
        let k = 1.0 / (i as f64 + 1.0);
        c = norm([c[0] + (far[0] - c[0]) * k, c[1] + (far[1] - c[1]) * k, c[2] + (far[2] - c[2]) * k]);
    }
    let spread = pts.iter().map(|p| dot(*p, c).acos()).fold(0.0, f64::max);
    let half_span = (spread + FRAME_MARGIN_KM / EARTH_KM).max(FRAME_SPAN_MIN_KM / EARTH_KM).min(std::f64::consts::FRAC_PI_2);
    let zoom = (FRAME_FILL * half / (r1 * half_span.sin())).clamp(crate::touch::motion::ZOOM_MIN, FRAME_ZOOM_MAX);
    Some((crate::model::LonLat::from_xyz(c), zoom))
}

/// The clip's camera: the lamps framed (`frame`), the take's own pinch kept by half against its first
/// zoom `zoom0`, the take's roll; the take's own camera when there are no lamps. Continuous in the
/// lamps, so the spring after it never sees a jump.
pub fn lean(take: Camera, l: &Lean, zoom0: f64) -> Camera {
    match frame(l) {
        None => take,
        Some((c, z)) => {
            let pinch = (take.zoom / zoom0.max(1e-6)).powf(FRAME_PINCH);
            Camera { lon: c.lon, lat: c.lat, zoom: (z * pinch).clamp(crate::touch::motion::ZOOM_MIN, FRAME_ZOOM_MAX), roll: take.roll }
        }
    }
}

fn cam_quat(c: Camera) -> [f64; 4] {
    crate::touch::camera::mat_quat(c.rotation())
}

fn qmul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

fn qconj(q: [f64; 4]) -> [f64; 4] {
    [q[0], -q[1], -q[2], -q[3]]
}

/// A unit quaternion as its rotation vector (axis × angle, the short way).
fn rot_log(q: [f64; 4]) -> [f64; 3] {
    let q = if q[0] < 0.0 { [-q[0], -q[1], -q[2], -q[3]] } else { q };
    let s = (q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if s < 1e-12 {
        return [2.0 * q[1], 2.0 * q[2], 2.0 * q[3]];
    }
    let a = 2.0 * s.atan2(q[0]) / s;
    [q[1] * a, q[2] * a, q[3] * a]
}

fn rot_exp(v: [f64; 3]) -> [f64; 4] {
    let th = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if th < 1e-12 {
        return [1.0, v[0] / 2.0, v[1] / 2.0, v[2] / 2.0];
    }
    let (s, c) = (th / 2.0).sin_cos();
    [c, v[0] / th * s, v[1] / th * s, v[2] / th * s]
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
    /// The song that played from its start as the take began, by the name the shell gave it (R12:
    /// the reader conducts to it; the export lays it under the clip from the take's first moment).
    #[serde(default)]
    pub song: Option<String>,
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
    /// Where the take's song begins in its clip, seconds (the still opening beat), and how long it
    /// runs before the clip's end; None without a song, or for a clip not at the take's own speed
    /// (a slowed song is not the song).
    pub fn song_in_clip(&self) -> Option<(f64, f64)> {
        self.song.as_ref()?;
        if (self.speed - 1.0).abs() > 1e-9 {
            return None;
        }
        Some((self.framing.hold_in, self.clip_seconds() - self.framing.hold_in))
    }

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
    /// The reader asked for the studio's menu (`Intent::Menu`, R13): the shell polls and clears it.
    pub menu_wanted: bool,
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
    pub cam: ClipCamera,
    /// The drawn motion's seconds the clip has added in its held beats (the date held, the sea moving).
    pub held: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LonLat;

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
        let take = Take { start, events: vec![], t0: 10.0, t1: 14.0, width: 100, height: 100, ppp: 1.0, trim: None, end: None, speed: 0.5, framing: Framing::default(), song: None };
        let f = take.framing;
        assert!((take.clip_seconds() - (f.hold_in + 8.0 + f.hold_out)).abs() < 1e-9);
        assert_eq!(take.clip_at(0.2).1, ClipPart::Before);
        let (t, part, _) = take.clip_at(f.hold_in + 4.0);
        assert_eq!(part, ClipPart::During);
        assert!((t - 12.0).abs() < 1e-9);
        let late = f.hold_in + 8.0 + 0.1;
        assert_eq!(take.clip_at(late).1, ClipPart::After);
        assert!((take.clip_at(late).0 - 14.0).abs() < 1e-9);
    }

    /// A take camera's path: still, then a pinch in sparse steps (20 events a second, the zoom jumping
    /// at each), then a fling turning east at 40° a second until the take ends mid-fling.
    fn take_cam(t: f64) -> Camera {
        let zoom = if t < 0.5 { 1.2 } else if t < 1.0 { 1.2 * (1.0 + ((t - 0.5) * 20.0).floor() / 10.0) } else { 2.4 };
        let lon = if t < 1.0 { -40.0 } else { -40.0 + 40.0 * (t - 1.0) };
        Camera::new(lon, 20.0, zoom)
    }

    /// The clip's camera through a whole clip at `fps`: (clip seconds, camera) a frame.
    fn run_clip(fps: f64) -> Vec<(f64, Camera)> {
        let start = TakeStart { floats: vec![], next_id: 1, clock: Clock { day: 0.0, running: true, rate: 1.0 }, camera: take_cam(0.0), chosen: vec![], followed: vec![], riding: None, seconds: 0.0, night: false, reduce_motion: false, credits_open: false };
        let take = Take { start, events: vec![], t0: 0.0, t1: 1.6, width: 100, height: 100, ppp: 1.0, trim: None, end: None, speed: 1.0, framing: Framing::default(), song: None };
        let mut cc = ClipCamera::new(take.start.camera);
        let n = (take.clip_seconds() * fps).ceil() as usize;
        (0..n)
            .map(|i| {
                let tau = i as f64 / fps;
                let (t, part, through) = take.clip_at(tau);
                (tau, cc.step(take_cam(t), part, through, 1.0 / fps, &take.framing, &Lean::default()))
            })
            .collect()
    }

    #[test]
    fn the_clip_eases_in_follows_without_kicks_and_comes_to_rest() {
        let f = Framing::default();
        let c = run_clip(60.0);
        // the opening: a touch wider at the first frame, the take's own framing by the beat's end
        assert!((c[0].1.zoom - 1.2 * (-f.open).exp()).abs() < 1e-9, "{:?}", c[0].1);
        let at = |s: f64| c.iter().find(|(t, _)| *t >= s - 1e-9).unwrap().1;
        assert!((at(f.hold_in).zoom - 1.2).abs() < 1e-3);
        // the pinch's steps come out smooth: the zoom's rate never changes by more than a little a frame
        let mut worst: f64 = 0.0;
        for w in c.windows(3) {
            let r0 = w[1].1.zoom.ln() - w[0].1.zoom.ln();
            let r1 = w[2].1.zoom.ln() - w[1].1.zoom.ln();
            worst = worst.max((r1 - r0).abs());
        }
        // (the take's own zoom jumps by ln 1.1 ≈ 0.095 at a step; the clip's rate changes by a sliver)
        assert!(worst < 0.004, "the zoom's rate jumped by {worst} in a frame");
        // the fling at the take's end is carried on and dies away: still moving east just after,
        // and at rest by the clip's last frame
        let lon = |i: usize| c[i].1.lon;
        let n = c.len();
        let after = c.iter().position(|(t, _)| *t >= f.hold_in + 1.6).unwrap();
        assert!(lon(after + 6) - lon(after) > 0.5, "the drift stopped dead");
        let last = (lon(n - 1) - lon(n - 2)) * 60.0;
        assert!(last.abs() < 1.5, "still turning {last}°/s at the end");
        assert!(lon(n - 1) > take_cam(1.6).lon + 5.0, "the drift carried {}°", lon(n - 1) - take_cam(1.6).lon);
    }

    #[test]
    fn the_clip_camera_is_the_same_at_20_and_60_fps() {
        let a = run_clip(60.0);
        let b = run_clip(20.0);
        for (tau, cam) in &b {
            let other = a.iter().min_by(|x, y| (x.0 - tau).abs().partial_cmp(&(y.0 - tau).abs()).unwrap()).unwrap().1;
            assert!((cam.lon - other.lon).abs() < 1.0 && (cam.zoom.ln() - other.zoom.ln()).abs() < 0.03, "at {tau}: {cam:?} vs {other:?}");
        }
    }

    #[test]
    fn the_clip_follows_over_a_pole_without_a_swing() {
        // a turn straight over the north pole: lon and roll flip, the picture doesn't
        let mut cc = ClipCamera::new(Camera::new(0.0, 80.0, 2.0));
        let mut last = cc.cam;
        let f = Framing::default();
        for i in 0..120 {
            let lat = 80.0 + i as f64 * 0.2;
            let target = if lat <= 90.0 { Camera::new(0.0, lat, 2.0) } else { Camera { lon: 180.0, lat: 180.0 - lat, zoom: 2.0, roll: 180.0 } };
            let cam = cc.step(target, ClipPart::During, 0.0, 1.0 / 60.0, &f, &Lean::default());
            let d = LonLat::new(cam.lon, cam.lat).to_xyz();
            let e = LonLat::new(last.lon, last.lat).to_xyz();
            let step = (d[0] * e[0] + d[1] * e[1] + d[2] * e[2]).clamp(-1.0, 1.0).acos().to_degrees();
            assert!(step < 1.0, "jumped {step}° at {cam:?}");
            last = cam;
        }
    }

    #[test]
    fn the_frame_fills_six_tenths_with_the_lamps_and_shows_a_lone_lamps_tens_of_km() {
        let km_pt = |zoom: f64| 166.0 * zoom / EARTH_KM;
        // a lone lamp, a short line: close enough that 40 km is plainly visible (≥ 30 points)
        let lone = Lean { lamps: vec![LonLat::new(-72.0, 35.0)], trail: vec![LonLat::new(-72.2, 35.05)], r1: 166.0, half: 180.0, focus: None };
        let (c, z) = frame(&lone).unwrap();
        assert!(40.0 * km_pt(z) >= 30.0 && (c.lon + 72.1).abs() < 0.2, "{c:?} {z}");
        // two lamps a thousand km apart: their span (with the margin) fills 60% of the width
        let two = Lean { lamps: vec![LonLat::new(-72.0, 35.0), LonLat::new(-61.0, 37.0)], trail: vec![], r1: 166.0, half: 180.0, focus: None };
        let (c, z) = frame(&two).unwrap();
        let cam = Camera::new(c.lon, c.lat, z);
        let reach = two.lamps.iter().map(|p| { let v = cam.view(p.to_xyz()); (v[0] * v[0] + v[1] * v[1]).sqrt() * 166.0 * z }).fold(0.0, f64::max);
        assert!(reach > 0.45 * 180.0 && reach < 0.6 * 180.0, "the furthest lamp {reach} pt from the centre");
        // continuous: the line growing a little moves the frame a little
        let mut more = lone.clone();
        more.trail.push(LonLat::new(-72.25, 35.06));
        let (c2, z2) = frame(&more).unwrap();
        let (c1, z1) = frame(&lone).unwrap();
        assert!((c2.lon - c1.lon).abs() < 0.05 && (z2 / z1 - 1.0).abs() < 0.1, "{c1:?} {z1} → {c2:?} {z2}");
        // the clip frames the lamps whatever the take's zoom; no lamps, the take's camera
        let a = lean(Camera::new(-50.0, 30.0, 2.0), &two, 1.0);
        assert!((a.zoom / z - 1.0).abs() < 1e-6, "{a:?}");
        assert_eq!(lean(Camera::new(-100.0, 10.0, 1.3), &Lean::default(), 1.3), Camera::new(-100.0, 10.0, 1.3));
        // a lamp cued alone: the frame leans in on it, then eases back to both; never a jump
        assert_eq!(focus_weight(-0.1), 0.0);
        let mut last = frame(&two).unwrap();
        for i in 0..=100 {
            let age = i as f64 * 0.05;
            let f = Lean { focus: Some((vec![LonLat::new(-72.0, 35.0)], vec![], focus_weight(age))), ..two.clone() };
            let now = frame(&f).unwrap();
            assert!((now.1 / last.1).ln().abs() < 0.15 && (now.0.lon - last.0.lon).abs() < 0.8, "at {age}: {last:?} → {now:?}");
            let cam = Camera::new(now.0.lon, now.0.lat, now.1);
            let v = cam.view(LonLat::new(-72.0, 35.0).to_xyz());
            assert!((v[0] * v[0] + v[1] * v[1]).sqrt() * 166.0 * now.1 <= 0.62 * 180.0, "the cued lamp out of frame at {age}");
            let o = cam.view(LonLat::new(-61.0, 37.0).to_xyz());
            assert!((o[0] * o[0] + o[1] * o[1]).sqrt() * 166.0 * now.1 <= 0.91 * 180.0, "the other lamp out of frame at {age}");
            if (age - 1.0).abs() < 1e-9 {
                assert!(now.1 > 1.1 * z && (now.0.lon + 72.0).abs() < 6.0, "not in on the lamp: {now:?}");
            }
            last = now;
        }
        assert!((last.1 / z - 1.0).abs() < 1e-6, "not back out to both: {last:?}");
    }
}
