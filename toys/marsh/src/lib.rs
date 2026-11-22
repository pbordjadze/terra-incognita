//! Magnet Marsh (a working title): a drop of water from the salt marsh at Woods Hole, Massachusetts, under a
//! dark-field microscope, where Richard Blakemore saw in 1975 that some bacteria all swim one way: each
//! carries a chain of magnetite, a compass needle, and the Earth's field steers it (`drop.rs`). The field
//! here is the one the phone feels (`Ctx::sensors`): turn round and the swarm wheels to keep north; hold a
//! magnet to the phone and they pour toward it. A finger is a magnet's pole held over the drop. Without a
//! compass (a desktop browser) north is the top of the screen.

pub mod drop;

use drop::{Drop, RULES};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::switches::Switch;
use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};

/// Where it is: the salt marsh at Woods Hole, Massachusetts (lon, lat).
pub const PLACE: (f64, f64) = (-70.67, 41.53);
pub const SEED: u64 = 0xB1A4_E140;
/// How many cells are in the drop.
const CELLS: usize = 900;
/// The field when the phone has no compass: the Earth's across a phone lying flat, north up (µT).
const NORTH_UP: [f32; 2] = [0.0, -20.0];
/// How quickly the shown field follows the phone's (seconds): the compass jitters.
const SETTLE: f32 = 0.15;

pub struct MagnetMarsh {
    drop: Drop,
    backdrop: Backdrop,
    painter: Painter,
    fingers: Vec<(u64, [f32; 2])>,
    /// The field across the glass the cells feel, eased toward the phone's (µT, screen directions).
    b: [f32; 2],
    t: f64,
    size_pt: [f64; 2],
}

/// The drop's centre, half-width and half-height for a screen of `s` points: an oval filling most of it.
fn place(s: [f64; 2]) -> ([f32; 2], f32, f32) {
    let rx = (s[0] * 0.47) as f32;
    let ry = (s[1] * 0.41).max(s[0] * 0.47) as f32;
    ([s[0] as f32 * 0.5, s[1] as f32 * 0.47], rx, ry)
}

impl MagnetMarsh {
    /// The field the phone feels across the glass, or north up without a compass.
    fn phone_field(ctx: &Ctx) -> [f32; 2] {
        match ctx.sensors.field_on_screen() {
            Some(f) => [f[0] as f32, f[1] as f32],
            None => NORTH_UP,
        }
    }
}

