//! The lights on Terra's Earth: one for each toy it holds, at the place where the toy is real. Every light is
//! the same plain light of its toy's colour (painting-company row 355: a new toy needs only its place, its
//! colour and its postcard's words, `places.rs`). Lights that would touch from afar gather into one, which
//! shows how many it holds as small dots of their colours round it; pinched closer, or touched, they part.
//! From afar a light glints now and then; touched, it stirs.

use std::sync::Arc;
use toy_kit::paint::Painter;
use toy_kit::{Assets, Ctx, Globe, LonLat};

use crate::rooms::Room;

mod fresh;
mod places;
pub use places::doors;

pub type Opener = fn(&Ctx, &Arc<dyn Assets>) -> anyhow::Result<Box<dyn Room>>;

pub struct Door {
    pub id: &'static str,
    /// Its toy's name (its postcard's heading; VoiceOver says it).
    pub name: &'static str,
    /// Where it is, in words (its postcard's second line).
    pub place: &'static str,
    /// What it is, in one short line (its postcard's third).
    pub line: &'static str,
    /// Where the light is: where the toy is real.
    pub at: [f64; 2],
    pub tint: [f32; 3],
    /// Its note when touched (a pentatonic degree).
    pub degree: i32,
    pub open: Opener,
}

/// Lights that would come closer than this many of their sizes gather into one.
const GATHER: f64 = 2.6;

/// A light as shown: one toy's, or several gathered (`doors` holds them, nearest the middle first).
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub doors: Vec<usize>,
    /// Where it is on screen (points) and how far it faces the reader.
    pub at: [f64; 2],
    pub facing: f64,
}

/// What a touch found.
pub enum Hit {
    Door(usize),
    /// Several gathered: the place to come down toward (their middle).
    Many(LonLat),
}

