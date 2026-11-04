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
    /// Where every car was, every SAMPLE seconds, and whether its brake lights were on: drawn inside
    /// the ring as the paper's time-space diagram wrapped round, the past spiralling inward.
    hist: std::collections::VecDeque<[(f32, f32); N]>,
    next_sample: f64,
    /// Until the first touch, one car breathes a soft halo: the thing to touch.
    touched: bool,
    invite: Option<usize>,
    still: bool,
    /// When the toy opened (the shell's clock).
    born: f64,
}

const SAMPLE: f64 = 0.2;
/// Seconds of the past drawn inside the ring, and how far inward a second goes (m).
const PAST: f64 = 40.0;
/// Cars drawn larger than life so they read on a phone (length, width); the road with them.
const LONG: f64 = 1.25;
const WIDE: f64 = 1.45;
pub const ROAD: f64 = 3.0;

struct Layout {
    c: [f64; 2],
    /// points a metre
    s: f64,
}

impl Jam {
    fn layout(&self) -> Layout {
        let [w, h] = self.points;
        let s = (w.min(h * 0.75) * 0.97) / (2.0 * (RADIUS + ROAD + 0.4));
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
        Ok(Jam { ring, ground, painter, sound, points: ctx.points(), t: 0.0, taps: Vec::new(), paint, hist: Default::default(), next_sample: 0.0, touched: false, invite: None, still: ctx.reduce_motion, born: 0.0 })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.points = ctx.points();
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        if e.phase == Phase::Down {
            self.touched = true;
            let reach = 34.0_f64.max(self.layout().s * 4.0);
            if let Some(i) = self.car_at([e.x, e.y], reach) {
                self.tap_car(ctx, i);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.points = ctx.points();
        self.t = t;
        if self.invite.is_none() {
            self.invite = Some(self.car_near_angle(-50.0));
            self.born = t;
        }
        self.ring.step(dt);
        self.taps.retain(|(_, at)| t - at < 2.0);
        while t >= self.next_sample {
            self.next_sample += SAMPLE;
            if self.next_sample < t - 1.0 {
                self.next_sample = t;
            }
            let now: [(f32, f32); N] = std::array::from_fn(|i| (self.ring.cars[i].x as f32, self.ring.cars[i].lights as f32));
            self.hist.push_front(now);
            self.hist.truncate((PAST / SAMPLE) as usize + 1);
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
            p: [(l.c[0] * k) as f32, (l.c[1] * k) as f32, RADIUS as f32, ROAD as f32, 0.0, 0.0, 0.0, 0.0],
        };
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        self.ground.render(gpu, &mut enc, target, size, &params);

        // where each car is, and its corners
        let at = |p: [f64; 2], h: [f64; 2], fwd: f64, side: f64| -> [f32; 2] {
            // side: + to the car's right (screen: heading turned clockwise); drawn larger than life
            let (fwd, side) = (fwd * LONG, side * WIDE);
            let r = [-h[1], h[0]];
            px([p[0] + l.s * (h[0] * fwd + r[0] * side), p[1] + l.s * (h[1] * fwd + r[1] * side)])
        };
        let placed: Vec<([f64; 2], [f64; 2])> = self.ring.cars.iter().map(|c| self.place(&l, c.x)).collect();
        let half = CAR / 2.0;

        // the past, inside the ring: each car's track drawn inward as it ages (the paper's
        // time-space diagram wrapped round), with a red glow where its brake lights were on; the cars'
        // tracks wind forward, and a jam shows as one red arm winding backward against them
        let r_in = RADIUS - ROAD - 1.6;
        let r_min = 6.0;
        let inward = (r_in - r_min) / PAST;
        let polar = |x: f32, age: f64| -> [f32; 2] {
            let th = x as f64 / RADIUS;
            let r = r_in - inward * age;
            px([l.c[0] + l.s * r * th.cos(), l.c[1] - l.s * r * th.sin()])
        };
        let ns = self.hist.len();
        for j in 0..ns.saturating_sub(1) {
            let (a, b) = (&self.hist[j], &self.hist[j + 1]);
            let age = j as f64 * SAMPLE + (self.t - (self.next_sample - SAMPLE)).clamp(0.0, SAMPLE);
            let fade = (1.0 - age / PAST).clamp(0.0, 1.0) as f32;
            let fade = fade * fade.min(1.0);
            for i in 0..N {
                let red = 0.5 * (a[i].1 + b[i].1);
                let p = polar(a[i].0, age);
                let q = polar(b[i].0, age + SAMPLE);
                // each car's own track, faint, winding forward
                self.painter.glow(p, q, 0.07 * m, 0.10 * m, [0.70, 0.78, 0.95, 0.10 * fade]);
                // the brake light's red, a soft wide blob, so the red of many cars over time merges
                // into one arm (the jam), not into each car's track
                if red > 0.15 && j % 2 == 0 {
                    // spread along the ring by about a car's spacing, so neighbours merge
                    let (u, v) = (polar(a[i].0 - 2.8, age), polar(a[i].0 + 2.8, age));
                    self.painter.glow(u, v, 0.7 * m, 1.4 * m, [1.0, 0.09, 0.05, 0.055 * red * fade]);
                }
            }
        }
        // light on the ground: headlights ahead, the red of brake lights behind
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (p, h) = placed[i];
            self.painter.glow(at(p, h, half + 1.0, 0.0), at(p, h, half + 7.0, 0.0), 1.5 * m, 2.6 * m, [1.0, 0.86, 0.62, 0.07]);
            let red = c.lights as f32;
            if red > 0.01 {
                self.painter.glow(at(p, h, -half - 0.6, 0.0), at(p, h, -half - 2.2, 0.0), 1.6 * m, 3.0 * m, [1.0, 0.08, 0.05, 0.55 * red]);
            }
        }
        self.painter.render(gpu, &mut enc, target, size, None);

        // the cars
        let wd = WIDE as f32;
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (p, h) = placed[i];
            let col = PAINT[self.paint[i]];
            let dim = |c: [f32; 3], f: f32, a: f32| [c[0] * f, c[1] * f, c[2] * f, a];
            // shadow (the last of the sky's light, from above and a little west)
            let sh = |q: [f32; 2]| [q[0] + 0.25 * m, q[1] + 0.35 * m];
            self.painter.line(sh(at(p, h, -1.75, 0.0)), sh(at(p, h, 1.75, 0.0)), 0.95 * m * wd, 0.55 * m, [0.0, 0.0, 0.02, 0.5]);
            // body: two strokes side by side make a rounded box
            for side in [-0.36, 0.36] {
                self.painter.line(at(p, h, -1.72, side), at(p, h, 1.72, side), 0.48 * m * wd, 0.6, dim(col, 0.80, 1.0));
            }
            // the roof, a little lighter (it faces the sky), between dark glass
            self.painter.line(at(p, h, -0.95, 0.0), at(p, h, 0.45, 0.0), 0.56 * m * wd, 0.08 * m, dim(col, 0.92, 1.0));
            self.painter.line(at(p, h, 0.86, -0.50), at(p, h, 0.86, 0.50), 0.24 * m * wd, 0.6, [0.05, 0.07, 0.10, 0.95]);
            self.painter.line(at(p, h, -1.30, -0.46), at(p, h, -1.30, 0.46), 0.17 * m * wd, 0.6, [0.05, 0.07, 0.10, 0.9]);
            // the lamps, unlit
            for side in [-0.6, 0.6] {
                self.painter.disc(at(p, h, -2.08, side), 0.17 * m * wd, 0.6, [0.30, 0.04, 0.04, 1.0]);
                self.painter.disc(at(p, h, 2.10, side), 0.16 * m * wd, 0.6, [0.85, 0.84, 0.78, 1.0]);
            }
            let _ = c;
        }
        // lamps alight
        let wd = WIDE as f32;
        for (i, c) in self.ring.cars.iter().enumerate() {
            let (p, h) = placed[i];
            for side in [-0.6, 0.6] {
                self.painter.glow(at(p, h, 2.15, side), at(p, h, 2.15, side), 0.12 * m, 0.35 * m, [1.0, 0.92, 0.75, 0.45]);
            }
            let red = c.lights as f32;
            if red > 0.01 {
                for side in [-0.6, 0.6] {
                    let q = at(p, h, -2.1, side);
                    self.painter.glow(q, q, 0.22 * m * wd, 0.25 * m, [1.0, 0.30, 0.18, 1.0 * red]);
                    self.painter.glow(q, q, 0.4 * m, 2.2 * m, [1.0, 0.07, 0.04, 0.55 * red]);
                }
            }
        }
        // the invitation: until the first touch, one car breathes a soft halo (after a moment's
        // first look at the even flow)
        let since = self.t - self.born;
        if let (false, Some(i), true) = (self.touched, self.invite, since > 1.2) {
            let (p, _) = placed[i];
            let come = ((since - 1.2) / 0.6).min(1.0) as f32;
            let breathe = if self.still { 0.8 } else { 0.55 + 0.45 * (0.5 + 0.5 * ((since - 1.2) * 2.4).sin()) as f32 };
            let r = (4.4 + 0.5 * breathe) * m;
            self.painter.glow_ring(px(p), r, 0.22 * m, 0.35 * m, [1.0, 0.97, 0.90, 0.55 * come * breathe]);
            self.painter.glow(px(p), px(p), r * 0.8, r * 0.6, [1.0, 0.95, 0.85, 0.06 * come * breathe]);
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

    fn settings(&mut self, ctx: &mut Ctx) {
        self.still = ctx.reduce_motion;
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
        // the moving car farthest round the ring from any braking one (for a second jam of its own)
        let lit: Vec<f64> = self.ring.cars.iter().filter(|c| c.lights > 0.3).map(|c| c.x).collect();
        let clear = self
            .ring
            .cars
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let d = |x: f64| lit.iter().map(|&y| (x - y).rem_euclid(traffic::RING).min((y - x).rem_euclid(traffic::RING))).fold(traffic::RING, f64::min);
                d(a.x).total_cmp(&d(b.x))
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        a.push(("clear".into(), self.place(&l, self.ring.cars[clear].x).0));
        if let Some(i) = self.invite {
            a.push(("invited".into(), self.place(&l, self.ring.cars[i].x).0));
        }
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
                "Inside the ring, the last 40 seconds wind inward: each car's track, red where it braked.",
            ],
            "Real: the 230 m ring and its 22 cars in Nagoya, the even 30 km/h, the jam's 20 km/h backward. Drawn: the hour, the cars (larger than life), the ground.",
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
  // the ring's asphalt (drawn wider than the real lane, as the cars are drawn larger)
  let d = abs(r - R);
  let road = 1.0 - smoothstep(u.p0.w, u.p0.w + 1.5 * pxm, d);
  var asphalt = vec3<f32>(0.085, 0.092, 0.112) * (0.92 + 0.16 * fbm(m * 1.7));
  // the tyres' track, a little darker and smoother, either side of the middle
  let track = exp(-pow((d - 1.0) / 0.5, 2.0));
  asphalt = asphalt * (1.0 - 0.18 * track);
  col = mix(col, asphalt, road);
  // the painted edges, worn
  let paint = (1.0 - smoothstep(0.06, 0.06 + 1.2 * pxm, abs(d - u.p0.w + 0.15))) * (0.55 + 0.45 * smoothstep(0.25, 0.65, fbm(m * 2.3)));
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
