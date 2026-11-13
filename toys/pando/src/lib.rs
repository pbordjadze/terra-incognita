//! Pando (a working title): the quaking aspens of Pando, on the Fishlake Plateau in Utah, in late
//! September gold. It looks like a forest; it is one tree, some 47,000 stems from one root. Four
//! scales, one wind: the grove, where a gust runs over the crowns as a pale wave (grove.rs); one stem,
//! its leaves twisting on their flat stalks (stem.rs); one leaf (leaf.rs); and its skin under the
//! lens, where the pores open in light and close in a finger's shade (skin.rs). A drag is wind at
//! every scale; a tap goes one scale in, and from the skin a tap rises back out to the grove.

pub mod grove;
pub mod leaf;
pub mod skin;
pub mod sound;
pub mod stem;
pub mod wind;

use grove::{Grove, View};
use leaf::OneLeaf;
use skin::Skin;
use stem::Stem;
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::info::{Demo, Rules, Stroke};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Synth, Toy};
use wind::Wind;

/// Where the toy is: Pando, beside Utah State Route 25 near Fish Lake (lon, lat).
pub const PLACE: (f64, f64) = (-111.75, 38.525);

/// A dive's zoom: how far the scale left behind grows (or the one arrived at starts small).
const ZOOM: f32 = 9.0;
/// How small the scale arrived at starts (and how far the skin shrinks on the way out).
const ARRIVE: f32 = 2.4;
const DIVE_S: f64 = 1.8;
const RISE_S: f64 = 2.6;
/// A tap: quicker than this, and moving less than this many points.
const TAP_S: f64 = 0.45;
const TAP_PT: f64 = 14.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scale {
    Grove,
    Stem,
    Leaf,
    Skin,
}

/// A 2D camera over a scene drawn in its own pixels: the scene's point `focus` is put at the screen's
/// middle, `zoom` times larger.
#[derive(Clone, Copy)]
pub struct Cam {
    pub focus: [f32; 2],
    pub zoom: f32,
    pub mid: [f32; 2],
}

impl Cam {
    pub fn still(size: [f32; 2]) -> Cam {
        let mid = [size[0] * 0.5, size[1] * 0.5];
        Cam {
            focus: mid,
            zoom: 1.0,
            mid,
        }
    }
    pub fn pt(&self, q: [f32; 2]) -> [f32; 2] {
        [
            self.mid[0] + (q[0] - self.focus[0]) * self.zoom,
            self.mid[1] + (q[1] - self.focus[1]) * self.zoom,
        ]
    }
    pub fn len(&self, r: f32) -> f32 {
        r * self.zoom
    }
}

/// A move between scales: from one to the next (or from the skin all the way out), through the
/// colour of what was touched. `at` is where in the scale left (px) the dive goes; `back` where in
/// the scale arrived at it comes out of, for the rise.
struct Dive {
    from: Scale,
    to: Scale,
    at: [f32; 2],
    back: [f32; 2],
    start: f64,
    colour: [f32; 3],
}

impl Dive {
    fn rising(&self) -> bool {
        self.to == Scale::Grove
    }
    fn length(&self) -> f64 {
        if self.rising() {
            RISE_S
        } else {
            DIVE_S
        }
    }
}

struct Finger {
    id: u64,
    down: [f64; 2],
    down_t: f64,
    last: [f64; 2],
    last_t: f64,
    travelled: f64,
}

pub struct Pando {
    backdrop: Backdrop,
    painter: Painter,
    sound: sound::Sound,
    wind: Wind,
    grove: Grove,
    stem: Option<Stem>,
    one: Option<OneLeaf>,
    skin: Option<Skin>,
    scale: Scale,
    tree: usize,
    dive: Option<Dive>,
    finger: Option<Finger>,
    t: f64,
    soft: bool,
    /// The leaf's facing, smoothed, as its skin feels the sun.
    facing: f32,
    /// The screen (px) and its pixels a point, for the plans' anchors.
    px: [f32; 2],
    ppp: f64,
}