/// The lights' life: their glints, a touch's stir, a bare tap's ring.
pub struct Life {
    pub doors: Vec<Door>,
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

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Life {
    pub fn new() -> Life {
        let doors = doors();
        let n = doors.len();
        Life {
            doors,
            glint_at: (0..n).map(|i| 1.5 + i as f64 * 1.37).collect(),
            stir_at: vec![-100.0; n],
            rings: Vec::new(),
            beckon: None,
            new: vec![false; n],
            away: Vec::new(),
        }
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

    /// Where light `i` is on screen (points) and how far it faces the reader.
    pub fn spot(&self, globe: &Globe, i: usize) -> ([f64; 2], f64) {
        let d = &self.doors[i];
        globe.project(LonLat::new(d.at[0], d.at[1]))
    }

    /// The light's size, points: grows a little as the reader comes closer, and with the screen (the reading
    /// scale twice: a big screen's Earth is held farther off and is larger, and its lights must read from
    /// across it as a phone's do; painting-company row 246).
    pub fn size(globe: &Globe) -> f64 {
        11.0 * globe.reading().powi(2) * globe.camera.zoom.clamp(1.0, 5.0).sqrt()
    }

    /// The lights as shown now, those facing the reader: each toy's alone, unless lights would touch, when
    /// they gather into one at their middle (`GATHER`), the gathering taken greedily from the light nearest the
    /// middle of the screen outward so the one the reader looks at keeps its place.
    pub fn shown(&self, globe: &Globe) -> Vec<Shown> {
        let view = globe.view();
        let mid = [
            view.width as f64 / view.ppp / 2.0,
            view.height as f64 / view.ppp / 2.0,
        ];
        let mut lit: Vec<(usize, [f64; 2], f64)> = (0..self.doors.len())
            .map(|i| {
                let (p, f) = self.spot(globe, i);
                (i, p, f)
            })
            .filter(|&(_, _, f)| f > 0.3)
            .collect();
        let dist = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        lit.sort_by(|a, b| dist(a.1, mid).total_cmp(&dist(b.1, mid)));
        let room = Self::size(globe) * GATHER;
        let mut out: Vec<Shown> = Vec::new();
        for (i, p, f) in lit {
            match out.iter_mut().find(|s| dist(s.at, p) < room) {
                Some(s) => {
                    let n = s.doors.len() as f64;
                    s.at = [
                        (s.at[0] * n + p[0]) / (n + 1.0),
                        (s.at[1] * n + p[1]) / (n + 1.0),
                    ];
                    s.facing = s.facing.min(f);
                    s.doors.push(i);
                }
                None => out.push(Shown {
                    doors: vec![i],
                    at: p,
                    facing: f,
                }),
            }
        }
        out
    }

    /// What is under a point: the nearest light within reach (its own size, at least a fingertip's 30 points
    /// at the reading scale), a toy's or a gathering's.
    pub fn hit(&self, globe: &Globe, p: [f64; 2]) -> Option<Hit> {
        let reach = (Self::size(globe) * 1.8).max(30.0 * globe.reading());
        let s = self
            .shown(globe)
            .into_iter()
            .filter(|s| s.facing > 0.35)
            .map(|s| {
                let d = (s.at[0] - p[0]).hypot(s.at[1] - p[1]);
                (s, d)
            })
            .filter(|(s, d)| *d < reach * if s.doors.len() > 1 { 1.3 } else { 1.0 })
            .min_by(|a, b| a.1.total_cmp(&b.1))?
            .0;
        if s.doors.len() == 1 {
            return Some(Hit::Door(s.doors[0]));
        }
        globe.unproject(s.at[0], s.at[1]).map(Hit::Many)
    }

    /// The toy's light nearest a point among those shown alone and facing the reader.
    pub fn nearest(&self, globe: &Globe, p: [f64; 2]) -> Option<usize> {
        self.shown(globe)
            .into_iter()
            .filter(|s| s.facing > 0.35 && s.doors.len() == 1)
            .map(|s| (s.doors[0], (s.at[0] - p[0]).hypot(s.at[1] - p[1])))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// Every light into the painter (pixels: `k` pixels a point). `fade` dims them all; `chosen` is the light
    /// whose postcard is up (a ring round it).
    #[allow(clippy::too_many_arguments)] // a drawing call: each argument is one thing it draws with
    pub fn paint(
        &self,
        p: &mut Painter,
        globe: &Globe,
        t: f64,
        k: f64,
        fade: f64,
        reduce_motion: bool,
        chosen: Option<usize>,
    ) {
        let kf = k as f32;
        let r = Self::size(globe);
        let px = |pt: [f64; 2]| [(pt[0] * k) as f32, (pt[1] * k) as f32];
        // bare taps: a soft gold ring where the finger was
        for (ll, t0) in &self.rings {
            let (s, f) = globe.project(*ll);
            if f > 0.0 {
                let u = ((t - t0) / 1.6).clamp(0.0, 1.0);
                let a = (1.0 - u).powf(1.5) * smooth(0.0, 0.2, f) * 0.8;
                p.ring(
                    px(s),
                    ((6.0 + 34.0 * u) * k) as f32,
                    1.4 * kf,
                    1.0,
                    [0.69, 0.54, 0.24, a as f32],
                );
            }
        }
        let view = globe.view();
        let [sw, sh] = [view.width as f64 / view.ppp, view.height as f64 / view.ppp];
        for s in self.shown(globe) {
            let edge = s.at[0].min(sw - s.at[0]).min(s.at[1]).min(sh - s.at[1]);
            let vis = smooth(0.3, 0.5, s.facing) * smooth(r * 0.9, r * 1.6, edge) * fade;
            if vis <= 0.01 {
                continue;
            }
            let c = px(s.at);
            let a = vis as f32;
            if s.doors.len() > 1 {
                self.paint_many(p, &s.doors, c, r, k, a);
                continue;
            }
            let i = s.doors[0];
            let tint = self.doors[i].tint;
            // the beckoning light (until the first touch): a slow ring breathing out of it
            if Some(i) == self.beckon && !reduce_motion {
                let ph = (t / 2.6).rem_euclid(1.0);
                let al = ((1.0 - ph).powf(1.5) * 0.7) as f32 * a;
                p.ring(
                    c,
                    ((r * (1.1 + 1.6 * ph)) * k) as f32,
                    1.6 * kf,
                    1.5,
                    [tint[0], tint[1], tint[2], al],
                );
            }
            Self::paint_light(p, c, r, k, tint, a, globe.night);
            if chosen == Some(i) {
                p.ring(
                    c,
                    (r * 1.75 * k) as f32,
                    1.6 * kf,
                    1.0,
                    [1.0, 1.0, 1.0, 0.95 * a],
                );
            }
            self.paint_new(p, i, c, r, k, t, a, reduce_motion);
            // touched: it stirs (a ring of its colour going out)
            let u = t - self.stir_at[i];
            if (0.0..1.0).contains(&u) {
                let al = ((1.0 - u).powf(1.4) * 0.9) as f32 * a;
                p.ring(
                    c,
                    ((r * 0.6 + r * 2.6 * u) * k) as f32,
                    2.0 * kf,
                    1.5,
                    [tint[0], tint[1], tint[2], al],
                );
            }
            let gt = t - self.glint_at[i];
            if (0.0..1.3).contains(&gt) && globe.camera.zoom < 2.2 && !reduce_motion {
                Self::paint_glint(
                    p,
                    c,
                    r,
                    k,
                    (std::f64::consts::PI * gt / 1.3).sin().powi(2) as f32 * a,
                );
            }
        }
    }

    /// One plain light: a soft pale halo so it reads from across the Earth, its colour, a white rim.
    fn paint_light(
        p: &mut Painter,
        c: [f32; 2],
        r: f64,
        k: f64,
        tint: [f32; 3],
        a: f32,
        night: f32,
    ) {
        let kf = k as f32;
        p.disc(
            c,
            (r * 1.2 * k) as f32,
            (r * 1.1 * k) as f32,
            [1.0, 0.99, 0.95, 0.5 * a * (1.0 - night)],
        );
        p.disc(
            c,
            (r * 1.0 * k) as f32,
            (r * 0.9 * k) as f32,
            [tint[0], tint[1], tint[2], 0.35 * a],
        );
        p.disc(
            c,
            (r * 0.55 * k) as f32,
            1.0,
            [tint[0], tint[1], tint[2], a],
        );
        p.ring(
            c,
            (r * 0.55 * k) as f32,
            1.3 * kf,
            0.8,
            [1.0, 1.0, 1.0, 0.9 * a],
        );
    }

    /// A gathering: a pale light a little larger than one, with a small dot of each toy's colour round it.
    fn paint_many(&self, p: &mut Painter, doors: &[usize], c: [f32; 2], r: f64, k: f64, a: f32) {
        let kf = k as f32;
        // the core in its members' colours mixed, so it reads as their light, not an empty pin
        let n = doors.len().max(1) as f32;
        let mut m = [0.0f32; 3];
        for &i in doors {
            for (c, t) in m.iter_mut().zip(self.doors[i].tint) {
                *c += t / n;
            }
        }
        p.disc(
            c,
            (r * 1.5 * k) as f32,
            (r * 1.2 * k) as f32,
            [m[0], m[1], m[2], 0.4 * a],
        );
        p.disc(c, (r * 0.7 * k) as f32, 1.0, [m[0], m[1], m[2], a]);
        p.ring(
            c,
            (r * 0.7 * k) as f32,
            1.2 * kf,
            0.8,
            [1.0, 1.0, 1.0, 0.9 * a],
        );
        let n = doors.len();
        for (j, &i) in doors.iter().enumerate() {
            // (in an arc over the top, a crown of its toys, closing to a ring as they grow many)
            let step = (std::f64::consts::TAU / n as f64).min(0.9);
            let ang = -std::f64::consts::FRAC_PI_2 + step * (j as f64 - (n - 1) as f64 / 2.0);
            let q = [
                c[0] + (ang.cos() * r * 1.15 * k) as f32,
                c[1] + (ang.sin() * r * 1.15 * k) as f32,
            ];
            let t = self.doors[i].tint;
            p.disc(q, (r * 0.32 * k) as f32, 1.0, [t[0], t[1], t[2], a]);
            p.ring(
                q,
                (r * 0.32 * k) as f32,
                0.9 * kf,
                0.8,
                [1.0, 1.0, 1.0, 0.85 * a],
            );
        }
    }

    /// From afar, now and then, a glint: a soft swell of light round it, so the eye is drawn before it knows
    /// to what (no cross: still, a cross read as a sight).
    fn paint_glint(p: &mut Painter, c: [f32; 2], r: f64, k: f64, g: f32) {
        let kf = k as f32;
        p.disc(
            c,
            (r * 2.2 * k) as f32,
            (r * 1.8 * k) as f32,
            [1.0, 0.97, 0.88, 0.45 * g],
        );
        p.glow(c, c, 3.0 * kf, 4.0 * kf, [1.0, 0.95, 0.8, 0.7 * g]);
    }
}
