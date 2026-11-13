//! The mold's tubes, by Tero et al.'s own model of them (A. Tero et al. 2010, "Rules for biologically
//! inspired adaptive network design", Science 327, 439–442): the plasmodium as a mesh of tubes, each
//! with a conductivity D. Food sources pump the protoplasm: each step one oat flake, chosen at random,
//! is the source of a flux I0 and the others its sinks alike; the pressures follow Kirchhoff's law over
//! the mesh (Poiseuille flow, Q = D/L · Δp), and each tube's D grows with the flux through it and decays
//! without it, dD/dt = f(|Q|) − D, f(Q) = (1+a)Q^γ / (1+aQ^γ). Tubes that carry much thicken; the rest
//! wither. What is left is the network.
//!
//! Before that, the mold has to reach the flakes: it spreads from where it was put (Tokyo, as in the
//! experiment) as a sheet over the agar, slower where the plate is lit (Tero et al. lit the mountains,
//! which the mold shies from); a tube joins the mesh when the sheet covers both its ends.

/// One node of the mesh: where it is (plate cells), when the sheet reaches it (s, infinite if never),
/// the light on it (0..1).
#[derive(Clone, Copy)]
pub struct Node {
    pub at: [f32; 2],
    pub reach: f64,
    pub light: f32,
}

/// One tube: its two nodes, its length (cells), its conductivity, the light along it.
#[derive(Clone, Copy)]
pub struct Tube {
    pub a: u32,
    pub b: u32,
    pub len: f32,
    pub d: f32,
    pub light: f32,
    /// The last flux through it, |Q| (for the streaming drawn along it).
    pub q: f32,
}

/// The flux the source flake pumps (Tero et al.'s I0).
const I0: f64 = 6.0;
/// f's steepness and saturation (Tero et al.'s γ and a: γ > 1 thins the mesh to a network).
const GAMMA: f64 = 1.6;
const SAT: f64 = 1.0;
/// A tube's conductivity when the sheet first covers it, and the least it falls to (so a withered
/// tube can thicken again, as the mold's does when food appears beyond it).
pub const D_NEW: f32 = 1.0;
const D_MIN: f32 = 1e-4;
/// How much the plate's light holds back a tube's growth (Tero et al.'s illumination).
const SHY: f32 = 0.85;
/// The most conjugate-gradient iterations a solve takes (each starts from the last, so few are needed).
const ITERATIONS: usize = 40;
/// The sheet's speed: plate cells a second on dark agar, and how much light slows it.
pub const SPREAD: f64 = 13.0;
const SLOW: f64 = 5.0;
/// The most light the sheet will cover.
const DARK_ENOUGH: f32 = 0.6;

pub struct Net {
    pub nodes: Vec<Node>,
    pub tubes: Vec<Tube>,
    /// Each node's tubes (index into `tubes`).
    pub around: Vec<Vec<u32>>,
    /// The oat flakes: their nodes.
    pub food: Vec<u32>,
    /// The adaptation's own clock (Tero et al.'s time), and the steps taken.
    pub time: f64,
    pub steps: u64,
    seed: u64,
    /// The pressures of the last solve, and each flake's last as the source (its next solve starts
    /// from it).
    p: Vec<f64>,
    cache: std::collections::HashMap<u32, Vec<f64>>,
    /// How much the tubes changed in the last steps (small once the network has settled).
    pub change: f32,
}

pub fn rand(s: &mut u64) -> f64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

