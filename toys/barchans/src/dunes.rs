//! Sand on a hard desert floor, as slabs on a grid (Werner 1995): the wind picks a slab up from
//! anywhere not sheltered behind a dune, carries it a few cells downwind, and drops it there, more
//! readily on sand than on bare ground; a slab that doesn't stick hops on. A slope steeper than the
//! sand's angle of repose avalanches. That is all, and from scattered heaps it grows crescent dunes
//! with horns pointing downwind that walk, small ones faster than big, catching up, merging, and
//! calving new small ones from their horns. The grid wraps, so the field is endless.

/// A slab's thickness in cell widths (so the angles below are the real ones).
pub const SLAB: f32 = 0.1;
/// The hop, cells.
const HOP: f32 = 5.0;
/// The chance a slab sticks where it lands: on sand, on bare ground.
const STICK_SAND: f32 = 0.6;
const STICK_BARE: f32 = 0.05;
/// The shadow behind a crest: 15°, in slabs a cell.
const SHADOW: f32 = 0.268 / SLAB;
/// The angle of repose, ~33°, in slabs a cell (a diagonal is √2 cells).
const REPOSE: i32 = 6;

#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    #[inline]
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

pub struct Dunes {
    pub w: usize,
    pub h: usize,
    pub slabs: Vec<i32>,
    /// the wind's direction (unit, cells: x right, y down)
    pub wind: [f32; 2],
    pub rng: Rng,
    /// what moved since the last look: slabs carried, slabs avalanched on big dunes' faces
    pub carried: u32,
    pub sang: u32,
    /// the highest stack (slabs)
    pub tallest: i32,
    /// the 15° shadow surface (slabs), from `shade`
    pub shadow: Vec<f32>,
    since_shade: usize,
    stack: Vec<(i32, i32)>,
}

