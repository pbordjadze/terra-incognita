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
    /// (R16) A journey clip: through its last third the frame holds the whole braid, both lines since
    /// the clip began (or their most recent part, if all of it would be wider than the opening).
    #[serde(default)]
    pub braid: bool,
}

impl Default for Framing {
    fn default() -> Framing {
        Framing {
            hold_in: 0.8,
            hold_out: 1.5,
            follow: 0.3,
            open: 0.1,
            settle: 0.3,
            braid: false,
        }
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
/// The target's velocity, for the clip spring's lead, is smoothed over this many seconds (R14).
pub const LEAD_SMOOTH: f64 = 0.25;
/// The clip's zoom follows this many times slower than its turn (R14).
pub const ZOOM_SLOWER: f64 = 2.0;

/// The clip's own camera (R10, Motion): it follows the take's camera in rotation (so a turn over a pole
/// is as smooth as anywhere) and the zoom in its logarithm, each on a critically damped spring, so its
/// speed never jumps: a pinch recorded at 20 events a second comes out as one gesture, not steps, and a
/// fling's start and a ride's catch are eased. It also makes the clip's two beats: the opening eases in
/// from `Framing::open` wider; the end carries the take camera's last drift on, dying away over
/// `Framing::settle`, so the clip comes to rest rather than stopping.
#[derive(Clone, Debug)]
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
    /// The framed target last frame and its smoothed velocity (R14), for the spring's lead.
    last_target: Option<Camera>,
    target_w: [f64; 3],
    target_vz: f64,
    /// How much of the lead is taken: all of it during the take, fading out over the end beat's
    /// first 0.3 s (its drift is already carried on and settled there).
    lead_scale: f64,
    /// (R15) The clip's seconds so far, the framed camera before its lift (its turn's spring
    /// velocity, its zoom's), and the frame the end beat pulls out from.
    t: f64,
    fc: Option<Camera>,
    fw: [f64; 3],
    fzv: f64,
    end_from: Option<(crate::model::LonLat, f64)>,
    /// The opening's widest zoom (R16): the braid is never framed wider than the clip began.
    open_zoom: Option<f64>,
    /// (R18) A journey's establishing zoom, its first frame's.
    establish: Option<f64>,
    /// (R20) The limiter's own velocities: the zoom's (log a second) and the turn's (radians a second,
    /// world frame), so the rates themselves ease in and out.
    lim_vz: f64,
    lim_w: [f64; 3],
    /// (R22) A journey's pair: its centre last frame and that centre's smoothed angular velocity
    /// (world frame, radians a second), for the pan's lead and its limits in the pair's own frame.
    last_c: Option<crate::model::LonLat>,
    cv: [f64; 3],
    /// (R22) The zoom the frame wants last frame and its smoothed rate (log a second), for the zoom's
    /// lead as a pair parts.
    last_want: Option<f64>,
    want_rate: f64,
    /// (R22) Each lamp's place last frame and its smoothed velocity (unit-sphere xyz a second): the
    /// frame holds where each will be `LAMP_LEAD` seconds on.
    lamp_prev: Vec<[f64; 3]>,
    lamp_v: Vec<[f64; 3]>,
}

impl ClipCamera {
    pub fn new(start: Camera) -> ClipCamera {
        ClipCamera {
            cam: start,
            w: [0.0; 3],
            vz: 0.0,
            seen: None,
            take_w: [0.0; 3],
            take_vz: 0.0,
            zoom0: start.zoom,
            last_target: None,
            target_w: [0.0; 3],
            target_vz: 0.0,
            lead_scale: 1.0,
            t: 0.0,
            fc: None,
            fw: [0.0; 3],
            fzv: 0.0,
            end_from: None,
            open_zoom: None,
            establish: None,
            lim_vz: 0.0,
            lim_w: [0.0; 3],
            last_c: None,
            cv: [0.0; 3],
            last_want: None,
            want_rate: 0.0,
            lamp_prev: Vec::new(),
            lamp_v: Vec::new(),
        }
    }

    /// One frame of the clip, `dt` seconds after the last: `take` is the replay's camera at this
    /// frame, `part` and `through` where the frame falls (`Take::clip_at`). Returns the camera to draw.
    pub fn step(
        &mut self,
        take: Camera,
        part: ClipPart,
        through: f64,
        dt: f64,
        f: &Framing,
        lamps: &Lean,
    ) -> Camera {
        let t = self.t;
        self.t += dt;
        if let Some(cam) = self.framed(take, part, through, t, dt, f, lamps) {
            self.cam = cam;
            return cam;
        }
        let take = lean(take, lamps, self.zoom0);
        match part {
            ClipPart::Before => {
                // the opening: the take's first framing, from a touch wider, eased in (still at both ends)
                let e = crate::touch::motion::ease_in_out(through);
                self.cam = Camera {
                    zoom: take.zoom * (-f.open * (1.0 - e)).exp(),
                    ..take
                };
                self.w = [0.0; 3];
                self.vz = 0.0;
                self.seen = Some(take);
                self.cam
            }
            ClipPart::During => {
                self.lead_scale = 1.0;
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
                self.lead_scale = (1.0 - u / 0.3).max(0.0);
                let carry = if f.settle > 0.0 {
                    f.settle * (1.0 - (-u / f.settle).exp())
                } else {
                    0.0
                };
                let turn = [
                    self.take_w[0] * carry,
                    self.take_w[1] * carry,
                    self.take_w[2] * carry,
                ];
                let q = qmul(rot_exp(turn), cam_quat(end));
                let target = Camera::from_rotation(
                    crate::touch::camera::quat_mat(q),
                    end.zoom * (self.take_vz * carry).exp(),
                );
                self.spring(target, dt, f)
            }
        }
    }

