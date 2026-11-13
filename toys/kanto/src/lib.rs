//! Kanto (a working title; painting-company row 217's family, made local): the slime mold experiment
//! of Tero et al. (2010). A plate cut to the Kanto plain, oat flakes laid at its towns, the mountains
//! lit (the mold shies from light); the yellow plasmodium put down at Tokyo spreads over the agar as a
//! sheet, finds the flakes, and thins to a network of tubes between them (net.rs, the authors' own
//! model). When it has settled, the region's real railways come up faintly under it, as the paper laid
//! them side by side. A tap lays another flake, and the network reroutes to take it in.

pub mod net;

use net::{Net, Tube};
use serde::{Deserialize, Serialize};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::gpu::textures::{decode, Texture};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};

/// Where the toy is: Tokyo, where the mold was put down (lon, lat).
pub const PLACE: (f64, f64) = (139.6917, 35.6895);

/// The mold's colour: Physarum polycephalum's plasmodium is egg-yolk yellow.
const YOLK: [f32; 3] = [1.0, 0.80, 0.16];
/// The mesh's spacing (plate cells, a cell about 220 m).
const SPACING: f32 = 14.0;
/// Tero et al.'s time a step, and the steps a second of ours.
const DT: f64 = 0.05;
const STEPS_A_SECOND: f64 = 70.0;
/// The shuttle streaming's period as drawn (s; the real mold's is a minute or two).
const PULSE_S: f32 = 6.0;
/// The most flakes a finger may add; past it, the oldest is eaten up.
const MAX_FED: usize = 8;
/// How long the network takes to settle (s): the railways come up after it, and again this long
/// (less a little) after a flake is laid.
const SETTLED_S: f64 = 115.0;

#[derive(Deserialize)]
struct Towns {
    plate: Plate,
    towns: Vec<Town>,
    rails: Vec<Vec<[f32; 2]>>,
}

#[derive(Deserialize, Clone, Copy)]
struct Plate {
    west: f64,
    east: f64,
    south: f64,
    north: f64,
    w: usize,
    h: usize,
}

#[derive(Deserialize)]
struct Town {
    cell: [f32; 2],
}

#[derive(Serialize, Deserialize)]
struct Saved {
    t: f64,
}

pub struct Kanto {
    backdrop: Backdrop,
    _plate_tex: Texture,
    painter: Painter,
    net: Net,
    size: [usize; 2],
    bounds: Plate,
    /// The last frame's view, in points (for `project`).
    seen: Option<(View, f32)>,
    /// The flakes a finger laid (nodes), oldest first.
    fed: Vec<u32>,
    t: f64,
    /// When a finger last laid a flake.
    last_feed: f64,
    /// The model steps owed (a fraction carried from frame to frame).
    owed: f64,
    /// The railways' showing, 0..1, and their lines (cells).
    rails: f32,
    lines: Vec<Vec<[f32; 2]>>,
    /// Where each node is drawn: eased toward its thick tubes' other ends, so a tube's run reads as
    /// one smooth tube, not the mesh's zigzag.
    shown: Vec<[f32; 2]>,
    soft: bool,
}

/// Where the plate lies on the screen: its scale (px a cell) and its top-left corner (px).
struct View {
    k: f32,
    off: [f32; 2],
}

impl View {
    fn of(ctx: &Ctx, size: [usize; 2]) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        // the whole plate, as large as the screen takes it
        let k = (w / size[0] as f32).min(h / size[1] as f32);
        let off = [
            0.5 * (w - k * size[0] as f32),
            0.5 * (h - k * size[1] as f32),
        ];
        View { k, off }
    }

    fn px(&self, c: [f32; 2]) -> [f32; 2] {
        [self.off[0] + c[0] * self.k, self.off[1] + c[1] * self.k]
    }

    fn cell(&self, px: [f32; 2]) -> [f32; 2] {
        [
            (px[0] - self.off[0]) / self.k,
            (px[1] - self.off[1]) / self.k,
        ]
    }
}