fn smooth(x: f64) -> f32 {
    let x = x.clamp(0.0, 1.0) as f32;
    x * x * (3.0 - 2.0 * x)
}

fn lerp2(a: [f32; 2], b: [f32; 2], k: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k]
}

impl Pando {
    fn size(ctx: &Ctx) -> [f32; 2] {
        [ctx.width as f32, ctx.height as f32]
    }

    fn aspect(ctx: &Ctx) -> f32 {
        ctx.height as f32 / ctx.width as f32
    }

    /// The wind at the tree we went into (m/s).
    fn tree_wind(&self) -> f32 {
        let t = &self.grove.trees[self.tree];
        self.wind.at([t.x, t.z])
    }

    /// Where the tree we went into stands in the grove's picture (px).
    fn tree_px(&self, ctx: &Ctx) -> [f32; 2] {
        let s = Self::size(ctx);
        let v = View::of(s[0], s[1]);
        let t = &self.grove.trees[self.tree];
        v.project([t.x + t.d, t.h * 0.72, t.z])
            .unwrap_or([s[0] * 0.5, s[1] * 0.5])
    }

    fn begin(&mut self, from: Scale, to: Scale, at: [f32; 2], back: [f32; 2], colour: [f32; 3]) {
        self.dive = Some(Dive {
            from,
            to,
            at,
            back,
            start: self.t,
            colour,
        });
    }

    fn tap(&mut self, ctx: &mut Ctx, q: [f32; 2]) {
        let s = Self::size(ctx);
        let w = s[0];
        let aspect = Self::aspect(ctx);
        match self.scale {
            Scale::Grove => {
                let v = View::of(s[0], s[1]);
                if let Some(i) = self.grove.hit(&v, q) {
                    self.tree = i;
                    let t = &self.grove.trees[i];
                    self.stem = Some(Stem::new(t.seed, aspect));
                    let at = self.tree_px(ctx);
                    self.begin(Scale::Grove, Scale::Stem, at, at, [0.93, 0.66, 0.20]);
                    ctx.haptic();
                }
            }
            Scale::Stem => {
                let Some(stem) = self.stem.as_ref() else {
                    return;
                };
                if let Some(i) = stem.hit([q[0] / w, q[1] / w], aspect) {
                    let l = &stem.leaves[i];
                    let (_, m) = stem.blade(l, aspect);
                    let colour = grove::tone(l.seed);
                    self.one = Some(OneLeaf::new(l.seed, l.freq));
                    self.begin(Scale::Stem, Scale::Leaf, [m[0] * w, m[1] * w], q, colour);
                    ctx.haptic();
                }
            }
            Scale::Leaf => {
                let seed = self.one.as_ref().map_or(1, |o| o.leaf.seed);
                self.skin = Some(Skin::new(seed, aspect));
                self.begin(Scale::Leaf, Scale::Skin, q, q, [0.95, 0.72, 0.28]);
                ctx.haptic();
            }
            Scale::Skin => {
                let back = self.tree_px(ctx);
                self.begin(
                    Scale::Skin,
                    Scale::Grove,
                    [s[0] * 0.5, s[1] * 0.5],
                    back,
                    [0.96, 0.78, 0.52],
                );
                ctx.haptic();
            }
        }
    }

