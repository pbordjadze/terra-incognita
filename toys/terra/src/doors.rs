//! The lights on Terra's Earth: one for each toy it holds, at the place where the toy is real and where
//! it opens, each alive with a small likeness of its toy's own rule (the whales' song-rings travel their
//! real routes, the fireflies' cities fall into step, the bell's ring crosses the globe). From afar a
//! light glints now and then; touched, it stirs.

use std::sync::Arc;
use toy_kit::paint::Painter;
use toy_kit::{Assets, Ctx, Globe, LonLat};

use crate::rooms::{self, Room};

pub type Opener = fn(&Ctx, &Arc<dyn Assets>) -> anyhow::Result<Box<dyn Room>>;

pub struct Door {
    pub id: &'static str,
    /// Its toy's name (VoiceOver says it).
    pub name: &'static str,
    /// Where the light is: where the toy opens (the whales' is their middle, as it moves).
    pub at: [f64; 2],
    pub tint: [f32; 3],
    /// A night toy: the Earth falls into night as the camera dives to it.
    pub night: bool,
    /// Its note when touched (a pentatonic degree).
    pub degree: i32,
    pub open: Opener,
}

// Europe's cities the mold's light joins (lon, lat), and the threads between them.
const MOLD_CITIES: [[f64; 2]; 8] = [
    [13.40, 52.52], // Berlin
    [21.01, 52.23], // Warsaw
    [14.42, 50.09], // Prague
    [25.28, 54.69], // Vilnius
    [27.56, 53.90], // Minsk
    [30.52, 50.45], // Kyiv
    [16.37, 48.21], // Vienna
    [19.04, 47.50], // Budapest
];
const MOLD_THREADS: [(usize, usize); 10] = [(0, 2), (0, 1), (1, 3), (1, 4), (3, 4), (1, 5), (2, 6), (6, 7), (7, 5), (4, 5)];

// The eastern United States' cities, each a firefly, round the Great Smoky Mountains (whose fireflies
// really do flash in step, the toy's rule says).
const FIREFLY_CITIES: [[f64; 2]; 16] = [
    [-74.01, 40.71],
    [-75.17, 39.95],
    [-77.04, 38.91],
    [-71.06, 42.36],
    [-84.39, 33.75],
    [-80.84, 35.23],
    [-86.78, 36.16],
    [-87.63, 41.88],
    [-83.05, 42.33],
    [-80.00, 40.44],
    [-83.00, 39.96],
    [-86.16, 39.77],
    [-90.20, 38.63],
    [-83.92, 35.96],
    [-78.64, 35.78],
    [-84.51, 39.10],
];

pub const WHALES: usize = 0;
pub const FIREFLIES: usize = 3;

/// Every toy Terra holds, one line each. A new toy: its crate in Cargo.toml, its line here (where it opens,
/// its colour, its note), its name in tools/toys/terra/manifest.py; a likeness of its own in `paint` is
/// optional (without one it is a breathing light of its colour).
pub fn doors() -> Vec<Door> {
    vec![
        Door { id: "whales", name: "Whales", at: [-125.0, 30.0], tint: [0.80, 0.60, 0.26], night: true, degree: 0, open: rooms::open::<whales::Whales> },
        Door { id: "murmuration", name: "Murmuration", at: [murmuration::PLACE.0, murmuration::PLACE.1], tint: [0.93, 0.58, 0.42], night: false, degree: 4, open: rooms::open::<murmuration::Murmuration> },
        Door { id: "mold", name: "Mold", at: [21.5, 51.2], tint: [0.95, 0.76, 0.28], night: true, degree: 2, open: rooms::open::<mold::Mold> },
        Door { id: "fireflies", name: "Fireflies", at: [-81.5, 38.0], tint: [0.62, 0.92, 0.36], night: true, degree: 6, open: rooms::open::<fireflies::Fireflies> },
        Door { id: "bell", name: "Bell", at: [-73.05, -38.24], tint: [0.62, 0.42, 0.22], night: false, degree: -2, open: rooms::open::<bell::Bell> },
        Door { id: "swell", name: "Swell", at: [-178.0, 20.0], tint: [0.20, 0.45, 0.70], night: false, degree: 3, open: rooms::open::<swell::Swell> },
        Door { id: "jam", name: "Ghost Jam", at: [136.91, 35.18], tint: [0.86, 0.20, 0.12], night: true, degree: 5, open: rooms::open::<jam::Jam> },
        Door { id: "meander", name: "Meander", at: [-67.03, -12.97], tint: [0.30, 0.52, 0.62], night: false, degree: 7, open: rooms::open::<meander::Meander> },
        Door { id: "frost", name: "Frost", at: [142.79, 63.46], tint: [0.22, 0.42, 0.78], night: false, degree: 9, open: rooms::open::<frost::Frost> },
    ]
}

