//! Battle Ink (a working title, painting-company row 206): Cannae, 216 BC, on its real ground, as a
//! copperplate engraving come alive. Each army is thousands of tapered burin cuts, one a small body of
//! men, combed along the army's motion: Rome in iron-gall blue-black, Carthage in Tyrian purple. A
//! finger's line is an order in ink: Carthaginian men in its reach ride it the way it was drawn and,
//! where it ends, hold and then find the fight (the ink-orders spike's rules, sim.rs). Where the two
//! inks meet they fold into marbled chevrons instead of greying; the paper keeps a stain of where each
//! current ran and the cuts of the fallen stay engraved, so the page ends as a print of the day.
//!
//! The ground is Cannae's relief from Terrarium elevation, the Aufidus traced along the valley floor
//! (tools/toys/battle/build.py); the real day's movements lie under it as a faint graphite ghost.
//! Simulation in `update` on the CPU (3,000 units); the drawing on the GPU (battle.wgsl).

mod sim;

use sim::{Sim, DEAD, FLEEING, FORMED, GONE, KINDS};
use toy_kit::{Ctx, Phase, PointerEvent, Toy};

/// The battle's box (world units) that the view keeps whole on any screen.
const BOX_C: [f32; 2] = [530.0, 455.0];
const BOX: [f32; 2] = [600.0, 560.0];
/// Sim seconds a second, and while a finger draws (the day slows under the pen).
const PACE: f32 = 1.6;
const PACE_DRAWING: f32 = 0.6;
const STEP: f32 = 1.0 / 30.0;
/// The day begins at the first touch, or after this long untouched.
const IDLE_START: f64 = 14.0;
/// The world sheets (density, intent): a cell of this many units.
const DCELL: f32 = 4.0;
const ICELL: f32 = 8.0;
/// The stain sheet: units a texel.
const SCELL: f32 = 2.0;
const MAX_SEGS: usize = 8192;
/// Seconds from the day's decision to the still page.
const SETTLE: f64 = 6.0;
/// Room a man takes in the pressed pocket, and in the ring that closes round it (units²).
const CORE_AREA: f32 = 9.0;
const RING_AREA: f32 = 21.0;
/// The pen combs the cuts this near its passage (units), for this long (s).
const COMB_R: f32 = 56.0;
const COMB_LIFE: f32 = 3.0;
/// How near the pen's fresh passage Hannibal's men are raked along it (units).
const RAKE_R: f32 = 30.0;
const MAX_DOTS: usize = 8192;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    screen: [f32; 4],
    off: [f32; 4],
    world: [f32; 4],
    ground: [f32; 4],
    misc: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Cut {
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Seg {
    ab: [f32; 4],
    sw: [f32; 4],
    ka: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Dot {
    pr: [f32; 4],
    v: [f32; 4],
}

#[derive(serde::Deserialize)]
struct GroundMeta {
    x0: f32,
    y0: f32,
    cell: f32,
    w: u32,
    h: u32,
    river: Vec<[f32; 2]>,
}

struct Touch {
    id: u64,
    x: f64,
    y: f64,
    seen: f64,
    moved: bool,
    stroke: Option<usize>,
    reset: bool,
}

/// What the drawing remembers of each unit: the cut's direction (eased), its swell's phase.
#[derive(Clone, Copy, Default)]
struct Look {
    dx: f32,
    dy: f32,
    curv: f32,
    ph: f32,
}

pub struct Battle {
    sim: Sim,
    river: Vec<[f32; 2]>,
    looks: Vec<Look>,
    acc: f32,
    clock: f64,
    started_at: Option<f64>,
    touches: Vec<Touch>,
    drops: Vec<(f32, f32, f64)>,
    /// the pen's fresh passage (x, y, tangent, when): the cuts near it comb along it at once,
    /// before the men have taken a step
    comb: Vec<[f32; 5]>,
    ghost: Vec<Seg>,
    page_t: f64,
    // world sheets on the CPU
    dw: usize,
    dh: usize,
    dens: [Vec<f32>; 2],
    dens_rgba: Vec<u8>,
    iw: usize,
    ih: usize,
    intent: [Vec<V3>; 2],
    tmp: Vec<f32>,
    tmp3: Vec<V3>,
    /// once a side has no man standing: where it fell (its fallen's middle), per side
    fallen_at: [Option<[f32; 2]>; 2],
    decided_at: Option<f64>,
    /// the victors' re-forming round the pocket: unit, from, to
    ring: Vec<(usize, f32, f32, f32, f32)>,
    initial: [u32; 2],
    dead: Vec<f32>,
    deaths_drawn: usize,
    pending_dots: Vec<Dot>,
    clear_stain: bool,
    // GPU
    uniform: wgpu::Buffer,
    cuts: wgpu::Buffer,
    segs: wgpu::Buffer,
    dots: wgpu::Buffer,
    n_cuts: u32,
    n_segs: u32,
    n_dots: u32,
    ground_tex: wgpu::Texture,
    dens_tex: wgpu::Texture,
    stain_tex: wgpu::Texture,
    ink_tex: wgpu::Texture,
    ground_rect: [f32; 4],
    sampler: wgpu::Sampler,
    u_bind: wgpu::BindGroup,
    page_layout: wgpu::BindGroupLayout,
    page_bind: wgpu::BindGroup,
    p_cut: wgpu::RenderPipeline,
    p_seg: wgpu::RenderPipeline,
    p_dot: wgpu::RenderPipeline,
    p_page: wgpu::RenderPipeline,
}

fn f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let e = ((x >> 23) & 0xff) as i32 - 127 + 15;
    let m = x & 0x7f_ffff;
    if e <= 0 {
        return sign;
    }
    if e >= 31 {
        return sign | 0x7c00;
    }
    sign | ((e as u16) << 10) | ((m >> 13) as u16)
}

fn tex(ctx: &Ctx, label: &str, w: u32, h: u32, format: wgpu::TextureFormat, render: bool) -> wgpu::Texture {
    let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    if render {
        usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }
    ctx.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

/// The real day, as the accounts give it, in world units: Hasdrubal's ride up the river and round
/// the Roman rear, the Africans turning in on both wings, the Roman press, the crescent as it stood
/// and as it was bent back (the ink-orders spike's ghost).
fn ghost_paths() -> Vec<Vec<[f32; 2]>> {
    let mut v: Vec<Vec<[f32; 2]>> = vec![
        vec![[262., 610.], [250., 470.], [272., 385.], [292., 262.], [380., 196.], [520., 180.], [650., 196.], [720., 262.], [716., 340.]],
        vec![[338., 600.], [330., 520.], [348., 452.], [392., 410.]],
        vec![[700., 600.], [706., 520.], [690., 452.], [646., 410.]],
        vec![[520., 260.], [520., 470.]],
    ];
    for (bow, y0) in [(46.0f32, 540.0f32), (-40.0, 560.0)] {
        v.push(
            (0..=12)
                .map(|c| {
                    let u = c as f32 / 12.0 * 2.0 - 1.0;
                    [365.0 + c as f32 / 12.0 * 300.0, y0 - bow * (u * std::f32::consts::FRAC_PI_2).cos()]
                })
                .collect(),
        );
    }
    v
}

/// A polyline smoothed (Chaikin) into segments with their length along.
fn polyline_segs(pts: &[[f32; 2]], out: &mut Vec<Seg>, width: f32, kind: f32, alpha: f32) {
    let mut p: Vec<[f32; 2]> = pts.to_vec();
    for _ in 0..3 {
        let mut q = vec![p[0]];
        for w in p.windows(2) {
            let (a, b) = (w[0], w[1]);
            q.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25]);
            q.push([a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
        }
        q.push(*p.last().unwrap());
        p = q;
    }
    let mut s = 0.0;
    for w in p.windows(2) {
        let l = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        out.push(Seg { ab: [w[0][0], w[0][1], w[1][0], w[1][1]], sw: [s, s + l, width, 0.0], ka: [kind, alpha, 0.0, 0.0] });
        s += l;
    }
}

impl Battle {
    fn ink_target(ctx: &Ctx) -> wgpu::Texture {
        tex(ctx, "battle ink", ctx.width, ctx.height, wgpu::TextureFormat::Rgba8Unorm, true)
    }

    fn page_bind(ctx: &Ctx, layout: &wgpu::BindGroupLayout, g: &wgpu::Texture, i: &wgpu::Texture, s: &wgpu::Texture, d: &wgpu::Texture, smp: &wgpu::Sampler) -> wgpu::BindGroup {
        let v = |t: &wgpu::Texture| t.create_view(&Default::default());
        let (gv, iv, sv, dv) = (v(g), v(i), v(s), v(d));
        ctx.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("battle page"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&gv) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&iv) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&sv) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&dv) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(smp) },
            ],
        })
    }

    /// Pixels a world unit, and where the world's origin lies on screen (px).
    fn view(ctx: &Ctx) -> (f32, [f32; 2]) {
        let [wp, hp] = ctx.points();
        let s_pt = ((wp as f32) / BOX[0]).min((hp as f32) / BOX[1]);
        let s = s_pt * ctx.ppp as f32;
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        (s, [w * 0.5 - BOX_C[0] * s, h * 0.5 - BOX_C[1] * s])
    }

    fn to_world(ctx: &Ctx, x: f64, y: f64) -> (f32, f32) {
        let (s, o) = Self::view(ctx);
        let p = ctx.ppp as f32;
        ((x as f32 * p - o[0]) / s, (y as f32 * p - o[1]) / s)
    }

    fn fresh_page(&mut self) {
        self.sim = Sim::new(self.river.clone());
        self.looks = self.sim.units.iter().map(|u| Look { dx: u.fx, dy: u.fy, curv: 0.0, ph: u.j }).collect();
        self.acc = 0.0;
        self.started_at = None;
        self.drops.clear();
        self.comb.clear();
        self.deaths_drawn = 0;
        self.fallen_at = [None, None];
        self.decided_at = None;
        self.ring.clear();
        self.count_initial();
        self.pending_dots.clear();
        self.clear_stain = true;
        self.page_t = self.clock;
        for t in self.touches.iter_mut() {
            t.stroke = None;
        }
    }

    /// When Rome is beaten in a pocket, Hannibal's men still standing close up round it: each keeps
    /// its bearing from the pocket's middle and steps to a ring just outside the fallen mass, the
    /// nearer men on the inside, so where they stood all round, a closed ring; where they stood on
    /// one side, an arc there. (Victors re-forming; nothing is placed where no man stood.)
    fn plan_ring(&mut self, c: [f32; 2]) {
        use std::f32::consts::{PI, TAU};
        // the pocket: Rome's fallen and the men ringed in with them, pressed together round its
        // middle, each keeping its bearing, the nearer nearer: a dense mass
        let mut core: Vec<(usize, f32, f32)> = self
            .sim
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| KINDS[u.kind as usize].side == 0 && (u.state == DEAD || u.state == FORMED || (u.state == FLEEING && u.trapped)))
            .map(|(i, u)| {
                let (dx, dy) = (u.x - c[0], u.y - c[1]);
                (i, (dx * dx + dy * dy).sqrt(), dy.atan2(dx))
            })
            .collect();
        if core.is_empty() {
            return;
        }
        let n = core.len() as f32;
        let r_core = (n * CORE_AREA / PI).sqrt();
        let mut rank = vec![0usize; core.len()];
        let mut by_d: Vec<usize> = (0..core.len()).collect();
        by_d.sort_by(|&a, &b| core[a].1.partial_cmp(&core[b].1).unwrap());
        for (k, &ix) in by_d.iter().enumerate() {
            rank[ix] = k;
        }
        core.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        let mut rank2 = vec![0usize; core.len()];
        {
            let mut by_d: Vec<usize> = (0..core.len()).collect();
            by_d.sort_by(|&a, &b| core[a].1.partial_cmp(&core[b].1).unwrap());
            for (k, &ix) in by_d.iter().enumerate() {
                rank2[ix] = k;
            }
        }
        let _ = rank;
        for (k, &(i, _, _)) in core.iter().enumerate() {
            // bearing by the order round, radius by nearness: an even, full mass
            let th = -PI + (k as f32 + 0.5) / n * TAU + (self.sim.units[i].j - 0.5) * 0.05;
            let r = r_core * ((rank2[k] as f32 + 0.5) / n).sqrt();
            let u = &self.sim.units[i];
            self.ring.push((i, u.x, u.y, c[0] + r * th.cos(), c[1] + r * th.sin()));
        }
        // Hannibal's men close up round it, the nearer on the inside: all round it if they stood
        // all round it (then evenly round, in the order they stood), else an arc where they stood
        let r0 = r_core - 1.5;
        let mut men: Vec<(usize, f32, f32)> = self
            .sim
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| KINDS[u.kind as usize].side == 1 && u.state == FORMED)
            .map(|(i, u)| {
                let (dx, dy) = (u.x - c[0], u.y - c[1]);
                (i, (dx * dx + dy * dy).sqrt(), dy.atan2(dx))
            })
            .filter(|m| m.1 < 260.0)
            .collect();
        if men.is_empty() {
            return;
        }
        const BINS: usize = 36;
        let mut seen = [false; BINS];
        for m in &men {
            seen[(((m.2 + PI) / TAU * BINS as f32) as usize).min(BINS - 1)] = true;
        }
        let round = seen.iter().filter(|&&b| b).count() as f32 / BINS as f32 > 0.7;
        if round {
            men.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
            let m = men.len() as f32;
            let r1 = (r0 * r0 + m * RING_AREA / PI).sqrt();
            // spread over the annulus: bearing by order round, radius by nearness
            let mut by_d: Vec<usize> = (0..men.len()).collect();
            by_d.sort_by(|&a, &b| men[a].1.partial_cmp(&men[b].1).unwrap());
            let mut rank = vec![0usize; men.len()];
            for (k, &ix) in by_d.iter().enumerate() {
                rank[ix] = k;
            }
            for (k, &(i, _, _)) in men.iter().enumerate() {
                let a = -PI + (k as f32 + 0.5) / m * TAU;
                let f = (rank[k] as f32 + 0.5) / m;
                let r = (r0 * r0 + f * (r1 * r1 - r0 * r0)).sqrt();
                let u = &self.sim.units[i];
                self.ring.push((i, u.x, u.y, c[0] + r * a.cos(), c[1] + r * a.sin()));
            }
        } else {
            men.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let mut filled = [0usize; BINS];
            let wedge = TAU / BINS as f32;
            for (i, _d, th) in men {
                let b = (((th + PI) / TAU * BINS as f32) as usize).min(BINS - 1);
                let k = filled[b] as f32;
                filled[b] += 1;
                let r = (r0 * r0 + 2.0 * k * RING_AREA / wedge).sqrt();
                let a = th + (self.sim.units[i].j - 0.5) * wedge * 0.8;
                let u = &self.sim.units[i];
                self.ring.push((i, u.x, u.y, c[0] + r * a.cos(), c[1] + r * a.sin()));
            }
        }
    }

    fn count_initial(&mut self) {
        self.initial = [0, 0];
        for u in &self.sim.units {
            self.initial[KINDS[u.kind as usize].side as usize] += 1;
        }
    }

    fn start(&mut self) {
        if self.started_at.is_none() {
            self.started_at = Some(self.clock);
            self.sim.started = true;
        }
    }

    /// The armies' densities (for the marbling) and currents (for the cuts), smoothed.
    fn fields(&mut self) {
        let (dw, dh, iw, ih) = (self.dw, self.dh, self.iw, self.ih);
        for s in 0..2 {
            self.dens[s].iter_mut().for_each(|v| *v = 0.0);
            if s == 0 {
                self.dead.iter_mut().for_each(|v| *v = 0.0);
            }
            self.intent[s].iter_mut().for_each(|v| *v = V3::default());
        }
        for u in &self.sim.units {
            if u.state == GONE {
                continue;
            }
            let kd = KINDS[u.kind as usize];
            let s = kd.side as usize;
            let wgt = match u.state {
                FORMED => 1.0,
                FLEEING => if u.trapped { 0.8 } else { 0.2 },
                _ => 0.8,
            };
            let i = ((u.x - sim::WX0) / DCELL) as i64;
            let j = ((u.y - sim::WY0) / DCELL) as i64;
            if i >= 0 && j >= 0 && (i as usize) < dw && (j as usize) < dh {
                self.dens[s][j as usize * dw + i as usize] += wgt;
                if u.state == DEAD {
                    self.dead[j as usize * dw + i as usize] += 1.0;
                }
            }
            if u.state < DEAD {
                let i = ((u.x - sim::WX0) / ICELL) as i64;
                let j = ((u.y - sim::WY0) / ICELL) as i64;
                if i >= 0 && j >= 0 && (i as usize) < iw && (j as usize) < ih {
                    let sp = (kd.speed * 0.8).max(1.0);
                    let c = &mut self.intent[s][j as usize * iw + i as usize].0;
                    c[0] += u.vx / sp + u.fx * 0.45;
                    c[1] += u.vy / sp + u.fy * 0.45;
                    c[2] += 1.0;
                }
            }
        }
        // only the ground the armies are on (and a margin) is smoothed
        let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for u in self.sim.units.iter().filter(|u| u.state != GONE) {
            bx0 = bx0.min(u.x);
            by0 = by0.min(u.y);
            bx1 = bx1.max(u.x);
            by1 = by1.max(u.y);
        }
        let rect = |c: f32, w: usize, h: usize| {
            let m = 4.0;
            [
                (((bx0 - sim::WX0) / c - m).max(0.0)) as usize,
                (((by0 - sim::WY0) / c - m).max(0.0)) as usize,
                ((((bx1 - sim::WX0) / c + m) as usize).min(w)).max(1),
                ((((by1 - sim::WY0) / c + m) as usize).min(h)).max(1),
            ]
        };
        let (dr, ir) = (rect(DCELL, dw, dh), rect(ICELL, iw, ih));
        let mut tmp = std::mem::take(&mut self.tmp);
        let mut tmp3 = std::mem::take(&mut self.tmp3);
        for s in 0..2 {
            for _ in 0..2 {
                blur(&mut self.dens[s], &mut tmp, dw, dr, 9.0 / 9.0 * 2.2 / 9.0 * 1.0);
                blur(&mut self.intent[s], &mut tmp3, iw, ir, 1.0);
            }
        }
        blur(&mut self.dead, &mut tmp, dw, dr, 1.0 / 9.0);
        self.tmp = tmp;
        self.tmp3 = tmp3;
        for k in 0..dw * dh {
            let r = 1.0 - (-self.dens[0][k] * 0.9).exp();
            let g = 1.0 - (-self.dens[1][k] * 0.9).exp();
            self.dens_rgba[k * 4] = (r * 255.0) as u8;
            self.dens_rgba[k * 4 + 1] = (g * 255.0) as u8;
        }
    }

    fn intent_at(&self, s: usize, x: f32, y: f32) -> [f32; 2] {
        let fx = ((x - sim::WX0) / ICELL - 0.5).clamp(0.0, self.iw as f32 - 1.001);
        let fy = ((y - sim::WY0) / ICELL - 0.5).clamp(0.0, self.ih as f32 - 1.001);
        let (i, j) = (fx as usize, fy as usize);
        let (tx, ty) = (fx - i as f32, fy - j as f32);
        let g = &self.intent[s];
        let w = self.iw;
        let mut out = [0.0f32; 3];
        for (k, wt) in [(j * w + i, (1.0 - tx) * (1.0 - ty)), (j * w + i + 1, tx * (1.0 - ty)), ((j + 1) * w + i, (1.0 - tx) * ty), ((j + 1) * w + i + 1, tx * ty)] {
            for c in 0..3 {
                out[c] += g[k].0[c] * wt;
            }
        }
        let n = out[2] + 0.25;
        [out[0] / n, out[1] / n]
    }

    /// Each cut eases toward its army's current through it (or the stroke that carries it).
    fn ease_looks(&mut self, dt: f32, motion: bool) {
        for i in 0..self.sim.units.len() {
            let u = &self.sim.units[i];
            if u.state >= DEAD {
                continue;
            }
            let s = KINDS[u.kind as usize].side as usize;
            let mut t = self.intent_at(s, u.x, u.y);
            let mut rate: f32 = 3.0;
            if u.on_ink {
                if let Some(k) = self.sim.ink_at(u.x, u.y) {
                    t = [k.tx * 2.0 + t[0] * 0.3, k.ty * 2.0 + t[1] * 0.3];
                    rate = 9.0;
                }
            }
            if let Some(c) = self.fallen_at[1 - s] {
                // the day decided: Hannibal's men, ringing the fallen, comb round the ring they
                // closed (tangent to it, each the way it already leans); a winning Rome stands as it is
                let lk = self.looks[i];
                if s == 1 {
                    let (dx, dy) = (u.x - c[0], u.y - c[1]);
                    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
                    let tg = [-dy / l, dx / l];
                    let sg = if tg[0] * lk.dx + tg[1] * lk.dy >= 0.0 { 1.0 } else { -1.0 };
                    t = [tg[0] * sg, tg[1] * sg];
                    rate = 1.6;
                } else {
                    t = [lk.dx, lk.dy];
                }
            } else if let Some(c) = self.fallen_at[s] {
                // Rome's last men, pressed in the pocket, face out at the ring
                let (dx, dy) = (u.x - c[0], u.y - c[1]);
                let l = (dx * dx + dy * dy).sqrt().max(1e-3);
                t = [dx / l, dy / l];
                rate = 1.6;
            }
            if !self.comb.is_empty() {
                let mut c = [0.0f32; 3];
                for p in &self.comb {
                    let d = ((p[0] - u.x).powi(2) + (p[1] - u.y).powi(2)).sqrt();
                    if d < COMB_R {
                        let fresh = 1.0 - (self.clock as f32 - p[4]) / COMB_LIFE;
                        let w = (1.0 - d / COMB_R).powi(2) * fresh.max(0.0);
                        c[0] += p[2] * w;
                        c[1] += p[3] * w;
                        c[2] += w;
                    }
                }
                if c[2] > 0.02 {
                    let side_k: f32 = if s == 1 { 1.0 } else { 0.8 };
                    let k = (c[2] * 3.0).min(1.0) * side_k;
                    let l0 = (t[0] * t[0] + t[1] * t[1]).sqrt().max(1e-3);
                    let lc = (c[0] * c[0] + c[1] * c[1]).sqrt().max(1e-3);
                    t = [t[0] / l0 * (1.0 - k) + c[0] / lc * k, t[1] / l0 * (1.0 - k) + c[1] / lc * k];
                    rate = rate.max(14.0 * k);
                }
            }
            let l = (t[0] * t[0] + t[1] * t[1]).sqrt();
            let (tx, ty) = if l > 0.2 { (t[0] / l, t[1] / l) } else { (u.fx, u.fy) };
            let lk = &mut self.looks[i];
            let k = (rate * dt).min(1.0);
            lk.dx += (tx - lk.dx) * k;
            lk.dy += (ty - lk.dy) * k;
            let n = (lk.dx * lk.dx + lk.dy * lk.dy).sqrt().max(1e-4);
            lk.dx /= n;
            lk.dy /= n;
            let (ldx, ldy) = (lk.dx, lk.dy);
            // the current's bend along the cut
            let half = 5.0;
            let a = self.intent_at(s, u.x - ldx * half, u.y - ldy * half);
            let b = self.intent_at(s, u.x + ldx * half, u.y + ldy * half);
            let curv = if (a[0] * a[0] + a[1] * a[1]) > 0.04 && (b[0] * b[0] + b[1] * b[1]) > 0.04 {
                let d = (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]);
                (d / (2.0 * half)).clamp(-0.12, 0.12)
            } else {
                0.0
            };
            let lk = &mut self.looks[i];
            lk.curv += (curv - lk.curv) * k;
            if motion {
                let sp = (u.vx * u.vx + u.vy * u.vy).sqrt();
                lk.ph = (lk.ph - (sp * 0.035 + 0.12) * dt).rem_euclid(1.0);
            }
        }
    }

    fn build_cuts(&self, out: &mut Vec<Cut>) {
        out.clear();
        let settle = self.decided_at.map_or(0.0, |d| (((self.clock - d) / SETTLE) as f32).clamp(0.0, 1.0));
        for (i, u) in self.sim.units.iter().enumerate() {
            if u.state == GONE {
                continue;
            }
            let kd = KINDS[u.kind as usize];
            let lk = self.looks[i];
            let sp = (u.vx * u.vx + u.vy * u.vy).sqrt();
            let (mut len, mut width, mut alpha) = if kd.cav { (10.5, 1.0, 1.0) } else { (7.6, 0.9, 1.0) };
            len += (sp * 0.12).min(4.0) + u.j * 1.5;
            if kd.side == 1 {
                width *= 1.12;
            }
            let mut curv = lk.curv;
            let mut ph = lk.ph;
            let mut dir = [lk.dx, lk.dy];
            match u.state {
                FLEEING => {
                    if u.trapped {
                        width *= 0.85;
                    } else {
                        // men who got away leave the page: a short trail, then gone
                        let k = ((u.since - 1.0) / 4.0).clamp(0.0, 1.0);
                        width *= 1.0 - 0.45 * k;
                        alpha = (1.0 - k) * (1.0 - settle);
                        if alpha <= 0.0 {
                            continue;
                        }
                    }
                }
                DEAD => {
                    // the fallen stay engraved where they fell, laid over into the plate's hatching
                    // (Rome's one way, Carthage's the other), so where many fell the page keeps a
                    // tone, crisp, not a dust of dots
                    let k = (u.since / 1.2).min(1.0);
                    let h = if kd.side == 0 { [0.707f32, 0.707] } else { [0.707, -0.707] };
                    let (mut x, mut y) = (lk.dx + (h[0] - lk.dx) * k, lk.dy + (h[1] - lk.dy) * k);
                    let l = (x * x + y * y).sqrt().max(1e-3);
                    x /= l;
                    y /= l;
                    dir = [x, y];
                    len = 7.0 + 4.0 * k;
                    width *= 1.0 - 0.15 * k;
                    // a man who fell alone fades from the page; where many fell they stay, a mass
                    let di = (((u.x - sim::WX0) / DCELL) as usize).min(self.dw - 1);
                    let dj = (((u.y - sim::WY0) / DCELL) as usize).min(self.dh - 1);
                    let crowd = self.dead[dj * self.dw + di];
                    let lone = 0.12 + 0.13 * settle;
                    let a = ((crowd - lone) / 0.3).clamp(0.0, 1.0);
                    alpha = 0.95 * (1.0 - k + k * a * a * (3.0 - 2.0 * a));
                    // the victors' own fallen, lying among Rome's, leave the page as it settles
                    if kd.side == 1 && self.fallen_at[0].is_some() {
                        alpha *= 1.0 - settle;
                    }
                    if alpha < 0.02 {
                        continue;
                    }
                    curv = 0.0;
                    ph = 0.25;
                }
                _ => {}
            }
            // as the page stills, men standing alone fade, so it ends in masses, not confetti
            if settle > 0.0 && u.state != DEAD {
                let di = (((u.x - sim::WX0) / DCELL) as usize).min(self.dw - 1);
                let dj = (((u.y - sim::WY0) / DCELL) as usize).min(self.dh - 1);
                let v = self.dens_rgba[(dj * self.dw + di) * 4 + kd.side as usize] as f32 / 255.0;
                let a = ((v - 0.35) / 0.3).clamp(0.0, 1.0);
                alpha *= 1.0 - settle + settle * a * a * (3.0 - 2.0 * a);
                if alpha < 0.02 {
                    continue;
                }
            }
            out.push(Cut {
                a: [u.x, u.y, dir[0], dir[1]],
                b: [len, width, curv, ph],
                c: [kd.side as f32, alpha, 0.0, 0.0],
            });
        }
    }
}

