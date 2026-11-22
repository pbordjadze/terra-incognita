//! Magnet Marsh (a working title): a drop of water from the salt marsh at Woods Hole, Massachusetts, under a
//! dark-field microscope, where Richard Blakemore saw in 1975 that some bacteria all swim one way: each
//! carries a chain of magnetite, a compass needle, and the Earth's field steers it (`drop.rs`). The field
//! here is the one the phone feels (`Ctx::sensors`): turn round and the swarm wheels to keep north; hold a
//! magnet to the phone and they pour toward it. A finger is a magnet's pole held over the drop. Without a
//! compass (a desktop browser) north is the top of the screen.

pub mod drop;
pub mod plot;
pub mod sc4020;

use drop::{Drop, RULES};
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
    painter: Painter,
    /// The SC 4020's film, and the silt it plots on the glass.
    film: toy_kit::film::Film,
    silt: Vec<[f32; 2]>,
    /// Each plotted cell's last places, one a frame.
    trails: Vec<Vec<[f32; 2]>>,
    /// The film frame (in threes) whose places were last added to the paths.
    laid: Option<i64>,
    reduce_motion: bool,
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
        let drop = Drop::new(c, r, ry, CELLS, b, SEED);
        Ok(MagnetMarsh {
            silt: plot::silt(&drop),
            trails: Vec::new(),
            laid: None,
            drop,
            painter: Painter::new(&ctx.gpu),
            film: sc4020::new(&ctx.gpu, [ctx.width, ctx.height]),
            reduce_motion: ctx.reduce_motion,
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
        self.silt = plot::silt(&self.drop);
        self.trails.clear();
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
        // each plotted cell's path gains a point every third film frame, whether or not a frame is drawn
        let frame = (t * sc4020::FRAMES_HZ).floor() as i64 / plot::EVERY;
        if self.laid != Some(frame) {
            self.laid = Some(frame);
            plot::lay(&mut self.trails, &self.drop);
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let size = [ctx.width, ctx.height];
        self.film.resize(gpu, size);
        self.reduce_motion = ctx.reduce_motion;
        // a frame plotted and photographed 24 times a second
        let Some(f) = self.film.next(gpu, target, self.t) else {
            return;
        };
        let fingers: Vec<[f32; 2]> = self.fingers.iter().map(|f| f.1).collect();
        plot::plot(
            &mut self.painter,
            &self.drop,
            &self.trails,
            &self.silt,
            &fingers,
            ctx.ppp as f32,
        );
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let scene = &self.film.scene.clone();
        self.painter
            .render(gpu, &mut enc, scene, size, Some([0.0, 0.0, 0.0, 1.0]));
        self.film
            .read(gpu, &mut enc, f, sc4020::values(self.reduce_motion));
        self.film.film(&mut enc, target);
        gpu.queue.submit([enc.finish()]);
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

    fn style(&self) -> Option<info::Style> {
        Some(info::Style {
            name: "Force, Mass and Motion",
            maker: "Frank W. Sinden",
            place: "Bell Labs, New Jersey",
            year: 1965,
        })
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