    /// (R15) The clip framing its lamps, None without any (the take's own camera then, as before).
    /// The centre follows the lamps' frame on a slow spring (`PAN_HALF`); the zoom aims at the frame's,
    /// never deeper than holds every lamp and the newest of its line from where the centre is now
    /// (`hold_zoom`), so when the pair parts or runs the camera pulls back rather than pans, and closes
    /// in again as they near; out on a quick spring, in on a slow one. The opening starts `OPEN_WIDE`
    /// wider and eases in over `OPEN_SECS` of the clip; the end beat pulls out to the take's whole
    /// lines. The result lifted (`lifted`) so the lower third stays clear.
    #[allow(clippy::too_many_arguments)]
    fn framed(
        &mut self,
        take: Camera,
        part: ClipPart,
        through: f64,
        t: f64,
        dt: f64,
        f: &Framing,
        l: &Lean,
    ) -> Option<Camera> {
        // (R22) a journey frames the pair and its last few days of line, as close as the coast in view
        // allows (`journey_lean`), coming in as they close and pulling out as they part; the braid's
        // whole shape is the end beat's
        let journey = f.braid;
        let mut jl;
        let l: &Lean = if journey {
            jl = journey_lean(l);
            // (R22) where each lamp will be `LAMP_LEAD` seconds on, held in the frame too, so the
            // limited camera pulls out and pans ahead of a pair that parts or runs, not after it
            let now: Vec<[f64; 3]> = jl.lamps.iter().map(|p| p.to_xyz()).collect();
            if now.len() != self.lamp_prev.len() || dt <= 0.0 {
                self.lamp_v = vec![[0.0; 3]; now.len()];
            } else {
                let k = 1.0 - (-dt / LAMP_SMOOTH).exp();
                for (i, (a, b)) in self.lamp_prev.iter().zip(&now).enumerate() {
                    for j in 0..3 {
                        self.lamp_v[i][j] += ((b[j] - a[j]) / dt - self.lamp_v[i][j]) * k;
                    }
                }
            }
            // against the pair's mean motion: their parting and closing (their run together is the
            // pan's own lead)
            let n = self.lamp_v.len().max(1) as f64;
            let m = self.lamp_v.iter().fold([0.0; 3], |a, v| {
                [a[0] + v[0] / n, a[1] + v[1] / n, a[2] + v[2] / n]
            });
            for (p, v) in now.iter().zip(&self.lamp_v) {
                let d = [
                    (v[0] - m[0]) * LAMP_LEAD,
                    (v[1] - m[1]) * LAMP_LEAD,
                    (v[2] - m[2]) * LAMP_LEAD,
                ];
                jl.trail.push(crate::model::LonLat::from_xyz([
                    p[0] + d[0],
                    p[1] + d[1],
                    p[2] + d[2],
                ]));
            }
            self.lamp_prev = now;
            &jl
        } else {
            l
        };
        let (c, z) = frame(l)?;
        // (R17) centred on the pair itself (its bounding circle's centre), the lines held by the hold
        // box from there rather than pulling the pair to the frame's edge
        let c = if l.focus.is_none() {
            lamps_centre(&l.lamps).unwrap_or(c)
        } else {
            c
        };
        // (R22) the pair's centre's own turn (as the camera's quaternions turn, so it composes with them),
        // smoothed over `PAIR_SMOOTH`
        if journey {
            let qc = cam_quat(Camera {
                lon: c.lon,
                lat: c.lat,
                zoom: 1.0,
                roll: take.roll,
            });
            if let Some(p) = self.last_c.filter(|_| dt > 0.0) {
                let r = rot_log(qmul(
                    qc,
                    qconj(cam_quat(Camera {
                        lon: p.lon,
                        lat: p.lat,
                        zoom: 1.0,
                        roll: take.roll,
                    })),
                ));
                let k = 1.0 - (-dt / PAIR_SMOOTH).exp();
                for i in 0..3 {
                    self.cv[i] += (r[i] / dt - self.cv[i]) * k;
                }
            }
            self.last_c = Some(c);
        }
        // (R17) in open sea, out far enough that the nearest coast is in view, when it is within
        // `COAST_PULL` of the pair's own scale (gently: never further, so the pair stays readable).
        // (R22) Not in a journey: there the pair is the picture, its coast the opening's and the end's
        let z = match l.land_km.filter(|_| !journey) {
            Some(d) => {
                let see = COAST_SEEN * l.half / (l.r1 * (d / 6371.0).sin().max(1e-6));
                if see < z {
                    z.min(see.max(z / COAST_PULL))
                } else {
                    z
                }
            }
            None => z,
        };
        let (c, z) = match part {
            ClipPart::After => {
                let from = *self.end_from.get_or_insert((c, z));
                let whole = Lean {
                    trail: l
                        .trail
                        .iter()
                        .copied()
                        .chain(l.whole.iter().map(|w| w.1))
                        .collect(),
                    focus: None,
                    whole: Vec::new(),
                    ..l.clone()
                };
                let (cw, zw) = frame(&whole).unwrap_or((c, z));
                // never closer than where it was: the end opens out to the shape, it does not dive in
                let zw = zw.min(from.1);
                // (R19) the pull-out takes the first `END_PULL` seconds of the end beat; the rest holds
                // still on the shape, so it reads as an ending
                let e =
                    crate::touch::motion::ease_in_out((through * f.hold_out / END_PULL).min(1.0));
                let (a, b) = (from.0.to_xyz(), cw.to_xyz());
                let v = [
                    a[0] + (b[0] - a[0]) * e,
                    a[1] + (b[1] - a[1]) * e,
                    a[2] + (b[2] - a[2]) * e,
                ];
                (
                    crate::model::LonLat::from_xyz(v),
                    (from.1.ln() + (zw.ln() - from.1.ln()) * e).exp(),
                )
            }
            _ => {
                self.end_from = None;
                (c, z)
            }
        };
        // (R16) a journey's last third: the frame eases over to the braid, its whole lines
        let wb = match part {
            ClipPart::During if f.braid && JOURNEY_BRAID => {
                // (R20) from just after the opening, not only the last third: the trail since the
                // clip began (or its recent part) held in frame through the middle too
                let ss = |x: f64| {
                    let x = x.clamp(0.0, 1.0);
                    x * x * (3.0 - 2.0 * x)
                };
                ss((t - OPEN_SECS) / BRAID_IN).max(ss((through - BRAID_FROM) / BRAID_EASE))
            }
            _ => 0.0,
        };
        let braid = if wb > 0.0 {
            self.open_zoom.map(|open| braid_lean(l, open / BRAID_WIDER))
        } else {
            None
        };
        let (c, z) = match &braid {
            Some(lb) => {
                let (cb, zb) = frame(lb).unwrap_or((c, z));
                let (a, b) = (c.to_xyz(), cb.to_xyz());
                let v = [
                    a[0] + (b[0] - a[0]) * wb,
                    a[1] + (b[1] - a[1]) * wb,
                    a[2] + (b[2] - a[2]) * wb,
                ];
                (
                    crate::model::LonLat::from_xyz(v),
                    (z.ln() + (zb.ln() - z.ln()) * wb).exp(),
                )
            }
            None => (c, z),
        };
        // (the end beat's pull-out is its own: the frame's last, then the shape)
        if part != ClipPart::After {
            self.end_from = Some((c, z));
        }
        // opening `OPEN_WIDE` wider, but no wider than `OPEN_LEAST` (a close pair opens on its coast, a
        // pair already at a regional scale on its region, not the hemisphere)
        let wide = (z / OPEN_WIDE).max(z.min(OPEN_LEAST));
        if self.open_zoom.is_none() {
            self.open_zoom = Some(wide);
        }
        // (R18) a journey opens on where it is: wide enough that the nearest coast (or, far from any,
        // the globe's limb) is in the first frame, already moving, easing in to the pair in one
        // unbroken ease-out over `OPEN_SECS`; from there it only pulls out (R17)
        let (wide, e) = if f.braid {
            let from = *self.establish.get_or_insert_with(|| {
                let w = match l.land_far_km {
                    Some(d) => OPEN_COAST_SEEN * l.half / (l.r1 * (d / 6371.0).sin().max(1e-6)),
                    None => OPEN_LIMB,
                };
                w.clamp(crate::touch::motion::ZOOM_MIN, z / OPEN_LEAST_WIDER)
            });
            let u = (t / OPEN_SECS).clamp(0.0, 1.0);
            (from, 1.0 - (1.0 - u).powi(3))
        } else {
            (wide, crate::touch::motion::ease_in_out(t / OPEN_SECS))
        };
        let opening = f.braid && t < OPEN_SECS;
        let target = Camera {
            lon: c.lon,
            lat: c.lat,
            zoom: (wide.ln() + (z.ln() - wide.ln()) * e).exp(),
            roll: take.roll,
        };
        let fc = match self.fc {
            Some(fc) if part != ClipPart::Before => fc,
            _ => {
                // the opening beat: the frame itself, the camera still, widening less as it eases
                let target = self.limit(self.fc, target, dt, l);
                self.fc = Some(target);
                self.fw = [0.0; 3];
                self.fzv = 0.0;
                return Some(lifted(target, l));
            }
        };
        // the centre, slowly; (R22) in a journey aimed ahead of the pair by the lag the spring leaves
        // at its speed (2τ·v), so a pair running at a close zoom stays where the frame put it
        let tau = PAN_HALF / HALF_TO_TAU;
        let qt = cam_quat(Camera {
            zoom: 1.0,
            ..target
        });
        let qt = if journey {
            qmul(
                rot_exp([
                    self.cv[0] * 2.0 * tau,
                    self.cv[1] * 2.0 * tau,
                    self.cv[2] * 2.0 * tau,
                ]),
                qt,
            )
        } else {
            qt
        };
        let x = rot_log(qmul(cam_quat(Camera { zoom: 1.0, ..fc }), qconj(qt)));
        let mut nx = [0.0; 3];
        for i in 0..3 {
            let (a, v) = crate::touch::motion::spring(x[i], self.fw[i], 0.0, tau, dt);
            nx[i] = a;
            self.fw[i] = v;
        }
        let moved = Camera::from_rotation(
            crate::touch::camera::quat_mat(qmul(rot_exp(nx), qt)),
            fc.zoom,
        );
        // the zoom: the frame's, but no deeper than holds them all from where the centre now is
        let hold = match &braid {
            // the braid's points held as the frame eases over to them, so the zoom opens with the ease
            Some(lb) => {
                (hold_zoom(moved, l).ln() * (1.0 - wb) + hold_zoom(moved, lb).ln() * wb).exp()
            }
            None => hold_zoom(moved, l),
        };
        // (R22) and from where the camera is: in a journey the limited pan may trail the spring's aim
        // behind a pair that runs at a close zoom, so the box is held from both
        let hold = if journey {
            hold.min(hold_zoom(
                Camera {
                    zoom: fc.zoom,
                    ..fc
                },
                l,
            ))
        } else {
            hold
        };
        let want = target.zoom.min(hold);
        // (R17-R21 a journey's zoom eased outward only; R22: it comes back in as the pair closes, and
        // pulls out ahead of a pair that parts by how fast its frame has been widening, `ZOOM_LEAD`
        // seconds of it, so the limited zoom isn't outrun)
        let lead = if journey && !opening {
            if let Some(p) = self.last_want.filter(|_| dt > 0.0) {
                let k = 1.0 - (-dt / PAIR_SMOOTH).exp();
                self.want_rate += ((want / p).ln() / dt - self.want_rate) * k;
            }
            self.last_want = Some(want);
            (self.want_rate.min(0.0) * ZOOM_LEAD).exp()
        } else {
            1.0
        };
        let (want, hold) = (want * lead, hold * lead);
        let half = if want < moved.zoom {
            ZOOM_OUT_HALF
        } else if journey {
            JOURNEY_IN_HALF
        } else {
            ZOOM_IN_HALF
        };
        let (lz, vz) = crate::touch::motion::spring(
            moved.zoom.ln(),
            self.fzv,
            want.ln(),
            half / HALF_TO_TAU,
            dt,
        );
        self.fzv = vz;
        // a lamp never leaves: past the hold by more than a little, the zoom is held there
        let zoom = lz.exp().min(hold * 1.12);
        // (R18) the opening's ease is the zoom itself (a spring after it would still be coming in when
        // the outward-only rule began, and stop short); after it, outward only
        let zoom = if opening {
            target.zoom.min(hold * 1.12)
        } else {
            zoom
        };
        let base = if journey { self.cv } else { [0.0; 3] };
        let fc = self.limit_moving(Some(fc), Camera { zoom, ..moved }, dt, l, base);
        self.fc = Some(fc);
        Some(lifted(fc, l))
    }