/// A 3x3 box blur, separable, over the rows and columns `r` (the rest left as is), times `gain`.
fn blur<T: Copy + Default + std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>>(g: &mut [T], tmp: &mut Vec<T>, w: usize, r: [usize; 4], gain: f32) {
    let [x0, y0, x1, y1] = r;
    tmp.resize(g.len(), T::default());
    for y in y0..y1 {
        for x in x0..x1 {
            let a = g[y * w + x.saturating_sub(1)];
            let c = g[y * w + (x + 1).min(w - 1)];
            tmp[y * w + x] = a + g[y * w + x] + c;
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            let a = tmp[y.saturating_sub(1).max(y0) * w + x];
            let c = tmp[(y + 1).min(y1 - 1) * w + x];
            g[y * w + x] = (a + tmp[y * w + x] + c) * gain;
        }
    }
}

#[derive(Clone, Copy, Default)]
struct V3([f32; 3]);
impl std::ops::Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        V3([self.0[0] + o.0[0], self.0[1] + o.0[1], self.0[2] + o.0[2]])
    }
}
impl std::ops::Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, k: f32) -> V3 {
        V3([self.0[0] * k, self.0[1] * k, self.0[2] * k])
    }
}

impl Toy for Battle {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let meta: GroundMeta = serde_json::from_slice(&ctx.assets.read("ground.json")?)?;
        let img = image::load_from_memory(&ctx.assets.read("ground.png")?)?.to_luma_alpha16();
        let (gw, gh) = (img.width(), img.height());
        anyhow::ensure!(gw == meta.w && gh == meta.h, "ground.png is not ground.json's grid");
        let mut gdata: Vec<u16> = Vec::with_capacity((gw * gh * 2) as usize);
        for p in img.pixels() {
            gdata.push(f16(p.0[0] as f32 / 100.0));
            gdata.push(f16(p.0[1] as f32 / 100.0 - 320.0));
        }
        let device = &ctx.gpu.device;
        let ground_tex = tex(ctx, "battle ground", gw, gh, wgpu::TextureFormat::Rg16Float, false);
        ctx.gpu.queue.write_texture(
            ground_tex.as_image_copy(),
            bytemuck::cast_slice(&gdata),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(gw * 4), rows_per_image: None },
            wgpu::Extent3d { width: gw, height: gh, depth_or_array_layers: 1 },
        );
        // the ground's texels are centred on the grid's points
        let ground_rect = [meta.x0 - meta.cell * 0.5, meta.y0 - meta.cell * 0.5, meta.cell * gw as f32, meta.cell * gh as f32];

        let dw = ((sim::WX1 - sim::WX0) / DCELL) as usize;
        let dh = ((sim::WY1 - sim::WY0) / DCELL) as usize;
        let iw = ((sim::WX1 - sim::WX0) / ICELL) as usize;
        let ih = ((sim::WY1 - sim::WY0) / ICELL) as usize;
        let sw = ((sim::WX1 - sim::WX0) / SCELL) as u32;
        let sh = ((sim::WY1 - sim::WY0) / SCELL) as u32;
        let dens_tex = tex(ctx, "battle density", dw as u32, dh as u32, wgpu::TextureFormat::Rgba8Unorm, false);
        let stain_tex = tex(ctx, "battle stain", sw, sh, wgpu::TextureFormat::Rgba8Unorm, true);
        let ink_tex = Self::ink_target(ctx);

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("battle uniform"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vbuf = |label: &str, size: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let river = meta.river.clone();
        let sim0 = Sim::new(river.clone());
        let cuts = vbuf("battle cuts", (sim0.units.len() + 64) * std::mem::size_of::<Cut>());
        let segs = vbuf("battle segs", MAX_SEGS * std::mem::size_of::<Seg>());
        let dots = vbuf("battle dots", MAX_DOTS * std::mem::size_of::<Dot>());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("battle linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let u_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("battle u"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let u_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("battle u"),
            layout: &u_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });
        let t2 = |b: u32| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let page_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("battle page"),
            entries: &[
                t2(0),
                t2(1),
                t2(2),
                t2(3),
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let page_bind = Self::page_bind(ctx, &page_layout, &ground_tex, &ink_tex, &stain_tex, &dens_tex, &sampler);
        let shader = ctx.gpu.shader("battle", include_str!("battle.wgsl"));

        let max_blend = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Max },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Max },
        };
        let add_blend = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let inst_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("battle inst"), bind_group_layouts: &[&u_layout], push_constant_ranges: &[] });
        let pipe = |label: &str, vs: &str, fs: &str, attrs: &[wgpu::VertexAttribute], stride: usize, strip: bool, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&inst_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    buffers: &[wgpu::VertexBufferLayout { array_stride: stride as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: attrs }],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState { topology: if strip { wgpu::PrimitiveTopology::TriangleStrip } else { wgpu::PrimitiveTopology::TriangleList }, ..Default::default() },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            })
        };
        let a3 = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let a2 = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let p_cut = pipe("battle cut", "vs_cut", "fs_cut", &a3, std::mem::size_of::<Cut>(), true, max_blend);
        let p_seg = pipe("battle seg", "vs_seg", "fs_seg", &a3, std::mem::size_of::<Seg>(), true, max_blend);
        let p_dot = pipe("battle dot", "vs_dot", "fs_dot", &a2, std::mem::size_of::<Dot>(), true, add_blend);
        let page_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("battle page"), bind_group_layouts: &[&u_layout, &page_layout], push_constant_ranges: &[] });
        let p_page = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("battle page"),
            layout: Some(&page_pl),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_page"), buffers: &[], compilation_options: Default::default() },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_page"),
                targets: &[Some(wgpu::ColorTargetState { format: ctx.gpu.format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        let mut ghost = vec![];
        for p in ghost_paths() {
            polyline_segs(&p, &mut ghost, 0.7, 1.0, 0.32);
        }
        let looks = sim0.units.iter().map(|u| Look { dx: u.fx, dy: u.fy, curv: 0.0, ph: u.j }).collect();
        let mut b = Battle {
            sim: sim0,
            river,
            looks,
            acc: 0.0,
            clock: 0.0,
            started_at: None,
            touches: vec![],
            drops: vec![],
            comb: vec![],
            ghost,
            page_t: -10.0,
            dw,
            dh,
            dens: [vec![0.0; dw * dh], vec![0.0; dw * dh]],
            dens_rgba: vec![0; dw * dh * 4],
            iw,
            ih,
            intent: [vec![V3::default(); iw * ih], vec![V3::default(); iw * ih]],
            tmp: vec![],
            tmp3: vec![],
            fallen_at: [None, None],
            decided_at: None,
            ring: vec![],
            initial: [0, 0],
            dead: vec![0.0; dw * dh],
            deaths_drawn: 0,
            pending_dots: vec![],
            clear_stain: true,
            uniform,
            cuts,
            segs,
            dots,
            n_cuts: 0,
            n_segs: 0,
            n_dots: 0,
            ground_tex,
            dens_tex,
            stain_tex,
            ink_tex,
            ground_rect,
            sampler,
            u_bind,
            page_layout,
            page_bind,
            p_cut,
            p_seg,
            p_dot,
            p_page,
        };
        b.count_initial();
        Ok(b)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.ink_tex = Self::ink_target(ctx);
        self.page_bind = Self::page_bind(ctx, &self.page_layout, &self.ground_tex, &self.ink_tex, &self.stain_tex, &self.dens_tex, &self.sampler);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                self.touches.push(Touch { id: e.id, x: e.x, y: e.y, seen: self.clock, moved: false, stroke: None, reset: false });
            }
            Phase::Move => {
                let Some(k) = self.touches.iter().position(|t| t.id == e.id) else { return };
                let (x0, y0) = (self.touches[k].x, self.touches[k].y);
                if !self.touches[k].moved && ((e.x - x0).powi(2) + (e.y - y0).powi(2)).sqrt() > 6.0 && !self.touches[k].reset {
                    self.touches[k].moved = true;
                    self.start();
                    let (wx, wy) = Self::to_world(ctx, x0, y0);
                    self.touches[k].stroke = Some(self.sim.begin_stroke(wx, wy));
                }
                if let Some(s) = self.touches[k].stroke {
                    let (wx, wy) = Self::to_world(ctx, e.x, e.y);
                    let before = self.sim.strokes[s].pts.len();
                    self.sim.extend_stroke(s, wx, wy);
                    let pts = &self.sim.strokes[s].pts;
                    if pts.len() > before && pts.len() >= 2 {
                        let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
                        let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1e-3);
                        self.comb.push([b[0], b[1], (b[0] - a[0]) / l, (b[1] - a[1]) / l, self.clock as f32]);
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some(k) = self.touches.iter().position(|t| t.id == e.id) else { return };
                let t = self.touches.remove(k);
                if let Some(s) = t.stroke {
                    let (wx, wy) = Self::to_world(ctx, e.x, e.y);
                    self.sim.extend_stroke(s, wx, wy);
                    self.sim.end_stroke(s);
                } else if !t.moved && !t.reset && e.phase == Phase::Up {
                    // a tap: a drop of ink spreads its ring through the men
                    let (wx, wy) = Self::to_world(ctx, t.x, t.y);
                    self.start();
                    self.sim.pulse(wx, wy);
                    self.drops.push((wx, wy, self.clock));
                    self.pending_dots.push(Dot { pr: [wx, wy, 9.0, 0.0], v: [0.0, 0.0, 0.5, 0.0] });
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        self.clock += dt;
        // a finger held still a long beat: a fresh page
        let now = self.clock;
        let mut reset = false;
        for t in self.touches.iter_mut() {
            if !t.moved && !t.reset && now - t.seen > 1.6 {
                t.reset = true;
                reset = true;
            }
        }
        if reset {
            ctx.haptic();
            self.fresh_page();
        }
        if self.started_at.is_none() && self.clock - self.page_t > IDLE_START {
            self.start();
        }
        let drawing = self.touches.iter().any(|t| t.stroke.is_some());
        let mut pace = if drawing { PACE_DRAWING } else { PACE };
        if let Some(d) = self.decided_at {
            // the pocket's own settling carries the page from here; the fight itself stops at
            // once (so no man falls outside the pocket after it is drawn), the rest within a beat
            pace *= if self.fallen_at[0].is_some() { 0.0 } else { (1.0 - ((self.clock - d) / SETTLE) as f32).clamp(0.0, 1.0) };
        }
        self.acc += dt as f32 * pace;
        let before: Vec<u32> = self.sim.strokes.iter().map(|s| s.carried).collect();
        let mut n = 0;
        let t0 = std::time::Instant::now();
        while self.acc >= STEP && n < 8 {
            self.sim.step(STEP);
            self.acc -= STEP;
            n += 1;
        }
        if n > 0 && std::env::var("BATTLE_CENSUS").is_ok() && (self.sim.time / 5.0).floor() != ((self.sim.time - n as f32 * STEP) / 5.0).floor() {
            let c = self.sim.census();
            eprintln!("census t={:5.1} rome {:?} | carthage {:?}", self.sim.time, c[0], c[1]);
        }
        // the first men a stroke takes answer the finger
        if self.sim.strokes.iter().zip(before.iter()).any(|(s, b)| *b == 0 && s.carried > 0) {
            ctx.haptic();
        }
        // the day decided: one army has (almost) no man standing, or it has gone on long enough;
        // then time slows to a stop and the page is left as it lies
        if self.sim.started && n > 0 && self.decided_at.is_none() {
            let c = self.sim.census();
            let (r0, c0) = (self.initial[0].max(1) as f32, self.initial[1].max(1) as f32);
            let rome = c[0][0] as f32 / r0;
            let carth = c[1][0] as f32 / c0;
            if rome < 0.06 || carth < 0.12 || self.sim.time > 80.0 {
                self.decided_at = Some(self.clock);
                let pocket = rome < 0.06;
                if std::env::var("BATTLE_CENSUS").is_ok() {
                    eprintln!("decided at sim {:.1}: rome {:?} carthage {:?}", self.sim.time, c[0], c[1]);
                }
                // a pocket closes round a beaten Rome; any other ending (Carthage broken, or the day
                // run long) is left exactly as it lies
                let loser = if pocket { 0 } else { 2 };
                let (mut sx, mut sy, mut k) = (0.0f32, 0.0f32, 0.0f32);
                for u in self.sim.units.iter().filter(|_| loser < 2) {
                    if KINDS[u.kind as usize].side as usize == loser && (u.state == DEAD || (u.state == FLEEING && u.trapped)) {
                        sx += u.x;
                        sy += u.y;
                        k += 1.0;
                    }
                }
                if k > 0.0 {
                    let c = [sx / k, sy / k];
                    self.fallen_at[loser] = Some(c);
                    if loser == 0 {
                        self.plan_ring(c);
                    }
                }
            }
        }
        if let Some(d) = self.decided_at {
            let p = (((self.clock - d) / SETTLE) as f32).clamp(0.0, 1.0);
            let e = p * p * (3.0 - 2.0 * p);
            for &(i, sx, sy, tx, ty) in &self.ring {
                let u = &mut self.sim.units[i];
                {
                    u.x = sx + (tx - sx) * e;
                    u.y = sy + (ty - sy) * e;
                    u.vx *= 0.9;
                    u.vy *= 0.9;
                }
            }
        }
        let t1 = t0.elapsed().as_secs_f64() * 1e3;
        let now32 = self.clock as f32;
        self.comb.retain(|p| now32 - p[4] < COMB_LIFE);
        // the pen rakes: Hannibal's men under its fresh passage are drawn along it at once
        if self.sim.started && self.decided_at.is_none() {
            let fresh: Vec<[f32; 5]> = self.comb.iter().filter(|p| now32 - p[4] < 0.35).copied().collect();
            if !fresh.is_empty() {
                for u in self.sim.units.iter_mut() {
                    if u.state != FORMED || KINDS[u.kind as usize].side != 1 {
                        continue;
                    }
                    for p in &fresh {
                        let d = ((p[0] - u.x).powi(2) + (p[1] - u.y).powi(2)).sqrt();
                        if d < RAKE_R {
                            let w = (1.0 - d / RAKE_R) * dt as f32 * 22.0;
                            u.x += p[2] * w;
                            u.y += p[3] * w;
                        }
                    }
                }
            }
        }
        self.fields();
        let t2 = t0.elapsed().as_secs_f64() * 1e3;
        let still = self.decided_at.map_or(false, |d| self.clock - d > SETTLE);
        self.ease_looks(dt as f32, !ctx.reduce_motion && !still);
        if std::env::var("BATTLE_TIMES").is_ok() {
            eprintln!("times: {n} steps {t1:.1} ms, fields {:.1} ms, looks {:.1} ms", t2 - t1, t0.elapsed().as_secs_f64() * 1e3 - t2);
        }
        // the page keeps no wash: the armies' record is their cuts
        self.deaths_drawn = self.sim.deaths.len();
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let (s, o) = Self::view(ctx);
        let fade = ((self.clock - self.page_t) / 1.2).clamp(0.0, 1.0) as f32;
        let un = Uniform {
            screen: [ctx.width as f32, ctx.height as f32, ctx.ppp as f32, s],
            off: [o[0], o[1], self.clock as f32, if ctx.night { 1.0 } else { 0.0 }],
            world: [sim::WX0, sim::WY0, sim::WX1 - sim::WX0, sim::WY1 - sim::WY0],
            ground: self.ground_rect,
            misc: [if ctx.reduce_motion { 1.0 } else { 0.0 }, fade, (s / ctx.ppp as f32 / 0.63).sqrt().clamp(1.0, 1.8), 0.0],
        };
        let q = &ctx.gpu.queue;
        q.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&un));
        let mut cuts = Vec::with_capacity(self.sim.units.len());
        self.build_cuts(&mut cuts);
        self.n_cuts = cuts.len() as u32;
        q.write_buffer(&self.cuts, 0, bytemuck::cast_slice(&cuts));
        // the finger's strokes, the drops, the ghost
        let mut segs: Vec<Seg> = self.ghost.clone();
        let t = self.sim.time;
        for st in &self.sim.strokes {
            let age = t - st.born;
            let alpha = 0.32 + 0.4 * (-age / 20.0).exp();
            for w in st.pts.windows(2) {
                if segs.len() >= MAX_SEGS - 64 {
                    break;
                }
                segs.push(Seg { ab: [w[0][0], w[0][1], w[1][0], w[1][1]], sw: [w[0][2], w[1][2], 0.9, 40.0], ka: [0.0, alpha, 0.0, 0.0] });
            }
            if st.done {
                let e = st.pts.last().unwrap();
                // where it ends the ink pools: hold, then find the fight
                segs.push(Seg { ab: [e[0], e[1], e[0], e[1]], sw: [100.0, 100.0, 3.4, 0.0], ka: [0.0, alpha * 0.9, 0.0, 0.0] });
            }
        }
        for &(x, y, born) in &self.drops {
            let age = (self.clock - born) as f32;
            let r = 3.0 + (age * 2.0).min(2.5);
            segs.push(Seg { ab: [x, y, x, y], sw: [100.0, 100.0, r, 0.0], ka: [0.0, 0.85 * (-age / 30.0).exp().max(0.35), 0.0, 0.0] });
        }
        self.n_segs = segs.len().min(MAX_SEGS) as u32;
        q.write_buffer(&self.segs, 0, bytemuck::cast_slice(&segs[..self.n_segs as usize]));
        let nd = self.pending_dots.len().min(MAX_DOTS);
        self.n_dots = nd as u32;
        if nd > 0 {
            q.write_buffer(&self.dots, 0, bytemuck::cast_slice(&self.pending_dots[..nd]));
        }
        self.pending_dots.drain(..nd);
        q.write_texture(
            self.dens_tex.as_image_copy(),
            &self.dens_rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(self.dw as u32 * 4), rows_per_image: None },
            wgpu::Extent3d { width: self.dw as u32, height: self.dh as u32, depth_or_array_layers: 1 },
        );

        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let stain_view = self.stain_tex.create_view(&Default::default());
        if self.n_dots > 0 || self.clear_stain {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("battle stain"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &stain_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: if self.clear_stain { wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT) } else { wgpu::LoadOp::Load },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if self.n_dots > 0 {
                p.set_pipeline(&self.p_dot);
                p.set_bind_group(0, &self.u_bind, &[]);
                p.set_vertex_buffer(0, self.dots.slice(..));
                p.draw(0..4, 0..self.n_dots);
            }
            self.clear_stain = false;
        }
        let ink_view = self.ink_tex.create_view(&Default::default());
        {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("battle ink"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &ink_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            p.set_bind_group(0, &self.u_bind, &[]);
            if self.n_segs > 0 {
                p.set_pipeline(&self.p_seg);
                p.set_vertex_buffer(0, self.segs.slice(..));
                p.draw(0..4, 0..self.n_segs);
            }
            if self.n_cuts > 0 {
                p.set_pipeline(&self.p_cut);
                p.set_vertex_buffer(0, self.cuts.slice(..));
                p.draw(0..(2 * 8 + 2), 0..self.n_cuts);
            }
        }
        {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("battle page"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            p.set_pipeline(&self.p_page);
            p.set_bind_group(0, &self.u_bind, &[]);
            p.set_bind_group(1, &self.page_bind, &[]);
            p.draw(0..3, 0..1);
        }
        q.submit([enc.finish()]);
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        ctx.haptic();
        self.fresh_page();
    }
}

toy_kit::ios_shell!(Battle);