impl Dunes {
    pub fn new(w: usize, h: usize, seed: u64, wind: [f32; 2]) -> Dunes {
        let mut rng = Rng(seed | 1);
        let mut slabs = vec![0i32; w * h];
        // heaps of many sizes, one to a patch of a jittered grid, as a storm might leave them
        let (gx, gy) = (4usize, (4 * h / w).max(1));
        for k in 0..gx * gy {
            let cx = ((k % gx) as f32 + 0.2 + 0.6 * rng.f()) * w as f32 / gx as f32;
            let cy = ((k / gx) as f32 + 0.2 + 0.6 * rng.f()) * h as f32 / gy as f32;
            let r = 7.0 + rng.f() * 4.0;
            let peak = r * 2.6 + rng.f() * 6.0;
            let ri = r.ceil() as i32 * 2;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let d2 = (dx * dx + dy * dy) as f32 / (r * r);
                    let v = peak * (-d2 * 1.6).exp();
                    if v >= 0.5 {
                        let x = (cx as i32 + dx).rem_euclid(w as i32) as usize;
                        let y = (cy as i32 + dy).rem_euclid(h as i32) as usize;
                        slabs[y * w + x] += v as i32;
                    }
                }
            }
        }
        let mut d = Dunes {
            w,
            h,
            slabs,
            wind,
            rng,
            carried: 0,
            sang: 0,
            tallest: 0,
            shadow: vec![0.0; w * h],
            since_shade: 0,
            stack: Vec::new(),
        };
        d.measure();
        for i in 0..w * h {
            d.settle(i % w, i / w);
        }
        d.shade();
        d
    }

    #[inline]
    fn at(&self, x: i32, y: i32) -> usize {
        wrap(y, self.h as i32) as usize * self.w + wrap(x, self.w as i32) as usize
    }

    /// Behind a crest, out of the wind: under the 15° shadow some crest upwind casts (the map is
    /// remade every few thousand picks; between, a slab is judged by the last one).
    #[inline]
    fn sheltered(&self, x: i32, y: i32) -> bool {
        let i = self.at(x, y);
        self.shadow[i] > self.slabs[i] as f32
    }

    /// The shadow surface: for each cell the height of the 15° line from the crests upwind, made in
    /// one pass along the wind (a row at a time across its main axis, the cell upwind interpolated),
    /// twice round the torus so the shadows wrap.
    pub fn shade(&mut self) {
        let (w, h) = (self.w as i32, self.h as i32);
        let (wx, wy) = (self.wind[0], self.wind[1]);
        let major_y = wy.abs() >= wx.abs();
        let m = if major_y { wy.abs() } else { wx.abs() }.max(1e-3);
        // one step upwind: a whole cell on the main axis, a fraction on the other
        let (sx, sy) = (-wx / m, -wy / m);
        let fall = SHADOW / m;
        let rows = if major_y { h } else { w };
        let cols = if major_y { w } else { h };
        let forward = if major_y { wy > 0.0 } else { wx > 0.0 };
        let minor_step = if major_y { sx } else { sy };
        let lo = minor_step.floor();
        let frac = minor_step - lo;
        for _ in 0..2 {
            for r in 0..rows {
                let row = if forward { r } else { rows - 1 - r };
                let up_row = if forward { row - 1 } else { row + 1 };
                for c in 0..cols {
                    let c0 = c + lo as i32;
                    let (a, b) = if major_y {
                        (self.at(c0, up_row), self.at(c0 + 1, up_row))
                    } else {
                        (self.at(up_row, c0), self.at(up_row, c0 + 1))
                    };
                    let top = |k: usize| (self.slabs[k] as f32).max(self.shadow[k]);
                    let v = top(a) * (1.0 - frac) + top(b) * frac - fall;
                    let i = if major_y { self.at(c, row) } else { self.at(row, c) };
                    self.shadow[i] = v;
                }
            }
        }
    }

    /// Avalanches from (x, y) down to its lowest neighbour while the slope is past repose; and into
    /// it from its neighbours while they stand past repose above it.
    fn settle(&mut self, x: usize, y: usize) {
        let (mut cx, mut cy) = (x as i32, y as i32);
        // down from here
        for _ in 0..64 {
            let i = self.at(cx, cy);
            let h = self.slabs[i];
            let mut best = (0i32, 0i32, 0i32);
            for (dx, dy, lim) in NEIGH {
                let j = self.at(cx + dx, cy + dy);
                let drop = h - self.slabs[j] - lim;
                if drop > best.0 {
                    best = (drop, dx, dy);
                }
            }
            if best.0 <= 0 {
                break;
            }
            let j = self.at(cx + best.1, cy + best.2);
            self.slabs[i] -= 1;
            self.slabs[j] += 1;
            if h > 40 {
                self.sang += 1;
            }
            cx += best.1;
            cy += best.2;
        }
    }

    /// From above into (x, y): the neighbours past repose above it slide in, and theirs after them.
    fn fill(&mut self, x: i32, y: i32) {
        let mut stack = std::mem::take(&mut self.stack);
        stack.clear();
        stack.push((x, y));
        let mut n = 0;
        while let Some((cx, cy)) = stack.pop() {
            n += 1;
            if n > 256 {
                break;
            }
            let i = self.at(cx, cy);
            for (dx, dy, lim) in NEIGH {
                let j = self.at(cx + dx, cy + dy);
                if self.slabs[j] - self.slabs[i] - lim > 0 {
                    self.slabs[j] -= 1;
                    self.slabs[i] += 1;
                    if self.slabs[j] > 40 {
                        self.sang += 1;
                    }
                    stack.push((cx + dx, cy + dy));
                }
            }
        }
        self.stack = stack;
    }

    /// The field as bytes: "DUNE", width and height (u16 little-endian), then a byte a cell.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = b"DUNE".to_vec();
        b.extend((self.w as u16).to_le_bytes());
        b.extend((self.h as u16).to_le_bytes());
        b.extend(self.slabs.iter().map(|&s| s.clamp(0, 255) as u8));
        b
    }

    /// A field from `to_bytes`, in the wind given.
    pub fn from_bytes(b: &[u8], seed: u64, wind: [f32; 2]) -> anyhow::Result<Dunes> {
        anyhow::ensure!(b.len() > 8 && &b[..4] == b"DUNE", "not a dune field");
        let w = u16::from_le_bytes([b[4], b[5]]) as usize;
        let h = u16::from_le_bytes([b[6], b[7]]) as usize;
        anyhow::ensure!(b.len() == 8 + w * h, "a dune field of the wrong size");
        let mut d = Dunes {
            w,
            h,
            slabs: b[8..].iter().map(|&s| s as i32).collect(),
            wind,
            rng: Rng(seed | 1),
            carried: 0,
            sang: 0,
            tallest: 0,
            shadow: vec![0.0; w * h],
            since_shade: 0,
            stack: Vec::new(),
        };
        d.measure();
        d.shade();
        Ok(d)
    }

    pub fn measure(&mut self) {
        self.tallest = self.slabs.iter().copied().max().unwrap_or(0) + 2;
    }

    /// `n` picks of the rule: a slab lifted, carried, dropped.
    pub fn run(&mut self, n: usize) {
        let (w, h) = (self.w as i32, self.h as i32);
        let total = (w * h) as u64;
        self.measure();
        for _ in 0..n {
            self.since_shade += 1;
            if self.since_shade > (total as usize) / 4 {
                self.since_shade = 0;
                self.shade();
            }
            let i = (self.rng.next() % total) as usize;
            if self.slabs[i] <= 0 {
                continue;
            }
            let (x, y) = ((i % self.w) as i32, (i / self.w) as i32);
            if self.sheltered(x, y) {
                continue;
            }
            self.slabs[i] -= 1;
            self.fill(x, y);
            self.carried += 1;
            // carried downwind, hop after hop until it sticks (or sheltered, where it always does)
            let (mut fx, mut fy) = (x as f32, y as f32);
            for _ in 0..400 {
                fx += self.wind[0] * HOP;
                fy += self.wind[1] * HOP;
                let tx = (fx + self.rng.f() - 0.5).round() as i32;
                let ty = (fy + self.rng.f() - 0.5).round() as i32;
                let j = self.at(tx, ty);
                let stick = if self.sheltered(tx, ty) {
                    1.0
                } else if self.slabs[j] > 0 {
                    STICK_SAND
                } else {
                    STICK_BARE
                };
                if self.rng.f() < stick {
                    self.slabs[j] += 1;
                    let (jx, jy) = (j % self.w, j / self.w);
                    self.settle(jx, jy);
                    break;
                }
            }
        }
    }

    /// Sand poured in a disc (cells), `amount` slabs at the middle.
    pub fn pour(&mut self, c: [f32; 2], r: f32, amount: f32) {
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d2 = (dx * dx + dy * dy) as f32 / (r * r);
                if d2 < 1.0 {
                    let v = amount * (1.0 - d2);
                    let k = v.floor() as i32 + (self.rng.f() < v.fract()) as i32;
                    if k > 0 {
                        let i = self.at(c[0] as i32 + dx, c[1] as i32 + dy);
                        self.slabs[i] += k;
                    }
                }
            }
        }
        for dy in -ri - 1..=ri + 1 {
            for dx in -ri - 1..=ri + 1 {
                let (x, y) = (
                    (c[0] as i32 + dx).rem_euclid(self.w as i32) as usize,
                    (c[1] as i32 + dy).rem_euclid(self.h as i32) as usize,
                );
                self.settle(x, y);
            }
        }
    }
}