/// The lights' life: what moves in them, their glints, a touch's stir, a bare tap's ring.
pub struct Life {
    pub doors: Vec<Door>,
    whales: Option<[whales::route::Route; 2]>,
    /// The miniature whales' day (they swim their real routes, a day every four seconds).
    pub whale_day: f64,
    /// The fireflies' clocks (0..1) and their own paces.
    flies: Vec<(f64, f64)>,
    flies_since: f64,
    seed: u64,
    glint_at: Vec<f64>,
    stir_at: Vec<f64>,
    /// Bare taps: where (lon, lat) and when.
    rings: Vec<(LonLat, f64)>,
    /// The light that beckons until the first touch (Terra sets it at launch).
    pub beckon: Option<usize>,
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

/// The whales' first day in their own toy (whales/src/lib.rs's FIRST_DAY): the light shows them where
/// the toy will open.
const WHALE_FIRST_DAY: f64 = 323.0;

impl Life {
    pub fn new(assets: &dyn Assets) -> Life {
        let whales = assets.read("routes.json").ok().and_then(|j| whales::route::load(&j, ["humpback", "blue"]).ok());
        let doors = doors();
        let n = doors.len();
        let mut life = Life {
            doors,
            whales,
            whale_day: WHALE_FIRST_DAY,
            flies: Vec::new(),
            flies_since: 0.0,
            seed: 0x9E37_79B9_7F4A_7C15,
            glint_at: (0..n).map(|i| 1.5 + i as f64 * 1.37).collect(),
            stir_at: vec![-100.0; n],
            rings: Vec::new(),
            beckon: None,
        };
        life.scatter_flies();
        life.place_whales();
        life
    }

    fn scatter_flies(&mut self) {
        let mut s = self.seed;
        self.flies = FIREFLY_CITIES.iter().map(|_| (rand(&mut s), 0.9 + 0.2 * rand(&mut s))).collect();
        self.seed = s;
    }

    fn whale_spots(&self) -> Option<[LonLat; 2]> {
        let w = self.whales.as_ref()?;
        Some([w[0].lonlat(self.whale_day), w[1].lonlat(self.whale_day)])
    }

    fn place_whales(&mut self) {
        if let Some([a, b]) = self.whale_spots() {
            let (pa, pb) = (a.to_xyz(), b.to_xyz());
            let m = [pa[0] + pb[0], pa[1] + pb[1], pa[2] + pb[2]];
            let ll = LonLat::from_xyz(m);
            self.doors[WHALES].at = [ll.lon, ll.lat];
        }
    }