impl Kanto {
    fn view(&self, ctx: &Ctx) -> View {
        View::of(ctx, self.size)
    }

    /// The tubes: the sheet where it has just come, then each tube by its conductivity.
    fn draw_mold(&mut self, v: &View, ppp: f32) {
        let p = &mut self.painter;
        let t = self.t;
        // the advancing sheet: a soft yellow over the nodes it has just reached, thinning behind
        for n in &self.net.nodes {
            let age = t - n.reach;
            if !(0.0..8.0).contains(&age) {
                continue;
            }
            // the sheet goes thin on lit ground, which it crosses only where it must
            let dark = (1.0 - n.light / 0.6).clamp(0.0, 1.0) as f64;
            let a = (age / 0.8).min(1.0) * (-age / 3.0).exp() * dark * dark;
            let r = SPACING * v.k * 0.85;
            p.disc(
                v.px(n.at),
                r,
                r,
                [YOLK[0], YOLK[1] * 0.92, YOLK[2], 0.16 * a as f32],
            );
        }
        smooth(&self.net, &mut self.shown);
        let net = &self.net;
        let shown = &self.shown;
        let still = self.soft;
        // widths are relative to the thickest tube now: the network's own hierarchy, trunk to vein
        let dmax = net.tubes.iter().map(|t| t.d).fold(0.05f32, f32::max);
        let draw = |p: &mut Painter, tube: &Tube| {
            if tube.d <= 0.0 {
                return;
            }
            let (a, b) = (shown[tube.a as usize], shown[tube.b as usize]);
            let since = t - net.nodes[tube.a as usize]
                .reach
                .max(net.nodes[tube.b as usize].reach);
            let fresh = (since / 1.5).clamp(0.0, 1.0) as f32;
            let rel = tube.d / dmax;
            let s = rel.sqrt().min(1.0);
            let u = ((rel - 0.01) / 0.15).clamp(0.0, 1.0);
            let alpha = u * u * (3.0 - 2.0 * u) * fresh;
            if alpha <= 0.0 {
                return;
            }
            // the shuttle streaming: a slow wave of the protoplasm along the tubes, swelling them
            let m = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
            let wave = if still {
                0.0
            } else {
                ((m[0] * 0.8 + m[1]) * 0.03 - t as f32 * std::f32::consts::TAU / PULSE_S).sin()
            };
            let wid = (0.3 + 3.4 * s.powf(1.6)) * (1.0 + 0.1 * wave) * ppp;
            // thin veins dim, the trunks deep yolk with a lit crest; drawn opaque over the dark agar
            // (its colour dimmed in place of see-through) so joints don't pile up into beads
            let deep = (0.72 + 0.2 * s + 0.08 * wave) * (0.4 + 0.55 * s) * alpha;
            let base = [YOLK[0] * deep, YOLK[1] * deep, YOLK[2] * deep];
            let (pa, pb) = (v.px(a), v.px(b));
            p.line(pa, pb, wid, 0.6 * ppp, [base[0], base[1], base[2], 1.0]);
            if s > 0.6 {
                let c = (s - 0.6) / 0.4 * 0.55 * alpha * (0.8 + 0.2 * wave);
                let lit = [1.0, 0.93, 0.62];
                let mix = |k: usize| base[k] + (lit[k] - base[k]) * c;
                p.line(pa, pb, wid * 0.35, 0.6 * ppp, [mix(0), mix(1), mix(2), 1.0]);
            }
        };
        for tube in &net.tubes {
            draw(p, tube);
        }
    }

    /// The real railways over the mold, thin and pale, as the paper laid them side by side.
    fn draw_rails(&mut self, v: &View, ppp: f32) {
        if self.rails < 0.01 {
            return;
        }
        let a = self.rails;
        for line in &self.lines {
            for w in line.windows(2) {
                self.painter.line(
                    v.px(w[0]),
                    v.px(w[1]),
                    0.55 * ppp,
                    0.5 * ppp,
                    [0.92, 0.30, 0.26, 0.85 * a],
                );
            }
        }
    }

