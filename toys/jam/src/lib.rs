//! Ghost Jam (a working title): the circle at Nagoya where 22 cars made a traffic jam out of nothing
//! (Sugiyama et al. 2008), seen from above at dusk. The cars flow evenly round the ring; tap one and it
//! brakes for a moment, and the cars behind brake harder, until a knot of standing cars forms whose
//! brake lights crawl backward round the ring against the traffic, long after your car has driven on.
//! A second tap makes a second jam; two jams on one ring share the same cars and in time one feeds
//! the other. Everything the cars do is one rule (`traffic.rs`).

mod sound;
pub mod traffic;

use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};
use traffic::{Ring, CAR, N, RADIUS};

/// The cars' paint (sampled from the ordinary small cars of a Japanese car park; drawn).
const PAINT: [[f32; 3]; 8] = [
    [0.86, 0.87, 0.88], // white
    [0.60, 0.62, 0.65], // silver
    [0.09, 0.10, 0.11], // black
    [0.14, 0.20, 0.36], // navy
    [0.30, 0.34, 0.30], // olive grey
    [0.47, 0.60, 0.72], // pale blue
    [0.74, 0.68, 0.56], // champagne
    [0.86, 0.87, 0.88], // white again (the commonest)
];

pub struct Jam {
    ring: Ring,
    ground: Backdrop,
    painter: Painter,
    sound: sound::Engines,
    points: [f64; 2],
    t: f64,
    /// Where taps landed (screen points of the car, when), for the touch's ring.
    taps: Vec<([f64; 2], f64)>,
    paint: [usize; N],
    /// Red light left on the asphalt, by where on the ring (2° bins): the jam's path, fading.
    trail: Vec<f32>,
}

const BINS: usize = 180;
const TRAIL_FADE: f64 = 3.5;

struct Layout {
    c: [f64; 2],
    /// points a metre
    s: f64,
}

impl Jam {
    fn layout(&self) -> Layout {
        let [w, h] = self.points;
        let s = (w.min(h * 0.75) * 0.96) / (2.0 * (RADIUS + 2.6));
        Layout { c: [w * 0.5, h * 0.47], s }
    }

    /// A car's centre on screen (points) and its heading (unit, screen).
    fn place(&self, l: &Layout, x: f64) -> ([f64; 2], [f64; 2]) {
        let th = x / RADIUS;
        let p = [l.c[0] + l.s * RADIUS * th.cos(), l.c[1] - l.s * RADIUS * th.sin()];
        (p, [-th.sin(), -th.cos()])
    }

    fn car_at(&self, p: [f64; 2], reach: f64) -> Option<usize> {
        let l = self.layout();
        let mut best = None;
        let mut bd = reach;
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (q, _) = self.place(&l, c.x);
            let d = (q[0] - p[0]).hypot(q[1] - p[1]);
            if d < bd {
                bd = d;
                best = Some(i);
            }
        }
        best
    }

    /// The car nearest a point on the ring at angle `deg` (0 = right, 90 = top).
    fn car_near_angle(&self, deg: f64) -> usize {
        let l = self.layout();
        let th = deg.to_radians();
        let p = [l.c[0] + l.s * RADIUS * th.cos(), l.c[1] - l.s * RADIUS * th.sin()];
        self.car_at(p, f64::MAX).unwrap_or(0)
    }

    fn tap_car(&mut self, ctx: &mut Ctx, i: usize) {
        self.ring.tap(i);
        let l = self.layout();
        let (p, _) = self.place(&l, self.ring.cars[i].x);
        self.taps.push((p, self.t));
        if self.taps.len() > 8 {
            self.taps.remove(0);
        }
        self.sound.thump();
        ctx.haptic();
    }
}