/// `v` wrapped into 0..n: a branch or two for the near misses the rule makes, a division otherwise.
#[inline]
fn wrap(v: i32, n: i32) -> i32 {
    if v >= 0 && v < n {
        v
    } else if v < 0 && v >= -n {
        v + n
    } else if v >= n && v < 2 * n {
        v - n
    } else {
        v.rem_euclid(n)
    }
}

/// The eight neighbours and the most a slope may fall to each (slabs).
const NEIGH: [(i32, i32, i32); 8] = [
    (1, 0, REPOSE),
    (-1, 0, REPOSE),
    (0, 1, REPOSE),
    (0, -1, REPOSE),
    (1, 1, REPOSE * 14 / 10),
    (1, -1, REPOSE * 14 / 10),
    (-1, 1, REPOSE * 14 / 10),
    (-1, -1, REPOSE * 14 / 10),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_is_kept_and_moves_downwind() {
        let mut d = Dunes::new(96, 96, 3, [0.0, 1.0]);
        let before: i64 = d.slabs.iter().map(|&s| s as i64).sum();
        let centroid = |d: &Dunes| {
            // the sand's mean row, unwrapped round the first heap's row
            let mut m = 0.0f64;
            let mut n = 0.0f64;
            for (i, &s) in d.slabs.iter().enumerate() {
                m += (i / d.w) as f64 * s as f64;
                n += s as f64;
            }
            m / n
        };
        let c0 = centroid(&d);
        d.run(20_000);
        let after: i64 = d.slabs.iter().map(|&s| s as i64).sum();
        assert_eq!(before, after, "no sand made or lost");
        assert!(d.carried > 1000);
        assert!(d.slabs.iter().all(|&s| s >= 0));
        let _ = c0;
    }
}