    /// The oat flakes: small pale heaps, a little uneven.
    fn draw_flakes(&mut self, v: &View, ppp: f32) {
        let p = &mut self.painter;
        for (i, &f) in self.net.food.iter().enumerate() {
            let c = v.px(self.net.nodes[f as usize].at);
            let mut s = (i as u64 + 7) * 0x9e37_79b9;
            for _ in 0..4 {
                let dx = (net::rand(&mut s) as f32 - 0.5) * 3.2 * ppp;
                let dy = (net::rand(&mut s) as f32 - 0.5) * 2.4 * ppp;
                let r = (1.6 + net::rand(&mut s) as f32 * 1.2) * ppp;
                p.disc(
                    [c[0] + dx, c[1] + dy],
                    r,
                    0.8 * ppp,
                    [0.86, 0.80, 0.66, 0.95],
                );
            }
        }
    }

    fn feed(&mut self, ctx: &mut Ctx, px: [f32; 2]) {
        let v = self.view(ctx);
        let c = v.cell(px);
        if c[0] < 0.0 || c[1] < 0.0 || c[0] >= self.size[0] as f32 || c[1] >= self.size[1] as f32 {
            return;
        }
        let Some(n) = self.net.nearest(c) else {
            return;
        };
        let at = self.net.nodes[n as usize].at;
        if (at[0] - c[0]).hypot(at[1] - c[1]) > SPACING * 2.0 || self.net.food.contains(&n) {
            return;
        }
        if self.fed.len() >= MAX_FED {
            let old = self.fed.remove(0);
            self.net.food.retain(|&f| f != old);
        }
        self.fed.push(n);
        self.last_feed = self.t;
        self.net.food.push(n);
        ctx.haptic();
    }
}

const PLATE_WGSL: &str = r#"
// u.p0 = [the plate's left, top (px), its px a cell, 0]; u.p1 = [cells across, down]
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let uv = (px - u.p0.xy) / (u.p0.z * u.p1.xy);
  // the bench under the plate
  var col = vec3(0.020, 0.022, 0.028) + 0.01 * noise2(px * 0.02);
  if (all(uv >= vec2(0.0)) && all(uv <= vec2(1.0))) {
    let s = textureSampleLevel(tex, samp, uv, 0.0);
    let agar = smoothstep(0.35, 0.65, s.g);
    // the agar: a dim amber gel, a little uneven
    let gel = vec3(0.085, 0.070, 0.050) * (0.9 + 0.2 * noise2(px * 0.05));
    // the mountains, lit: a cool light on the gel
    let lit = mix(gel, vec3(0.20, 0.22, 0.25), s.r * 0.8);
    var plate = lit;
    col = mix(col, plate, agar);
  }
  return vec4(col, 1.0);
}
"#;

/// Each node's drawn place: its own, drawn a little toward the far ends of its thick tubes (a flake
/// stays where it lies). Recomputed from the mesh each frame, so it follows the tubes as they change.
fn smooth(net: &Net, shown: &mut Vec<[f32; 2]>) {
    // each node's weight: its thickest tube's; strands one lattice step apart pull together into one
    let weight: Vec<f32> = net
        .around
        .iter()
        .map(|ks| {
            let d = ks.iter().map(|&k| net.tubes[k as usize].d).fold(0.0f32, f32::max);
            (d - 0.05).max(0.0)
        })
        .collect();
    shown.clear();
    shown.extend(net.nodes.iter().map(|n| n.at));
    for _ in 0..2 {
        let last = shown.clone();
        for (i, ks) in net.around.iter().enumerate() {
            if weight[i] <= 0.0 {
                continue;
            }
            let mut s = [last[i][0] * weight[i], last[i][1] * weight[i], weight[i]];
            for &k in ks {
                let tube = &net.tubes[k as usize];
                let j = if tube.a as usize == i { tube.b } else { tube.a } as usize;
                let w = weight[j];
                s = [s[0] + last[j][0] * w, s[1] + last[j][1] * w, s[2] + w];
            }
            let mean = [s[0] / s[2], s[1] / s[2]];
            shown[i] = [
                last[i][0] + (mean[0] - last[i][0]) * 0.6,
                last[i][1] + (mean[1] - last[i][1]) * 0.6,
            ];
        }
    }
    for &f in &net.food {
        shown[f as usize] = net.nodes[f as usize].at;
    }
}

