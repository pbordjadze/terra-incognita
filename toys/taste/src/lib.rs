//! Taste (a working title; painting-company row 225): a small creature that keeps changing its shape
//! and colour. Nobody steers it. Rest a finger on it while you like what it is; it keeps trying on a
//! slightly different self, and keeps the new one as often as you held it compared with the old. So
//! it lingers where you linger and still wanders off now and then, and after a few minutes it pulls
//! back to show the ground it walked: every form it could be laid out as a wheel of colour, spikier
//! outward, its path across it, bright where you held it, and the forms it kept longest sitting there.
//!
//! The walk is `walk.rs` (a Metropolis walk whose target is the reader's held time); the creature is
//! drawn by `creature.rs`; everything else is here.

pub mod creature;
pub mod walk;

use creature::{Creatures, Instance};
use std::collections::BTreeMap;
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::sound::{pentatonic, Bells, Note};
use toy_kit::{Ctx, Phase, PointerEvent, Toy};
use walk::{Event, Form, Walker};

pub const SEED: u64 = 0x0007_a57e_2026;
/// The first creature: a soft violet star of five arms, half lit.
pub const START: Form = [0.74, 0.38, 0.32, 0.45];

/// The map: a form's place on the wheel (colour round, spikier outward), in units of the wheel's radius.
const HUB: f64 = 0.2;
pub fn on_map(f: &Form) -> [f64; 2] {
    let a = f[0] * std::f64::consts::TAU;
    let r = HUB + (1.0 - HUB) * f[1];
    [r * a.cos(), r * a.sin()]
}

/// When the creature first pulls back to show its path (steps taken, seconds held), and then every
/// further REVEAL_AGAIN steps.
const REVEAL_FIRST: usize = 56;
const REVEAL_HELD: f64 = 10.0;
const REVEAL_AGAIN: usize = 100;
/// The map's close zoom: how much larger the wheel is under the creature than when it pulls back.
const CLOSE: f64 = 4.0;

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    walker: Walker,
    revealed_at: usize,
    reveals: usize,
    seed: u64,
}

struct Finger {
    at: [f64; 2],
}

pub struct Taste {
    walker: Walker,
    ground: Backdrop,
    painter: Painter,
    creatures: Creatures,
    bells: Bells,
    points: [f64; 2],
    t: f64,
    fingers: BTreeMap<u64, Finger>,
    /// The zoom: 1 close on the creature, 0 the whole wheel; and where it is going.
    z: f64,
    z_to: f64,
    /// A pinch: the fingers' first spread and the zoom then.
    pinch: Option<(f64, f64)>,
    /// When the creature pulled back last (the walk's steps then), and how many times.
    revealed_at: usize,
    reveals: usize,
    /// While the wheel shows: when the path started drawing itself.
    replay: Option<f64>,
    /// How much a finger is on it now (eased), and where the creature looks.
    touch: f64,
    look: [f64; 2],
    /// Since a finger last lifted.
    free_since: f64,
    /// The liking at each kept form, worked out when the path changes (for the wheel's light).
    liked: Vec<f64>,
    liked_for: usize,
    still: bool,
    seed: u64,
    /// The last decision's moment (for a little swell when a form is kept).
    kept_at: f64,
    /// Whether a finger has rested on it yet (the reader is with it only after).
    ever_held: bool,
    /// The last frame a finger rested on it.
    held_at: f64,
    /// Touches landing on it: (where, when), each answered with a soft ring.
    pats: Vec<([f64; 2], f64)>,
}

struct View {
    /// the wheel's centre on screen and its radius, points
    o: [f64; 2],
    s: f64,
    /// the creature's place and radius, points
    c: [f64; 2],
    r: f64,
}

impl Taste {
    fn focus(&self) -> [f64; 2] {
        [self.points[0] * 0.5, self.points[1] * 0.46]
    }

