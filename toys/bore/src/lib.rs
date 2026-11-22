//! The Silver Dragon (a working title): the Qiantang's tidal bore at Yanguan, seen from the seawall among
//! the crowd and painted in ink-wash. The tide is solved along the estuary (`tide.rs`): the bay's funnel
//! piles it up and over the shallow sands its front catches itself up and stands as a wall, which runs up
//! the river at us and bursts along the wall (`spray.rs`) by the people come to watch (`crowd.rs`). A
//! finger drags the moon through its month: springs bring the biggest bores, neaps a ripple. The tides
//! the scene cuts to (a later day, the next tide) are kept ready in `data/tides.bin`.

pub mod crowd;
pub mod sound;
pub mod spray;
pub mod tide;
pub mod view;

use crowd::{Crowd, Stroke};
use sound::Sound;
use spray::Spray;
use tide::{age_moon, read_library, slot, Front, Snap, Tide, AGES, YANGUAN};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};
use view::{bow, ground, Camera, BREADTH};

/// Where it is: the tide-watching seawall at Yanguan, Haining (lon, lat).
pub const PLACE: (f64, f64) = (120.535, 30.395);

const COLS: usize = 512;
const ZMIN: f64 = -400.0;
const ZMAX: f64 = 9000.0;
/// The moon it opens on: a day after full, among the biggest tides.
const OPENING_AGE: usize = 8;

/// A cut to another tide: fading to the paper, swapping, fading back.
struct Cut {
    to: usize,
    at: f64,
    swapped: bool,
}

struct Drag {
    id: u64,
    x0: f64,
    moon0: f64,
}

pub struct Bore {
    library: Vec<Snap>,
    tide: Tide,
    front: Option<Front>,
    age: usize,
    /// the moon as drawn (follows the finger; settles on the library's ages)
    moon: f64,
    drag: Option<Drag>,
    on: [bool; 2],
    cut: Option<Cut>,
    fade: f32,
    /// when this tide began, and when its front went past us
    began: f64,
    passed: Option<f64>,
    rate: f64,
    wet: f32,
    /// how far down at the rail the eye is (0 up on the stand, 1 at the rail), and where it watches
    near: f64,
    watch: f64,
    cam: Camera,
    crowd: Crowd,
    spray: Spray,
    strokes: Vec<Stroke>,
    field: Field,
    cells: Vec<[f32; 4]>,
    painter: Painter,
    sound: Sound,
    t: f64,
    size_pt: [f64; 2],
    buzzed: bool,
}

impl Bore {
    fn moon_x(&self, moon: f64) -> f64 {
        self.size_pt[0] * (0.14 + 0.72 * moon)
    }

    fn begin(&mut self, k: usize) {
        self.tide = Tide::from_snap(&self.library[k]);
        self.began = self.t;
        self.passed = None;
        self.front = self.tide.front();
    }

    fn cut_to(&mut self, k: usize) {
        // a cut already going to the paper goes on to the new tide instead; one coming back from it turns
        // round where it is, so the paper never jumps
        let at = match &mut self.cut {
            Some(c) if !c.swapped => {
                c.to = k;
                return;
            }
            Some(_) => self.t - 0.45 * self.fade as f64,
            None => self.t,
        };
        self.cut = Some(Cut {
            to: k,
            at,
            swapped: false,
        });
    }

    /// The moon eases to the age it was let go at.
    fn ease_moon(&mut self, dt: f64) {
        if self.drag.is_none() {
            let to = age_moon(self.age);
            let mut d = to - self.moon;
            if d > 0.5 {
                d -= 1.0;
            } else if d < -0.5 {
                d += 1.0;
            }
            self.moon = (self.moon + d * (dt * 6.0).min(1.0)).rem_euclid(1.0);
        }
    }

    /// A cut: to the paper, the new tide, back.
    fn run_cut(&mut self, t: f64) {
        if let Some(c) = &mut self.cut {
            let k = t - c.at;
            if !c.swapped && k >= 0.45 {
                c.swapped = true;
                let to = c.to;
                self.begin(to);
            }
            let k = t - self.cut.as_ref().map_or(t, |c| c.at);
            self.fade = if k < 0.45 {
                smooth(0.0, 0.45, k) as f32
            } else {
                1.0 - smooth(0.45, 1.25, k) as f32
            };
            if k > 1.25 {
                self.cut = None;
                self.fade = 0.0;
            }
        }
    }