/// The mesh over the plate, the towns' flakes on it, the sheet spreading from Tokyo (the first town).
fn build(plate: &toy_kit::gpu::textures::Pixels, towns: &Towns, spacing: f32, seed: u64) -> Net {
    let (w, h) = (towns.plate.w, towns.plate.h);
    let px = |x: f32, y: f32, c: usize| -> u8 {
        let (x, y) = ((x as usize).min(w - 1), (y as usize).min(h - 1));
        plate.data[(y * w + x) * 4 + c]
    };
    let mut net = Net::new(
        w,
        h,
        spacing,
        |x, y| px(x, y, 1) > 128,
        |x, y| px(x, y, 0) as f32 / 255.0,
        seed,
    );
    for t in &towns.towns {
        if let Some(n) = net.nearest(t.cell) {
            if !net.food.contains(&n) {
                net.nodes[n as usize].at = t.cell;
                net.food.push(n);
            }
        }
    }
    if let Some(&tokyo) = net.food.first() {
        net.spread(tokyo, 0.0);
    }
    net
}

impl Toy for Kanto {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let plate = decode(&ctx.assets.read("plate.png")?, 4)?;
        let mut towns: Towns = serde_json::from_slice(&ctx.assets.read("plate.json")?)?;
        let seed = std::env::var("KANTO_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0x0a7_f1a4e);
        let net = build(&plate, &towns, SPACING, seed);
        let (w, h) = (towns.plate.w, towns.plate.h);
        let tex = Texture::from_pixels(&ctx.gpu, "kanto plate", &plate, false);
        Ok(Kanto {
            backdrop: Backdrop::textured(&ctx.gpu, "kanto", PLATE_WGSL, &tex.view),
            _plate_tex: tex,
            painter: Painter::new(&ctx.gpu),
            net,
            size: [w, h],
            bounds: towns.plate,
            seen: None,
            fed: Vec::new(),
            t: 0.0,
            last_feed: -1e9,
            owed: 0.0,
            rails: 0.0,
            lines: std::mem::take(&mut towns.rails),
            shown: Vec::new(),
            soft: ctx.reduce_motion,
        })
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        if e.phase == Phase::Down {
            let k = ctx.ppp as f32;
            self.feed(ctx, [e.x as f32 * k, e.y as f32 * k]);
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        self.seen = Some((self.view(ctx), ctx.ppp as f32));
        let dt = dt.min(0.1);
        self.t += dt;
        self.owed += dt * STEPS_A_SECOND;
        while self.owed >= 1.0 {
            self.net.adapt(self.t, DT);
            self.owed -= 1.0;
        }
        let settled = self.t > SETTLED_S && self.t - self.last_feed > SETTLED_S * 0.6;
        let want = if settled { 1.0 } else { 0.0 };
        self.rails += (want - self.rails) * (dt as f32 * if settled { 0.25 } else { 1.5 }).min(1.0);
        if std::env::var_os("KANTO_CENSUS").is_some() {
            let thick = self.net.tubes.iter().filter(|t| t.d > 0.1).count();
            println!(
                "t {:.2} steps {} change {:.3} thick {thick} rails {:.2}",
                self.t, self.net.steps, self.net.change, self.rails
            );
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let v = self.view(ctx);
        let ppp = ctx.ppp as f32;
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let params = BackdropParams {
            time: self.t as f32,
            scale: 1.0,
            center: [0.0, 0.0],
            p: [
                v.off[0],
                v.off[1],
                v.k,
                0.0,
                self.size[0] as f32,
                self.size[1] as f32,
                0.0,
                0.0,
            ],
        };
        self.backdrop
            .render(&ctx.gpu, &mut enc, target, size, &params);
        self.draw_flakes(&v, ppp);
        self.draw_mold(&v, ppp);
        self.draw_rails(&v, ppp);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.soft = ctx.reduce_motion;
    }

    fn demo(&self, ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let [w, h] = ctx.points();
        Some(toy_kit::info::Demo::new(
            "Slime mold on the Kanto plain. Tap to feed it.",
            vec![toy_kit::info::Stroke::tap(0.6, [0.30 * w, 0.40 * h])],
        ))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::lines(&[
            "In 2010, biologists laid oat flakes at the towns round Tokyo and let a slime mold grow from Tokyo between them.",
            "Tubes that carry more of its flow thicken; the rest wither. The mountains were lit, and the mold shies from light.",
            "In a day it made a network much like the region's railways. Tap to lay another flake.",
        ]))
    }

    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        let (v, ppp) = self.seen.as_ref()?;
        let b = &self.bounds;
        let c = [
            ((lonlat[0] - b.west) / (b.east - b.west) * b.w as f64) as f32,
            ((b.north - lonlat[1]) / (b.north - b.south) * b.h as f64) as f32,
        ];
        let px = v.px(c);
        Some([(px[0] / ppp) as f64, (px[1] / ppp) as f64])
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved { t: self.t }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, _saved: &str) {}
}

