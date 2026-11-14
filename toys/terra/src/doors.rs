//! The lights on Terra's Earth: one for each toy it holds, at the place where the toy is real and where
//! it opens, each alive with a small likeness of its toy's own rule (the fireflies' small wood flashes together,
//! the swell's crests close on Nazaré). From afar a
//! light glints now and then; touched, it stirs.

use std::sync::Arc;
use toy_kit::paint::Painter;
use toy_kit::{Assets, Ctx, Globe, LonLat};

use crate::rooms::Room;

mod fresh;
mod likeness;
mod places;
pub use places::{doors, SCENES};

pub type Opener = fn(&Ctx, &Arc<dyn Assets>) -> anyhow::Result<Box<dyn Room>>;

pub struct Door {
    pub id: &'static str,
    /// Its toy's name (VoiceOver says it).
    pub name: &'static str,
    /// Where the light is: where the toy opens.
    pub at: [f64; 2],
    pub tint: [f32; 3],
    /// A night toy: the Earth falls into night as the camera dives to it.
    pub night: bool,
    /// Its note when touched (a pentatonic degree).
    pub degree: i32,
    pub open: Opener,
}

// The fireflies' light: a small wood of them about Elkmont, flashing together in bursts and dark between
// (the toy's rule, small): where each is in the light (a part of its size), and its own little lag.
const FIREFLY_SPOTS: usize = 18;
/// The likeness's beat (s): a burst of five flashes half a second apart, then dark.
const FIREFLY_BEAT: f64 = 6.0;

/// The lights' life: what moves in them, their glints, a touch's stir, a bare tap's ring.
pub struct Life {
    pub doors: Vec<Door>,
    /// The fireflies: where each is in the light (x, y, a part of its size) and its lag (s).
    flies: Vec<(f64, f64, f64)>,
    seed: u64,
    glint_at: Vec<f64>,
    stir_at: Vec<f64>,
    /// Bare taps: where (lon, lat) and when.
    rings: Vec<(LonLat, f64)>,
    /// The light that beckons until the first touch (Terra sets it at launch).
    pub beckon: Option<usize>,
    /// Lights whose toys an update brought and that have not been opened yet (`fresh`).
    new: Vec<bool>,
    /// Toys known before, which this build's map no longer holds (kept in the saved list).
    away: Vec<String>,
}

impl Default for Life {
    fn default() -> Self {
        Self::new()
    }
}