    /// The spray where the front meets the stones (and where the crowd steps back from it), how hard it
    /// slaps, and the promenade left wet.
    fn splash(
        &mut self,
        ctx: &mut Ctx,
        bore: Option<(Front, f64, f64)>,
        dt: f64,
    ) -> (Option<(f64, f64)>, f64) {
        let mut burst = None;
        let mut slap = 0.0;
        if let Some((f, z, k)) = bore {
            let level = f.behind;
            let x = (view::WALL_TOP - level).max(0.0);
            let zi = z + bow(x, z);
            if zi > -20.0 && zi < 2500.0 {
                let broken = smooth(1.2, 1.45, f.froude);
                self.spray
                    .burst([x, level, zi], f.height() * k, broken, dt, self.rate);
                let near = (1.0 - zi.max(0.0) / 400.0).max(0.0);
                burst = Some((zi, (f.height() * (0.5 + broken) * k).min(1.0)));
                slap = (f.height() * (0.5 + broken) * k * near).min(1.0);
                if zi < 40.0 && slap > 0.3 && !self.buzzed {
                    self.buzzed = true;
                    ctx.haptic();
                }
                if f.height() > 0.7 && broken > 0.5 && zi < 25.0 {
                    self.wet = (self.wet + dt as f32 * 0.8).min(1.0);
                }
            }
        } else {
            self.buzzed = false;
        }
        self.wet = (self.wet - dt as f32 * 0.03).max(0.0);
        (burst, slap)
    }

    /// The eye goes down to the rail as the bore comes close, and back up the stand once it's past.
    fn follow(&mut self, bore: Option<(Front, f64, f64)>, dt: f64) {
        let (want, zi) = match bore {
            Some((_, z, k)) if self.passed.is_none() => (smooth(420.0, 90.0, z) * k, z),
            Some(_) => (0.0, self.watch),
            // a tide with no front: go down to the steps and watch the water climb them
            None if self.passed.is_none() => (0.3, 60.0),
            None => (0.0, self.watch),
        };
        let speed = if want > self.near { 0.5 } else { 0.45 };
        self.near += (want - self.near).clamp(-speed * dt, speed * dt);
        self.watch += (zi.max(-10.0) - self.watch) * (dt * 2.0).min(1.0);
        if self.cut.as_ref().is_some_and(|c| c.swapped) {
            self.near = 0.0;
        }
        self.cam = Camera::following(self.near, self.watch);
    }

    /// What's heard: the roar as it nears, its hiss as it breaks.
    fn voice(&mut self, bore: Option<(Front, f64, f64)>, slap: f64) {
        let (roar, hiss, pan) = match bore {
            Some((f, z, k)) => {
                let near = 1.0 / (1.0 + z.abs() / 900.0);
                let broken = smooth(1.15, 1.45, f.froude);
                (
                    (f.height() * k * near * 0.9) as f32,
                    (broken * k * near) as f32,
                    (0.3 - 0.5 * near) as f32,
                )
            }
            None => (0.0, 0.0, 0.0),
        };
        let crowd = 0.25 + roar.min(1.0) * 0.5;
        let quiet = 1.0 - self.fade;
        self.sound.set(
            roar.min(1.0) * quiet,
            hiss.min(1.0) * quiet,
            crowd * quiet,
            slap as f32 * quiet,
            pan,
        );
    }

    /// The front, if the tide has one: where it is down the river from us (m) and how strong (0..1).
    fn bore(&self) -> Option<(Front, f64, f64)> {
        let f = self.front?;
        let k = smooth(0.12, 0.3, f.steep);
        (k > 0.0).then_some((f, YANGUAN - f.x, k))
    }

    /// How many times faster than the river the scene runs now: the bore comes on quickly from far off
    /// and slows to near its own pace as it reaches us; after it, the flood rises, then time hurries on.
    fn pace(&self) -> f64 {
        match self.bore() {
            Some((_, z, _)) if z > -60.0 && self.passed.is_none() => {
                (1.0 + (z.max(0.0) / 25.0).powf(1.3)).min(300.0)
            }
            _ => match self.passed {
                Some(p) => (5.0 + 60.0 * (self.t - p).powi(2)).min(300.0),
                // no bore: hours of flood in seconds, so the water climbs the steps where you can watch it
                None => 900.0,
            },
        }
    }
}