impl Net {
    /// A mesh over the agar: a jittered triangular lattice `spacing` cells apart, its nodes where
    /// `agar(x, y)` says the plate is, its tubes between neighbours whose middle is on the plate too.
    pub fn new(
        w: usize,
        h: usize,
        spacing: f32,
        agar: impl Fn(f32, f32) -> bool,
        light: impl Fn(f32, f32) -> f32,
        seed: u64,
    ) -> Net {
        let mut s = seed | 1;
        let row_h = spacing * 0.866;
        let rows = (h as f32 / row_h) as usize + 1;
        let cols = (w as f32 / spacing) as usize + 1;
        let mut grid = vec![u32::MAX; rows * cols];
        let mut nodes = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let jx = (rand(&mut s) as f32 - 0.5) * 0.45 * spacing;
                let jy = (rand(&mut s) as f32 - 0.5) * 0.45 * spacing;
                let x = (c as f32 + 0.5 * (r % 2) as f32) * spacing + jx;
                let y = r as f32 * row_h + jy;
                if x < 1.0 || y < 1.0 || x > w as f32 - 2.0 || y > h as f32 - 2.0 || !agar(x, y) {
                    continue;
                }
                grid[r * cols + c] = nodes.len() as u32;
                nodes.push(Node {
                    at: [x, y],
                    reach: f64::INFINITY,
                    light: light(x, y),
                });
            }
        }
        let mut tubes = Vec::new();
        let mut around = vec![Vec::new(); nodes.len()];
        for r in 0..rows {
            for c in 0..cols {
                let a = grid[r * cols + c];
                if a == u32::MAX {
                    continue;
                }
                // east, and the two below (which two depends on the row's offset)
                let odd = r % 2;
                let next = [
                    (r, c + 1),
                    (r + 1, c + odd),
                    (r + 1, (c + odd).wrapping_sub(1)),
                ];
                for (rr, cc) in next {
                    if rr >= rows || cc >= cols {
                        continue;
                    }
                    let b = grid[rr * cols + cc];
                    if b == u32::MAX {
                        continue;
                    }
                    let (pa, pb) = (nodes[a as usize].at, nodes[b as usize].at);
                    let mid = [(pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5];
                    if !agar(mid[0], mid[1]) {
                        continue;
                    }
                    let len = (pb[0] - pa[0]).hypot(pb[1] - pa[1]);
                    around[a as usize].push(tubes.len() as u32);
                    around[b as usize].push(tubes.len() as u32);
                    tubes.push(Tube {
                        a,
                        b,
                        len,
                        d: 0.0,
                        light: (nodes[a as usize].light
                            + nodes[b as usize].light
                            + light(mid[0], mid[1]))
                            / 3.0,
                        q: 0.0,
                    });
                }
            }
        }
        let n = nodes.len();
        Net {
            nodes,
            tubes,
            around,
            food: Vec::new(),
            time: 0.0,
            steps: 0,
            seed: s,
            p: vec![0.0; n],
            cache: Default::default(),
            change: 1.0,
        }
    }

    /// The node nearest a point (cells), moved onto it: where an oat flake lies.
    pub fn nearest(&self, at: [f32; 2]) -> Option<u32> {
        (0..self.nodes.len() as u32)
            .filter(|&i| !self.around[i as usize].is_empty())
            .min_by(|&i, &j| {
                let d = |k: u32| {
                    let p = self.nodes[k as usize].at;
                    (p[0] - at[0]).powi(2) + (p[1] - at[1]).powi(2)
                };
                d(i).total_cmp(&d(j))
            })
    }

    /// The sheet spreads from `from` (a node) at `t0`: each node's reach is its quickest way over the
    /// agar, slower where it is lit (Dijkstra). A node already reached keeps the earlier time.
    pub fn spread(&mut self, from: u32, t0: f64) {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let mut best = vec![f64::INFINITY; self.nodes.len()];
        best[from as usize] = t0;
        let mut heap = BinaryHeap::new();
        heap.push((Reverse(OrdF(t0)), from));
        while let Some((Reverse(OrdF(t)), i)) = heap.pop() {
            if t > best[i as usize] {
                continue;
            }
            for &k in &self.around[i as usize] {
                let tube = &self.tubes[k as usize];
                let j = if tube.a == i { tube.b } else { tube.a };
                // the brightest-lit ground it never covers
                if tube.light > DARK_ENOUGH {
                    continue;
                }
                let cost = tube.len as f64 * (1.0 + SLOW * tube.light as f64) / SPREAD;
                let tj = t + cost;
                if tj < best[j as usize] {
                    best[j as usize] = tj;
                    heap.push((Reverse(OrdF(tj)), j));
                }
            }
        }
        for (n, b) in self.nodes.iter_mut().zip(best) {
            n.reach = n.reach.min(b);
        }
    }

    /// Whether the sheet covers a tube at `t` (both its ends reached).
    pub fn covered(&self, tube: &Tube, t: f64) -> bool {
        self.nodes[tube.a as usize].reach <= t && self.nodes[tube.b as usize].reach <= t
    }

    /// One step of Tero et al.'s adaptation at plate time `t` (s), `dt` of the model's own time: a
    /// random one of the reached flakes pumps I0 through the covered mesh, the others drawing it off
    /// alike; each tube's D eases toward
    /// f(|Q|), held back by the light.
    pub fn adapt(&mut self, t: f64, dt: f64) {
        // tubes the sheet has just covered start full
        for k in 0..self.tubes.len() {
            if self.tubes[k].d == 0.0 && self.covered(&self.tubes[k], t) {
                self.tubes[k].d = D_NEW;
            }
        }
        let fed: Vec<u32> = self
            .food
            .iter()
            .copied()
            .filter(|&f| self.nodes[f as usize].reach <= t)
            .collect();
        if fed.len() < 2 {
            return;
        }
        let a = fed[(rand(&mut self.seed) * fed.len() as f64) as usize % fed.len()];
        let sinks: Vec<u32> = fed.iter().copied().filter(|&f| f != a).collect();
        // from where this flake's flow last stood: the tubes have changed little since
        self.p = self
            .cache
            .remove(&a)
            .unwrap_or_else(|| vec![0.0; self.nodes.len()]);
        self.solve(a, &sinks, t);
        self.cache.insert(a, self.p.clone());
        let mut change = 0.0f32;
        for tube in self.tubes.iter_mut() {
            if tube.d == 0.0 {
                continue;
            }
            let g = tube.d as f64 / tube.len as f64;
            let q = (g * (self.p[tube.a as usize] - self.p[tube.b as usize])).abs();
            let qg = q.powf(GAMMA);
            let f = (1.0 + SAT) * qg / (1.0 + SAT * qg) * (1.0 - SHY * tube.light) as f64;
            let d = tube.d as f64 + dt * (f - tube.d as f64);
            let d = (d as f32).max(D_MIN);
            change += (d - tube.d).abs();
            tube.d = d;
            tube.q = tube.q * 0.9 + 0.1 * q as f32;
        }
        self.change = self.change * 0.98 + 0.02 * change / dt as f32;
        self.time += dt;
        self.steps += 1;
    }

    /// The pressures with I0 in at `src` and out at the `sinks` alike (the last held at 0), over the covered mesh:
    /// conjugate gradients, preconditioned by the diagonal, from the last solve's pressures.
    fn solve(&mut self, src: u32, sinks: &[u32], t: f64) {
        let n = self.nodes.len();
        let live: Vec<bool> = self.nodes.iter().map(|x| x.reach <= t).collect();
        let mut diag = vec![0.0f64; n];
        for tube in &self.tubes {
            if tube.d > 0.0 {
                let g = tube.d as f64 / tube.len as f64;
                diag[tube.a as usize] += g;
                diag[tube.b as usize] += g;
            }
        }
        // the last sink is held at 0 and takes what the others leave
        let sink = *sinks.last().unwrap_or(&src) as usize;
        let apply = |x: &[f64], out: &mut [f64], tubes: &[Tube]| {
            out.iter_mut().for_each(|v| *v = 0.0);
            for tube in tubes {
                if tube.d <= 0.0 {
                    continue;
                }
                let (a, b) = (tube.a as usize, tube.b as usize);
                let g = tube.d as f64 / tube.len as f64;
                let f = g * (x[a] - x[b]);
                out[a] += f;
                out[b] -= f;
            }
            out[sink] = x[sink];
        };
        let mut rhs = vec![0.0f64; n];
        rhs[src as usize] = I0;
        for &k in sinks {
            rhs[k as usize] -= I0 / sinks.len() as f64;
        }
        let x = &mut self.p;
        for i in 0..n {
            if !live[i] || diag[i] == 0.0 {
                x[i] = 0.0;
            }
        }
        x[sink] = 0.0;
        let mut ax = vec![0.0; n];
        apply(x, &mut ax, &self.tubes);
        let mut r: Vec<f64> = (0..n)
            .map(|i| {
                if i == sink || diag[i] == 0.0 {
                    0.0
                } else {
                    rhs[i] - ax[i]
                }
            })
            .collect();
        let pre = |i: usize| {
            if i == sink || diag[i] == 0.0 {
                0.0
            } else {
                1.0 / diag[i]
            }
        };
        let mut z: Vec<f64> = (0..n).map(|i| r[i] * pre(i)).collect();
        let mut d = z.clone();
        let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let tol = 1e-8 * I0 * I0;
        for _ in 0..ITERATIONS {
            if rz.abs() < tol {
                break;
            }
            apply(&d, &mut ax, &self.tubes);
            ax[sink] = 0.0;
            let dad: f64 = d.iter().zip(&ax).map(|(a, b)| a * b).sum();
            if dad <= 0.0 {
                break;
            }
            let alpha = rz / dad;
            for i in 0..n {
                x[i] += alpha * d[i];
                r[i] -= alpha * ax[i];
            }
            for i in 0..n {
                z[i] = r[i] * pre(i);
            }
            let rz2: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
            let beta = rz2 / rz;
            rz = rz2;
            for i in 0..n {
                d[i] = z[i] + beta * d[i];
            }
        }
    }
}

/// An f64 that sorts (no NaN reaches it).
#[derive(PartialEq, PartialOrd)]
struct OrdF(f64);
impl Eq for OrdF {}
impl Ord for OrdF {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}
