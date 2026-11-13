//! A lightning channel grown by dielectric breakdown (Niemeyer, Pietronero & Wiesmann 1984): the air
//! between the cloud and the lake is a grid of cells; the channel is a conductor at the cloud's
//! potential (0), the lake is at 1, and the potential between them obeys Laplace's equation. Each step
//! one cell beside the channel breaks down, chosen with a chance of its potential to the power η: so
//! the channel reaches toward the water and forks on the way, never twice the same.
//!
//! A sheet (`Bolt::sheet`) is the same rule with no ground: a crawler spreading under the cloud base
//! from where it began, every edge of its grid at 1.

/// Cells across and up the vertical grid; one is `CELL_M` metres.
pub const NX: usize = 120;
pub const NZ: usize = 120;
pub const CELL_M: f64 = 10.0;
/// η: 1 is a bushy coral, 3 a near-straight line; real lightning sits near 2.
const ETA: f64 = 1.9;
/// Gauss–Seidel sweeps after each new cell (over-relaxed), and before the first.
const SWEEPS: usize = 6;
const FIRST_SWEEPS: usize = 30;
/// The window a new cell relaxes, cells round it.
const NEAR: usize = 9;
const OMEGA: f64 = 1.85;

/// One cell of the channel: where it is (grid), the cell it grew from, when it grew (0..1 of the
/// growth), and how much current it carries (1 on the main channel, less out on the branches).
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub x: f32,
    pub z: f32,
    pub parent: usize,
    pub order: f32,
    pub current: f32,
    pub main: bool,
}

pub struct Bolt {
    pub cells: Vec<Cell>,
    /// the cell that touched the ground (index), when it did
    pub ground: Option<usize>,
    pub nx: usize,
    pub nz: usize,
}

/// A small, fast, seedable generator (xorshift64*).
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

struct Grid {
    nx: usize,
    nz: usize,
    phi: Vec<f64>,
    /// fixed: on the channel (0) or a boundary
    fixed: Vec<bool>,
    owner: Vec<usize>,
}

impl Grid {
    fn idx(&self, x: usize, z: usize) -> usize {
        z * self.nx + x
    }

    fn relax(&mut self, sweeps: usize) {
        self.relax_near(self.nx / 2, self.nz / 2, self.nx.max(self.nz), sweeps);
    }

    /// Over-relaxed Gauss–Seidel in a window of `r` cells round (x, z): a new cell of the channel
    /// changes the potential mostly near it.
    fn relax_near(&mut self, cx: usize, cz: usize, r: usize, sweeps: usize) {
        let nx = self.nx;
        let (x0, x1) = (cx.saturating_sub(r).max(1), (cx + r).min(self.nx - 2));
        let (z0, z1) = (cz.saturating_sub(r).max(1), (cz + r).min(self.nz - 2));
        for _ in 0..sweeps {
            for z in z0..=z1 {
                for x in x0..=x1 {
                    let i = z * nx + x;
                    if self.fixed[i] {
                        continue;
                    }
                    let avg =
                        0.25 * (self.phi[i - 1] + self.phi[i + 1] + self.phi[i - nx] + self.phi[i + nx]);
                    self.phi[i] += OMEGA * (avg - self.phi[i]);
                }
            }
        }
    }

    /// Marks the free cells round a new channel cell `i` (of channel index `k`) as candidates.
    fn open_round(&mut self, i: usize, k: usize, front: &mut Vec<(usize, usize)>, seen: &mut [bool]) {
        let (x, z) = ((i % self.nx) as i64, (i / self.nx) as i64);
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let (a, b) = (x + dx, z + dz);
            if a < 1 || b < 1 || a >= self.nx as i64 - 1 || b >= self.nz as i64 - 1 {
                continue;
            }
            let j = b as usize * self.nx + a as usize;
            if !self.fixed[j] && !seen[j] {
                seen[j] = true;
                front.push((j, k));
            }
        }
    }

    /// The candidates beside the channel and their weights φ^η.
    fn weigh(&self, front: &[(usize, usize)], out: &mut Vec<(usize, usize, f64)>) {
        out.clear();
        for &(i, from) in front {
            if self.fixed[i] {
                continue;
            }
            let w = self.phi[i].max(0.0).powf(ETA);
            if w > 0.0 {
                out.push((i, from, w));
            }
        }
    }
}