    fn view(&self) -> View {
        let [w, h] = self.points;
        let far = 0.47 * w.min(h * 0.62);
        let z = self.z.clamp(0.0, 1.0);
        let ez = z * z * (3.0 - 2.0 * z);
        let s = far * CLOSE.powf(ez);
        let m = on_map(&self.walker.display());
        let f = self.focus();
        // the wheel's centre: on the focus when pulled back, under the creature when close
        let o = [f[0] - m[0] * s * ez, f[1] + m[1] * s * ez];
        let c = [o[0] + m[0] * s, o[1] - m[1] * s];
        let r_close = 0.32 * w.min(h * 0.5);
        let r_far = 0.06 * w.min(h * 0.62);
        let r = r_far * (r_close / r_far).powf(ez);
        View { o, s, c, r }
    }

    fn map_point(v: &View, f: &Form) -> [f64; 2] {
        let m = on_map(f);
        [v.o[0] + m[0] * v.s, v.o[1] - m[1] * v.s]
    }

    fn holding(&self) -> bool {
        if self.fingers.len() != 1 || self.z <= 0.9 || self.pinch.is_some() {
            return false;
        }
        // on it, or near it: a fingertip's slack round its rim
        let v = self.view();
        self.fingers.values().all(|f| (f.at[0] - v.c[0]).hypot(f.at[1] - v.c[1]) < v.r * 1.35 + 30.0)
    }

    fn reveal(&mut self) {
        self.z_to = 0.0;
        self.replay = Some(self.t);
        self.revealed_at = self.walker.steps;
        self.reveals += 1;
    }

    fn come_back(&mut self) {
        self.z_to = 1.0;
        self.replay = None;
    }

    fn refresh_liking(&mut self) {
        if self.liked_for != self.walker.path.len() || self.liked.len() != self.walker.path.len() {
            self.liked = self.walker.path.iter().map(|s| self.walker.memory.liking(&s.form)).collect();
            self.liked_for = self.walker.path.len();
        }
    }

    /// The kept forms to show as small creatures on the wheel: the longest kept first, none crowding another.
    fn keepsakes(&self, v: &View, upto: usize) -> Vec<usize> {
        let path = &self.walker.path;
        let n = upto.min(path.len());
        let mut order: Vec<usize> = (0..n).collect();
        let worth = |i: usize| path[i].dwell * (0.3 + self.liked.get(i).copied().unwrap_or(0.3));
        order.sort_by(|a, b| worth(*b).total_cmp(&worth(*a)));
        let gap = v.r * 2.3;
        let mut chosen: Vec<usize> = Vec::new();
        for i in order {
            if path[i].dwell < 2.0 || chosen.len() >= 16 {
                continue;
            }
            let p = Self::map_point(v, &path[i].form);
            if chosen.iter().all(|&j| {
                let q = Self::map_point(v, &path[j].form);
                (p[0] - q[0]).hypot(p[1] - q[1]) > gap
            }) {
                chosen.push(i);
            }
        }
        chosen
    }

    fn sound_of(f: &Form, amp: f32) -> Note {
        // the colour picks the note (a pentatonic run round the wheel), the spikes its brightness, the
        // glow how long it rings
        let degree = (f[0] * 10.0).round() as i32 + 3;
        Note { freq: pentatonic(196.0, degree), amp, decay: 1.4 + 2.6 * f[3] as f32, pan: 0.0, bright: 0.3 + 2.2 * f[1] as f32 }
    }
}

