//! Bubblers (a working title): low tide on a beach at Phuket, seen from straight above, a hand's
//! breadth of sand filling the screen. Sand-bubbler crabs feed out from their burrows in spokes,
//! leaving the sifted sand behind them as little balls, and the beach fills with starbursts nobody
//! drew (`beach.rs`). A tap is a footstep: the near crabs bolt, their neighbours see them go and bolt
//! too, and a while later the eyes come up again. A finger drawn through the sand leaves a furrow they
//! work around. Shaken, or now and then on its own, the tide comes in and takes it all. It opens close on
//! one crab rolling its balls, and the eye lifts to the whole beach covered in starbursts.
//! painting-company row 327.

pub mod beach;
pub mod beads;
pub mod sound;

use beach::{Beach, CELL};
use sound::{Sound, Tick};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: a beach on Phuket's west coast (lon, lat).
pub const PLACE: (f64, f64) = (98.30, 7.90);

/// The screen is this many centimetres across.
const WIDTH_CM: f32 = 30.0;
/// The tide went out this long before the toy opens (seconds): the crabs are out and their starbursts
/// begun.
const OPENING_S: f32 = 25.0;
/// The opening: close in on one crab at work, this many times nearer, held this long, then the eye
/// pulls back over this long to see its neighbours' starbursts round it, still near enough to see a crab
/// run (seconds).
const CLOSE: f32 = 4.5;
const FAR: f32 = 1.7;
const HOLD_S: f32 = 2.5;
const PULL_S: f32 = 9.0;
/// The tide comes on its own after this long untouched by one (seconds).
const TIDE_EVERY_S: f32 = 210.0;

pub struct Bubblers {
    beach: Beach,
    field: Field,
    cells: Vec<[f32; 4]>,
    painter: Painter,
    sound: Sound,
    k: f32,
    t: f64,
    last_tide: f32,
    fingers: Vec<(u64, [f32; 2], f64)>,
    size_pt: [f64; 2],
    /// the crab the opening looks at (cm), and whether to pull back at all (not with Reduce Motion)
    focus: [f32; 2],
    pull: bool,
}

impl Bubblers {
    /// Where the eye is (its centre, cm) and how near (1 the whole width of the beach).
    fn view(&self) -> ([f32; 2], f32) {
        let mid = [
            self.size_pt[0] as f32 / self.k * 0.5,
            self.size_pt[1] as f32 / self.k * 0.5,
        ];
        let a = if self.pull {
            ((self.t as f32 - HOLD_S) / PULL_S).clamp(0.0, 1.0)
        } else {
            1.0
        };
        // eased both ends, the eye rising like a head lifted from the sand
        let e = a * a * (3.0 - 2.0 * a);
        let zoom = CLOSE.powf(1.0 - e) * FAR.powf(e);
        // (drifting halfway toward the middle, so the view never leaves the beach)
        let c = [
            self.focus[0] + (mid[0] - self.focus[0]) * e * 0.5,
            self.focus[1] + (mid[1] - self.focus[1]) * e * 0.5,
        ];
        (c, zoom)
    }

    /// Where a place on the beach is heard, left (-1) to right (1) of the screen as it is now.
    fn pan(&self, at: [f32; 2]) -> f32 {
        let (c, z) = self.view();
        let half = self.size_pt[0] as f32 / self.k * 0.5;
        ((at[0] - c[0]) * z / half).clamp(-1.0, 1.0)
    }

    /// A point on screen (points) as a place on the beach (cm).
    fn cm(&self, p: [f64; 2]) -> [f32; 2] {
        let (c, z) = self.view();
        let k = self.k * z;
        [
            c[0] + (p[0] as f32 - self.size_pt[0] as f32 * 0.5) / k,
            c[1] + (p[1] as f32 - self.size_pt[1] as f32 * 0.5) / k,
        ]
    }

    fn rustle(&self, at: [f32; 2], amp: f32) -> Tick {
        Tick {
            delay: 0.0,
            amp,
            tau: 0.07,
            bright: 0.55,
            pan: self.pan(at) * 0.7,
        }
    }
}