impl Toy for Jam {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let ground = Backdrop::new(&ctx.gpu, "jam ground", GROUND_WGSL);
        let painter = Painter::new(&ctx.gpu);
        let mut paint = [0; N];
        let mut h: u32 = 0x9e3779b9;
        for p in paint.iter_mut() {
            h ^= h << 13;
            h ^= h >> 17;
            h ^= h << 5;
            *p = (h % PAINT.len() as u32) as usize;
        }
        let ring = Ring::new(0x5eed_2008);
        let sound = sound::Engines::new();
        Ok(Jam { ring, ground, painter, sound, points: ctx.points(), t: 0.0, taps: Vec::new(), paint, trail: vec![0.0; BINS] })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.points = ctx.points();
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        if e.phase == Phase::Down {
            let reach = 34.0_f64.max(self.layout().s * 4.0);
            if let Some(i) = self.car_at([e.x, e.y], reach) {
                self.tap_car(ctx, i);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.points = ctx.points();
        self.t = t;
        self.ring.step(dt);
        self.taps.retain(|(_, at)| t - at < 2.0);
        let fade = (-dt / TRAIL_FADE).exp() as f32;
        for b in self.trail.iter_mut() {
            *b *= fade;
        }
        for c in &self.ring.cars {
            if c.lights > 0.05 {
                let at = (c.x - CAR * 0.6).rem_euclid(traffic::RING) / traffic::RING * BINS as f64;
                let b = at as usize % BINS;
                let add = (c.lights * dt * 0.5) as f32;
                self.trail[b] = (self.trail[b] + add).min(1.0);
            }
        }
        let l = self.layout();
        let cars: Vec<(f32, f32, f32)> = self
            .ring
            .cars
            .iter()
            .map(|c| {
                let (p, _) = self.place(&l, c.x);
                let pan = ((p[0] - l.c[0]) / (l.s * RADIUS)) as f32;
                (c.v as f32, c.acc as f32, pan)
            })
            .collect();
        self.sound.set(&cars);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp;
        let size = [ctx.width, ctx.height];
        let l = self.layout();
        let px = |p: [f64; 2]| [(p[0] * k) as f32, (p[1] * k) as f32];
        let m = (l.s * k) as f32; // pixels a metre
        let params = BackdropParams {
            time: self.t as f32,
            scale: m,
            center: [0.0, 0.0],
            p: [(l.c[0] * k) as f32, (l.c[1] * k) as f32, RADIUS as f32, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        self.ground.render(gpu, &mut enc, target, size, &params);

        // where each car is, and its corners
        let at = |p: [f64; 2], h: [f64; 2], fwd: f64, side: f64| -> [f32; 2] {
            // side: + to the car's right (screen: heading turned clockwise)
            let r = [-h[1], h[0]];
            px([p[0] + l.s * (h[0] * fwd + r[0] * side), p[1] + l.s * (h[1] * fwd + r[1] * side)])
        };
        let placed: Vec<([f64; 2], [f64; 2])> = self.ring.cars.iter().map(|c| self.place(&l, c.x)).collect();
        let half = CAR / 2.0;

        // the jam's path: red light lingering on the asphalt where cars stood braking
        for (b, &v) in self.trail.iter().enumerate() {
            if v > 0.01 {
                let x = (b as f64 + 0.5) / BINS as f64 * traffic::RING;
                let (p, h) = self.place(&l, x);
                let a = at(p, h, -1.0, 0.0);
                let e = at(p, h, 1.0, 0.0);
                self.painter.glow(a, e, 1.4 * m, 1.6 * m, [0.9, 0.06, 0.04, 0.16 * v.sqrt()]);
            }
        }
        // light on the ground: headlights ahead, the red of brake lights behind
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (p, h) = placed[i];
            self.painter.glow(at(p, h, half + 1.0, 0.0), at(p, h, half + 7.0, 0.0), 1.5 * m, 2.6 * m, [1.0, 0.86, 0.62, 0.07]);
            let red = c.lights as f32;
            if red > 0.01 {
                self.painter.glow(at(p, h, -half - 0.6, 0.0), at(p, h, -half - 2.2, 0.0), 1.2 * m, 2.4 * m, [1.0, 0.10, 0.06, 0.30 * red]);
            }
        }
        self.painter.render(gpu, &mut enc, target, size, None);

        // the cars
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (p, h) = placed[i];
            let col = PAINT[self.paint[i]];
            let dim = |c: [f32; 3], f: f32, a: f32| [c[0] * f, c[1] * f, c[2] * f, a];
            // shadow (the last of the sky's light, from above and a little west)
            let sh = |q: [f32; 2]| [q[0] + 0.25 * m, q[1] + 0.35 * m];
            self.painter.line(sh(at(p, h, -1.75, 0.0)), sh(at(p, h, 1.75, 0.0)), 0.95 * m, 0.55 * m, [0.0, 0.0, 0.02, 0.5]);
            // body: two strokes side by side make a rounded box
            for side in [-0.36, 0.36] {
                self.painter.line(at(p, h, -1.72, side), at(p, h, 1.72, side), 0.48 * m, 0.6, dim(col, 0.80, 1.0));
            }
            // the roof, a little lighter (it faces the sky), between dark glass
            self.painter.line(at(p, h, -0.95, 0.0), at(p, h, 0.45, 0.0), 0.56 * m, 0.08 * m, dim(col, 0.92, 1.0));
            self.painter.line(at(p, h, 0.86, -0.50), at(p, h, 0.86, 0.50), 0.24 * m, 0.6, [0.05, 0.07, 0.10, 0.95]);
            self.painter.line(at(p, h, -1.30, -0.46), at(p, h, -1.30, 0.46), 0.17 * m, 0.6, [0.05, 0.07, 0.10, 0.9]);
            // the lamps, unlit
            for side in [-0.6, 0.6] {
                self.painter.disc(at(p, h, -2.08, side), 0.17 * m, 0.6, [0.30, 0.04, 0.04, 1.0]);
                self.painter.disc(at(p, h, 2.10, side), 0.16 * m, 0.6, [0.85, 0.84, 0.78, 1.0]);
            }
            let _ = c;
        }
        // lamps alight
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (p, h) = placed[i];
            for side in [-0.6, 0.6] {
                self.painter.glow(at(p, h, 2.15, side), at(p, h, 2.15, side), 0.12 * m, 0.35 * m, [1.0, 0.92, 0.75, 0.45]);
            }
            let red = c.lights as f32;
            if red > 0.01 {
                for side in [-0.6, 0.6] {
                    let q = at(p, h, -2.1, side);
                    self.painter.glow(q, q, 0.16 * m, 0.25 * m, [1.0, 0.18, 0.10, 0.95 * red]);
                    self.painter.glow(q, q, 0.25 * m, 1.1 * m, [1.0, 0.06, 0.04, 0.35 * red]);
                }
            }
        }
        // a touch: a soft ring from the car
        for &(p, at_t) in &self.taps {
            let a = ((self.t - at_t) / 0.9).clamp(0.0, 1.0) as f32;
            if a < 1.0 {
                self.painter.glow_ring(px(p), (3.0 + 6.0 * a) * m, 0.25 * m, 0.4 * m, [1.0, 0.95, 0.9, 0.5 * (1.0 - a)]);
            }
        }
        self.painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let l = self.layout();
        let mut a: Vec<(String, [f64; 2])> = self.ring.cars.iter().enumerate().map(|(i, c)| (format!("car{i}"), self.place(&l, c.x).0)).collect();
        // the cars at the ring's bottom (nearest the thumb) and top, as they are now
        a.push(("near".into(), self.place(&l, self.ring.cars[self.car_near_angle(-90.0)].x).0));
        a.push(("far".into(), self.place(&l, self.ring.cars[self.car_near_angle(90.0)].x).0));
        a.push(("centre".into(), l.c));
        a
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        // the car coming round the bottom right, where the band will be easy to follow
        let l = self.layout();
        let i = self.car_near_angle(-60.0);
        // aim where the car will be when the ghost finger lands (0.5 s on)
        let c = &self.ring.cars[i];
        let (p, _) = self.place(&l, c.x + c.v * 0.5);
        Some(toy_kit::info::Demo::new("Tap a car: it brakes, and a jam runs backward.", vec![toy_kit::info::Stroke::tap(0.5, p)]))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "Every driver does one thing: keep a safe gap to the car ahead, and speed up when there's room.",
                "Brake once and the car behind brakes a little harder, and the next harder still, until some stop: a jam that moves backward while every car drives forward.",
            ],
            "Real: the 230 m ring and its 22 cars in Nagoya, the even 30 km/h, the jam's 20 km/h backward. Drawn: the hour, the cars, the ground.",
        ))
    }
}