    pub fn update(&mut self, t: f64, dt: f64) {
        // the whales swim on (a slow year: back where they started after twenty-four minutes)
        self.whale_day = WHALE_FIRST_DAY + t * 0.25;
        self.place_whales();
        // the fireflies: each clock fills at its own pace; a neighbour's flash nudges it ahead (the
        // toy's rule, sixteen cities strong); every forty seconds they scatter and begin again
        if t - self.flies_since > 40.0 {
            self.flies_since = t;
            self.scatter_flies();
        }
        let n = self.flies.len() as f64;
        let mut flashed = 0.0;
        for f in &mut self.flies {
            f.0 += dt * f.1 * 0.8;
            if f.0 >= 1.0 {
                f.0 -= 1.0;
                flashed += 1.0;
            }
        }
        if flashed > 0.0 {
            for f in &mut self.flies {
                // nudged ahead by each flash, more the fuller its clock already is
                f.0 += 0.09 * flashed / n * 4.0 * f.0;
                if f.0 >= 1.0 {
                    f.0 = 0.999;
                }
            }
        }
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
        let mut out: Vec<([f64; 2], f64)> = (0..self.doors.len()).map(|i| self.true_spot(globe, i)).collect();
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
                        let (ux, uy) = if d > 1e-3 { (dx / d, dy / d) } else { (1.0, 0.0) };
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

    /// The light's size, points: grows a little as the reader comes closer.
    pub fn size(globe: &Globe) -> f64 {
        13.0 * globe.reading() * globe.camera.zoom.clamp(1.0, 5.0).sqrt()
    }

    /// The light under a point, if any (the nearest within reach).
    pub fn hit(&self, globe: &Globe, p: [f64; 2]) -> Option<usize> {
        let reach = (Self::size(globe) * 1.8).max(30.0);
        (0..self.doors.len())
            .filter_map(|i| {
                let (s, facing) = self.spot(globe, i);
                let d = (s[0] - p[0]).hypot(s[1] - p[1]);
                (facing > 0.35 && d < reach).then_some((i, d))
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
    pub fn paint(&self, p: &mut Painter, globe: &Globe, t: f64, k: f64, fade: f64, reduce_motion: bool, focus: Option<(usize, f64, f64)>, skip: Option<usize>) {
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
                p.ring(c, ((6.0 + 34.0 * u) * k) as f32, 1.4 * kf, 1.0, [0.69, 0.54, 0.24, a as f32]);
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
            let [sw, sh] = [globe.view().width as f64 / globe.view().ppp, globe.view().height as f64 / globe.view().ppp];
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
                p.line(truth, c, 0.45 * kf, 0.8, [tint[0] * 0.6, tint[1] * 0.6, tint[2] * 0.6, 0.6 * a]);
                p.disc(truth, 1.3 * kf, 0.8, [tint[0] * 0.6, tint[1] * 0.6, tint[2] * 0.6, 0.8 * a]);
            }
            let at = |ll: [f64; 2]| at(ll).map(|(q, f)| ([q[0] + ox, q[1] + oy], f));
            // the beckoning light (until the first touch): a slow ring breathing out of it
            if Some(i) == self.beckon && !reduce_motion {
                let ph = (t / 2.6).rem_euclid(1.0);
                p.ring(c, ((r * (1.1 + 1.6 * ph)) * k) as f32, 1.6 * kf, 1.5, [tint[0], tint[1], tint[2], ((1.0 - ph).powf(1.5) * 0.7) as f32 * a]);
            }
            // a soft halo of its colour, so it reads from across the Earth
            p.disc(c, (r * 1.15 * k) as f32, (r * 1.1 * k) as f32, [1.0, 0.99, 0.95, 0.55 * a * halo * (1.0 - globe.night)]);
            p.disc(c, (r * 1.0 * k) as f32, (r * 1.0 * k) as f32, [tint[0], tint[1], tint[2], 0.22 * a * halo]);
            match d.id {
                "whales" => {
                    if let Some(spots) = self.whale_spots() {
                        let cols = [[0.86, 0.62, 0.20], [0.20, 0.42, 0.76]];
                        for (w, ll) in spots.iter().enumerate() {
                            let Some((wc, _)) = at([ll.lon, ll.lat]) else { continue };
                            for j in 0..2 {
                                let ph = (tm * 0.42 + j as f64 * 0.5 + w as f64 * 0.27).rem_euclid(1.0);
                                let rr = r * (0.25 + 3.2 * ph);
                                let col = cols[w];
                                p.ring(wc, (rr * k) as f32, 1.3 * kf, 1.0, [col[0], col[1], col[2], ((1.0 - ph).powf(1.6) * 0.95) as f32 * a]);
                            }
                            let col = cols[w];
                            p.disc(wc, 2.4 * kf, 1.0, [col[0], col[1], col[2], 0.95 * a]);
                        }
                    }
                }
                "murmuration" => {
                    // a window onto the toy's own dusk: the sky's blue over a low warm glow, the reeds' dark
                    // line under it, and a flock in it folding and turning, one dark body
                    let sky_a = a * halo;
                    p.disc(c, (r * 1.05 * k) as f32, (r * 0.25 * k) as f32, [0.30, 0.36, 0.50, 0.92 * sky_a]);
                    p.disc([c[0], c[1] + (r * 0.55 * k) as f32], (r * 0.62 * k) as f32, (r * 0.55 * k) as f32, [1.0, 0.70, 0.36, 0.75 * sky_a]);
                    p.disc([c[0], c[1] + (r * 0.62 * k) as f32], (r * 0.16 * k) as f32, (r * 0.08 * k) as f32, [1.0, 0.92, 0.70, 0.8 * sky_a]);
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
                        let q = [c[0] + ((x * ct - y * st) * k) as f32, c[1] + ((x * st + y * ct) * k) as f32];
                        p.disc(q, 0.95 * kf, 0.8, [0.13, 0.11, 0.15, 0.85 * a]);
                    }
                }
                "mold" | "fireflies" => {
                    let cities: &[[f64; 2]] = if d.id == "mold" { &MOLD_CITIES } else { &FIREFLY_CITIES };
                    let pts: Vec<Option<([f32; 2], f64)>> = cities.iter().map(|&ll| at(ll)).collect();
                    if d.id == "mold" {
                        // threads reaching city to city, growing, held, then let go
                        for (e, &(x, y)) in MOLD_THREADS.iter().enumerate() {
                            let (Some((pa, _)), Some((pb, _))) = (pts[x], pts[y]) else { continue };
                            let ph = (tm * 0.09 + e as f64 * 0.137).rem_euclid(1.0);
                            let grow = smooth(0.0, 0.45, ph);
                            let fade_out = 1.0 - smooth(0.82, 1.0, ph);
                            let g = grow as f32;
                            let tip = [pa[0] + (pb[0] - pa[0]) * g, pa[1] + (pb[1] - pa[1]) * g];
                            let al = (fade_out as f32) * a;
                            p.line(pa, tip, 0.95 * kf, 0.9, [0.80, 0.55, 0.06, 0.95 * al]);
                            p.glow(pa, tip, 1.6 * kf, 2.0 * kf, [1.0, 0.78, 0.25, 0.14 * al]);
                        }
                        for q in pts.iter().flatten() {
                            p.disc(q.0, 1.9 * kf, 1.0, [0.72, 0.48, 0.05, a]);
                        }
                    } else {
                        for (j, q) in pts.iter().enumerate() {
                            let Some((q, _)) = q else { continue };
                            let clock = self.flies[j].0;
                            let flash = (-(clock / 0.06).powi(2)).exp().max((-((1.0 - clock) / 0.02).powi(2)).exp());
                            let fl = if reduce_motion { 0.3 } else { flash as f32 };
                            p.disc(*q, (1.4 + 0.8 * fl) * kf, 1.0, [0.22 + 0.5 * fl, 0.38 + 0.5 * fl, 0.10 + 0.1 * fl, 0.95 * a]);
                            p.glow(*q, *q, (2.0 + 3.5 * fl) * kf, 3.5 * kf, [0.75, 1.0, 0.35, 0.7 * fl * a]);
                        }
                    }
                }
                "bell" | "swell" => {
                    // rings on the sphere about the place: the bell's one, crossing the whole Earth;
                    // the swell's train of crests, slower and close
                    let centre = LonLat::new(d.at[0], d.at[1]).to_xyz();
                    let (period, reach, crests, width) = if d.id == "bell" { (5.0, 0.55, 2, 3.0) } else { (9.0, 0.7, 3, 2.4) };
                    for j in 0..crests {
                        let ph = (tm / period - j as f64 * if d.id == "bell" { 0.5 } else { 0.07 }).rem_euclid(1.0);
                        let theta = 0.02 + reach * ph;
                        let al = ((1.0 - ph).powf(1.2) * smooth(0.0, 0.05, ph)) as f32 * a;
                        sphere_ring(p, globe, centre, theta, k, [ox, oy], width * kf, [tint[0] * 0.8, tint[1] * 0.8, tint[2] * 0.8, al]);
                    }
                    p.disc(c, 2.2 * kf, 1.0, [tint[0], tint[1], tint[2], 0.9 * a]);
                }
                "meander" => {
                    // a window onto the floodplain: the forest's dark green, the river's tan line wandering
                    // through it (its bends slowly growing and moving on), and a cut-off lake's dark crescent
                    let w = a * halo;
                    p.disc(c, (r * 1.05 * k) as f32, (r * 0.2 * k) as f32, [0.10, 0.25, 0.17, 0.95 * w]);
                    let lake = [c[0] - (r * 0.38 * k) as f32, c[1] - (r * 0.1 * k) as f32];
                    p.ring(lake, (r * 0.28 * k) as f32, (r * 0.09 * k) as f32, 1.0, [0.07, 0.12, 0.22, 0.8 * a]);
                    let n = 28;
                    let mut prev: Option<[f32; 2]> = None;
                    for j in 0..=n {
                        let u = j as f64 / n as f64 * 2.0 - 1.0;
                        let amp = 0.42 + 0.12 * (tm * 0.13).sin();
                        let x = amp * (u * 4.4 + tm * 0.25).sin() * (1.0 - u * u).sqrt();
                        let q = [c[0] + (x * r * k) as f32, c[1] + (u * r * 0.92 * k) as f32];
                        if let Some(pp) = prev {
                            p.line(pp, q, (0.075 * r * k) as f32, 1.0, [0.80, 0.66, 0.44, 0.95 * a]);
                        }
                        prev = Some(q);
                    }
                }
                "jam" => {
                    // a window onto the night ring: 22 cars round it, and the jam a touch starts, a knot of
                    // brake lights running backward round the ring as the cars go forward
                    let w = a * halo;
                    p.disc(c, (r * 1.05 * k) as f32, (r * 0.2 * k) as f32, [0.07, 0.08, 0.11, 0.95 * w]);
                    p.ring(c, (r * 0.68 * k) as f32, (0.16 * r * k) as f32, 1.0, [0.22, 0.23, 0.27, 0.9 * a]);
                    let jam_at = -tm * 0.35;
                    for j in 0..22 {
                        let base = j as f64 / 22.0 * std::f64::consts::TAU;
                        let d = (base - jam_at).rem_euclid(std::f64::consts::TAU);
                        let near = (-(d.min(std::f64::consts::TAU - d) / 0.55).powi(2)).exp();
                        // the cars bunch at the knot and spread after it
                        let ang = base + tm * 0.5 - 0.25 * near * (d - std::f64::consts::PI).signum();
                        let q = [c[0] + (ang.cos() * r * 0.68 * k) as f32, c[1] + (ang.sin() * r * 0.68 * k) as f32];
                        let red = near as f32;
                        p.disc(q, (0.08 * r * k) as f32 + 0.6 * kf, 1.0, [0.95, 0.9 - 0.75 * red, 0.75 - 0.65 * red, 0.95 * a]);
                        if red > 0.2 {
                            p.glow(q, q, (0.12 * r * k) as f32, (0.2 * r * k) as f32, [1.0, 0.2, 0.1, 0.5 * red * a]);
                        }
                    }
                }
                "frost" => {
                    // a window of dark glass at -50°: frost growing on it in six-armed feathers, then fading
                    // as if a finger had warmed it, and growing back
                    let w = a * halo;
                    p.disc(c, (r * 1.05 * k) as f32, (r * 0.2 * k) as f32, [0.06, 0.10, 0.20, 0.95 * w]);
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
                                p.line(o, [o[0] + (ba.cos() * bl * k) as f32, o[1] + (ba.sin() * bl * k) as f32], 0.6 * kf, 0.8, ice);
                            }
                        }
                    }
                }
                _ => {
                    // a toy without a likeness of its own yet: a light of its colour, breathing
                    let ph = (tm / 3.0 + i as f64 * 0.37).rem_euclid(1.0);
                    p.ring(c, ((r * (0.3 + 1.2 * ph)) * k) as f32, 1.6 * kf, 1.2, [tint[0], tint[1], tint[2], ((1.0 - ph) * 0.85) as f32 * a]);
                    p.disc(c, 3.0 * kf, 1.0, [tint[0], tint[1], tint[2], 0.95 * a]);
                }
            }
            // touched: it stirs (a ring of its colour going out)
            let st = t - self.stir_at[i];
            if (0.0..1.0).contains(&st) {
                let u = st;
                p.ring(c, ((r * 0.6 + r * 2.6 * u) * k) as f32, 2.0 * kf, 1.5, [tint[0], tint[1], tint[2], ((1.0 - u).powf(1.4) * 0.9) as f32 * a]);
            }
            // from afar, now and then, a glint: the eye is drawn before it knows to what
            let gt = t - self.glint_at[i];
            if (0.0..1.3).contains(&gt) && globe.camera.zoom < 2.2 && !reduce_motion {
                let g = (std::f64::consts::PI * gt / 1.3).sin().powi(2) as f32 * a;
                let long = (r * 1.25 * k) as f32;
                let short = long * 0.45;
                let w = [1.0, 0.97, 0.88, 0.95 * g];
                p.glow([c[0] - long, c[1]], [c[0] + long, c[1]], 0.8 * kf, 1.6 * kf, w);
                p.glow([c[0], c[1] - long], [c[0], c[1] + long], 0.8 * kf, 1.6 * kf, w);
                let dg = short * std::f32::consts::FRAC_1_SQRT_2;
                p.glow([c[0] - dg, c[1] - dg], [c[0] + dg, c[1] + dg], 0.6 * kf, 1.2 * kf, w);
                p.glow([c[0] - dg, c[1] + dg], [c[0] + dg, c[1] - dg], 0.6 * kf, 1.2 * kf, w);
                p.glow(c, c, 3.0 * kf, 4.0 * kf, [1.0, 0.95, 0.8, 0.8 * g]);
            }
        }
    }
}

/// A circle on the sphere: the places `theta` radians from `centre`, drawn where they face the reader.
fn sphere_ring(p: &mut Painter, globe: &Globe, centre: [f64; 3], theta: f64, k: f64, off: [f32; 2], width: f32, col: [f32; 4]) {
    let c = centre;
    // two directions square to the centre
    let a = if c[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let cross = |x: [f64; 3], y: [f64; 3]| [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
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
        let w = [c[0] * ct + (u[0] * cp + v[0] * sp) * st, c[1] * ct + (u[1] * cp + v[1] * sp) * st, c[2] * ct + (u[2] * cp + v[2] * sp) * st];
        let (s, f) = globe.project(LonLat::from_xyz(w));
        let q = ([(s[0] * k) as f32 + off[0], (s[1] * k) as f32 + off[1]], f);
        if let Some(pq) = prev {
            if pq.1 > 0.02 && q.1 > 0.02 {
                let fa = smooth(0.02, 0.25, pq.1.min(q.1)) as f32;
                p.line(pq.0, q.0, width * 0.5, 1.0, [col[0], col[1], col[2], col[3] * fa]);
            }
        }
        prev = Some(q);
    }
}