    /// `next` approached from `prev` over `dt` with the zoom's and the pan's speeds within `ZOOM_RATE`
    /// and `PAN_RATE` (R19) and their changes within `ZOOM_ACCEL` and `PAN_ACCEL` (R20: the critic saw
    /// the opening's constant zoom rate stop dead in one frame): each eases in, and brakes in time to
    /// arrive without overshooting (its speed no more than `sqrt(2·accel·distance)`).
    fn limit(&mut self, prev: Option<Camera>, next: Camera, dt: f64, l: &Lean) -> Camera {
        self.limit_moving(prev, next, dt, l, [0.0; 3])
    }

    /// (R22) `limit` in a frame turning at `base` (radians a second, world frame): the pan's speed and
    /// its change are limited relative to the pair's own motion, so at a close zoom the camera keeps up
    /// with a pair that runs (the ground slides as fast as the pair goes) while anything else it does is
    /// as eased as ever. `base` is itself smoothed (`PAIR_SMOOTH`), so the camera never jerks with it.
    fn limit_moving(
        &mut self,
        prev: Option<Camera>,
        next: Camera,
        dt: f64,
        l: &Lean,
        base: [f64; 3],
    ) -> Camera {
        let Some(prev) = prev else { return next };
        if dt <= 0.0 {
            return prev;
        }
        fn ease(err: f64, v: f64, rate: f64, accel: f64, dt: f64) -> f64 {
            let want = err.signum()
                * (err.abs() / dt)
                    .min((2.0 * accel * err.abs()).sqrt())
                    .min(rate);
            v + (want - v).clamp(-accel * dt, accel * dt)
        }
        // the zoom, in its log
        self.lim_vz = ease(
            (next.zoom / prev.zoom).ln(),
            self.lim_vz,
            ZOOM_RATE,
            ZOOM_ACCEL,
            dt,
        );
        let zoom = prev.zoom * (self.lim_vz * dt).exp();
        // the turn, as a vector (world frame), its limits in screen points at this zoom
        let scale = l.r1.max(1.0) * zoom;
        let (qa, qb) = (
            cam_quat(Camera { zoom: 1.0, ..prev }),
            cam_quat(Camera { zoom: 1.0, ..next }),
        );
        // what is left to turn once the frame's own turn over this step is taken out
        let qa_moved = qmul(rot_exp([base[0] * dt, base[1] * dt, base[2] * dt]), qa);
        let r = rot_log(qmul(qb, qconj(qa_moved)));
        let ang = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        let (rate, accel) = (PAN_RATE / scale, PAN_ACCEL / scale);
        let speed = if ang > 1e-12 {
            (ang / dt).min((2.0 * accel * ang).sqrt()).min(rate)
        } else {
            0.0
        };
        let want = if ang > 1e-12 {
            [
                r[0] / ang * speed + base[0],
                r[1] / ang * speed + base[1],
                r[2] / ang * speed + base[2],
            ]
        } else {
            base
        };
        let dv = [
            want[0] - self.lim_w[0],
            want[1] - self.lim_w[1],
            want[2] - self.lim_w[2],
        ];
        let dvn = (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]).sqrt();
        let k = if dvn > accel * dt {
            accel * dt / dvn
        } else {
            1.0
        };
        for i in 0..3 {
            self.lim_w[i] += dv[i] * k;
        }
        let step = [self.lim_w[0] * dt, self.lim_w[1] * dt, self.lim_w[2] * dt];
        let q = qmul(rot_exp(step), qa);
        Camera::from_rotation(crate::touch::camera::quat_mat(q), zoom)
    }

    fn spring(&mut self, target: Camera, dt: f64, f: &Framing) -> Camera {
        if f.follow <= 0.0 {
            self.cam = target;
            return target;
        }
        let tau = f.follow / HALF_TO_TAU;
        let qt = cam_quat(target);
        // (R14) the target's own velocity, smoothed over `LEAD_SMOOTH`: the spring aims ahead of a
        // moving target by the lag a critically damped spring leaves at a steady speed (2τ·v), so a
        // pair crossing the frame quickly at a close zoom stays where the frame put it
        if let Some(prev) = self.last_target.filter(|_| dt > 0.0) {
            let r = rot_log(qmul(qt, qconj(cam_quat(prev))));
            let k = 1.0 - (-dt / LEAD_SMOOTH).exp();
            for i in 0..3 {
                self.target_w[i] += (r[i] / dt - self.target_w[i]) * k;
            }
            self.target_vz += ((target.zoom.ln() - prev.zoom.ln()) / dt - self.target_vz) * k;
        }
        self.last_target = Some(target);
        let lead = 2.0 * tau * self.lead_scale;
        let qt = qmul(
            rot_exp([
                self.target_w[0] * lead,
                self.target_w[1] * lead,
                self.target_w[2] * lead,
            ]),
            qt,
        );
        let target = Camera {
            zoom: target.zoom * (self.target_vz * lead * ZOOM_SLOWER).exp(),
            ..target
        };
        // where the camera lies from the target, as a turn (world frame): cam = exp(x) · target
        let x = rot_log(qmul(cam_quat(self.cam), qconj(qt)));
        let mut nx = [0.0; 3];
        for i in 0..3 {
            let (a, v) = crate::touch::motion::spring(x[i], self.w[i], 0.0, tau, dt);
            nx[i] = a;
            self.w[i] = v;
        }
        // (R14) the zoom breathes on a spring twice as slow: as a pair nears or parts the frame closes
        // and opens calmly, whatever the journey's pace
        let (lz, vz) = crate::touch::motion::spring(
            self.cam.zoom.ln(),
            self.vz,
            target.zoom.ln(),
            tau * ZOOM_SLOWER,
            dt,
        );
        self.vz = vz;
        self.cam = Camera::from_rotation(
            crate::touch::camera::quat_mat(qmul(rot_exp(nx), qt)),
            lz.exp(),
        );
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
    /// The lamps' whole lines since the take began, each point with its day (R15), for the end beat's
    /// pull-out to the shape and (R16) a journey's braid held through its last third.
    pub whole: Vec<(f64, crate::model::LonLat)>,
    /// (R16) The closest the frame may come for the land near the lamps (`coast_cap`): `FRAME_ZOOM_COAST`
    /// near a coast, where the sea's shelf shading still looks right, `FRAME_ZOOM_MAX` out at sea.
    pub zoom_cap: f64,
    /// (R17) The nearest land to the lamps, km, if within `LAND_LOOK_KM`.
    pub land_km: Option<f64>,
    /// (R18) The nearest land within `OPEN_LOOK_KM`, for a journey's establishing opening.
    pub land_far_km: Option<f64>,
    /// (R21) The scale of what carries the things framed, against the sea's: 1 (or 0, unset) for
    /// lamps, `WIND_SCALE` when every thing framed is a seed on the wind. The frame's least spans and
    /// margin are km at the sea's scale, multiplied by it, so a seed is framed at the wind's scale.
    pub scale: f64,
    /// (R22) Framed for a journey (`journey_lean`): the pair closer (`JOURNEY_FILL`,
    /// `JOURNEY_PAIR_MIN_KM`, `JOURNEY_MARGIN_KM`) and as deep as `JOURNEY_ZOOM_MAX`.
    pub journey: bool,
}

/// (R21) How much wider the wind's things are framed than the sea's: the frame's least spans and
/// margin in km times this. The wind runs about 20x the sea in km and is shown at about a seventh of
/// its pace (`AIR_RATE` 0.3 against 2 days a second), so a seed crosses about 3x a lamp's km a second
/// of clip; framed 5x wider, it moves on the screen a little slower than a lamp, and a pair's spread
/// over its life (~300-1500 km) fills the frame as a dancing pair's 15-150 km do (`sea/MEDIA.md`: the
/// memory length 7x, the spread in a day ~8x).
pub const WIND_SCALE: f64 = 5.0;
/// (R21) The days of line the frame holds, against the sea's, for seeds: a seed's memory is a third
/// of a lamp's, its whole life about a day (`sea/MEDIA.md` › trail ~3 T): `FRAME_TRAIL` × this.
pub const WIND_TRAIL: f64 = 0.2;
/// (R21) A seed come down is framed for this many days after (about a second of clip at the wind's
/// pace), then let go, so the frame follows what still flies; with every one down, they all are.
pub const DOWN_FRAMED_DAYS: f64 = 0.3;

impl Lean {
    /// The deepest the frame may come: `FRAME_ZOOM_MAX`, or (R22) a journey's cap where that is
    /// deeper (`journey_lean`).
    pub fn deepest(&self) -> f64 {
        if self.journey {
            JOURNEY_ZOOM_MAX
        } else {
            FRAME_ZOOM_MAX
        }
    }