toy_kit::ios_shell!(Kanto);

#[cfg(test)]
mod probe {
    use super::*;

    /// The model alone, timed: `cargo test --release -p kanto -- --ignored --nocapture probe`.
    #[test]
    #[ignore]
    fn timed() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data/");
        let plate = decode(&std::fs::read(format!("{dir}plate.png")).unwrap(), 4).unwrap();
        let towns: Towns =
            serde_json::from_slice(&std::fs::read(format!("{dir}plate.json")).unwrap()).unwrap();
        let spacing: f32 = std::env::var("SP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(SPACING);
        let net = build(&plate, &towns, spacing, 1);
        let mut net = net;
        println!("nodes {} tubes {}", net.nodes.len(), net.tubes.len());
        let t0 = std::time::Instant::now();
        let mut t = 0.0;
        for i in 0..std::env::var("N").ok().and_then(|v| v.parse().ok()).unwrap_or(9000) {
            t += 1.0 / STEPS_A_SECOND;
            net.adapt(t, DT);
            if i % 1500 == 0 {
                let mut ds: Vec<f32> = net
                    .tubes
                    .iter()
                    .map(|t| t.d)
                    .filter(|&d| d > 0.01)
                    .collect();
                ds.sort_by(f32::total_cmp);
                let q = |f: f32| {
                    ds.get((ds.len() as f32 * f) as usize)
                        .copied()
                        .unwrap_or(0.0)
                };
                let dm = net.tubes.iter().map(|t| t.d).fold(0.05f32, f32::max);
                let joined = net
                    .food
                    .iter()
                    .filter(|&&f| {
                        net.around[f as usize]
                            .iter()
                            .any(|&k| net.tubes[k as usize].d > 0.08 * dm)
                    })
                    .count();
                println!("  joined {joined}/{}", net.food.len());
                println!(
                    "  d>0.01 {} q50 {:.3} q90 {:.3} q99 {:.3} max {:.3}",
                    ds.len(),
                    q(0.5),
                    q(0.9),
                    q(0.99),
                    ds.last().copied().unwrap_or(0.0)
                );
                let thick = net.tubes.iter().filter(|t| t.d > 0.1).count();
                println!(
                    "{i} t {t:.1} change {:.3} thick {thick} ms/step {:.3}",
                    net.change,
                    t0.elapsed().as_secs_f64() * 1000.0 / (i + 1) as f64
                );
            }
        }
    }
}