fn pick(cands: &[(usize, usize, f64)], rng: &mut Rng) -> Option<(usize, usize)> {
    let total: f64 = cands.iter().map(|c| c.2).sum();
    if total <= 0.0 {
        return None;
    }
    let mut r = rng.f() * total;
    for c in cands {
        r -= c.2;
        if r <= 0.0 {
            return Some((c.0, c.1));
        }
    }
    cands.last().map(|c| (c.0, c.1))
}

impl Bolt {
    /// A cloud-to-ground stroke: from the cloud base (top row) near the middle down to the lake (z 0).
    pub fn strike(seed: u64) -> Bolt {
        let (nx, nz) = (NX, NZ);
        let mut rng = Rng(seed | 1);
        let mut g = Grid {
            nx,
            nz,
            phi: vec![0.0; nx * nz],
            fixed: vec![false; nx * nz],
            owner: vec![usize::MAX; nx * nz],
        };
        // the lake (bottom row, grid z = 0 is the top here: row nz-1 is the water) at 1, the sides
        // falling linearly from it to the cloud, the top row (the cloud base) at 0
        for z in 0..nz {
            let f = z as f64 / (nz - 1) as f64;
            for x in 0..nx {
                let i = g.idx(x, z);
                g.phi[i] = f;
                if z == 0 || z == nz - 1 || x == 0 || x == nx - 1 {
                    g.fixed[i] = true;
                }
            }
        }
        let x0 = (nx as i64 / 2 + rng.range(-6.0, 6.0) as i64) as usize;
        let mut cells = vec![Cell {
            x: x0 as f32,
            z: 0.0,
            parent: 0,
            order: 0.0,
            current: 0.0,
            main: false,
        }];
        let seed_i = g.idx(x0, 1);
        g.fixed[seed_i] = true;
        g.phi[seed_i] = 0.0;
        g.owner[seed_i] = 0;
        cells[0].z = 1.0;
        g.relax_near(x0, 1, 12, FIRST_SWEEPS);
        let mut cands = Vec::new();
        let mut front = Vec::new();
        let mut seen = vec![false; nx * nz];
        g.open_round(seed_i, 0, &mut front, &mut seen);
        let mut ground = None;
        for _ in 0..nx * nz / 2 {
            g.weigh(&front, &mut cands);
            let Some((i, from)) = pick(&cands, &mut rng) else {
                break;
            };
            let (x, z) = (i % nx, i / nx);
            let k = cells.len();
            cells.push(Cell {
                x: x as f32,
                z: z as f32,
                parent: from,
                order: 0.0,
                current: 0.0,
                main: false,
            });
            g.fixed[i] = true;
            g.phi[i] = 0.0;
            g.owner[i] = k;
            if z >= nz - 2 {
                ground = Some(k);
                break;
            }
            g.open_round(i, k, &mut front, &mut seen);
            front.retain(|&(j, _)| !g.fixed[j]);
            g.relax_near(x, z, NEAR, SWEEPS);
        }
        let mut b = Bolt {
            cells,
            ground,
            nx,
            nz,
        };
        b.finish();
        b
    }