impl Toy for Bubblers {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let k = size_pt[0] as f32 / WIDTH_CM;
        let h = size_pt[1] as f32 / k;
        let mut beach = Beach::new(WIDTH_CM, h, 0x0B0B_B1E5);
        let dt = 0.05;
        for _ in 0..(OPENING_S / dt) as usize {
            beach.step(dt);
        }
        // the clock starts at zero for the toy; the beach keeps its own
        let field = Field::new(
            &ctx.gpu,
            "bubblers-sand",
            [beach.gw as u32, beach.gh as u32],
            include_str!("sand.wgsl"),
            None,
        );
        let n = beach.gw * beach.gh;
        // the opening looks at the crab nearest the middle that is out feeding
        let mid = [WIDTH_CM * 0.5, h * 0.45];
        let d2 = |c: &beach::Crab| (c.home[0] - mid[0]).powi(2) + (c.home[1] - mid[1]).powi(2);
        let focus = beach
            .crabs
            .iter()
            .filter(|c| c.dug)
            .min_by(|a, b| d2(a).total_cmp(&d2(b)))
            .map_or(mid, |c| c.home);
        Ok(Bubblers {
            last_tide: beach.t,
            beach,
            field,
            cells: vec![[0.0; 4]; n],
            painter: Painter::new(&ctx.gpu),
            sound: Sound::default(),
            k,
            t: 0.0,
            fingers: Vec::new(),
            size_pt,
            focus,
            pull: !ctx.reduce_motion,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        // the beach keeps its size; the screen shows it from the top left at the same scale
        self.size_pt = ctx.points();
        self.k = self.size_pt[0] as f32 / WIDTH_CM;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = self.cm([e.x, e.y]);
        match e.phase {
            Phase::Down => {
                self.fingers.retain(|f| f.0 != e.id);
                self.fingers.push((e.id, p, self.t));
                self.beach.footstep(p, 1.0);
                ctx.haptic();
                self.sound.send(&[self.rustle(p, 0.25)]);
            }
            Phase::Move => {
                let t = self.t;
                let mut step = None;
                if let Some(f) = self.fingers.iter_mut().find(|f| f.0 == e.id) {
                    let last = f.1;
                    self.beach.furrow(last, p);
                    f.1 = p;
                    if t - f.2 > 0.14 {
                        f.2 = t;
                        step = Some(p);
                    }
                }
                if let Some(q) = step {
                    self.beach.footstep(q, 0.6);
                }
            }
            _ => self.fingers.retain(|f| f.0 != e.id),
        }
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        if self.beach.tide.is_none() {
            self.beach.bring_tide();
            self.last_tide = self.beach.t;
            ctx.haptic();
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = (dt as f32).min(0.1);
        self.beach.step(dt);
        if self.beach.t - self.last_tide > TIDE_EVERY_S && self.beach.tide.is_none() {
            self.beach.bring_tide();
            self.last_tide = self.beach.t;
        }
        let mut ticks = Vec::new();
        for (i, p) in self.beach.laid.iter().enumerate().take(6) {
            let h = ((p[0] * 131.0 + p[1] * 71.0).sin() * 0.5 + 0.5).fract();
            ticks.push(Tick {
                delay: i as f32 * 0.011 + h * dt,
                amp: 0.10 + 0.08 * h,
                tau: 0.0035,
                bright: 0.75 + 0.25 * h,
                pan: self.pan(*p) * 0.8,
            });
        }
        let bolted = self.beach.bolted.len();
        if bolted > 0 {
            let at = self.beach.bolted[0];
            ticks.push(self.rustle(at, (0.06 * bolted as f32).min(0.3)));
        }
        if !ticks.is_empty() {
            self.sound.send(&ticks);
        }
        let wash = self
            .beach
            .front()
            .map(|f| (1.0 - f / self.beach.h).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        self.sound.wash(wash);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let b = &self.beach;
        if b.dirty {
            for i in 0..b.gw * b.gh {
                self.cells[i] = [b.worked[i], b.wet[i], b.furrow[i], 0.0];
            }
            self.field.upload(&ctx.gpu, &self.cells);
            self.beach.dirty = false;
        }
        let b = &self.beach;
        let ppp = ctx.ppp as f32;
        let (eye, zoom) = self.view();
        let s = self.k * ppp * zoom;
        let half = [ctx.width as f32 * 0.5, ctx.height as f32 * 0.5];
        let px = |p: [f32; 2]| [(p[0] - eye[0]) * s + half[0], (p[1] - eye[1]) * s + half[1]];
        let params = BackdropParams {
            time: b.t,
            scale: s,
            center: eye,
            p: [
                b.front().unwrap_or(-100.0),
                b.h,
                CELL,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        };
        beads::burrows(&mut self.painter, b, &px, s);
        beads::balls(&mut self.painter, b, &px, s);
        for c in &b.crabs {
            beads::crab(&mut self.painter, c, s, b.t, &px);
        }
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

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Sand-bubbler crabs at low tide. Tap to step.",
            vec![info::Stroke::tap(0.5, [s[0] * 0.5, s[1] * 0.45])],
        ))
    }

    fn style(&self) -> Option<info::Style> {
        Some(info::Style {
            name: "Bead Game",
            maker: "Ishu Patel",
            place: "National Film Board of Canada",
            year: 1977,
        })
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::lines(&[
            "On the beaches of Phuket at low tide, sand-bubbler crabs feed outward from their burrows, leaving the sifted sand behind in little balls.",
            "Each run starts beside the last, on unsifted sand, so the balls spread into starbursts.",
            "A crab that sees a neighbour bolt bolts too. Your tap is a footstep; drag to draw in the sand; shake, and the tide comes in.",
        ]))
    }

    fn switches(&self) -> Vec<toy_kit::switches::Switch> {
        toy_kit::switches::Switch::all(&beach::RULES, &self.beach.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(r) = self.beach.on.get_mut(i) {
            *r = on;
        }
    }
}

toy_kit::ios_shell!(Bubblers);