impl Toy for Bore {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let library = read_library(&ctx.assets.read("tides.bin")?)?;
        anyhow::ensure!(
            library.len() == 4 * AGES,
            "the tide library is the wrong size"
        );
        let on = [true, true];
        let field = Field::new(
            &ctx.gpu,
            "bore",
            [COLS as u32, 2],
            include_str!("river.wgsl"),
            None,
        );
        let tide = Tide::from_snap(&library[slot(on, OPENING_AGE)]);
        let mut toy = Bore {
            front: tide.front(),
            tide,
            library,
            age: OPENING_AGE,
            moon: age_moon(OPENING_AGE),
            drag: None,
            on,
            cut: None,
            fade: 0.0,
            began: 0.0,
            passed: None,
            rate: 1.0,
            wet: 0.0,
            near: 0.0,
            watch: 400.0,
            cam: Camera::at_the_rail(),
            crowd: Crowd::new(),
            spray: Spray::default(),
            strokes: Vec::new(),
            field,
            cells: vec![[0.0; 4]; COLS * 2],
            painter: Painter::new(&ctx.gpu),
            sound: Sound::default(),
            t: 0.0,
            size_pt: ctx.points(),
            buzzed: false,
        };
        toy.rate = toy.pace();
        Ok(toy)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                if self.drag.is_none() {
                    self.drag = Some(Drag {
                        id: e.id,
                        x0: e.x,
                        moon0: self.moon,
                    });
                }
            }
            Phase::Move => {
                let w = self.size_pt[0] * 0.72;
                if let Some(d) = self.drag.as_ref().filter(|d| d.id == e.id) {
                    self.moon = (d.moon0 + (e.x - d.x0) / w).rem_euclid(1.0);
                }
            }
            _ => {
                if self.drag.as_ref().is_some_and(|d| d.id == e.id) {
                    self.drag = None;
                    // the moon settles on the nearest day the library keeps, and the scene goes to that
                    // day's tide as its front comes up the river
                    let age = (self.moon * AGES as f64).round() as usize % AGES;
                    if age != self.age {
                        self.age = age;
                        self.cut_to(slot(self.on, age));
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = dt.min(0.1);
        self.ease_moon(dt);
        self.run_cut(t);
        // the river, at its pace
        self.rate += (self.pace() - self.rate) * (dt * 3.0).min(1.0);
        self.tide.run(dt * self.rate);
        self.front = self.tide.front();
        let bore = self.bore();
        if let Some((_, z, _)) = bore {
            if z < -30.0 && self.passed.is_none() {
                self.passed = Some(t);
            }
        }
        // after it's gone by and the flood has risen a while, the next tide (or with no bore, after a
        // while of the tide rising)
        let since = t - self.began;
        let next = match self.passed {
            Some(p) => t - p > 9.0,
            None => bore.is_none() && since > 16.0,
        };
        if next && self.cut.is_none() {
            self.cut_to(slot(self.on, self.age));
        }
        let (burst, slap) = self.splash(ctx, bore, dt);
        let tide = &self.tide;
        self.spray.update(dt, self.rate, |x, z| {
            ground(x).max(tide.eta_at(YANGUAN - z))
        });
        self.crowd.update(t, dt, burst);
        self.follow(bore, dt);
        if let Some((f, z, k)) = bore {
            let broken = smooth(1.2, 1.45, f.froude);
            self.spray.roll(z, f.behind, f.height() * k, broken, dt);
        }
        self.voice(bore, slap);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let ppp = ctx.ppp;
        let size = [ctx.width, ctx.height];
        self.fill_cells(ppp);
        self.field.upload(&ctx.gpu, &self.cells);
        let c = self.cam.uniforms();
        let zf = self.bore().map_or(1e4, |(_, z, _)| z) as f32;
        let params = BackdropParams {
            time: self.t as f32,
            scale: 1.0,
            center: [0.0, 0.0],
            p: [c[0], c[1], c[2], c[3], c[4], c[5], zf, self.fade],
        };
        // the crowd, the rail and the spray, the far ones first
        self.strokes.clear();
        self.crowd.draw(
            &self.cam,
            self.size_pt,
            ppp,
            self.t,
            self.fade,
            &mut self.strokes,
        );
        self.spray
            .draw(&self.cam, self.size_pt, ppp, self.fade, &mut self.strokes);
        rail(&self.cam, self.size_pt, ppp, self.fade, &mut self.strokes);
        self.strokes.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for s in &self.strokes {
            s.paint(&mut self.painter);
        }
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        let y = s[1] * 0.085;
        let from = self.moon_x(self.moon);
        let to = self.moon_x(((self.moon + 0.25) % 1.0).min(0.95));
        Some(info::Demo::new(
            "The tide arrives as one wave. Drag the moon.",
            vec![info::Stroke::drag(0.4, 1.4, [from, y], [to, y])],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::lines(&[
            "At Yanguan on China's Qiantang River the tide comes up the river as one wave: the bore.",
            "The bay narrows like a funnel and piles the tide up; over the shallows its crest outruns its trough and stands up as a wall.",
            "It is biggest just after a full or new moon. Drag the moon.",
        ]))
    }

    fn switches(&self) -> Vec<toy_kit::switches::Switch> {
        toy_kit::switches::Switch::all(&tide::RULES, &self.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(o) = self.on.get_mut(i) {
            if *o != on {
                *o = on;
                self.cut_to(slot(self.on, self.age));
            }
        }
    }
}

impl Bore {
    /// The river's profile and the scene's numbers, as the shader reads them (`river.wgsl`'s header).
    fn fill_cells(&mut self, ppp: f64) {
        let bore = self.bore();
        let tide = &self.tide;
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for k in 0..COLS {
            let u = (k as f64 + 0.5) / COLS as f64;
            let z = ZMIN + (ZMAX - ZMIN) * u * u;
            let x = YANGUAN - z;
            let sim = tide.eta_at(x);
            let (ahead, behind) = match bore {
                Some((f, zf, _)) => {
                    let ahead = if z < zf - 400.0 { sim } else { f.ahead };
                    let behind = if z < zf + 400.0 {
                        f.behind
                    } else {
                        let join = tide.eta_at(YANGUAN - zf - 400.0);
                        sim + (f.behind - join) * (-(z - zf - 400.0) / 800.0).exp()
                    };
                    (ahead, behind)
                }
                None => (sim, sim),
            };
            lo = lo.min(ahead).min(behind);
            hi = hi.max(ahead).max(behind);
            self.cells[k] = [ahead as f32, behind as f32, tide.speed_at(x) as f32, 0.0];
        }
        let (k0, k1) = match bore {
            Some((f, _, k)) => {
                let dh = f.height();
                let fr = f.froude;
                let broken = smooth(1.2, 1.42, fr);
                let lam = (std::f64::consts::TAU * f.depth / (3.0 * (fr - 1.0)).max(0.01).sqrt())
                    .clamp(8.0, 80.0);
                let face = broken * (1.5 + 2.5 * dh) + (1.0 - broken) * lam * 0.3;
                hi += dh * 0.6 + 0.4;
                (
                    [f.ahead as f32, f.behind as f32, fr as f32, k as f32],
                    [face as f32, f.depth as f32, lam as f32, broken as f32],
                )
            }
            None => ([0.0; 4], [1.0, 1.0, 10.0, 0.0]),
        };
        let s = self.size_pt;
        let mx = self.moon_x(self.moon);
        let my = s[1] * (0.085 + 0.02 * ((self.moon - 0.5) * 2.0).powi(2));
        self.cells[COLS] = k0;
        self.cells[COLS + 1] = k1;
        self.cells[COLS + 2] = [
            (mx * ppp) as f32,
            (my * ppp) as f32,
            (s[0] * 0.05 * ppp) as f32,
            self.moon as f32,
        ];
        self.cells[COLS + 3] = [(lo - 0.6) as f32, (hi + 0.6) as f32, self.wet, 0.0];
    }
}

/// The promenade's rail along the wall's edge: posts and a top bar, in a firm stroke.
fn rail(cam: &Camera, size: [f64; 2], ppp: f64, fade: f32, out: &mut Vec<Stroke>) {
    let col = [0.075, 0.075, 0.085, 0.9 * (1.0 - fade)];
    let x = -0.35;
    let top = view::WALL_TOP + 1.05;
    let mut z = 7.0;
    let mut last: Option<([f64; 2], f64)> = None;
    while z < 900.0 {
        let a = cam.project([x, view::WALL_TOP, z], size);
        let b = cam.project([x, top, z], size);
        if let (Some((a, za)), Some((b, _))) = (a, b) {
            let k = cam.scale(za, size) * ppp;
            let (a, b) = ([a[0] * ppp, a[1] * ppp], [b[0] * ppp, b[1] * ppp]);
            out.push(Stroke::line(za, a, b, (0.03 * k).max(0.5), col));
            if let Some((lb, lz)) = last {
                out.push(Stroke::line(lz.max(za), lb, b, (0.025 * k).max(0.45), col));
            }
            last = Some((b, za));
        }
        z += 2.5 * (1.0 + z / 60.0);
    }
    let _ = BREADTH;
}

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

toy_kit::ios_shell!(Bore);