/// The test ground at dusk: flat concrete in slabs, the ring's asphalt with its painted edges, the
/// tyres' polished track, the camera at the centre on its tripod. `p0.xy` the centre (px), `u.scale`
/// pixels a metre, `p0.z` the ring's radius.
const GROUND_WGSL: &str = r#"
fn fbm(p: vec2<f32>) -> f32 {
  var a = 0.0; var w = 0.5; var q = p;
  for (var i = 0; i < 4; i = i + 1) { a = a + w * noise2(q); q = q * 2.03 + vec2<f32>(17.1, 9.3); w = w * 0.5; }
  return a;
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let m = (px - u.p0.xy) / u.scale;          // metres from the ring's centre
  let r = length(m);
  let R = u.p0.z;
  let pxm = 1.0 / u.scale;                   // a pixel, in metres
  // concrete in 6 m slabs, mottled
  var col = vec3<f32>(0.150, 0.165, 0.200);
  col = col * (0.90 + 0.18 * fbm(m * 0.35)) * (0.96 + 0.06 * noise2(m * 3.1));
  let g = abs(fract(m / 6.0 + 0.5) - 0.5) * 6.0;
  let seam = 1.0 - smoothstep(0.02, 0.02 + 1.5 * pxm, min(g.x, g.y));
  col = col * (1.0 - 0.14 * seam);
  // the ring's asphalt, 4.4 m wide
  let d = abs(r - R);
  let road = 1.0 - smoothstep(2.2, 2.2 + 1.5 * pxm, d);
  var asphalt = vec3<f32>(0.085, 0.092, 0.112) * (0.92 + 0.16 * fbm(m * 1.7));
  // the tyres' track, a little darker and smoother, either side of the middle
  let track = exp(-pow((d - 0.75) / 0.35, 2.0));
  asphalt = asphalt * (1.0 - 0.18 * track);
  col = mix(col, asphalt, road);
  // the painted edges, worn
  let paint = (1.0 - smoothstep(0.06, 0.06 + 1.2 * pxm, abs(d - 2.05))) * (0.55 + 0.45 * smoothstep(0.25, 0.65, fbm(m * 2.3)));
  col = mix(col, vec3<f32>(0.62, 0.63, 0.63), paint * 0.6);
  // the camera at the centre: three legs and its head, and their shadow
  var cam = 0.0;
  for (var k = 0; k < 3; k = k + 1) {
    let a = f32(k) * 2.094 + 0.4;
    let e = vec2<f32>(cos(a), sin(a)) * 0.75;
    let h = clamp(dot(m, e) / dot(e, e), 0.0, 1.0);
    cam = max(cam, 1.0 - smoothstep(0.03, 0.03 + 1.2 * pxm, length(m - e * h)));
  }
  let shadow = 1.0 - smoothstep(0.0, 1.2, length(m - vec2<f32>(0.9, 1.3)) - 0.15);
  col = col * (1.0 - 0.25 * shadow);
  col = mix(col, vec3<f32>(0.05, 0.05, 0.06), cam);
  col = mix(col, vec3<f32>(0.04, 0.04, 0.05), 1.0 - smoothstep(0.16, 0.16 + 1.2 * pxm, r));
  col = mix(col, vec3<f32>(0.30, 0.34, 0.40), 1.0 - smoothstep(0.05, 0.05 + 1.2 * pxm, length(m - vec2<f32>(-0.05, -0.05))));
  // dusk: the sky's last light from the top of the screen, falling off to the edges
  let uv = px / u.size;
  col = col * (1.06 - 0.22 * uv.y) * vec3<f32>(0.92, 0.97, 1.08);
  let v = length((uv - vec2<f32>(0.5, 0.47)) * vec2<f32>(1.0, u.size.y / u.size.x));
  col = col * (1.0 - 0.45 * smoothstep(0.35, 1.05, v));
  return vec4<f32>(col, 1.0);
}
"#;

toy_kit::ios_shell!(Jam);