    /// A crawler under the cloud: grown from the middle of a flat grid for `n` cells, every edge at 1.
    pub fn sheet(seed: u64, n: usize) -> Bolt {
        let (nx, nz) = (96usize, 44usize);
        let mut rng = Rng(seed | 1);
        let mut g = Grid {
            nx,
            nz,
            phi: vec![1.0; nx * nz],
            fixed: vec![false; nx * nz],
            owner: vec![usize::MAX; nx * nz],
        };
        for z in 0..nz {
            for x in 0..nx {
                if z == 0 || z == nz - 1 || x == 0 || x == nx - 1 {
                    let i = g.idx(x, z);
                    g.fixed[i] = true;
                }
            }
        }
        let (x0, z0) = (nx / 2, nz / 2);
        let s = g.idx(x0, z0);
        g.fixed[s] = true;
        g.phi[s] = 0.0;
        g.owner[s] = 0;
        let mut cells = vec![Cell {
            x: x0 as f32,
            z: z0 as f32,
            parent: 0,
            order: 0.0,
            current: 0.0,
            main: false,
        }];
        g.relax(120);
        let mut cands = Vec::new();
        let mut front = Vec::new();
        let mut seen = vec![false; nx * nz];
        g.open_round(s, 0, &mut front, &mut seen);
        for _ in 0..n {
            g.weigh(&front, &mut cands);
            let Some((i, from)) = pick(&cands, &mut rng) else {
                break;
            };
            let (x, z) = (i % nx, i / nx);
            let k = cells.len();
            cells.push(Cell {
                x: x as f32,
                z: z as f32,
                parent: from,
                order: 0.0,
                current: 0.0,
                main: false,
            });
            g.fixed[i] = true;
            g.phi[i] = 0.0;
            g.owner[i] = k;
            if x <= 1 || z <= 1 || x >= nx - 2 || z >= nz - 2 {
                break;
            }
            g.open_round(i, k, &mut front, &mut seen);
            front.retain(|&(j, _)| !g.fixed[j]);
            g.relax_near(x, z, NEAR, SWEEPS);
        }
        let mut b = Bolt {
            cells,
            ground: None,
            nx,
            nz,
        };
        b.finish();
        b
    }

    /// When each cell grew (0..1) and the current each carries: the main channel (ground to cloud) 1,
    /// a branch by how many cells hang from it.
    fn finish(&mut self) {
        let n = self.cells.len();
        for (k, c) in self.cells.iter_mut().enumerate() {
            c.order = k as f32 / (n.max(2) - 1) as f32;
        }
        let mut below = vec![1usize; n];
        for k in (1..n).rev() {
            let p = self.cells[k].parent;
            below[p] += below[k];
        }
        for k in 0..n {
            self.cells[k].current = ((below[k] as f32) / (n as f32)).sqrt().min(1.0) * 0.6;
        }
        let mut k = self.ground.unwrap_or_else(|| self.deepest());
        loop {
            self.cells[k].main = true;
            self.cells[k].current = 1.0;
            if k == 0 {
                break;
            }
            k = self.cells[k].parent;
        }
    }

    /// The cell farthest from the start (the sheet's longest reach).
    fn deepest(&self) -> usize {
        let c0 = self.cells[0];
        let mut best = (0.0f32, 0usize);
        for (k, c) in self.cells.iter().enumerate() {
            let d = (c.x - c0.x).powi(2) + (c.z - c0.z).powi(2);
            if d > best.0 {
                best = (d, k);
            }
        }
        best.1
    }

    /// Where the channel touched the water, across the grid (cells).
    pub fn foot(&self) -> f32 {
        self.ground.map(|k| self.cells[k].x).unwrap_or(self.nx as f32 * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strike_reaches_the_water_and_forks() {
        for seed in 1..4 {
            let b = Bolt::strike(seed);
            assert!(b.ground.is_some(), "seed {seed} never reached the lake");
            let main = b.cells.iter().filter(|c| c.main).count();
            assert!(main >= NZ - 3, "the main channel spans the gap");
            assert!(b.cells.len() > main + 10, "and it forks on the way");
        }
    }

    #[test]
    fn two_strikes_differ() {
        let (a, b) = (Bolt::strike(5), Bolt::strike(6));
        let same = a
            .cells
            .iter()
            .zip(&b.cells)
            .filter(|(p, q)| p.x == q.x && p.z == q.z)
            .count();
        assert!(same < a.cells.len().min(b.cells.len()) / 2);
    }
}
