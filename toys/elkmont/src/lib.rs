//! Elkmont (a working title; painting-company row 217's family, made local): the synchronous
//! fireflies, Photinus carolinus, in the woods at Elkmont in the Great Smoky Mountains on a June
//! night, seen from the trail. They come out as the light goes; at first a few flash at random, and as
//! more come out their bursts fall together, until the whole wood flashes as one about every twelve
//! seconds and goes dark between (swarm.rs). The females answer from the leaf litter. A finger is a
//! small light the males read as one of their own: hold it in the dark and the wood's next burst
//! starts from it.

pub mod forest;
pub mod sound;
pub mod swarm;

use forest::{Forest, EYE};
use serde::{Deserialize, Serialize};
use swarm::Swarm;
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};

/// Where the toy is: the Elkmont trailhead by the Little River (lon, lat).
pub const PLACE: (f64, f64) = (-83.582, 35.656);

/// The fireflies' light (P. carolinus flashes yellow-green, near 560 nm).
const LIGHT: [f32; 3] = [0.72, 1.0, 0.30];
/// How far along the finger's ray the light is held (m).
const LAMP_M: f32 = 5.5;
/// The light's own flash: about as long and as often as a male's in a burst.
const LAMP_EVERY_S: f64 = swarm::INTERFLASH_S;
/// Its burst: this many flashes, then dark until the finger is lifted and put down again.
const LAMP_BURST: u32 = 6;
/// The finger id a quick tap's light keeps until its one flash.
const TAPPED: u64 = u64::MAX;
/// Depth slabs, far to near: each its trunks and leaves, then its flies, so a nearer trunk hides a fly.
const SLABS: [f32; 6] = [1000.0, 30.0, 16.0, 9.0, 5.0, 0.0];

pub struct Elkmont {
    backdrop: Backdrop,
    painter: Painter,
    swarm: Swarm,
    forest: Forest,
    t: f64,
    /// The finger's light: which finger, where (points), its next flash, the flashes given.
    lamp: Option<(u64, [f64; 2], f64, u32)>,
    lamp_last: f64,
    lamp_at: [f32; 3],
    soft: bool,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    t: f64,
}

/// The view: the focal length (px) and the horizon's height (px from the top).
struct View {
    f: f32,
    cx: f32,
    horizon: f32,
}

impl View {
    fn of(ctx: &Ctx, sway: f32) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        // the woods fill the width; on a wide screen, not so close the flies' band is a sliver
        let f = (0.5 * w / 0.58).max(0.6 * h);
        View {
            f,
            cx: 0.5 * w + sway,
            horizon: 0.42 * h,
        }
    }
    /// A place in the woods on the screen (px), and its depth.
    fn project(&self, p: [f32; 3]) -> Option<([f32; 2], f32)> {
        if p[2] < 0.3 {
            return None;
        }
        Some((
            [
                self.cx + self.f * p[0] / p[2],
                self.horizon - self.f * (p[1] - EYE) / p[2],
            ],
            p[2],
        ))
    }
    /// The place `m` metres along the ray through a screen point (px).
    fn unproject(&self, px: [f32; 2], m: f32) -> [f32; 3] {
        let d = [
            (px[0] - self.cx) / self.f,
            (self.horizon - px[1]) / self.f,
            1.0,
        ];
        let n = (d[0] * d[0] + d[1] * d[1] + 1.0).sqrt();
        [d[0] / n * m, EYE + d[1] / n * m, d[2] / n * m]
    }
}

/// The haze of the far woods (as the backdrop's), for the trunks going back into it.
fn haze(dusk: f32) -> [f32; 3] {
    let a = [0.034, 0.044, 0.068];
    let b = [0.13, 0.17, 0.26];
    [
        a[0] + (b[0] - a[0]) * dusk,
        a[1] + (b[1] - a[1]) * dusk,
        a[2] + (b[2] - a[2]) * dusk,
    ]
}

impl Elkmont {
    /// How much of the day's last light is left (1 at the start, 0 by night).
    fn dusk(&self) -> f32 {
        (1.0 - self.t / (swarm::EMERGE_S * 1.1)).clamp(0.0, 1.0) as f32
    }

    fn sway(&self, ctx: &Ctx) -> f32 {
        if self.soft {
            return 0.0;
        }
        (self.t * 0.07).sin() as f32 * 0.012 * ctx.width as f32
    }