impl Toy for Taste {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let ground = Backdrop::new(&ctx.gpu, "taste ground", GROUND_WGSL);
        let painter = Painter::new(&ctx.gpu);
        let creatures = Creatures::new(&ctx.gpu);
        Ok(Taste {
            walker: Walker::new(SEED, START),
            ground,
            painter,
            creatures,
            bells: Bells::new(),
            points: ctx.points(),
            t: 0.0,
            fingers: BTreeMap::new(),
            z: 1.0,
            z_to: 1.0,
            pinch: None,
            revealed_at: 0,
            reveals: 0,
            replay: None,
            touch: 0.0,
            look: [0.0, 0.0],
            free_since: 0.0,
            liked: Vec::new(),
            liked_for: usize::MAX,
            still: ctx.reduce_motion,
            seed: SEED,
            kept_at: -10.0,
            ever_held: false,
            held_at: 0.0,
            pats: Vec::new(),
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.points = ctx.points();
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.fingers.insert(e.id, Finger { at: p });
                let v = self.view();
                if self.fingers.len() == 1 && self.z > 0.9 && (p[0] - v.c[0]).hypot(p[1] - v.c[1]) < v.r * 1.35 + 30.0 {
                    self.pats.push((p, e.t));
                    if self.pats.len() > 6 {
                        self.pats.remove(0);
                    }
                    let mut n = Self::sound_of(&self.walker.display(), 0.07);
                    n.freq *= 2.0;
                    n.decay = 0.9;
                    self.bells.strike(n);
                }
                if self.fingers.len() == 2 {
                    let v: Vec<[f64; 2]> = self.fingers.values().map(|f| f.at).collect();
                    self.pinch = Some(((v[0][0] - v[1][0]).hypot(v[0][1] - v[1][1]).max(10.0), self.z));
                }
            }
            Phase::Move => {
                if let Some(f) = self.fingers.get_mut(&e.id) {
                    f.at = p;
                }
                if let (Some((d0, z0)), 2) = (self.pinch, self.fingers.len()) {
                    let v: Vec<[f64; 2]> = self.fingers.values().map(|f| f.at).collect();
                    let d = (v[0][0] - v[1][0]).hypot(v[0][1] - v[1][1]).max(10.0);
                    // fingers apart: closer; together: the whole wheel
                    let z = (z0 + (d / d0).ln() / CLOSE.ln()).clamp(0.0, 1.0);
                    self.z = z;
                    self.z_to = z;
                }
            }
            Phase::Up | Phase::Cancel => {
                let f = self.fingers.remove(&e.id);
                if self.pinch.is_some() {
                    if self.fingers.is_empty() {
                        self.pinch = None;
                        if self.z < 0.6 {
                            self.z_to = 0.0;
                            if self.replay.is_none() {
                                self.replay = Some(self.t - 100.0);
                            }
                        } else {
                            self.come_back();
                        }
                    }
                } else if let Some(f) = f {
                    // a touch on the wheel: back to the creature
                    let _ = f;
                    if self.z_to < 0.5 {
                        self.come_back();
                    }
                }
                if self.fingers.is_empty() {
                    self.free_since = e.t;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.points = ctx.points();
        self.t = t;
        self.pats.retain(|(_, at)| t - at < 1.0);
        // the zoom eases to where it is going (unless fingers hold it)
        if self.pinch.is_none() {
            let rate = if self.still { 6.0 } else { 1.6 };
            self.z += (self.z_to - self.z) * (1.0 - (-rate * dt).exp());
            if (self.z - self.z_to).abs() < 1e-3 {
                self.z = self.z_to;
            }
        }
        let held = self.holding();
        if held {
            self.ever_held = true;
            self.held_at = t;
        }
        let target = if held { 1.0 } else { 0.0 };
        self.touch += (target - self.touch) * (1.0 - (-5.0 * dt).exp());
        // the creature looks toward the finger, or wanders its gaze
        let v = self.view();
        let want = match self.fingers.values().next().filter(|_| held) {
            Some(f) => {
                let d = [(f.at[0] - v.c[0]) / v.r, (f.at[1] - v.c[1]) / v.r];
                let l = d[0].hypot(d[1]).max(1.0);
                [d[0] / l, d[1] / l]
            }
            None => [0.5 * (t * 0.31).sin(), 0.4 * (t * 0.23 + 1.0).sin()],
        };
        let ease = 1.0 - (-3.0 * dt).exp();
        for (l, w) in self.look.iter_mut().zip(want) {
            *l += (w - *l) * ease;
        }
        // the walk moves while the creature is close; while the wheel shows it waits
        if self.z > 0.95 && self.z_to > 0.5 {
            // the reader is with it while a finger is on it or just was; before the first touch, watching
            // counts as with it (what it shows unheld is what it has shown in vain)
            let engaged = held || !self.ever_held || t - self.held_at < walk::ENGAGED;
            let ev = self.walker.step(dt, if held { dt } else { 0.0 }, engaged);
            match ev {
                Some(Event::Keeps) => {
                    self.kept_at = t;
                    let f = self.walker.x;
                    self.bells.strike(Self::sound_of(&f, if held { 0.16 } else { 0.09 }));
                    if held {
                        ctx.haptic();
                    }
                }
                Some(Event::Declines) => {
                    let f = self.walker.x;
                    let mut n = Self::sound_of(&f, 0.035);
                    n.freq *= 0.5;
                    n.bright = 0.0;
                    n.decay = 0.7;
                    self.bells.strike(n);
                }
                _ => {}
            }
            // a few minutes in, it pulls back to show where it has been
            let due = if self.reveals == 0 { REVEAL_FIRST } else { self.revealed_at + REVEAL_AGAIN };
            if self.walker.steps >= due
                && self.walker.memory.held_total() >= REVEAL_HELD
                && self.fingers.is_empty()
                && t - self.free_since > 1.2
                && self.walker.phase == walk::Phase::Rest
            {
                self.reveal();
            }
        }
        // a hum while it is held, its pitch the creature's colour
        let f = self.walker.display();
        self.bells.bed(if held { 0.35 } else { 0.0 }, 55.0 * 2f32.powf(f[0] as f32));
        // the replay's notes: each keepsake rings as the traveller passes it
        if self.replay.is_some() {
            self.refresh_liking();
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp;
        let size = [ctx.width, ctx.height];
        self.refresh_liking();
        let v = self.view();
        let px = |p: [f64; 2]| [(p[0] * k) as f32, (p[1] * k) as f32];
        let now = self.walker.display();
        let away = (1.0 - self.z).clamp(0.0, 1.0) as f32;
        let params = BackdropParams {
            time: self.t as f32,
            scale: (v.s * k) as f32,
            center: [0.0, 0.0],
            p: [(v.o[0] * k) as f32, (v.o[1] * k) as f32, HUB as f32, away, now[0] as f32, now[3] as f32, (v.c[0] * k) as f32, (v.c[1] * k) as f32],
        };
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        self.ground.render(gpu, &mut enc, target, size, &params);

        // how far the path has drawn itself (the replay), in stops
        let path = &self.walker.path;
        let n = path.len();
        let (upto, travel) = match self.replay {
            Some(at) => {
                let secs = (2.5 + 0.09 * n as f64).min(9.0);
                let u = ((self.t - at - 1.2) / secs).clamp(0.0, 1.0);
                let u = u * u * (3.0 - 2.0 * u);
                let x = u * (n.saturating_sub(1)) as f64;
                (x, Some(x))
            }
            None => ((n.saturating_sub(1)) as f64, None),
        };
        let shown = |i: usize| (i as f64) <= upto + 1e-9;

        // the light where it was held: a glow at each kept form, as bright as it was liked, as wide as long kept
        let path_a = 0.07 + 0.93 * away * away;
        for (i, s) in path.iter().enumerate() {
            if !shown(i) {
                break;
            }
            let like = self.liked.get(i).copied().unwrap_or(0.3) as f32;
            let p = Self::map_point(&v, &s.form);
            let col = hsv(s.form[0], 0.55, 1.0);
            let r = (v.r * (0.5 + 0.35 * s.dwell.sqrt().min(4.0))) as f32 * k as f32;
            let strength = (like * like * 0.5 + 0.03) * path_a;
            self.painter.glow(px(p), px(p), r * 0.3, r, [col[0], col[1], col[2], strength]);
        }
        // the path: from each kept form to the next, in the colour it was; the tries it turned down,
        // short faint twigs toward them
        let line_w = ((0.6 + 0.6 * away as f64) * k) as f32;
        for i in 0..n {
            if !shown(i) {
                break;
            }
            let a = Self::map_point(&v, &path[i].form);
            let col = hsv(path[i].form[0], 0.5, 1.0);
            for dcl in &path[i].declined {
                let b = Self::map_point(&v, dcl);
                let e = [a[0] + (b[0] - a[0]) * 0.32, a[1] + (b[1] - a[1]) * 0.32];
                self.painter.line(px(a), px(e), line_w * 0.45, 0.8, [col[0], col[1], col[2], 0.22 * path_a]);
            }
            if i + 1 < n {
                let frac = (upto - i as f64).clamp(0.0, 1.0);
                if frac <= 0.0 {
                    continue;
                }
                let b = Self::map_point(&v, &path[i + 1].form);
                let e = [a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac];
                self.painter.line(px(a), px(e), line_w, 0.8, [col[0], col[1], col[2], 0.55 * path_a]);
            }
        }
        // a touch on it: a ring spreading from the finger
        for &(p, at) in &self.pats {
            let a = ((self.t - at) / 0.9).clamp(0.0, 1.0) as f32;
            if a < 1.0 {
                self.painter.glow_ring(px(p), ((18.0 + 50.0 * a as f64) * k) as f32, (2.5 * k) as f32, (5.0 * k) as f32, [1.0, 0.97, 0.92, 0.45 * (1.0 - a)]);
            }
        }
        // a finger on it: a soft ring where it rests
        if self.touch > 0.02 {
            if let Some(f) = self.fingers.values().next() {
                let br = if self.still { 1.0 } else { 0.85 + 0.15 * (self.t * 3.0).sin() } as f32;
                self.painter.glow_ring(px(f.at), (26.0 * k) as f32, (3.0 * k) as f32, (6.0 * k) as f32, [1.0, 0.97, 0.92, 0.35 * self.touch as f32 * br]);
            }
        }
        self.painter.render(gpu, &mut enc, target, size, None);

        // the keepsakes on the wheel (as it pulls back), then the traveller, then the creature itself
        let tclock = self.t as f32;
        if away > 0.02 {
            let small = View { r: v.r.min(0.06 * self.points[0].min(self.points[1] * 0.62)) * 1.15, ..v };
            let keep = self.keepsakes(&small, (upto.floor() as usize) + 1);
            for &i in &keep {
                let p = Self::map_point(&v, &path[i].form);
                let appear = ((upto - i as f64) * 0.8).clamp(0.0, 1.0) as f32;
                self.creatures.list.push(Instance::new(px(p), (small.r * 0.9 * k) as f32, away * appear, &path[i].form, i as f32 * 1.7, 0.0, [0.0, 0.0]));
            }
            if let Some(x) = travel {
                let i = (x.floor() as usize).min(n.saturating_sub(1));
                let j = (i + 1).min(n - 1);
                let f = walk::lerp(&path[i].form, &path[j].form, x - i as f64);
                let p = Self::map_point(&v, &f);
                let done = x >= (n.saturating_sub(1)) as f64 - 1e-6;
                if !done {
                    self.creatures.list.push(Instance::new(px(p), (small.r * 1.05 * k) as f32, away, &f, 0.0, 0.0, [0.0, 0.0]));
                }
            }
        }
        let swell = {
            let a = ((self.t - self.kept_at) / 0.5).clamp(0.0, 1.0);
            1.0 + 0.05 * (a * std::f64::consts::PI).sin() * if self.still { 0.0 } else { 1.0 }
        };
        let bob = if self.still { 0.0 } else { 3.0 * (self.t * 0.7).sin() * self.z };
        let c = [v.c[0], v.c[1] + bob];
        // while the path is still drawing itself, the creature waits faint at its place
        let wait = match travel {
            Some(x) if x < (n.saturating_sub(1)) as f64 - 1e-6 => 0.35,
            _ => 1.0,
        };
        self.creatures.list.push(Instance::new(px(c), (v.r * swell * k) as f32, wait, &now, 0.0, self.touch as f32, [self.look[0] as f32, self.look[1] as f32]));
        self.creatures.render(gpu, &mut enc, target, size, tclock, self.still);
        gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.still = ctx.reduce_motion;
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.bells.synth())
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        // a new creature, and a clean slate: nothing remembered
        self.seed = self.seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(self.walker.steps as u64 + 1);
        let mut r = walk::Rng(self.seed | 1);
        let start: Form = std::array::from_fn(|_| 0.15 + 0.7 * r.f());
        self.walker = Walker::new(self.seed, start);
        self.revealed_at = 0;
        self.reveals = 0;
        self.replay = None;
        self.z_to = 1.0;
        self.liked_for = usize::MAX;
        self.ever_held = false;
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved { walker: self.walker.clone(), revealed_at: self.revealed_at, reveals: self.reveals, seed: self.seed }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            self.walker = s.walker;
            self.walker.phase = walk::Phase::Rest;
            self.walker.clock = 0.0;
            self.walker.y = self.walker.x;
            self.revealed_at = s.revealed_at;
            self.reveals = s.reveals;
            self.seed = s.seed;
            self.liked_for = usize::MAX;
        }
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let v = self.view();
        let mut a = vec![("creature".to_string(), v.c), ("centre".to_string(), self.focus())];
        let f = self.focus();
        a.push(("pinch-a".into(), [f[0] - 90.0, f[1] - 60.0]));
        a.push(("pinch-b".into(), [f[0] + 90.0, f[1] + 60.0]));
        a.push(("aside".into(), [self.points[0] * 0.5, self.points[1] * 0.85]));
        a
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let v = self.view();
        let p = [v.c[0] + v.r * 0.15, v.c[1] + v.r * 0.25];
        Some(toy_kit::info::Demo::new("Rest a finger on it while you like what it is.", vec![toy_kit::info::Stroke::hold(0.4, 3.2, p)]))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "It keeps trying on a slightly different self, and now and then a very different one.",
                "If you held forms like the new one as much as the old, it keeps it. If less, it keeps it only sometimes: the less, the more rarely. Only while your finger is on it, or just was, does it learn.",
                "So it lingers where you linger, and still wanders off. Every so often it pulls back to show where it has been: colour round the wheel, spikier outward, bright where you held it. Pinch to see it any time; shake for a new one.",
            ],
            "Real: nothing but your finger, and how long it rested on each form. Drawn: the creature, every form it could be, its sounds.",
        ))
    }
}