    /// A drag's move: wind at the scale we're at, from `a` to `b` (points) over `dt` seconds.
    fn blow(&mut self, ctx: &Ctx, a: [f64; 2], b: [f64; 2], dt: f64) {
        let k = ctx.ppp as f32;
        let s = Self::size(ctx);
        let (pa, pb) = (
            [a[0] as f32 * k, a[1] as f32 * k],
            [b[0] as f32 * k, b[1] as f32 * k],
        );
        let dt = dt.max(1.0 / 120.0) as f32;
        match self.scale {
            Scale::Grove => {
                let v = View::of(s[0], s[1]);
                // a drag high on the picture is over the far crowns: hold it below the horizon
                let floor = v.horizon + 0.04 * s[1];
                let (pa, pb) = ([pa[0], pa[1].max(floor)], [pb[0], pb[1].max(floor)]);
                if let (Some(ga), Some(gb)) = (v.ground(pa), v.ground(pb)) {
                    let vel = [(gb[0] - ga[0]) / dt, (gb[1] - ga[1]) / dt];
                    // a finger's sweep is a few metres wide close by, tens of metres far off
                    let size = 4.0 + gb[1] * 0.08;
                    let cap = 4.0 + gb[1] * 0.08;
                    let sp = (vel[0] * vel[0] + vel[1] * vel[1]).sqrt();
                    let vel = if sp > cap {
                        [vel[0] / sp * cap, vel[1] / sp * cap]
                    } else {
                        vel
                    };
                    self.wind.push(gb, vel, size);
                }
            }
            Scale::Stem => {
                if let Some(stem) = self.stem.as_mut() {
                    let w = s[0];
                    stem.breathe(
                        [pb[0] / w, pb[1] / w],
                        [(pb[0] - pa[0]) / w / dt, (pb[1] - pa[1]) / w / dt],
                    );
                }
            }
            Scale::Leaf => {
                if let Some(one) = self.one.as_mut() {
                    let d = ((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2)).sqrt();
                    one.breathe(d / s[0] / dt);
                }
            }
            Scale::Skin => {
                if let Some(skin) = self.skin.as_mut() {
                    skin.shade_at = [pb[0] / s[0], pb[1] / s[0]];
                }
            }
        }
    }

    /// What to draw now: one scale, its camera, and the colour over it (with its strength).
    fn shown(&self, ctx: &Ctx) -> (Scale, Cam, [f32; 3], f32) {
        let size = Self::size(ctx);
        let still = Cam::still(size);
        let Some(d) = &self.dive else {
            return (self.scale, still, [0.0; 3], 0.0);
        };
        let u = (self.t - d.start) / d.length();
        let (zoom, arrive) = if self.soft {
            (1.0, 1.0)
        } else {
            (ZOOM, ARRIVE)
        };
        let veil = if u < 0.5 {
            smooth((u - 0.22) / 0.28)
        } else {
            1.0 - smooth((u - 0.5) / 0.32)
        };
        if u < 0.5 {
            let e = (u / 0.5) as f32;
            let e = e * e;
            let cam = if d.rising() {
                Cam {
                    zoom: arrive.powf(-e),
                    ..still
                }
            } else {
                Cam {
                    focus: lerp2(still.focus, d.at, smooth(u / 0.35)),
                    zoom: zoom.powf(e),
                    ..still
                }
            };
            (d.from, cam, d.colour, veil)
        } else {
            let e = ((u - 0.5) / 0.5).min(1.0) as f32;
            let e = 1.0 - (1.0 - e) * (1.0 - e);
            let cam = if d.rising() {
                Cam {
                    focus: lerp2(d.back, still.focus, e),
                    zoom: zoom.powf(1.0 - e),
                    ..still
                }
            } else {
                Cam {
                    zoom: arrive.powf(-(1.0 - e)),
                    ..still
                }
            };
            (d.to, cam, d.colour, veil)
        }
    }
}