impl Toy for MagnetMarsh {
    fn senses(&self) -> toy_kit::Senses {
        // the field through the phone, or the compass where there is no field (`Sensors::field_on_screen`)
        toy_kit::Senses {
            shake: false,
            gravity: false,
            field: true,
            heading: true,
        }
    }
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let (c, r, ry) = place(size_pt);
        let b = Self::phone_field(ctx);
        Ok(MagnetMarsh {
            drop: Drop::new(c, r, ry, CELLS, b, SEED),
            backdrop: Backdrop::new(&ctx.gpu, "marsh", include_str!("drop.wgsl")),
            painter: Painter::new(&ctx.gpu),
            fingers: Vec::new(),
            b,
            t: 0.0,
            size_pt,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
        let (c, r, ry) = place(self.size_pt);
        let (c0, r0, ry0) = (self.drop.c, self.drop.r, self.drop.ry);
        let at = |p: [f32; 2]| {
            [
                c[0] + (p[0] - c0[0]) * r / r0,
                c[1] + (p[1] - c0[1]) * ry / ry0,
            ]
        };
        for cell in &mut self.drop.cells {
            cell.p = at(cell.p);
            cell.v *= r / r0;
        }
        for d in &mut self.drop.drifters {
            d.p = at(d.p);
            (d.v, d.len) = (d.v * r / r0, d.len * r / r0);
        }
        self.drop.um *= r0 / r;
        (self.drop.c, self.drop.r, self.drop.ry) = (c, r, ry);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x as f32, e.y as f32];
        match e.phase {
            Phase::Down => {
                self.fingers.push((e.id, p));
                ctx.haptic();
            }
            Phase::Move => {
                if let Some(f) = self.fingers.iter_mut().find(|f| f.0 == e.id) {
                    f.1 = p;
                }
            }
            _ => self.fingers.retain(|f| f.0 != e.id),
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = (dt as f32).min(0.1);
        let want = Self::phone_field(ctx);
        let k = 1.0 - (-dt / SETTLE).exp();
        self.b = [
            self.b[0] + (want[0] - self.b[0]) * k,
            self.b[1] + (want[1] - self.b[1]) * k,
        ];
        let poles: Vec<[f32; 2]> = self.fingers.iter().map(|f| f.1).collect();
        // two half steps: a strong magnet turns a cell fast
        for _ in 0..2 {
            self.drop.step(dt * 0.5, self.b, &poles);
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let k = ctx.ppp as f32;
        let d = &self.drop;
        let m = (self.b[0] * self.b[0] + self.b[1] * self.b[1]).sqrt();
        let params = BackdropParams {
            time: self.t as f32,
            scale: 1.0,
            center: [0.0, 0.0],
            p: [
                d.c[0] * k,
                d.c[1] * k,
                d.r * k,
                k,
                (m / 60.0).min(1.0),
                d.ry * k,
                0.0,
                0.0,
            ],
        };
        let swim = d.on[1];
        // the protists first, behind: big, dim, out of the field's reach
        for f in &d.drifters {
            let (dx, dy) = (f.a.cos(), f.a.sin());
            let h = f.len * 0.5 * k;
            let a = [f.p[0] * k - dx * h * 0.6, f.p[1] * k - dy * h * 0.6];
            let b = [f.p[0] * k + dx * h * 0.6, f.p[1] * k + dy * h * 0.6];
            self.painter
                .glow(a, b, h * 0.35, 4.0 * k, [0.40, 0.55, 0.60, 0.1]);
            // its edge: an oval outline, the light a dark field catches on a cell wall
            let (nx, ny) = (-dy, dx);
            let edge = |j: usize| {
                let t = j as f32 / 24.0 * std::f32::consts::TAU;
                let (u, v) = (t.cos() * h * 0.8, t.sin() * h * 0.4);
                [f.p[0] * k + dx * u + nx * v, f.p[1] * k + dy * u + ny * v]
            };
            for j in 0..24 {
                self.painter.glow(
                    edge(j),
                    edge(j + 1),
                    0.4 * k,
                    2.5 * k,
                    [0.6, 0.75, 0.8, 0.2],
                );
            }
        }
        for c in &d.cells {
            let wob = 0.35 * c.phase.sin();
            let (dx, dy) = ((c.a + wob).cos(), (c.a + wob).sin());
            let head = [c.p[0] * k, c.p[1] * k];
            // out of the plane of focus a cell is a dimmer, softer blur
            let soft = (0.9 + 8.0 * c.blur) * k;
            let g = c.glint * (1.0 - 0.85 * c.blur);
            if swim {
                // the streak a long exposure would leave: a quarter second of its swim
                let len = c.v * 0.25 * k;
                let tail = [head[0] - dx * len, head[1] - dy * len];
                self.painter.glow(
                    tail,
                    head,
                    0.45 * k,
                    soft * 1.3,
                    [0.45, 0.72, 1.0, 0.45 * g],
                );
            }
            if c.spiral {
                // a spirillum: a long corkscrew, seen as a wave
                let (nx, ny) = (-dy, dx);
                let mut prev = head;
                for j in 1..=6 {
                    let u = j as f32 / 6.0;
                    let w = (u * 9.0 + c.phase * 2.0).sin() * 0.8 * k;
                    let p = [
                        head[0] - dx * u * 7.0 * k + nx * w,
                        head[1] - dy * u * 7.0 * k + ny * w,
                    ];
                    self.painter
                        .glow(prev, p, 0.35 * k, soft, [0.8, 0.92, 1.0, 0.7 * g]);
                    prev = p;
                }
            } else {
                // a coccus: a short rod, its magnetite chain scattering bright
                // out of focus it blurs to a round smudge, not a dash
                let rod = 0.9 * k * (1.0 - c.blur).max(0.0);
                self.painter.glow(
                    [head[0] - dx * rod, head[1] - dy * rod],
                    [head[0] + dx * rod, head[1] + dy * rod],
                    0.5 * k,
                    soft,
                    [0.82, 0.93, 1.0, 0.85 * g],
                );
            }
        }
        // the finger's magnet: a warm soft glow where the pole is held over the drop
        for f in &self.fingers {
            let p = [f.1[0] * k, f.1[1] * k];
            self.painter
                .glow(p, p, 18.0 * k, 26.0 * k, [1.0, 0.7, 0.45, 0.07]);
        }
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.backdrop
            .render(&ctx.gpu, &mut enc, target, size, &params);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let d = &self.drop;
        vec![
            ("centre".into(), [d.c[0] as f64, d.c[1] as f64]),
            (
                "south-edge".into(),
                [d.c[0] as f64, (d.c[1] + d.ry * 0.8) as f64],
            ),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let d = &self.drop;
        let at = [
            d.c[0] as f64 + d.r as f64 * 0.3,
            d.c[1] as f64 + d.ry as f64 * 0.55,
        ];
        Some(info::Demo::new(
            "Marsh bacteria steer north. Your finger is a magnet.",
            vec![info::Stroke::hold(0.4, 2.4, at)],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "In 1975 at Woods Hole, Richard Blakemore saw bacteria from marsh mud all swim one way in a drop.",
                "Each carries a chain of magnetite, a tiny compass needle, so the Earth's field turns it as it swims.",
                "Turn round and they follow north. A finger, or a real magnet held to the phone, pulls them in.",
            ],
            "",
        ))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&RULES, &self.drop.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(o) = self.drop.on.get_mut(i) {
            *o = on;
        }
    }
}

toy_kit::ios_shell!(MagnetMarsh);
