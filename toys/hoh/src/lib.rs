//! Hoh (a working title): the rainforest at the foot of Mount Olympus, on Washington's Olympic
//! Peninsula, where some three and a half metres of rain a year (140 inches) grow moss over everything. Seed falls
//! everywhere, but on the moss carpet a seedling is smothered; only on a fallen log, raised out of it,
//! does one live. So the young trees stand in lines along the logs, the tallest shading the rest out,
//! and when the log has rotted away, two centuries on, the survivors stand in a row on stilts where it
//! lay (forest.rs). The years run slowly by themselves; a drag runs them faster; a tap on a giant fells
//! it, and it becomes the next log. painting-company row 343.

pub mod draw;
pub mod forest;
pub mod history;
pub mod sound;

use draw::{Fern, Scene, View, EYE};
use forest::{Forest, Z0};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::info::{Demo, Rules, Stroke};
use toy_kit::paint::Painter;
use toy_kit::switches::Switch;
use toy_kit::{Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: the Hoh Rain Forest, Olympic National Park (lon, lat; Wikipedia's).
pub const PLACE: (f64, f64) = (-123.9247, 47.8614);
pub const SEED: u64 = 0x0400_4f11;

/// The years a second at rest, and as fast as a drag can run them.
const PACE: f32 = 1.0;
const PACE_MAX: f32 = 60.0;
/// A tap: quicker than this, and moving less than this many points.
const TAP_S: f64 = 0.4;
const TAP_PT: f64 = 14.0;
/// How far from a trunk (points) a tap still takes it.
const REACH_PT: f32 = 22.0;

struct Finger {
    id: u64,
    down: [f64; 2],
    down_t: f64,
    last: [f64; 2],
    last_t: f64,
    travelled: f64,
}

pub struct Hoh {
    forest: Forest,
    field: Field,
    cells: Vec<[f32; 4]>,
    painter: Painter,
    sound: sound::Sound,
    ferns: Vec<Fern>,
    finger: Option<Finger>,
    /// Years a second over the resting pace, from the drag.
    hurry: f32,
    t: f64,
    px: [f32; 2],
    ppp: f64,
    thuds: u32,
    size: f32,
}

impl Hoh {
    fn view(&self) -> View {
        View::of(self.px[0], self.px[1])
    }

    /// The giant a tap at `q` (px) takes: the nearest to the eye whose trunk is under the finger.
    fn giant_at(&self, q: [f32; 2]) -> Option<usize> {
        let v = self.view();
        let reach = REACH_PT * self.ppp as f32;
        let mut best: Option<(usize, f32)> = None;
        for (i, t) in self.forest.trees.iter().enumerate() {
            if t.h < 25.0 || t.dying() {
                continue;
            }
            let Some(foot) = v.project([t.x, 0.0, t.z]) else {
                continue;
            };
            let hw = 0.5 * t.d() * v.f / t.z;
            if (q[0] - foot[0]).abs() > hw + reach || q[1] > foot[1] + reach {
                continue;
            }
            if best.is_none_or(|(_, z)| t.z < z) {
                best = Some((i, t.z));
            }
        }
        best.map(|(i, _)| i)
    }

    fn tap(&mut self, ctx: &mut Ctx, q: [f32; 2]) {
        let Some(k) = self.giant_at(q) else {
            return;
        };
        let v = self.view();
        let t = &self.forest.trees[k];
        let foot = v.project([t.x, 0.0, t.z]).unwrap_or(q);
        // pushed: it falls away from the side the finger touched
        let side = if q[0] <= foot[0] { 1.0 } else { -1.0 };
        self.size = t.h;
        let dir = self.forest.way(k, Some(side));
        self.forest.fell(k, dir);
        ctx.haptic();
    }
}

impl Toy for Hoh {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let forest = history::opening(SEED);
        let [gw, gh] = forest.grid();
        Ok(Hoh {
            field: Field::new(
                &ctx.gpu,
                "hoh",
                [gw as u32, gh as u32],
                include_str!("scene.wgsl"),
                None,
            ),
            cells: vec![[0.0; 4]; gw * gh],
            forest,
            painter: Painter::new(&ctx.gpu),
            sound: sound::Sound::default(),
            ferns: draw::ferns(0x0fe7),
            finger: None,
            hurry: 0.0,
            t: 0.0,
            px: [ctx.width as f32, ctx.height as f32],
            ppp: ctx.ppp,
            thuds: 0,
            size: 60.0,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.px = [ctx.width as f32, ctx.height as f32];
        self.ppp = ctx.ppp;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                if self.finger.is_none() {
                    self.finger = Some(Finger {
                        id: e.id,
                        down: [e.x, e.y],
                        down_t: e.t,
                        last: [e.x, e.y],
                        last_t: e.t,
                        travelled: 0.0,
                    });
                }
            }
            Phase::Move => {
                let Some(f) = self.finger.as_mut().filter(|f| f.id == e.id) else {
                    return;
                };
                let d = (e.x - f.last[0]).hypot(e.y - f.last[1]);
                let dt = (e.t - f.last_t).max(1.0 / 120.0);
                f.travelled += d;
                f.last = [e.x, e.y];
                f.last_t = e.t;
                if f.travelled > TAP_PT {
                    // the faster the drag, the faster the years
                    let want = ((d / dt) as f32 * 0.09).min(PACE_MAX - PACE);
                    self.hurry = self.hurry.max(want);
                }
            }
            Phase::Up | Phase::Cancel => {
                if self.finger.as_ref().is_none_or(|f| f.id != e.id) {
                    return;
                }
                let Some(f) = self.finger.take() else {
                    return;
                };
                let quick = e.t - f.down_t < TAP_S && f.travelled < TAP_PT;
                if e.phase == Phase::Up && quick {
                    let k = ctx.ppp as f32;
                    self.tap(ctx, [f.down[0] as f32 * k, f.down[1] as f32 * k]);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt.min(0.1) as f32;
        self.t += dt as f64;
        self.px = [ctx.width as f32, ctx.height as f32];
        self.ppp = ctx.ppp;
        // a held finger keeps the years running; let go, they ease back to the resting pace
        let held = self.finger.as_ref().is_some_and(|f| f.travelled > TAP_PT);
        if !held {
            self.hurry *= (-dt / 0.9).exp();
        }
        self.forest.run((PACE + self.hurry) * dt);
        self.forest.tick(dt);
        for h in std::mem::take(&mut self.forest.landed) {
            self.thuds += 1;
            self.size = h;
            ctx.haptic();
        }
        self.sound.set(sound::Mix {
            hurry: self.hurry,
            cracks: self.forest.started,
            thuds: self.thuds,
            size: self.size,
        });
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        for (c, (_, l)) in self.cells.iter_mut().zip(self.forest.cells()) {
            c[0] = l;
        }
        self.field.upload(&ctx.gpu, &self.cells);
        let v = self.view();
        let params = BackdropParams {
            time: self.t as f32,
            scale: 1.0,
            center: [self.px[0] * 0.5, self.px[1] * 0.5],
            p: [
                v.horizon,
                v.f,
                EYE,
                v.cx,
                forest::X,
                Z0,
                Forest::CELL,
                self.t as f32 * (1.0 + self.hurry * 0.2),
            ],
        };
        Scene {
            view: &v,
            forest: &self.forest,
            ferns: &self.ferns,
            ppp: ctx.ppp as f32,
            t: self.t as f32,
        }
        .paint(&mut self.painter);
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let mut out = Vec::new();
        let k = self.ppp;
        if k <= 0.0 {
            return out;
        }
        let v = self.view();
        let pt = |q: [f32; 2]| [q[0] as f64 / k, q[1] as f64 / k];
        // the near giant closest to the middle, a little up its trunk
        let giant = self
            .forest
            .trees
            .iter()
            .filter(|t| t.h > 40.0 && !t.dying() && t.z < 22.0 && t.x.abs() < t.z * 0.4)
            .min_by(|a, b| (a.x.abs() + a.z * 0.2).total_cmp(&(b.x.abs() + b.z * 0.2)));
        if let Some(t) = giant {
            if let Some(q) = v.project([t.x, t.base + EYE * 0.7, t.z]) {
                out.push(("giant".to_string(), pt(q)));
            }
        }
        if let Some(l) = self
            .forest
            .logs
            .iter()
            .min_by(|a, b| a.age.total_cmp(&b.age))
        {
            let m = [(l.a[0] + l.b[0]) * 0.5, (l.a[1] + l.b[1]) * 0.5];
            if let Some(q) = v.project([m[0], l.r(), m[1]]) {
                out.push(("newest log".to_string(), pt(q)));
            }
        }
        out
    }

    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        let [w, h] = ctx.points();
        Some(Demo::new(
            "The Hoh rainforest. Drag to run the years.",
            vec![Stroke::drag(
                0.4,
                1.4,
                [0.15 * w, 0.72 * h],
                [0.85 * w, 0.66 * h],
            )],
        ))
    }

    fn rules(&self) -> Option<Rules> {
        Some(Rules::lines(&[
            "In the Hoh, below Mount Olympus, seed falls everywhere, but the floor is so thick with moss that seedlings die there. Those on a fallen log live.",
            "Along the log the taller shade the rest out. The log rots away, and the winners stand in a row on stilts where it lay.",
            "Drag to run the years; tap a giant to fell it.",
        ]))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&forest::RULES, &self.forest.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if i < forest::RULES.len() {
            self.forest.set(i, on);
        }
    }
}

toy_kit::ios_shell!(Hoh);