impl Toy for Pando {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let seed = std::env::var("PANDO_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0x0009_a4d0_u32);
        Ok(Pando {
            backdrop: Backdrop::new(&ctx.gpu, "pando", include_str!("scene.wgsl")),
            painter: Painter::new(&ctx.gpu),
            sound: sound::Sound::default(),
            wind: Wind::new(seed),
            grove: Grove::new(seed ^ 0x51ce),
            stem: None,
            one: None,
            skin: None,
            scale: Scale::Grove,
            tree: 0,
            dive: None,
            finger: None,
            t: 0.0,
            soft: ctx.reduce_motion,
            facing: 1.0,
            px: [ctx.width as f32, ctx.height as f32],
            ppp: ctx.ppp,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                if self.finger.is_some() {
                    return;
                }
                self.finger = Some(Finger {
                    id: e.id,
                    down: [e.x, e.y],
                    down_t: e.t,
                    last: [e.x, e.y],
                    last_t: e.t,
                    travelled: 0.0,
                });
                if self.dive.is_none() && self.scale == Scale::Skin {
                    let w = ctx.width as f32;
                    let k = ctx.ppp as f32;
                    if let Some(skin) = self.skin.as_mut() {
                        skin.held = true;
                        skin.shade_at = [e.x as f32 * k / w, e.y as f32 * k / w];
                    }
                }
            }
            Phase::Move => {
                let Some(f) = self.finger.as_mut().filter(|f| f.id == e.id) else {
                    return;
                };
                let (a, dt) = (f.last, e.t - f.last_t);
                f.travelled += ((e.x - a[0]).powi(2) + (e.y - a[1]).powi(2)).sqrt();
                f.last = [e.x, e.y];
                f.last_t = e.t;
                if self.dive.is_none() {
                    self.blow(ctx, a, [e.x, e.y], dt);
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some(f) = self.finger.take().filter(|f| f.id == e.id) else {
                    return;
                };
                if let Some(skin) = self.skin.as_mut() {
                    skin.held = false;
                }
                let quick = e.t - f.down_t < TAP_S && f.travelled < TAP_PT;
                if e.phase == Phase::Up && quick && self.dive.is_none() {
                    let k = ctx.ppp as f32;
                    self.tap(ctx, [f.down[0] as f32 * k, f.down[1] as f32 * k]);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt.min(0.1);
        self.t += dt;
        self.px = Self::size(ctx);
        self.ppp = ctx.ppp;
        let dtf = dt as f32;
        self.wind.step(dtf);
        self.grove.step(&self.wind, dtf);
        let w = self.tree_wind();
        let t = self.t as f32;
        if let Some(stem) = self.stem.as_mut() {
            stem.step(w, t, dtf);
        }
        if let Some(one) = self.one.as_mut() {
            one.step(w, t, dtf);
            self.facing += (one.facing() - self.facing) * (1.0 - (-dtf / 0.6).exp());
        }
        if let Some(skin) = self.skin.as_mut() {
            skin.step(self.facing, dtf);
        }
        if let Some(d) = &self.dive {
            let u = (self.t - d.start) / d.length();
            if u >= 0.5 {
                self.scale = d.to;
            }
            if u >= 1.0 {
                self.dive = None;
                // what's left behind on the way out is let go
                if self.scale == Scale::Grove {
                    self.stem = None;
                    self.one = None;
                    self.skin = None;
                }
            }
        }
        let near = match self.shown(ctx).0 {
            Scale::Grove => 0.0,
            Scale::Stem => 1.0,
            Scale::Leaf => 2.0,
            Scale::Skin => 3.0,
        };
        let flutter = self.one.as_ref().map_or(0.0, |o| o.leaf.twist_v);
        let wind = if near > 0.5 {
            w
        } else {
            let s = Self::size(ctx);
            let v = View::of(s[0], s[1]);
            v.ground([s[0] * 0.5, s[1] * 0.75])
                .map_or(w, |g| self.wind.at(g))
        };
        self.sound.set(sound::Mix {
            wind,
            near,
            flutter,
        });
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let size = Self::size(ctx);
        let ppp = ctx.ppp as f32;
        let (scale, cam, veil_c, veil) = self.shown(ctx);
        let w = size[0];
        let aspect = Self::aspect(ctx);
        let v = View::of(size[0], size[1]);
        let mut p = [0.0f32; 8];
        p[4] = ppp;
        match scale {
            Scale::Grove => {
                p[1] = v.horizon;
                self.grove
                    .draw(&mut self.painter, &v, &cam, &self.wind, ppp, 1.0);
            }
            Scale::Stem => {
                p[0] = 1.0;
                if let Some(stem) = &self.stem {
                    stem.draw(&mut self.painter, &cam, w, aspect, ppp, 1.0);
                }
            }
            Scale::Leaf => {
                p[0] = 2.0;
                if let Some(one) = &self.one {
                    let (base, ang) = one.base();
                    p[2] = one.leaf.twist;
                    p[3] = ang - std::f32::consts::FRAC_PI_2;
                    p[5] = base[0] * w;
                    p[6] = base[1] * w;
                    p[7] = one.leaf.blade * w;
                    one.draw(&mut self.painter, &cam, w, 1.0);
                }
            }
            Scale::Skin => {
                p[0] = 3.0;
                if let Some(skin) = &self.skin {
                    p[5] = skin.shade_at[0] * w;
                    p[6] = skin.shade_at[1] * w;
                    p[7] = skin.shade;
                    skin.draw(&mut self.painter, &cam, w, 1.0);
                }
            }
        }
        if veil > 0.0 {
            let r = size[0].hypot(size[1]);
            self.painter.disc(
                [size[0] * 0.5, size[1] * 0.5],
                r,
                1.0,
                [veil_c[0], veil_c[1], veil_c[2], veil],
            );
        }
        let params = BackdropParams {
            time: self.t as f32,
            scale: cam.zoom,
            center: cam.focus,
            p,
        };
        let sz = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.backdrop
            .render(&ctx.gpu, &mut enc, target, sz, &params);
        self.painter.render(&ctx.gpu, &mut enc, target, sz, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.soft = ctx.reduce_motion;
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let mut out = Vec::new();
        let (k, w, h) = (self.ppp, self.px[0], self.px[1]);
        if k <= 0.0 {
            return out;
        }
        let pt = |q: [f32; 2]| [q[0] as f64 / k, q[1] as f64 / k];
        let v = View::of(w, h);
        // a near tree a little below the middle, for the plans to go into
        if let Some(i) = self.grove.hit(&v, [w * 0.5, h * 0.62]) {
            let t = &self.grove.trees[i];
            if let Some(c) = v.project([t.x + t.d, t.h * 0.72, t.z]) {
                out.push(("tree".to_string(), pt(c)));
            }
        }
        if let Some(stem) = &self.stem {
            let aspect = h / w;
            // the leaf nearest the picture's middle
            let mid = [0.6, aspect * 0.45];
            if let Some(l) = stem.leaves.iter().min_by(|a, b| {
                let da = stem.blade(a, aspect).1;
                let db = stem.blade(b, aspect).1;
                let d = |m: [f32; 2]| (m[0] - mid[0]).powi(2) + (m[1] - mid[1]).powi(2);
                d(da).total_cmp(&d(db))
            }) {
                let m = stem.blade(l, aspect).1;
                out.push(("leaf".to_string(), pt([m[0] * w, m[1] * w])));
            }
        }
        if let Some(one) = &self.one {
            let (base, ang) = one.base();
            let m = [
                base[0] + ang.cos() * one.leaf.blade * 0.5,
                base[1] + ang.sin() * one.leaf.blade * 0.5,
            ];
            out.push(("blade".to_string(), pt([m[0] * w, m[1] * w])));
        }
        out
    }

    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        let [w, h] = ctx.points();
        let line = match self.scale {
            Scale::Grove => "Pando's aspens. Drag to blow; tap a tree.",
            Scale::Stem => "One stem. Drag to blow; tap a leaf.",
            Scale::Leaf => "One leaf. Drag to blow; tap to look closer.",
            Scale::Skin => "Its pores. Hold a finger over them; tap to rise.",
        };
        let stroke = if self.scale == Scale::Skin {
            Stroke::hold(0.5, 4.0, [0.5 * w, 0.55 * h])
        } else {
            Stroke::drag(0.4, 1.0, [0.15 * w, 0.62 * h], [0.85 * w, 0.52 * h])
        };
        Some(Demo::new(line, vec![stroke]))
    }

    fn rules(&self) -> Option<Rules> {
        Some(Rules::lines(&[
            "Pando, in Utah, looks like a forest but is one aspen: over 40,000 stems from one root.",
            "Each leaf hangs on a stalk flattened side to side, so the faintest air flips it, and a gust runs over the grove as a pale wave.",
            "Drag to blow. Tap a tree, then a leaf, then closer: the pores on its underside open in light and close in your shade.",
        ]))
    }
}

toy_kit::ios_shell!(Pando);