fn rand(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Life {
    pub fn new() -> Life {
        let doors = doors();
        let n = doors.len();
        let mut life = Life {
            doors,
            flies: Vec::new(),
            seed: 0x9E37_79B9_7F4A_7C15,
            glint_at: (0..n).map(|i| 1.5 + i as f64 * 1.37).collect(),
            stir_at: vec![-100.0; n],
            rings: Vec::new(),
            beckon: None,
            new: vec![false; n],
            away: Vec::new(),
        };
        life.scatter_flies();
        life
    }

    fn scatter_flies(&mut self) {
        let mut s = self.seed;
        self.flies = (0..FIREFLY_SPOTS)
            .map(|_| {
                let a = rand(&mut s) * std::f64::consts::TAU;
                let d = rand(&mut s).sqrt() * 1.6;
                (a.cos() * d, a.sin() * d * 0.7, rand(&mut s) * 0.25)
            })
            .collect();
        self.seed = s;
    }

    /// The fireflies' likeness: a small wood of them in the light, flashing together, dark between.
    #[allow(clippy::too_many_arguments)] // the light's place, size and fade, as paint has them
    fn paint_flies(
        &self,
        p: &mut Painter,
        c: [f32; 2],
        rk: f64,
        kf: f32,
        tm: f64,
        a: f32,
        still: bool,
    ) {
        for &(x, y, lag) in &self.flies {
            let q = [c[0] + (x * rk) as f32, c[1] + (y * rk) as f32];
            let fl = if still {
                0.3
            } else {
                Self::fly_light(tm, lag) as f32
            };
            let ink = [
                0.22 + 0.5 * fl,
                0.38 + 0.5 * fl,
                0.10 + 0.1 * fl,
                (0.35 + 0.6 * fl) * a,
            ];
            p.disc(q, (0.9 + 0.7 * fl) * kf, 1.0, ink);
            p.glow(
                q,
                q,
                (1.6 + 3.0 * fl) * kf,
                3.0 * kf,
                [0.75, 1.0, 0.35, 0.7 * fl * a],
            );
        }
    }

    /// A likeness firefly's light at `t`: a burst of five flashes, then dark.
    fn fly_light(t: f64, lag: f64) -> f64 {
        let u = (t - lag).rem_euclid(FIREFLY_BEAT);
        if u > 2.6 {
            return 0.0;
        }
        let v = (u % 0.52) / 0.16;
        (-(v - 0.3).powi(2) * 6.0).exp()
    }

    pub fn update(&mut self, t: f64, _dt: f64) {
        self.rings.retain(|r| t - r.1 < 1.6);
        for (i, g) in self.glint_at.iter_mut().enumerate() {
            if t > *g + 1.4 {
                *g = t + 8.0 + (i as f64 * 1.9) % 5.0;
            }
        }
    }

    pub fn stir(&mut self, i: usize, t: f64) {
        self.stir_at[i] = t;
    }

    pub fn glint(&mut self, i: usize, t: f64) {
        self.glint_at[i] = t;
    }

    pub fn ring(&mut self, at: LonLat, t: f64) {
        self.rings.push((at, t));
    }

    /// Where light `i` truly is on screen (points) and how far it faces the reader.
    pub fn true_spot(&self, globe: &Globe, i: usize) -> ([f64; 2], f64) {
        let d = &self.doors[i];
        globe.project(LonLat::new(d.at[0], d.at[1]))
    }

    /// Where every light is shown (points): its true place, unless lights crowd (Europe's three from
    /// afar), when they are eased apart until each has its room; a fine line then joins each to its place.
    pub fn shown(&self, globe: &Globe) -> Vec<([f64; 2], f64)> {
        let mut out: Vec<([f64; 2], f64)> = (0..self.doors.len())
            .map(|i| self.true_spot(globe, i))
            .collect();
        let room = Self::size(globe) * 2.3;
        for _ in 0..24 {
            for i in 0..out.len() {
                for j in i + 1..out.len() {
                    if out[i].1 <= 0.3 || out[j].1 <= 0.3 {
                        continue;
                    }
                    let (a, b) = (out[i].0, out[j].0);
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let d = dx.hypot(dy);
                    if d < room {
                        let (ux, uy) = if d > 1e-3 {
                            (dx / d, dy / d)
                        } else {
                            (1.0, 0.0)
                        };
                        let push = (room - d) * 0.5 * 0.5;
                        out[i].0 = [a[0] - ux * push, a[1] - uy * push];
                        out[j].0 = [b[0] + ux * push, b[1] + uy * push];
                    }
                }
            }
        }
        out
    }

    /// Where light `i` is shown (points) and how far it faces the reader.
    pub fn spot(&self, globe: &Globe, i: usize) -> ([f64; 2], f64) {
        self.shown(globe)[i]
    }

    /// The light's size, points: grows a little as the reader comes closer, and with the screen (the reading
    /// scale twice: a big screen's Earth is held farther off and is larger, and its lights must read from
    /// across it as a phone's do; painting-company row 246).
    pub fn size(globe: &Globe) -> f64 {
        13.0 * globe.reading().powi(2) * globe.camera.zoom.clamp(1.0, 5.0).sqrt()
    }

    /// The light under a point, if any: the nearest whose middle is within reach (a light's own size, at
    /// least a fingertip's 30 points at the reading scale), counted from where it is shown (eased apart
    /// from a crowd).
    pub fn hit(&self, globe: &Globe, p: [f64; 2]) -> Option<usize> {
        let reach = (Self::size(globe) * 1.8).max(30.0 * globe.reading());
        let shown = self.shown(globe);
        (0..self.doors.len())
            .filter_map(|i| {
                let (s, facing) = shown[i];
                if facing <= 0.35 {
                    return None;
                }
                let mid = (s[0] - p[0]).hypot(s[1] - p[1]);
                (mid < reach).then_some((i, mid))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The light nearest a point among those facing the reader.
    pub fn nearest(&self, globe: &Globe, p: [f64; 2]) -> Option<usize> {
        (0..self.doors.len())
            .filter_map(|i| {
                let (s, facing) = self.spot(globe, i);
                (facing > 0.35).then_some((i, (s[0] - p[0]).hypot(s[1] - p[1])))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// Every light into the painter (pixels: `k` pixels a point). `fade` dims them all (a dive under way).
    /// `focus`: only that light, at that size (points) and its halo's strength, as a passage carries it down into its toy.
    #[allow(clippy::too_many_arguments)] // a drawing call: each argument is one thing it draws with
    pub fn paint(
        &self,
        p: &mut Painter,
        globe: &Globe,
        t: f64,
        k: f64,
        fade: f64,
        reduce_motion: bool,
        focus: Option<(usize, f64, f64)>,
        skip: Option<usize>,
    ) {
        let kf = k as f32;
        let r0 = Self::size(globe);
        let px = |pt: [f64; 2]| [(pt[0] * k) as f32, (pt[1] * k) as f32];
        let at = |ll: [f64; 2]| -> Option<([f32; 2], f64)> {
            let (s, f) = globe.project(LonLat::new(ll[0], ll[1]));
            (f > 0.0).then(|| (px(s), f))
        };
        let tm = if reduce_motion { 0.0 } else { t };
        // bare taps: a soft gold ring where the finger was
        for (ll, t0) in &self.rings {
            if let Some((c, f)) = at([ll.lon, ll.lat]) {
                let u = ((t - t0) / 1.6).clamp(0.0, 1.0);
                let a = (1.0 - u).powf(1.5) * smooth(0.0, 0.2, f) * 0.8;
                p.ring(
                    c,
                    ((6.0 + 34.0 * u) * k) as f32,
                    1.4 * kf,
                    1.0,
                    [0.69, 0.54, 0.24, a as f32],
                );
            }
        }
        let shown = self.shown(globe);
        for (i, d) in self.doors.iter().enumerate() {
            if skip == Some(i) {
                continue;
            }
            let (r, halo) = match focus {
                Some((f, _, _)) if f != i => continue,
                Some((_, size, halo)) => (size, halo as f32),
                None => (r0, 1.0),
            };
            let (s, facing) = shown[i];
            let [sw, sh] = [
                globe.view().width as f64 / globe.view().ppp,
                globe.view().height as f64 / globe.view().ppp,
            ];
            let edge = s[0].min(sw - s[0]).min(s[1]).min(sh - s[1]);
            let vis = smooth(0.3, 0.5, facing) * smooth(r * 0.9, r * 1.6, edge) * fade;
            if vis <= 0.01 {
                continue;
            }
            let c = px(s);
            let a = vis as f32;
            let tint = d.tint;
            // eased apart from a crowd: the whole likeness moves with it, and a fine line goes to its place
            let truth = px(self.true_spot(globe, i).0);
            let (ox, oy) = (c[0] - truth[0], c[1] - truth[1]);
            if ox.hypot(oy) > 3.0 * kf {
                p.line(
                    truth,
                    c,
                    0.45 * kf,
                    0.8,
                    [tint[0] * 0.6, tint[1] * 0.6, tint[2] * 0.6, 0.6 * a],
                );
                p.disc(
                    truth,
                    1.3 * kf,
                    0.8,
                    [tint[0] * 0.6, tint[1] * 0.6, tint[2] * 0.6, 0.8 * a],
                );
            }
            // the beckoning light (until the first touch): a slow ring breathing out of it
            if Some(i) == self.beckon && !reduce_motion {
                let ph = (t / 2.6).rem_euclid(1.0);
                p.ring(
                    c,
                    ((r * (1.1 + 1.6 * ph)) * k) as f32,
                    1.6 * kf,
                    1.5,
                    [
                        tint[0],
                        tint[1],
                        tint[2],
                        ((1.0 - ph).powf(1.5) * 0.7) as f32 * a,
                    ],
                );
            }
            // a soft halo of its colour, so it reads from across the Earth
            p.disc(
                c,
                (r * 1.15 * k) as f32,
                (r * 1.1 * k) as f32,
                [1.0, 0.99, 0.95, 0.55 * a * halo * (1.0 - globe.night)],
            );
            p.disc(
                c,
                (r * 1.0 * k) as f32,
                (r * 1.0 * k) as f32,
                [tint[0], tint[1], tint[2], 0.32 * a * halo],
            );
            match d.id {
                "murmuration" => {
                    // a window onto the toy's own dusk: the sky's blue over a low warm glow, the reeds' dark
                    // line under it, and a flock in it folding and turning, one dark body
                    let sky_a = a * halo;
                    p.disc(
                        c,
                        (r * 1.05 * k) as f32,
                        (r * 0.25 * k) as f32,
                        [0.30, 0.36, 0.50, 0.92 * sky_a],
                    );
                    p.disc(
                        [c[0], c[1] + (r * 0.55 * k) as f32],
                        (r * 0.62 * k) as f32,
                        (r * 0.55 * k) as f32,
                        [1.0, 0.70, 0.36, 0.75 * sky_a],
                    );
                    p.disc(
                        [c[0], c[1] + (r * 0.62 * k) as f32],
                        (r * 0.16 * k) as f32,
                        (r * 0.08 * k) as f32,
                        [1.0, 0.92, 0.70, 0.8 * sky_a],
                    );
                    let n = 140;
                    let turn = 0.6 * (tm * 0.21).sin();
                    let (st, ct) = (turn.sin(), turn.cos());
                    let stretch = 1.0 + 0.45 * (tm * 0.53).sin();
                    for b in 0..n {
                        let h1 = ((b as f64 * 12.9898).sin() * 43758.5453).fract().abs();
                        let h2 = ((b as f64 * 78.233).sin() * 43758.5453).fract().abs();
                        // a point in a disc, its density highest at the middle, the disc folding as a wave runs through
                        let ang = std::f64::consts::TAU * h1;
                        let rad = h2.sqrt();
                        let mut x = rad * ang.cos() * stretch;
                        let mut y = rad * ang.sin() / stretch * 0.6;
                        y += 0.28 * (x * 2.6 + tm * 1.7).sin();
                        x += 0.12 * (y * 3.1 + tm * 1.3).sin();
                        let (x, y) = (x * r * 0.8, y * r * 0.8 - r * 0.15);
                        let q = [
                            c[0] + ((x * ct - y * st) * k) as f32,
                            c[1] + ((x * st + y * ct) * k) as f32,
                        ];
                        p.disc(q, 0.95 * kf, 0.8, [0.13, 0.11, 0.15, 0.85 * a]);
                    }
                }
                "fireflies" => self.paint_flies(p, c, r * k, kf, tm, a, reduce_motion),
                "swell" => {
                    // the swell's crests closing in on Nazaré from all round, as the canyon focuses them there
                    let centre = LonLat::new(d.at[0], d.at[1]).to_xyz();
                    for j in 0..3 {
                        let ph = (tm / 6.0 - j as f64 / 3.0).rem_euclid(1.0);
                        let theta = 0.004 + 0.06 * (1.0 - ph);
                        let al = ph * smooth(1.0, 0.92, ph);
                        sphere_ring(
                            p,
                            globe,
                            centre,
                            theta,
                            k,
                            [ox, oy],
                            2.0 * kf,
                            [tint[0] * 0.8, tint[1] * 0.8, tint[2] * 0.8, al as f32 * a],
                        );
                    }
                    p.disc(c, 2.2 * kf, 1.0, [tint[0], tint[1], tint[2], 0.9 * a]);
                }
                "meander" => {
                    // a window onto the floodplain: the forest's dark green, the river's tan line wandering
                    // through it (its bends slowly growing and moving on), and a cut-off lake's dark crescent
                    let w = a * halo;
                    p.disc(
                        c,
                        (r * 1.05 * k) as f32,
                        (r * 0.2 * k) as f32,
                        [0.10, 0.25, 0.17, 0.95 * w],
                    );
                    let lake = [c[0] - (r * 0.38 * k) as f32, c[1] - (r * 0.1 * k) as f32];
                    p.ring(
                        lake,
                        (r * 0.28 * k) as f32,
                        (r * 0.09 * k) as f32,
                        1.0,
                        [0.07, 0.12, 0.22, 0.8 * a],
                    );
                    let n = 28;
                    let mut prev: Option<[f32; 2]> = None;
                    for j in 0..=n {
                        let u = j as f64 / n as f64 * 2.0 - 1.0;
                        let amp = 0.42 + 0.12 * (tm * 0.13).sin();
                        let x = amp * (u * 4.4 + tm * 0.25).sin() * (1.0 - u * u).sqrt();
                        let q = [c[0] + (x * r * k) as f32, c[1] + (u * r * 0.92 * k) as f32];
                        if let Some(pp) = prev {
                            p.line(
                                pp,
                                q,
                                (0.075 * r * k) as f32,
                                1.0,
                                [0.80, 0.66, 0.44, 0.95 * a],
                            );
                        }
                        prev = Some(q);
                    }
                }
                "throats" => {
                    // a window onto the hillside from above: the dry grass, and the lizards' three throats
                    // chasing each other round in a spiral (orange takes blue's rock, blue yellow's, yellow
                    // orange's), slowly turning
                    let w = a * halo;
                    p.disc(
                        c,
                        (r * 1.05 * k) as f32,
                        (r * 0.2 * k) as f32,
                        [0.58, 0.50, 0.32, 0.95 * w],
                    );
                    let throats = [[0.95, 0.52, 0.18], [0.24, 0.42, 0.85], [0.96, 0.86, 0.30]];
                    for (arm, col) in throats.iter().enumerate() {
                        let mut prev: Option<[f32; 2]> = None;
                        for j in 0..=24 {
                            let u = j as f64 / 24.0;
                            let ang = -tm * 0.6
                                + arm as f64 * std::f64::consts::TAU / 3.0
                                + u * std::f64::consts::TAU * 0.9;
                            let rad = r * 0.9 * u;
                            let q = [
                                c[0] + (ang.cos() * rad * k) as f32,
                                c[1] + (ang.sin() * rad * k) as f32,
                            ];
                            if let Some(pp) = prev {
                                p.line(
                                    pp,
                                    q,
                                    (0.16 * r * k * (0.4 + u)) as f32,
                                    1.0,
                                    [col[0], col[1], col[2], 0.9 * a],
                                );
                            }
                            prev = Some(q);
                        }
                    }
                }
                "frost" => {
                    // a window of dark glass at -50°: frost growing on it in six-armed feathers, then fading
                    // as if a finger had warmed it, and growing back
                    let w = a * halo;
                    p.disc(
                        c,
                        (r * 1.05 * k) as f32,
                        (r * 0.2 * k) as f32,
                        [0.06, 0.10, 0.20, 0.95 * w],
                    );
                    let ph = (tm / 9.0).rem_euclid(1.0);
                    let grow = smooth(0.0, 0.7, ph) * (1.0 - smooth(0.85, 1.0, ph));
                    let ice = [0.80, 0.90, 1.0, 0.9 * a];
                    for arm in 0..6 {
                        let ang = arm as f64 * std::f64::consts::FRAC_PI_3 + 0.3;
                        let (sa, ca) = ang.sin_cos();
                        let len = r * 0.85 * grow;
                        let tip = [c[0] + (ca * len * k) as f32, c[1] + (sa * len * k) as f32];
                        p.line(c, tip, 0.9 * kf, 0.8, ice);
                        for b in 1..4 {
                            let at = len * b as f64 / 4.0;
                            let bl = (len - at) * 0.45;
                            let o = [c[0] + (ca * at * k) as f32, c[1] + (sa * at * k) as f32];
                            for side in [-1.0f64, 1.0] {
                                let ba = ang + side * 1.0;
                                p.line(
                                    o,
                                    [
                                        o[0] + (ba.cos() * bl * k) as f32,
                                        o[1] + (ba.sin() * bl * k) as f32,
                                    ],
                                    0.6 * kf,
                                    0.8,
                                    ice,
                                );
                            }
                        }
                    }
                }
                _ => likeness::paint(p, d.id, c, r, k, tm, a, halo, i, tint),
            }
            self.paint_new(p, i, c, r, k, t, a * halo, reduce_motion);
            // touched: it stirs (a ring of its colour going out)
            let u = t - self.stir_at[i];
            if (0.0..1.0).contains(&u) {
                p.ring(
                    c,
                    ((r * 0.6 + r * 2.6 * u) * k) as f32,
                    2.0 * kf,
                    1.5,
                    [
                        tint[0],
                        tint[1],
                        tint[2],
                        ((1.0 - u).powf(1.4) * 0.9) as f32 * a,
                    ],
                );
            }
            // from afar, now and then, a glint: the eye is drawn before it knows to what
            let gt = t - self.glint_at[i];
            if (0.0..1.3).contains(&gt) && globe.camera.zoom < 2.2 && !reduce_motion {
                let g = (std::f64::consts::PI * gt / 1.3).sin().powi(2) as f32 * a;
                let long = (r * 1.25 * k) as f32;
                let short = long * 0.45;
                let w = [1.0, 0.97, 0.88, 0.95 * g];
                p.glow(
                    [c[0] - long, c[1]],
                    [c[0] + long, c[1]],
                    0.8 * kf,
                    1.6 * kf,
                    w,
                );
                p.glow(
                    [c[0], c[1] - long],
                    [c[0], c[1] + long],
                    0.8 * kf,
                    1.6 * kf,
                    w,
                );
                let dg = short * std::f32::consts::FRAC_1_SQRT_2;
                p.glow(
                    [c[0] - dg, c[1] - dg],
                    [c[0] + dg, c[1] + dg],
                    0.6 * kf,
                    1.2 * kf,
                    w,
                );
                p.glow(
                    [c[0] - dg, c[1] + dg],
                    [c[0] + dg, c[1] - dg],
                    0.6 * kf,
                    1.2 * kf,
                    w,
                );
                p.glow(c, c, 3.0 * kf, 4.0 * kf, [1.0, 0.95, 0.8, 0.8 * g]);
            }
        }
    }
}

/// A circle on the sphere: the places `theta` radians from `centre`, drawn where they face the reader.
#[allow(clippy::too_many_arguments)] // a drawing call: each argument is one thing it draws with
fn sphere_ring(
    p: &mut Painter,
    globe: &Globe,
    centre: [f64; 3],
    theta: f64,
    k: f64,
    off: [f32; 2],
    width: f32,
    col: [f32; 4],
) {
    let c = centre;
    // two directions square to the centre
    let a = if c[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let cross = |x: [f64; 3], y: [f64; 3]| {
        [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ]
    };
    let norm = |v: [f64; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let u = norm(cross(c, a));
    let v = cross(c, u);
    let (st, ct) = theta.sin_cos();
    let n = 72;
    let mut prev: Option<([f32; 2], f64)> = None;
    for i in 0..=n {
        let ph = std::f64::consts::TAU * i as f64 / n as f64;
        let (sp, cp) = ph.sin_cos();
        let w = [
            c[0] * ct + (u[0] * cp + v[0] * sp) * st,
            c[1] * ct + (u[1] * cp + v[1] * sp) * st,
            c[2] * ct + (u[2] * cp + v[2] * sp) * st,
        ];
        let (s, f) = globe.project(LonLat::from_xyz(w));
        let q = ([(s[0] * k) as f32 + off[0], (s[1] * k) as f32 + off[1]], f);
        if let Some(pq) = prev {
            if pq.1 > 0.02 && q.1 > 0.02 {
                let fa = smooth(0.02, 0.25, pq.1.min(q.1)) as f32;
                p.line(
                    pq.0,
                    q.0,
                    width * 0.5,
                    1.0,
                    [col[0], col[1], col[2], col[3] * fa],
                );
            }
        }
        prev = Some(q);
    }
}