    /// The medium's scale (1 for the sea, when unset).
    pub fn scale(&self) -> f64 {
        if self.scale > 0.0 {
            self.scale
        } else {
            1.0
        }
    }
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

/// The lamps' span and a margin fills this share of the view's width (R12: the round-11 critic; at
/// zoom 3 a whole take's conducting was a hook a few millimetres long. R14: the lamps' own span, not
/// their lines', which pulled the frame back along their wakes and left them at its top edge).
pub const FRAME_FILL: f64 = 0.6;
/// The clip lifts the lamps' centre this share of its height above its middle (R14), so the lower
/// third, where a platform lays its captions, stays clear without the frame being mostly empty sea.
pub const FRAME_LIFT: f64 = -0.02;
/// Days of each lamp's line the frame holds.
pub const FRAME_TRAIL: f64 = 15.0;
/// Days of line the frame holds whole; older, up to `FRAME_TRAIL`, a point counts half as far out from
/// its lamp by the end (R15: the lines weighted, the newest most).
pub const FRAME_TRAIL_FULL: f64 = 8.0;
/// Km of margin round the span (the cue's fan, the lamp's glow).
pub const FRAME_MARGIN_KM: f64 = 10.0;
/// The least half-span, km: a lone lamp with a short line is framed so a hundred km fill 60% of the
/// width's half, where a stroke's tens of km are tens of points.
pub const FRAME_SPAN_MIN_KM: f64 = 100.0;
/// The least half-span of two lamps or more, km (R14): a dancing pair 15–150 km apart is framed
/// tighter and tighter as it nears, down to this (or `FRAME_ZOOM_MAX`).
pub const FRAME_PAIR_MIN_KM: f64 = 60.0;
/// The deepest the frame comes in (the reader's own pinch stops at `motion::ZOOM_MAX` otherwise). R14:
/// 100 (a pair 80 km apart at 60% of the width); deeper, the coast's distance field shows its pixels.
pub const FRAME_ZOOM_MAX: f64 = 70.0;
/// (R19) Within `LAND_LOOK_KM` of land the frame comes no closer than this (R14-R18's 40); in open
/// sea, beyond it, up to `FRAME_ZOOM_OPEN_SEA`, so a far-out pair's paths fill about 60% of the width
/// (the coast's shelf shading that limits it is not in view).
pub const FRAME_ZOOM_NEAR: f64 = 40.0;
pub const FRAME_ZOOM_OPEN_SEA: f64 = 70.0;
/// Near a coast the frame comes no closer than this (R16, Visual: the shallow shelf's shading shows
/// its raster's steps above about 20); open water keeps `FRAME_ZOOM_MAX`.
pub const FRAME_ZOOM_COAST: f64 = 20.0;
/// The cap eases from `FRAME_ZOOM_COAST` with land within `COAST_NEAR_KM` of the lamps' centre to
/// `FRAME_ZOOM_MAX` with none within `COAST_FAR_KM`, so it never flips as a pair runs along a coast.
pub const COAST_NEAR_KM: f64 = 250.0;
pub const COAST_FAR_KM: f64 = 450.0;

/// How far to look for land, km (R17), and how the clip shows it: the nearest coast at `COAST_SEEN`
/// of the half-width, but never more than `COAST_PULL` times wider than the pair's own frame.
pub const LAND_LOOK_KM: f64 = 500.0;
/// (R18) A journey's opening looks this far for a coast, shows it at `OPEN_COAST_SEEN` of the
/// half-width, and with none that near opens at `OPEN_LIMB` (the globe whole, its limb in view); it is
/// always at least `OPEN_LEAST_WIDER` times wider than the pair's frame, so the first frame moves.
pub const OPEN_LOOK_KM: f64 = 3000.0;
pub const OPEN_COAST_SEEN: f64 = 0.8;
pub const OPEN_LIMB: f64 = 1.0;
pub const OPEN_LEAST_WIDER: f64 = 1.6;
pub const COAST_SEEN: f64 = 0.9;
pub const COAST_PULL: f64 = 1.4;

/// (R22) A journey frames its pair with this many days of their lines (the frame of R15-R21 held 15,
/// and R20's braid every day since the clip began: the pair a speck by the dance's middle).
pub const JOURNEY_TRAIL_DAYS: f64 = 0.5;
/// (R22) The deepest a journey's frame comes in open water (or with the coast out of view), and the
/// least half-span of its pair, km: a pair 30 km apart still a third of the width.
pub const JOURNEY_ZOOM_MAX: f64 = 160.0;
pub const JOURNEY_PAIR_MIN_KM: f64 = 15.0;
/// (R22) A journey's pair fills this share of the width (the frame's circle round it), with this
/// margin, km; and its zoom comes in with this half-life, seconds (`ZOOM_IN_HALF` elsewhere).
pub const JOURNEY_FILL: f64 = 0.75;
pub const JOURNEY_MARGIN_KM: f64 = 5.0;
pub const JOURNEY_IN_HALF: f64 = 0.5;
/// (R22) The coast counts as out of view when it lies beyond the clip's half-diagonal by this much
/// (the frame's centre is the pair's, the lift and the lag move it a little).
pub const COAST_OUT_MARGIN: f64 = 1.1;
/// (R22) Below the zoom at which the coast leaves the view, the cap eases from the coast's
/// (`coast_cap`) to that zoom over this factor of zoom, so the frame never jumps across it.
pub const COAST_OUT_EASE: f64 = 1.6;
/// (R22) With the coast in view a journey comes no closer than this (`FRAME_ZOOM_COAST`'s 20 kept the
/// dance near Victoria a speck; at 30 the coast's edge still reads clean on the device's clip, its
/// steps showing from about 40), easing to `FRAME_ZOOM_NEAR` with the coast further off as ever.
pub const JOURNEY_ZOOM_COAST: f64 = 30.0;
/// (R20's braid held through a journey's middle; R22: off, the pair framed instead)
pub const JOURNEY_BRAID: bool = false;
/// (R22) A journey's frame holds each lamp where it will be this many seconds on (its velocity smoothed
/// over `LAMP_SMOOTH`).
pub const LAMP_LEAD: f64 = 0.8;
pub const LAMP_SMOOTH: f64 = 0.2;
/// (R22) A journey's zoom pulls out this many seconds ahead of a parting pair's frame.
pub const ZOOM_LEAD: f64 = 1.0;
/// (R22) The pair's centre's velocity is smoothed over this many seconds for the pan's lead and limits.
pub const PAIR_SMOOTH: f64 = 0.5;

/// (R22) What a journey frames: the pair and its last `JOURNEY_TRAIL_DAYS` of line (from `whole`), its
/// cap the coast's only while the coast is in view (`journey_cap`).
pub fn journey_lean(l: &Lean) -> Lean {
    let last = l
        .whole
        .iter()
        .map(|w| w.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let trail = if last.is_finite() {
        l.whole
            .iter()
            .filter(|w| w.0 >= last - JOURNEY_TRAIL_DAYS)
            .map(|w| w.1)
            .collect()
    } else {
        l.trail.clone()
    };
    let mut j = Lean {
        trail,
        focus: None,
        journey: true,
        ..l.clone()
    };
    j.zoom_cap = JOURNEY_ZOOM_MAX;
    if let Some(d) = l.land_km {
        // the frame as close as it would come with no coast near, and the zoom at which the coast
        // leaves the view from there
        if let Some((_, open)) = frame(&j) {
            let diag = (l.half * l.half * (1.0 + (16.0f64 / 9.0).powi(2))).sqrt();
            let out = COAST_OUT_MARGIN * diag / (l.r1 * (d / EARTH_KM).sin().max(1e-6));
            let near = coast_cap(Some(d)).max(JOURNEY_ZOOM_COAST);
            j.zoom_cap = if open >= out {
                JOURNEY_ZOOM_MAX
            } else if open <= out / COAST_OUT_EASE {
                near
            } else {
                // eased, in the zoom's log, from the coast's cap to where the coast is out of view
                let x = (open / out).ln() / COAST_OUT_EASE.ln() + 1.0;
                let x = x * x * (3.0 - 2.0 * x);
                (near.ln() + (out.min(JOURNEY_ZOOM_MAX).max(near).ln() - near.ln()) * x).exp()
            }
            .max(FRAME_ZOOM_MAX.min(near));
        }
    }
    j
}

/// The centre of the lamps' smallest circle (their mean direction for two).
pub fn lamps_centre(lamps: &[crate::model::LonLat]) -> Option<crate::model::LonLat> {
    frame_of(lamps, &[], 166.0, 180.0, FRAME_ZOOM_MAX, 1.0, false).map(|f| f.0)
}

/// The zoom cap for land `km` from the lamps (None: none within `COAST_FAR_KM`).
pub fn coast_cap(km: Option<f64>) -> f64 {
    match km {
        // (R19) no land within `LAND_LOOK_KM`: open sea, where the paths may fill the frame
        None => FRAME_ZOOM_OPEN_SEA,
        Some(d) => {
            let x = ((d - COAST_NEAR_KM) / (COAST_FAR_KM - COAST_NEAR_KM)).clamp(0.0, 1.0);
            FRAME_ZOOM_COAST + (FRAME_ZOOM_NEAR - FRAME_ZOOM_COAST) * x * x * (3.0 - 2.0 * x)
        }
    }
}
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
    let cap = if l.zoom_cap > 0.0 {
        l.zoom_cap.min(l.deepest())
    } else {
        l.deepest()
    };
    let all = frame_of(&l.lamps, &l.trail, l.r1, l.half, cap, l.scale(), l.journey)?;
    match &l.focus {
        Some((lamps, trail, w)) if *w > 0.0 => {
            let one = frame_of(lamps, trail, l.r1, l.half, cap, l.scale(), l.journey)?;
            // the zoom in its log; the centre at least as far toward the lamp as keeps it no further
            // out on the screen than it lay in the frame of them all (the zoom grows faster than a
            // plain blend of centres would follow)
            let at = |w: f64| {
                let zoom = (all.1.ln() + (one.1.ln() - all.1.ln()) * w).exp();
                let wc = w.max(1.0 - all.1 / zoom);
                let (a, b) = (all.0.to_xyz(), one.0.to_xyz());
                let v = [
                    a[0] + (b[0] - a[0]) * wc,
                    a[1] + (b[1] - a[1]) * wc,
                    a[2] + (b[2] - a[2]) * wc,
                ];
                (crate::model::LonLat::from_xyz(v), zoom)
            };
            // never so far in that another lamp leaves the frame (the lean is as much as keeps them all)
            let holds = |(c, z): (crate::model::LonLat, f64)| {
                let cam = Camera::new(c.lon, c.lat, z);
                l.lamps.iter().all(|p| {
                    let v = cam.view(p.to_xyz());
                    v[2] > 0.0
                        && (v[0] * v[0] + v[1] * v[1]).sqrt() * l.r1 * z <= FOCUS_HOLD * l.half
                })
            };
            if holds(at(*w)) {
                return Some(at(*w));
            }
            let (mut lo, mut hi) = (0.0, *w);
            for _ in 0..14 {
                let mid = (lo + hi) / 2.0;
                if holds(at(mid)) {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            Some(at(lo))
        }
        _ => Some(all),
    }
}

#[allow(clippy::too_many_arguments)]
fn frame_of(
    lamps: &[crate::model::LonLat],
    trail: &[crate::model::LonLat],
    r1: f64,
    half: f64,
    cap: f64,
    scale: f64,
    journey: bool,
) -> Option<(crate::model::LonLat, f64)> {
    if lamps.is_empty() || r1 <= 0.0 || half <= 0.0 {
        return None;
    }
    // (R15) the lamps and their recent lines (weighted by `App::lean_for`), so a pair and its braid are
    // framed together (R14 framed the lamps alone; the critic saw the lines run off the top)
    let pts: Vec<[f64; 3]> = lamps.iter().chain(trail).map(|p| p.to_xyz()).collect();
    let dot = |a: [f64; 3], b: [f64; 3]| (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let norm = |v: [f64; 3]| {
        let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
        [v[0] / m, v[1] / m, v[2] / m]
    };
    let mut c = norm(
        pts.iter()
            .fold([0.0; 3], |a, p| [a[0] + p[0], a[1] + p[1], a[2] + p[2]]),
    );
    for i in 1..=40 {
        let far = *pts
            .iter()
            .min_by(|a, b| dot(**a, c).total_cmp(&dot(**b, c)))
            .unwrap();
        let k = 1.0 / (i as f64 + 1.0);
        c = norm([
            c[0] + (far[0] - c[0]) * k,
            c[1] + (far[1] - c[1]) * k,
            c[2] + (far[2] - c[2]) * k,
        ]);
    }
    let spread = pts.iter().map(|p| dot(*p, c).acos()).fold(0.0, f64::max);
    // (R21) at the medium's scale: a seed's least span and margin are the wind's
    // (R22) a journey's pair may come closer (`JOURNEY_PAIR_MIN_KM`, carried as a cap above
    // `FRAME_ZOOM_MAX`)
    let (pair_min, margin, fill) = if journey {
        (JOURNEY_PAIR_MIN_KM, JOURNEY_MARGIN_KM, JOURNEY_FILL)
    } else {
        (FRAME_PAIR_MIN_KM, FRAME_MARGIN_KM, FRAME_FILL)
    };
    let least = scale
        * if lamps.len() > 1 {
            pair_min
        } else {
            FRAME_SPAN_MIN_KM
        };
    let half_span = (spread + scale * margin / EARTH_KM)
        .max(least / EARTH_KM)
        .min(std::f64::consts::FRAC_PI_2);
    let zoom = (fill * half / (r1 * half_span.sin())).max(crate::touch::motion::ZOOM_MIN);
    Some((
        crate::model::LonLat::from_xyz(c),
        zoom.min(cap.max(crate::touch::motion::ZOOM_MIN)),
    ))
}

/// The clip's camera: the lamps framed (`frame`), the take's own pinch kept by half against its first
/// zoom `zoom0`, the take's roll; the take's own camera when there are no lamps. Continuous in the
/// lamps, so the spring after it never sees a jump.
pub fn lean(take: Camera, l: &Lean, zoom0: f64) -> Camera {
    match frame(l) {
        None => take,
        Some((c, z)) => {
            let pinch = (take.zoom / zoom0.max(1e-6)).powf(FRAME_PINCH);
            let zoom = (z * pinch).clamp(crate::touch::motion::ZOOM_MIN, FRAME_ZOOM_MAX);
            lifted(
                Camera {
                    lon: c.lon,
                    lat: c.lat,
                    zoom,
                    roll: take.roll,
                },
                l,
            )
        }
    }
}

/// The clip's framed centre follows the lamps' frame with this half-life, seconds (R15: slowly, so a
/// pair that parts or runs is held by pulling back, not by a fast pan at a close zoom).
pub const PAN_HALF: f64 = 1.0;
/// The framed zoom eases out (to hold what is leaving) and in (as a pair nears) with these half-lives.
pub const ZOOM_OUT_HALF: f64 = 0.3;
pub const ZOOM_IN_HALF: f64 = 0.9;
/// The opening starts this many times wider, the place readable, easing in over `OPEN_SECS` of the
/// clip (R15).
pub const OPEN_WIDE: f64 = 3.0;
pub const OPEN_SECS: f64 = 3.0;
/// The opening widens no further than this zoom unless the frame itself is wider.
pub const OPEN_LEAST: f64 = 6.0;
/// A journey clip's end beat: long enough for the pull-out to the dance's whole shape to land (R15).
pub const JOURNEY_HOLD_OUT: f64 = 3.2;
/// The box every lamp and its newest line keep within, as shares of the clip's width (either side)
/// and height (above and below its middle, after the lift): clear of the edges and the lower third.
pub const HOLD_X: f64 = 0.42;
pub const HOLD_UP: f64 = 0.44;
pub const HOLD_DOWN: f64 = 0.25;

/// (R19) However the frame's aims change (a coast coming into reach, the opening handing over, a
/// cue), the clip's camera moves no faster than this: its zoom by `ZOOM_RATE` in the log a second
/// (1.8% a frame at 30 frames a second), its centre by `PAN_RATE` points a second across the screen.
pub const ZOOM_RATE: f64 = 0.55;
pub const PAN_RATE: f64 = 120.0;
/// (R20) How fast those rates may change: the zoom's by this in the log a second a second (full rate
/// in about a second), the pan's in points a second a second.
pub const ZOOM_ACCEL: f64 = 0.6;
pub const PAN_ACCEL: f64 = 160.0;
/// A journey's end beat pulls out over this many seconds and holds still for the rest.
pub const END_PULL: f64 = 2.0;

/// A journey's braid is held from this share of its During part, eased over `BRAID_EASE` (R16).
pub const BRAID_FROM: f64 = 0.6;
pub const BRAID_EASE: f64 = 0.12;
/// (R20) After the opening the braid is taken in over this many seconds and held from then on.
pub const BRAID_IN: f64 = 2.0;
/// The braid may frame up to this many times wider than the opening (R16: the dance's whole braid since
/// the clip began wants about zoom 8 against an opening of 13; the end's pull-out goes wider still).
pub const BRAID_WIDER: f64 = 2.0;

/// The lamps with their whole lines since the clip began, or, if all of them would frame wider than
/// `floor` (the opening's zoom), their most recent part that frames no wider (R16).
pub fn braid_lean(l: &Lean, floor: f64) -> Lean {
    let with = |from: f64| Lean {
        trail: l
            .trail
            .iter()
            .copied()
            .chain(l.whole.iter().filter(|w| w.0 >= from).map(|w| w.1))
            .collect(),
        focus: None,
        whole: Vec::new(),
        ..l.clone()
    };
    let first = l.whole.iter().map(|w| w.0).fold(f64::INFINITY, f64::min);
    let last = l
        .whole
        .iter()
        .map(|w| w.0)
        .fold(f64::NEG_INFINITY, f64::max);
    if !first.is_finite() {
        return l.clone();
    }
    let fits = |from: f64| frame(&with(from)).is_none_or(|(_, z)| z >= floor * 0.999);
    if fits(first) {
        return with(first);
    }
    // the earliest day from which the lines still frame within the floor (closer as the day is later)
    let (mut lo, mut hi) = (first, last);
    for _ in 0..12 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            hi = mid
        } else {
            lo = mid
        }
    }
    with(hi)
}

/// The deepest zoom at which, from `cam`'s centre (before the lift), every lamp and its weighted line
/// lie within the hold box (R15).
pub fn hold_zoom(cam: Camera, l: &Lean) -> f64 {
    let (w, h) = (l.half * 2.0, l.half * 2.0 * 16.0 / 9.0);
    let up = FRAME_LIFT * h;
    let mut r = f64::INFINITY;
    for p in l.lamps.iter().chain(&l.trail) {
        let v = cam.view(p.to_xyz());
        if v[2] <= 0.0 {
            r = r.min(l.r1 * crate::touch::motion::ZOOM_MIN);
            continue;
        }
        if v[0].abs() > 1e-9 {
            r = r.min(HOLD_X * w / v[0].abs());
        }
        // y down on the screen: -r·vy - up, kept within [-HOLD_UP·h, HOLD_DOWN·h]
        if v[1] > 1e-9 {
            r = r.min((HOLD_UP * h - up).max(1.0) / v[1]);
        } else if v[1] < -1e-9 {
            r = r.min((HOLD_DOWN * h + up) / -v[1]);
        }
    }
    (r / l.r1).clamp(crate::touch::motion::ZOOM_MIN, l.deepest())
}

/// The camera turned about the screen's horizontal so what lay at its centre lies `FRAME_LIFT` of the
/// clip's height above it (R14), whatever the roll. The clip is 9:16: its height is `half` × 32/9.
fn lifted(cam: Camera, l: &Lean) -> Camera {
    let up = FRAME_LIFT * l.half * 2.0 * 16.0 / 9.0;
    let r = l.r1 * cam.zoom;
    if r <= up {
        return cam;
    }
    // a turn about view x by -asin(up / r) carries the centre's point up the screen by `up`
    cam.turned([1.0, 0.0, 0.0], -(up / r).asin())
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
    let q = if q[0] < 0.0 {
        [-q[0], -q[1], -q[2], -q[3]]
    } else {
        q
    };
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
    /// What a put makes (R21): the take's preset.
    #[serde(default)]
    pub medium: crate::model::Medium,
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
            TakeEvent::Frame { t }
            | TakeEvent::Wheel { t, .. }
            | TakeEvent::Key { t, .. }
            | TakeEvent::Shake { t }
            | TakeEvent::Intent { t, .. } => *t,
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
        Some((
            self.framing.hold_in,
            self.clip_seconds() - self.framing.hold_in,
        ))
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
            (
                self.t0 + a,
                ClipPart::Before,
                if f.hold_in > 0.0 {
                    tau / f.hold_in
                } else {
                    1.0
                },
            )
        } else if tau < f.hold_in + during {
            let u = tau - f.hold_in;
            (
                self.t0 + a + u * self.speed,
                ClipPart::During,
                if during > 0.0 { u / during } else { 1.0 },
            )
        } else {
            let u = tau - f.hold_in - during;
            (
                self.t0 + b,
                ClipPart::After,
                if f.hold_out > 0.0 {
                    (u / f.hold_out).min(1.0)
                } else {
                    1.0
                },
            )
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
        assert!(
            (one.lon - two.lon).abs() < 1e-9
                && (one.lat - two.lat).abs() < 1e-9
                && (one.zoom - two.zoom).abs() < 1e-9
        );
        // the short way round the date line
        assert!(one.lon > 170.0 || one.lon < -170.0);
    }

    #[test]
    fn clip_parts() {
        let start = TakeStart {
            floats: vec![],
            next_id: 1,
            clock: Clock {
                day: 0.0,
                running: true,
                rate: 1.0,
            },
            camera: Camera::new(0.0, 0.0, 1.0),
            chosen: vec![],
            followed: vec![],
            riding: None,
            seconds: 0.0,
            night: false,
            reduce_motion: false,
            credits_open: false,
            medium: Default::default(),
        };
        let take = Take {
            start,
            events: vec![],
            t0: 10.0,
            t1: 14.0,
            width: 100,
            height: 100,
            ppp: 1.0,
            trim: None,
            end: None,
            speed: 0.5,
            framing: Framing::default(),
            song: None,
        };
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
        let zoom = if t < 0.5 {
            1.2
        } else if t < 1.0 {
            1.2 * (1.0 + ((t - 0.5) * 20.0).floor() / 10.0)
        } else {
            2.4
        };
        let lon = if t < 1.0 {
            -40.0
        } else {
            -40.0 + 40.0 * (t - 1.0)
        };
        Camera::new(lon, 20.0, zoom)
    }

    /// The clip's camera through a whole clip at `fps`: (clip seconds, camera) a frame.
    fn run_clip(fps: f64) -> Vec<(f64, Camera)> {
        let start = TakeStart {
            floats: vec![],
            next_id: 1,
            clock: Clock {
                day: 0.0,
                running: true,
                rate: 1.0,
            },
            camera: take_cam(0.0),
            chosen: vec![],
            followed: vec![],
            riding: None,
            seconds: 0.0,
            night: false,
            reduce_motion: false,
            credits_open: false,
            medium: Default::default(),
        };
        let take = Take {
            start,
            events: vec![],
            t0: 0.0,
            t1: 1.6,
            width: 100,
            height: 100,
            ppp: 1.0,
            trim: None,
            end: None,
            speed: 1.0,
            framing: Framing::default(),
            song: None,
        };
        let mut cc = ClipCamera::new(take.start.camera);
        let n = (take.clip_seconds() * fps).ceil() as usize;
        (0..n)
            .map(|i| {
                let tau = i as f64 / fps;
                let (t, part, through) = take.clip_at(tau);
                (
                    tau,
                    cc.step(
                        take_cam(t),
                        part,
                        through,
                        1.0 / fps,
                        &take.framing,
                        &Lean::default(),
                    ),
                )
            })
            .collect()
    }

    #[test]
    fn the_clip_eases_in_follows_without_kicks_and_comes_to_rest() {
        let f = Framing::default();
        let c = run_clip(60.0);
        // the opening: a touch wider at the first frame, the take's own framing by the beat's end
        assert!(
            (c[0].1.zoom - 1.2 * (-f.open).exp()).abs() < 1e-9,
            "{:?}",
            c[0].1
        );
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
        assert!(
            worst < 0.004,
            "the zoom's rate jumped by {worst} in a frame"
        );
        // the fling at the take's end is carried on and dies away: still moving east just after,
        // and at rest by the clip's last frame
        let lon = |i: usize| c[i].1.lon;
        let n = c.len();
        let after = c.iter().position(|(t, _)| *t >= f.hold_in + 1.6).unwrap();
        assert!(lon(after + 6) - lon(after) > 0.5, "the drift stopped dead");
        let last = (lon(n - 1) - lon(n - 2)) * 60.0;
        assert!(last.abs() < 1.5, "still turning {last}°/s at the end");
        assert!(
            lon(n - 1) > take_cam(1.6).lon + 5.0,
            "the drift carried {}°",
            lon(n - 1) - take_cam(1.6).lon
        );
    }

    #[test]
    fn the_clip_camera_is_the_same_at_20_and_60_fps() {
        let a = run_clip(60.0);
        let b = run_clip(20.0);
        for (tau, cam) in &b {
            let other = a
                .iter()
                .min_by(|x, y| (x.0 - tau).abs().partial_cmp(&(y.0 - tau).abs()).unwrap())
                .unwrap()
                .1;
            assert!(
                (cam.lon - other.lon).abs() < 1.0 && (cam.zoom.ln() - other.zoom.ln()).abs() < 0.03,
                "at {tau}: {cam:?} vs {other:?}"
            );
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
            let target = if lat <= 90.0 {
                Camera::new(0.0, lat, 2.0)
            } else {
                Camera {
                    lon: 180.0,
                    lat: 180.0 - lat,
                    zoom: 2.0,
                    roll: 180.0,
                }
            };
            let cam = cc.step(
                target,
                ClipPart::During,
                0.0,
                1.0 / 60.0,
                &f,
                &Lean::default(),
            );
            let d = LonLat::new(cam.lon, cam.lat).to_xyz();
            let e = LonLat::new(last.lon, last.lat).to_xyz();
            let step = (d[0] * e[0] + d[1] * e[1] + d[2] * e[2])
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            assert!(step < 1.0, "jumped {step}° at {cam:?}");
            last = cam;
        }
    }

    /// (R21) Seeds are framed at the wind's scale; lamps exactly as before (scale unset and 1 alike).
    #[test]
    fn seeds_are_framed_at_the_winds_scale_and_lamps_as_ever() {
        let lean = |lamps: Vec<LonLat>, scale: f64| Lean {
            lamps,
            trail: vec![],
            r1: 166.0,
            half: 180.0,
            focus: None,
            whole: vec![],
            zoom_cap: 0.0,
            land_km: None,
            land_far_km: None,
            scale,
            journey: false,
        };
        let pair = vec![LonLat::new(-20.0, 45.0), LonLat::new(-19.5, 45.0)];
        let lone = vec![LonLat::new(-20.0, 45.0)];
        for p in [&pair, &lone] {
            assert_eq!(
                frame(&lean(p.clone(), 0.0)),
                frame(&lean(p.clone(), 1.0)),
                "a lamp's frame is the same, the scale unset or 1"
            );
        }
        // a seed pair 40 km apart: its least span is the wind's (300 km), so the frame is ~5x wider
        let (_, zl) = frame(&lean(pair.clone(), 1.0)).unwrap();
        let (_, zs) = frame(&lean(pair.clone(), WIND_SCALE)).unwrap();
        let half_km = |z: f64| 180.0 * EARTH_KM / (166.0 * z);
        assert!(
            zs < zl && half_km(zs) > 0.9 * WIND_SCALE * FRAME_PAIR_MIN_KM / FRAME_FILL,
            "lamps zoom {zl:.1}, seeds {zs:.1}"
        );
        // two seeds 1500 km apart fill the frame as two lamps 1500 km apart do: their own span wins (the
        // margin, 50 km for a seed against 10, is all that differs)
        let far = vec![LonLat::new(-30.0, 45.0), LonLat::new(-11.0, 45.0)];
        let (_, a) = frame(&lean(far.clone(), 1.0)).unwrap();
        let (_, b) = frame(&lean(far, WIND_SCALE)).unwrap();
        assert!((b / a - 1.0).abs() < 0.08, "{a} {b}");
    }

    #[test]
    fn the_frame_fills_six_tenths_with_the_lamps_and_shows_a_lone_lamps_tens_of_km() {
        let km_pt = |zoom: f64| 166.0 * zoom / EARTH_KM;
        // a lone lamp, a short line: close enough that 40 km is plainly visible (≥ 30 points)
        let lone = Lean {
            lamps: vec![LonLat::new(-72.0, 35.0)],
            trail: vec![LonLat::new(-72.2, 35.05)],
            r1: 166.0,
            half: 180.0,
            focus: None,
            whole: vec![],
            zoom_cap: 0.0,
            land_km: None,
            land_far_km: None,
            scale: 0.0,
            journey: false,
        };
        let (c, z) = frame(&lone).unwrap();
        assert!(
            40.0 * km_pt(z) >= 30.0 && (c.lon + 72.1).abs() < 0.2,
            "{c:?} {z}"
        );
        // two lamps a thousand km apart: their span (with the margin) fills 60% of the width
        let two = Lean {
            lamps: vec![LonLat::new(-72.0, 35.0), LonLat::new(-61.0, 37.0)],
            trail: vec![],
            r1: 166.0,
            half: 180.0,
            focus: None,
            whole: vec![],
            zoom_cap: 0.0,
            land_km: None,
            land_far_km: None,
            scale: 0.0,
            journey: false,
        };
        let (c, z) = frame(&two).unwrap();
        let cam = Camera::new(c.lon, c.lat, z);
        let reach = two
            .lamps
            .iter()
            .map(|p| {
                let v = cam.view(p.to_xyz());
                (v[0] * v[0] + v[1] * v[1]).sqrt() * 166.0 * z
            })
            .fold(0.0, f64::max);
        assert!(
            reach > 0.45 * 180.0 && reach < 0.6 * 180.0,
            "the furthest lamp {reach} pt from the centre"
        );
        // continuous: the line growing a little moves the frame a little
        let mut more = lone.clone();
        more.trail.push(LonLat::new(-72.25, 35.06));
        let (c2, z2) = frame(&more).unwrap();
        let (c1, z1) = frame(&lone).unwrap();
        assert!(
            (c2.lon - c1.lon).abs() < 0.05 && (z2 / z1 - 1.0).abs() < 0.1,
            "{c1:?} {z1} → {c2:?} {z2}"
        );
        // the clip frames the lamps whatever the take's zoom; no lamps, the take's camera
        let a = lean(Camera::new(-50.0, 30.0, 2.0), &two, 1.0);
        assert!((a.zoom / z - 1.0).abs() < 1e-6, "{a:?}");
        assert_eq!(
            lean(Camera::new(-100.0, 10.0, 1.3), &Lean::default(), 1.3),
            Camera::new(-100.0, 10.0, 1.3)
        );
        // a lamp cued alone: the frame leans in on it, then eases back to both; never a jump
        assert_eq!(focus_weight(-0.1), 0.0);
        let mut last = frame(&two).unwrap();
        for i in 0..=100 {
            let age = i as f64 * 0.05;
            let f = Lean {
                focus: Some((vec![LonLat::new(-72.0, 35.0)], vec![], focus_weight(age))),
                ..two.clone()
            };
            let now = frame(&f).unwrap();
            assert!(
                (now.1 / last.1).ln().abs() < 0.15 && (now.0.lon - last.0.lon).abs() < 0.8,
                "at {age}: {last:?} → {now:?}"
            );
            let cam = Camera::new(now.0.lon, now.0.lat, now.1);
            let v = cam.view(LonLat::new(-72.0, 35.0).to_xyz());
            assert!(
                (v[0] * v[0] + v[1] * v[1]).sqrt() * 166.0 * now.1 <= 0.62 * 180.0,
                "the cued lamp out of frame at {age}"
            );
            let o = cam.view(LonLat::new(-61.0, 37.0).to_xyz());
            assert!(
                (o[0] * o[0] + o[1] * o[1]).sqrt() * 166.0 * now.1 <= 0.91 * 180.0,
                "the other lamp out of frame at {age}"
            );
            if (age - 1.0).abs() < 1e-9 {
                assert!(
                    now.1 > 1.1 * z && (now.0.lon + 72.0).abs() < 6.0,
                    "not in on the lamp: {now:?}"
                );
            }
            last = now;
        }
        assert!(
            (last.1 / z - 1.0).abs() < 1e-6,
            "not back out to both: {last:?}"
        );
    }

    #[test]
    fn the_clip_puts_the_lamps_centre_a_little_above_the_middle_and_fills_six_tenths() {
        let two = Lean {
            lamps: vec![LonLat::new(-70.0, 35.0), LonLat::new(-66.0, 36.0)],
            trail: vec![],
            r1: 166.0,
            half: 180.0,
            focus: None,
            whole: vec![],
            zoom_cap: 0.0,
            land_km: None,
            land_far_km: None,
            scale: 0.0,
            journey: false,
        };
        for roll in [0.0, 40.0] {
            let cam = lean(
                Camera {
                    roll,
                    ..Camera::new(-50.0, 20.0, 1.0)
                },
                &two,
                1.0,
            );
            let r = 166.0 * cam.zoom;
            let pts: Vec<(f64, f64)> = two
                .lamps
                .iter()
                .map(|p| {
                    let v = cam.view(p.to_xyz());
                    (180.0 + r * v[0], 320.0 - r * v[1])
                })
                .collect();
            let (mx, my) = ((pts[0].0 + pts[1].0) / 2.0, (pts[0].1 + pts[1].1) / 2.0);
            let span = ((pts[0].0 - pts[1].0).powi(2) + (pts[0].1 - pts[1].1).powi(2)).sqrt();
            assert!(
                (my - (320.0 - FRAME_LIFT * 640.0)).abs() < 3.0,
                "roll {roll}: centre at y {my}"
            );
            if roll == 0.0 {
                assert!((mx - 180.0).abs() < 3.0, "centre at x {mx}");
            }
            // the span, less the margin, about 60% of the width
            assert!(span > 0.5 * 360.0 && span < 0.62 * 360.0, "span {span}");
        }
    }

    #[test]
    fn the_braid_is_held_whole_when_it_fits_and_its_recent_part_when_not() {
        // two lamps near (153, -29), their lines since day 0 running 1° a day south-west, 20 days
        let whole: Vec<(f64, LonLat)> = (0..=20)
            .flat_map(|d| {
                let d = d as f64;
                [
                    (
                        d,
                        LonLat::new(153.0 + (20.0 - d) * 0.1, -29.0 + (20.0 - d) * 0.1),
                    ),
                    (
                        d,
                        LonLat::new(153.2 + (20.0 - d) * 0.1, -29.1 + (20.0 - d) * 0.1),
                    ),
                ]
            })
            .collect();
        let l = Lean {
            lamps: vec![LonLat::new(153.0, -29.0), LonLat::new(153.2, -29.1)],
            trail: vec![],
            r1: 166.0,
            half: 180.0,
            focus: None,
            whole,
            zoom_cap: 0.0,
            land_km: None,
            land_far_km: None,
            scale: 0.0,
            journey: false,
        };
        // a wide floor: all of it
        let all = braid_lean(&l, 2.0);
        assert_eq!(all.trail.len(), 42);
        // a close floor: only the recent part, framed no wider than the floor
        let floor = (frame(&all).unwrap().1 * 1.5).min(FRAME_ZOOM_MAX * 0.8);
        let part = braid_lean(&l, floor);
        assert!(
            part.trail.len() < 42 && part.trail.len() >= 2,
            "{}",
            part.trail.len()
        );
        assert!(
            frame(&part).unwrap().1 >= floor * 0.99,
            "{:?}",
            frame(&part)
        );
    }

    #[test]
    fn the_clip_camera_never_steps_faster_than_its_limits_whatever_the_frame_asks() {
        // a pair whose framing jumps: a coast comes into reach mid-way (the cap and the pull change at
        // once), the lamps jump a degree (a cue), and the opening hands over to the follow
        let mut cc = ClipCamera::new(Camera::new(150.0, -30.0, 1.0));
        let f = Framing {
            braid: true,
            hold_out: JOURNEY_HOLD_OUT,
            ..Framing::default()
        };
        let mut last: Option<Camera> = None;
        let mut last_dz: Option<f64> = None;
        let fps = 30.0;
        for i in 0..300 {
            let t = i as f64 / fps;
            let jump = if i > 150 { 1.0 } else { 0.0 };
            let l = Lean {
                lamps: vec![
                    LonLat::new(156.7 + jump, -29.0),
                    LonLat::new(157.1 + jump, -29.0),
                ],
                trail: vec![LonLat::new(156.5, -28.9)],
                r1: 166.0,
                half: 180.0,
                focus: None,
                whole: vec![(0.0, LonLat::new(156.0, -28.0))],
                zoom_cap: if i > 40 { 20.0 } else { FRAME_ZOOM_OPEN_SEA },
                land_km: if i > 40 { Some(300.0) } else { None },
                land_far_km: Some(400.0),
                scale: 0.0,
                journey: false,
            };
            let part = if t < f.hold_in {
                ClipPart::Before
            } else if i < 270 {
                ClipPart::During
            } else {
                ClipPart::After
            };
            let through = if part == ClipPart::After {
                (i - 270) as f64 / 30.0 / f.hold_out
            } else {
                (t / 9.0).min(1.0)
            };
            let cam = cc.step(
                Camera::new(150.0, -30.0, 1.0),
                part,
                through,
                1.0 / fps,
                &f,
                &l,
            );
            if let Some(p) = last {
                let dz = (cam.zoom / p.zoom).ln();
                assert!(
                    dz.abs() <= ZOOM_RATE / fps + 1e-6,
                    "frame {i}: the zoom stepped {:.1}%",
                    dz.abs() * 100.0
                );
                if let Some(pd) = last_dz {
                    let ddz: f64 = dz - pd;
                    assert!(
                        ddz.abs() <= ZOOM_ACCEL / (fps * fps) + 1e-6,
                        "frame {i}: the zoom's rate changed by {ddz}"
                    );
                }
                last_dz = Some(dz);
            }
            last = Some(cam);
        }
    }

    #[test]
    fn a_journey_follows_a_running_pair_in_as_it_closes_and_keeps_it_in_frame() {
        // (R22) a pair running east at a dance's pace (~35 km a second of clip) and closing from ~170 km
        // to ~25 km apart: the journey's camera keeps both in frame and comes back in (R17-R21's only
        // pulled out), the pair a fair share of the width at the end
        let mut cc = ClipCamera::new(Camera::new(150.0, -40.0, 1.0));
        let f = Framing {
            braid: true,
            hold_out: JOURNEY_HOLD_OUT,
            ..Framing::default()
        };
        let fps = 60.0;
        let at = |t: f64| {
            let c = 150.0 + 0.4 * t;
            let gap = 2.0 - 1.7 * (t / 16.0).min(1.0);
            (
                LonLat::new(c - gap / 2.0, -40.0),
                LonLat::new(c + gap / 2.0, -40.0),
            )
        };
        let mut whole = Vec::new();
        let (mut mid, mut last) = (0.0, Camera::new(150.0, -40.0, 1.0));
        for i in 0..(20.0 * fps) as usize {
            let t = i as f64 / fps;
            let (a, b) = at(t);
            whole.push((2.0 * t, a));
            whole.push((2.0 * t, b));
            let l = Lean {
                lamps: vec![a, b],
                trail: vec![],
                r1: 166.0,
                half: 180.0,
                focus: None,
                whole: whole.clone(),
                zoom_cap: FRAME_ZOOM_OPEN_SEA,
                land_km: None,
                land_far_km: None,
                scale: 0.0,
                journey: false,
            };
            let part = if t < f.hold_in {
                ClipPart::Before
            } else {
                ClipPart::During
            };
            let cam = cc.step(
                Camera::new(150.0, -40.0, 1.0),
                part,
                (t / 30.0).min(1.0),
                1.0 / fps,
                &f,
                &l,
            );
            if t > OPEN_SECS {
                for p in [a, b] {
                    let v = cam.view(p.to_xyz());
                    let (x, y) = (
                        180.0 + 166.0 * cam.zoom * v[0],
                        320.0 - 166.0 * cam.zoom * v[1],
                    );
                    assert!(
                        v[2] > 0.0 && (0.0..=360.0).contains(&x) && (0.0..=640.0).contains(&y),
                        "{t:.2} s: a lamp at {x:.0},{y:.0} (zoom {:.1})",
                        cam.zoom
                    );
                }
            }
            if (t - 6.0).abs() < 0.5 / fps {
                mid = cam.zoom;
            }
            last = cam;
        }
        let (a, b) = at(20.0);
        let (va, vb) = (last.view(a.to_xyz()), last.view(b.to_xyz()));
        let span = 166.0 * last.zoom * (va[0] - vb[0]).abs() / 360.0;
        assert!(
            last.zoom > 2.0 * mid,
            "the zoom came in from {mid:.1} only to {:.1}",
            last.zoom
        );
        // 25 km apart is about 30% of the width at `JOURNEY_ZOOM_MAX`, which the zoom is still easing into
        assert!(
            span >= 0.2,
            "the pair spans {:.0}% of the width at the end",
            span * 100.0
        );
    }
}