    fn draw_slab(&mut self, v: &View, ppp: f32, near: f32, far: f32, dusk: f32, glow: f32) {
        let sl = Slab {
            v,
            ppp,
            near,
            far,
            hz: haze(dusk),
            glow,
        };
        let p = &mut self.painter;
        draw_trunks(p, &sl, &self.forest);
        draw_ferns(p, &sl, &self.forest);
        draw_leaves(p, &sl, &self.forest);
        draw_flies(p, &sl, &self.swarm, if self.soft { 0.75 } else { 1.0 });
        // the finger's light: a small cold-green LED, and its flash
        // drawn while held, and while a let-go flash fades
        let lit = self.lamp.is_some() || self.swarm.t - self.lamp_last < 0.5;
        if lit && self.lamp_at[2] >= near && self.lamp_at[2] < far {
            if let Some((c, _)) = v.project(self.lamp_at) {
                let g = Swarm::pulse(self.swarm.t - self.lamp_last);
                // its flash lights the leaf litter under it
                if let Some((under, z)) = v.project([self.lamp_at[0], 0.0, self.lamp_at[2]]) {
                    let r = v.f * 0.9 / z;
                    p.glow(under, under, r, r, [0.6, 1.0, 0.4, 0.10 * g]);
                }
                p.glow(
                    c,
                    c,
                    2.6 * ppp,
                    1.6 * ppp,
                    [
                        0.6,
                        1.0,
                        0.6,
                        if self.lamp.is_some() { 0.35 } else { 0.35 * g },
                    ],
                );
                p.glow(c, c, 18.0 * ppp, 22.0 * ppp, [0.6, 1.0, 0.4, 0.22 * g]);
                p.glow(c, c, 5.0 * ppp, 2.0 * ppp, [0.85, 1.0, 0.6, g]);
            }
        }
    }
}