fn hsv(h: f64, s: f64, v: f64) -> [f32; 3] {
    let f = |n: f64| {
        let k = (n + h * 6.0).rem_euclid(6.0);
        v - v * s * (k.min(4.0 - k).clamp(0.0, 1.0))
    };
    [f(5.0) as f32, f(3.0) as f32, f(1.0) as f32]
}

/// The ground: deep water-dark, and under it, faint, the wheel of every form (colour round, spikier
/// outward), clearer as the creature pulls back. `p0.xy` the wheel's centre (px), `u.scale` its radius
/// (px), `p0.z` the hub, `p0.w` how far pulled back; `p1.x` the creature's colour, `p1.y` its glow,
/// `p1.zw` its place (px).
const GROUND_WGSL: &str = r#"
fn hsv3(h: f32, s: f32, v: f32) -> vec3<f32> {
  let k = vec3<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0);
  let p = abs(fract(vec3<f32>(h) + k) * 6.0 - vec3<f32>(3.0));
  return v * mix(vec3<f32>(1.0), clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)), s);
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let away = u.p0.w;
  let m = (px - u.p0.xy) / u.scale * vec2<f32>(1.0, -1.0);
  let r = length(m);
  let hue = atan2(m.y, m.x) / 6.2831853;
  // the deep: near black, a breath of the creature's opposite colour, lighter toward the creature
  let uv = px / u.size;
  var col = vec3<f32>(0.028, 0.030, 0.042) + hsv3(u.p1.x + 0.5, 0.5, 0.025);
  let dc = length((px - u.p1.zw) / u.size.x);
  col = col + hsv3(u.p1.x, 0.45, 0.07) * exp(-dc * dc * 5.0) * (1.0 - away * 0.6);
  // the wheel: each place tinted with the colour a creature there would be, darker toward the hub
  let inside = 1.0 - smoothstep(1.0, 1.0 + 2.0 / u.scale, r);
  let tint = hsv3(hue, 0.5, 0.06 + 0.13 * smoothstep(u.p0.z * 0.5, 1.0, r));
  let vis = 0.35 + 0.65 * away;
  col = col + tint * inside * vis;
  // its rim, and the hub's ring, hairlines
  let hair = 1.2 / u.scale;
  let rim = 1.0 - smoothstep(0.0, hair, abs(r - 1.0));
  let hub = 1.0 - smoothstep(0.0, hair, abs(r - u.p0.z));
  col = col + vec3<f32>(0.10, 0.11, 0.13) * (rim + 0.6 * hub) * vis;
  // grain against banding, and a vignette
  col = col + (hash2(px) - 0.5) * 0.008;
  let vg = length((uv - vec2<f32>(0.5, 0.46)) * vec2<f32>(1.0, u.size.y / u.size.x));
  col = col * (1.0 - 0.35 * smoothstep(0.4, 1.2, vg));
  return vec4<f32>(col, 1.0);
}
"#;

toy_kit::ios_shell!(Taste);