impl Toy for Elkmont {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let seed = std::env::var("ELKMONT_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0x5eed_e1c5);
        Ok(Elkmont {
            backdrop: Backdrop::new(&ctx.gpu, "elkmont", forest::BACKDROP_WGSL),
            painter: Painter::new(&ctx.gpu),
            swarm: Swarm::new(seed),
            forest: Forest::new(seed ^ 0x7_4ee),
            t: 0.0,
            lamp: None,
            lamp_last: -100.0,
            lamp_at: [0.0, EYE, LAMP_M],
            soft: ctx.reduce_motion,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                if self.lamp.is_none_or(|l| l.0 == TAPPED) {
                    // the light flashes at once, then every interflash while held
                    self.lamp = Some((e.id, [e.x, e.y], self.swarm.t, 0));
                    ctx.haptic();
                }
            }
            Phase::Move => {
                if let Some(l) = self.lamp.as_mut() {
                    if l.0 == e.id {
                        l.1 = [e.x, e.y];
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                if let Some(l) = self.lamp.filter(|l| l.0 == e.id) {
                    if (2..LAMP_BURST).contains(&l.3) && e.phase == Phase::Up {
                        self.swarm.lamp_burst_done(self.lamp_at);
                    }
                    self.lamp = None;
                    if l.3 == 0 && e.phase == Phase::Up {
                        // a tap too quick for its flash still gives one, then lets go
                        self.lamp = Some((TAPPED, l.1, l.2, 0));
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt.min(0.1);
        self.t += dt;
        let v = View::of(ctx, self.sway(ctx));
        let k = ctx.ppp as f32;
        if let Some((_, at, next, given)) = self.lamp {
            let px = [at[0] as f32 * k, at[1] as f32 * k];
            let mut m = LAMP_M;
            // held low, the light lies just above the leaf litter, nearer
            let low = v.unproject(px, 1.0);
            if low[1] < EYE {
                m = m.min((EYE - 0.35) / (EYE - low[1]));
            }
            self.lamp_at = v.unproject(px, m);
            if self.swarm.t >= next {
                self.lamp_last = self.swarm.t;
                self.swarm.lamp(self.lamp_at);
                let given = given + 1;
                if given == LAMP_BURST {
                    self.swarm.lamp_burst_done(self.lamp_at);
                }
                if self.lamp.is_some_and(|l| l.0 == TAPPED) {
                    self.lamp = None;
                } else if let Some(l) = self.lamp.as_mut() {
                    // a burst like a male's, then dark while held
                    l.2 = if given >= LAMP_BURST {
                        f64::INFINITY
                    } else {
                        next + LAMP_EVERY_S
                    };
                    l.3 = given;
                }
            }
        }
        // fine steps, so a flash seen is answered on time
        let n = (dt / (1.0 / 60.0)).ceil().max(1.0) as usize;
        for _ in 0..n {
            self.swarm.step(dt / n as f64);
        }
        if std::env::var_os("ELKMONT_CENSUS").is_some() {
            let (b, r, w) = self.swarm.census();
            println!(
                "t {:.2} out {} burst {b} resting {r} ready {w} females {:.2} light {:.1}",
                self.t,
                self.swarm.out,
                (0..self.swarm.females.len())
                    .map(|i| self.swarm.female_glow(i))
                    .sum::<f32>(),
                self.swarm.light
            );
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let v = View::of(ctx, self.sway(ctx));
        let ppp = ctx.ppp as f32;
        let dusk = self.dusk();
        let glow = (self.swarm.light / 120.0).min(1.0) * if self.soft { 0.6 } else { 1.0 };
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let params = BackdropParams {
            time: self.t as f32,
            scale: 1.0,
            center: [0.0, 0.0],
            p: [
                v.horizon,
                v.f,
                dusk,
                glow,
                v.cx - 0.5 * ctx.width as f32,
                ppp,
                0.0,
                0.0,
            ],
        };
        self.backdrop
            .render(&ctx.gpu, &mut enc, target, size, &params);
        for w in SLABS.windows(2) {
            self.draw_slab(&v, ppp, w[1], w[0], dusk, glow);
            self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.soft = ctx.reduce_motion;
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(sound::Night::new()))
    }

    fn demo(&self, ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let [w, h] = ctx.points();
        Some(toy_kit::info::Demo::new(
            "Fireflies in the Smokies. Hold your light low.",
            vec![toy_kit::info::Stroke::hold(0.5, 6.0, [0.42 * w, 0.62 * h])],
        ))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::lines(&[
            "On June nights at Elkmont, in the Great Smoky Mountains, the fireflies flash together in waves.",
            "Each male follows three rules, and no one leads. The three dots show them: switch one off to see what it does.",
            "Hold your light low: a female may answer.",
        ]))
    }

    fn switches(&self) -> Vec<toy_kit::switches::Switch> {
        toy_kit::switches::Switch::all(&swarm::RULES, &self.swarm.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(o) = self.swarm.on.get_mut(i) {
            *o = on;
        }
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved { t: self.t }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            if s.t >= swarm::EMERGE_S {
                self.t = s.t;
                self.swarm.all_out();
            }
        }
    }
}

toy_kit::ios_shell!(Elkmont);

/// One depth slab's view and light, as its painters need them.
struct Slab<'a> {
    v: &'a View,
    ppp: f32,
    near: f32,
    far: f32,
    hz: [f32; 3],
    glow: f32,
}

impl Slab<'_> {
    fn fog(&self, z: f32) -> f32 {
        1.0 - (-z / 22.0).exp()
    }

    /// The dark of a thing at `z`, fading into the haze, lit a little by the swarm.
    fn ink(&self, z: f32, shade: f32) -> [f32; 4] {
        let (k, hz) = (self.fog(z), self.hz);
        let g = self.glow * (1.0 - k);
        [
            (0.010 + shade * 0.006) * (1.0 - k) + hz[0] * k + 0.30 * g * 0.2,
            (0.013 + shade * 0.010) * (1.0 - k) + hz[1] * k + 0.50 * g * 0.2,
            (0.014 + shade * 0.005) * (1.0 - k) + hz[2] * k + 0.10 * g * 0.2,
            1.0,
        ]
    }
}

/// The trunks in the slab.
fn draw_trunks(p: &mut Painter, sl: &Slab, forest: &Forest) {
    for tr in &forest.trunks {
        if tr.z < sl.near || tr.z >= sl.far {
            continue;
        }
        // the trunk in three lengths, narrowing as it goes up; its foot fades into the haze with
        // distance, its height stays a silhouette against the canopy's gaps
        let w0 = (sl.v.f * tr.r / tr.z).max(0.6 * sl.ppp);
        let foot = sl.ink(tr.z, 0.0);
        let k = (sl.fog(tr.z) * 0.35).min(1.0);
        let high = [
            0.010 * (1.0 - k) + 0.012 * k,
            0.013 * (1.0 - k) + 0.016 * k,
            0.016 * (1.0 - k) + 0.020 * k,
            1.0,
        ];
        let at = |y: f32| sl.v.project([tr.x + tr.lean * y, y, tr.z]).map(|(q, _)| q);
        // in short lengths, its colour going from the haze at its foot to dark up in the canopy
        const YS: [f32; 7] = [0.0, EYE + 1.0, 4.5, 7.0, 10.0, 14.0, 120.0];
        for (j, w) in YS.windows(2).enumerate() {
            let (Some(a), Some(b)) = (at(w[0]), at(w[1])) else {
                continue;
            };
            let u = (j as f32 / 4.0).min(1.0);
            let col = [
                foot[0] + (high[0] - foot[0]) * u,
                foot[1] + (high[1] - foot[1]) * u,
                foot[2] + (high[2] - foot[2]) * u,
                1.0,
            ];
            let wj = w0 * (1.0 - 0.05 * j as f32);
            p.line(a, b, wj, 0.6 * sl.ppp + wj * 0.06, col);
        }
    }
}

/// The ferns in the slab.
fn draw_ferns(p: &mut Painter, sl: &Slab, forest: &Forest) {
    // the ferns: fronds arching out of a crown, their leaflets in pairs, dark on the litter
    for fern in &forest.ferns {
        if fern.z < sl.near || fern.z >= sl.far {
            continue;
        }
        let col = sl.ink(fern.z, 0.4);
        for k in 0..6 {
            let a = fern.turn + k as f32 * 1.05;
            let (dx, dz) = (a.cos(), a.sin() * 0.5);
            let pt = |u: f32| {
                let r = fern.size * u;
                [
                    fern.x + dx * r,
                    0.03 + fern.size * (0.9 * u - 0.75 * u * u),
                    fern.z + dz * r,
                ]
            };
            let mut last: Option<[f32; 2]> = None;
            for i in 0..=12 {
                let u = i as f32 / 12.0;
                let Some((q, z)) = sl.v.project(pt(u)) else {
                    continue;
                };
                let wid = (sl.v.f * 0.006 / z).max(0.5 * sl.ppp);
                if let Some(l) = last {
                    p.line(l, q, wid, 0.6 * sl.ppp, col);
                }
                if i > 0 && i < 12 {
                    let leaf = sl.v.f * fern.size * 0.2 * (1.0 - u * 0.7) / z;
                    let (nx, ny) = (-dz, dx);
                    for sgn in [-1.0f32, 1.0] {
                        p.line(
                            q,
                            [
                                q[0] + sgn * nx * leaf,
                                q[1] + sgn * ny.abs() * leaf * 0.3 - leaf * 0.15,
                            ],
                            wid * 2.2,
                            0.6 * sl.ppp,
                            col,
                        );
                    }
                }
                last = Some(q);
            }
        }
    }
}

/// The rhododendron's clumps in the slab.
fn draw_leaves(p: &mut Painter, sl: &Slab, forest: &Forest) {
    for l in &forest.leaves {
        if l.at[2] < sl.near || l.at[2] >= sl.far {
            continue;
        }
        let Some((c, z)) = sl.v.project(l.at) else {
            continue;
        };
        let r = sl.v.f * l.size / z;
        let col = sl.ink(z, 1.0 + l.shade);
        // a clump of leaves: a dozen small discs round its centre, a flat top
        let mut k = (l.shade * 1000.0) as u64 | 1;
        for _ in 0..12 {
            let a = swarm::rand(&mut k) as f32 * std::f32::consts::TAU;
            let d = (swarm::rand(&mut k) as f32).sqrt();
            let q = [
                c[0] + a.cos() * d * r * 1.2,
                c[1] + a.sin().abs() * d * r * 0.55 - r * 0.15,
            ];
            p.disc(q, r * 0.32, r * 0.06 + sl.ppp, col);
        }
    }
}

/// The fireflies in the slab: a halo and a core each, blurred close by.
fn draw_flies(p: &mut Painter, sl: &Slab, sw: &Swarm, bright: f32) {
    let mut fly = |pos: [f32; 3], from: [f32; 3], g: f32, small: f32| {
        if g <= 0.002 || pos[2] < sl.near || pos[2] >= sl.far {
            return;
        }
        let (Some((a, z)), Some((b, _))) = (sl.v.project(pos), sl.v.project(from)) else {
            return;
        };
        let g = g * bright;
        let core = (sl.v.f * 0.016 / z).max(1.6 * sl.ppp) * small;
        // out of focus close by: a wider, softer disc
        let blur = ((4.5 - z) / 3.0).clamp(0.0, 1.0) * 14.0 * sl.ppp;
        let halo = (sl.v.f * 0.22 / z + 9.0 * sl.ppp) * small;
        let c = LIGHT;
        if small > 1.2 {
            // a female: her light lies on the leaf litter round her, a low pool
            let r = sl.v.f * 0.22 / z;
            p.glow(a, a, r, r * 0.8, [c[0], c[1], c[2], 0.16 * g]);
        }
        p.glow(
            b,
            a,
            halo,
            halo,
            [c[0], c[1], c[2], 0.035 * g * (z / 4.0).clamp(0.5, 1.0)],
        );
        p.glow(
            b,
            a,
            core + blur,
            0.7 * sl.ppp + blur,
            [0.6 + 0.4 * c[0], c[1], 0.32 + 0.48 * c[2], 0.85 * g],
        );
    };
    for (i, m) in sw.males[..sw.out].iter().enumerate() {
        fly(m.pos, m.pos, sw.male_glow(i), 1.0);
    }
    for (i, f) in sw.females.iter().enumerate() {
        fly(f.pos, f.pos, sw.female_glow(i), 1.3);
    }
}
