//! The tide up the Qiantang: one line of the estuary from Ganpu, where Hangzhou Bay ends, to above Yanguan,
//! solved as the shallow-water equations of a channel whose breadth changes (Saint-Venant, per unit
//! breadth): the tide is pushed in at the mouth, the river pushes out at the top, the bed drags. Two
//! things make the bore, and each is a switch: the bay narrows like a funnel, 100 km across at its mouth
//! to 3 at Yanguan, piling the tide up as it goes; and over the shallow sands the water's speed of
//! travel, √(g·depth), is faster at the crest than in the trough, so the tide's front catches itself up
//! and stands as a wall (Chanson 2011; Pan, Lin & Lu 2007; SOURCES.md).
//!
//! The scheme: finite volumes of 100 m, the Rusanov flux on a hydrostatic reconstruction (Audusse et
//! al. 2004: the sea at rest over the sloping bed stays at rest, and the sands may dry), Manning's
//! friction taken implicitly. Distances are metres along the estuary from the mouth (`x`), heights
//! metres above mean sea level.

/// Cells, and their length (m).
pub const N: usize = 1000;
pub const DX: f64 = 50.0;
/// Yanguan: 38 km up from Ganpu.
pub const YANGUAN: f64 = 38_000.0;
const G: f64 = 9.81;
/// Manning's n for the sandy bed.
const MANNING: f64 = 0.010;
/// The semidiurnal tide's period (M2, s).
pub const PERIOD: f64 = 44_712.0;
/// The river's discharge per unit breadth at the top (m²/s): the Qiantang's ~400 m³/s in a dry autumn over its 2.5 km.
const RIVER: f64 = 0.15;
const DRY: f64 = 1e-3;
/// The depth the linear tide takes over the bar (m): its water at rest, a little above mean sea.
const LINEAR_DEPTH: f64 = 2.5;

/// The rules as the sheet's switches name them, in `Tide::on`'s order.
pub const RULES: [&str; 2] = ["The bay narrows", "Crests outrun troughs"];

pub struct Tide {
    /// depth (m) and discharge per unit breadth (m²/s, positive up-river), a cell each
    pub h: Vec<f64>,
    pub q: Vec<f64>,
    /// the bed (m above mean sea level) and the breadth (m), a cell each
    pub z: Vec<f64>,
    pub b: Vec<f64>,
    /// seconds of the tide's own time
    pub s: f64,
    /// the moon's age through its month, 0..1 (0 new, 0.5 full)
    pub moon: f64,
    pub on: [bool; 2],
}

/// The tide's half-range at Ganpu for a moon this old (m), and whether the bay narrows: the moon's and
/// the sun's tides in step at new and full moon (springs), against each other at the quarters (neaps),
/// the biggest coming two days after the moon (the age of the tide). Hangzhou Bay narrows from 100 km
/// at its mouth to 20 at Ganpu and piles the tide up as it goes, by Green's law (the height grows as the
/// breadth's inverse square root, a little less for the bed's drag): a little over twice. A bay
/// that didn't narrow would bring the open sea's tide to Ganpu as it is.
pub fn half_range(moon: f64, funnel: bool) -> f64 {
    let lag = 2.0 / 29.53;
    let sea = 1.6 + 0.75 * (std::f64::consts::TAU * 2.0 * (moon - lag)).cos();
    if funnel {
        sea * 2.2
    } else {
        sea
    }
}

impl Tide {
    pub fn new(moon: f64, on: [bool; 2]) -> Tide {
        let mut t = Tide {
            h: vec![0.0; N],
            q: vec![0.0; N],
            z: vec![0.0; N],
            b: vec![0.0; N],
            s: 0.0,
            moon,
            on,
        };
        t.shape();
        let low = -half_range(moon, on[0]);
        for i in 0..N {
            t.h[i] = (low - t.z[i]).max(0.0).max(0.4);
            t.q[i] = -RIVER;
        }
        t
    }

    pub fn x(i: usize) -> f64 {
        (i as f64 + 0.5) * DX
    }

    /// The bed and the breadth for the switches as they are; the water's level kept where it is.
    pub fn shape(&mut self) {
        for i in 0..N {
            let x = Tide::x(i) / 1000.0;
            // the estuary narrows on from 20 km at Ganpu to 3 at Yanguan; its bed is the great sand bar,
            // rising from Ganpu's channel to less than a metre under mean sea level
            let b = if self.on[0] {
                (20.0 * (-(x / 38.0) * (20.0f64 / 3.0).ln()).exp()).max(2.5)
            } else {
                3.0
            };
            let z = -6.0 + 5.2 * smooth(0.0, 16.0, x);
            let level = self.z[i] + self.h[i];
            self.b[i] = b * 1000.0;
            if self.s > 0.0 {
                self.h[i] = if self.on[1] {
                    (level - z).max(0.0)
                } else {
                    level - z
                };
            }
            self.z[i] = z;
        }
    }

    /// The water's level at the mouth now (m).
    fn mouth(&self) -> f64 {
        -half_range(self.moon, self.on[0]) * (std::f64::consts::TAU * self.s / PERIOD).cos()
    }

    /// The level (m above mean sea) at cell i.
    pub fn eta(&self, i: usize) -> f64 {
        self.z[i] + self.h[i]
    }

    /// The level at a distance x up the estuary (m), linearly between cells.
    pub fn eta_at(&self, x: f64) -> f64 {
        let f = (x / DX - 0.5).clamp(0.0, (N - 1) as f64);
        let i = (f as usize).min(N - 2);
        let a = f - i as f64;
        self.eta(i) * (1.0 - a) + self.eta(i + 1) * a
    }

    pub fn speed_at(&self, x: f64) -> f64 {
        let i = ((x / DX) as usize).min(N - 1);
        if self.h[i] > 0.05 {
            self.q[i] / self.h[i]
        } else {
            0.0
        }
    }

    /// The longest step the scheme allows now (s).
    pub fn max_dt(&self) -> f64 {
        let mut m: f64 = (G * 10.0f64).sqrt();
        if !self.on[1] {
            return 0.8 * DX / (m + 2.0);
        }
        for i in 0..N {
            let h = self.h[i];
            if h > DRY {
                m = m.max((self.q[i] / h).abs() + (G * h).sqrt());
            }
        }
        0.8 * DX / m
    }

    /// Runs the tide on by `secs` of its own time, in as many steps as it needs.
    pub fn run(&mut self, secs: f64) {
        let mut left = secs;
        while left > 1e-6 {
            let dt = self.max_dt().min(left);
            self.step(dt);
            left -= dt;
        }
    }

    fn step(&mut self, dt: f64) {
        // Heun's two stages (strong-stability-preserving RK2) over the second-order fluxes, then the drag
        let (h0, q0) = (self.h.clone(), self.q.clone());
        let (dh, dq) = self.rates(&h0, &q0);
        let floor = if self.on[1] { 0.0 } else { f64::MIN };
        let mut h1: Vec<f64> = (0..N).map(|i| (h0[i] + dt * dh[i]).max(floor)).collect();
        let mut q1: Vec<f64> = (0..N).map(|i| q0[i] + dt * dq[i]).collect();
        self.s += dt;
        let (dh, dq) = self.rates(&h1, &q1);
        for i in 0..N {
            h1[i] = (0.5 * (h0[i] + h1[i] + dt * dh[i])).max(floor);
            q1[i] = 0.5 * (q0[i] + q1[i] + dt * dq[i]);
            if !self.on[1] {
                let d = (-self.z[i]).max(LINEAR_DEPTH);
                q1[i] /= 1.0 + dt * G * MANNING * MANNING * q1[i].abs() / d.powf(7.0 / 3.0);
            } else if h1[i] <= DRY {
                q1[i] = 0.0;
            } else {
                // the bed's drag, taken implicitly so a thin sheet stops rather than reverses
                let c = G * MANNING * MANNING * q1[i].abs() / h1[i].powf(7.0 / 3.0);
                q1[i] /= 1.0 + dt * c;
            }
        }
        self.h = h1;
        self.q = q1;
    }

    /// How fast depth and discharge change in each cell: the fluxes through its faces (MUSCL with minmod
    /// slopes on the level and the speed, then the hydrostatic reconstruction), the bed's push, and the
    /// funnel's squeeze. (`self.s` is the time the stage is at, for the sea at the mouth.)
    fn rates(&self, h: &[f64], q: &[f64]) -> (Vec<f64>, Vec<f64>) {
        if !self.on[1] {
            return self.rates_linear(h, q);
        }
        // ghost cells: the sea at the mouth, the river at the top
        let eta0 = self.mouth();
        let n = N + 4;
        let mut hh = vec![0.0; n];
        let mut uu = vec![0.0; n];
        let mut zz = vec![0.0; n];
        for i in 0..N {
            hh[i + 2] = h[i];
            uu[i + 2] = if h[i] > DRY { q[i] / h[i] } else { 0.0 };
            zz[i + 2] = self.z[i];
        }
        for g in 0..2 {
            zz[g] = self.z[0];
            hh[g] = (eta0 - self.z[0]).max(0.0);
            uu[g] = uu[2];
            zz[N + 2 + g] = self.z[N - 1];
            hh[N + 2 + g] = h[N - 1];
            uu[N + 2 + g] = uu[N + 1].min(-RIVER / h[N - 1].max(0.5));
        }
        // each cell's edges: (h, z, u) at its left (-) and right (+)
        let mut lo = vec![(0.0, 0.0, 0.0); n];
        let mut hi = vec![(0.0, 0.0, 0.0); n];
        for i in 1..n - 1 {
            let e = |k: usize| hh[k] + zz[k];
            let de = minmod(e(i) - e(i - 1), e(i + 1) - e(i));
            let dh = minmod(hh[i] - hh[i - 1], hh[i + 1] - hh[i]);
            let du = minmod(uu[i] - uu[i - 1], uu[i + 1] - uu[i]);
            let (hm, hp) = (hh[i] - 0.5 * dh, hh[i] + 0.5 * dh);
            let (em, ep) = (e(i) - 0.5 * de, e(i) + 0.5 * de);
            lo[i] = (hm, em - hm, uu[i] - 0.5 * du);
            hi[i] = (hp, ep - hp, uu[i] + 0.5 * du);
        }
        let mut fh = vec![0.0; N + 1];
        let mut fq = vec![0.0; N + 1];
        let mut sl = vec![0.0; N + 1];
        let mut sr = vec![0.0; N + 1];
        for f in 0..=N {
            // face f lies between cell f-1 and cell f (ghost-shifted: f+1 and f+2)
            let (l, r) = (hi[f + 1], lo[f + 2]);
            let zf = l.1.max(r.1);
            let hl = (l.0 + l.1 - zf).max(0.0);
            let hr = (r.0 + r.1 - zf).max(0.0);
            let (ul, ur) = (
                if hl > DRY { l.2 } else { 0.0 },
                if hr > DRY { r.2 } else { 0.0 },
            );
            let (ql, qr) = (hl * ul, hr * ur);
            let a = (ul.abs() + (G * hl).sqrt()).max(ur.abs() + (G * hr).sqrt());
            fh[f] = 0.5 * (ql + qr) - 0.5 * a * (hr - hl);
            fq[f] = 0.5 * (ql * ul + 0.5 * G * hl * hl + qr * ur + 0.5 * G * hr * hr)
                - 0.5 * a * (qr - ql);
            sl[f] = 0.5 * G * (l.0 * l.0 - hl * hl);
            sr[f] = 0.5 * G * (r.0 * r.0 - hr * hr);
        }
        let mut dh = vec![0.0; N];
        let mut dq = vec![0.0; N];
        for i in 0..N {
            let (m, p) = (lo[i + 2], hi[i + 2]);
            let u = uu[i + 2];
            let db = (self.b[(i + 1).min(N - 1)] - self.b[i.saturating_sub(1)])
                / (DX * if i == 0 || i == N - 1 { 1.0 } else { 2.0 });
            let w = db / self.b[i];
            dh[i] = -(fh[i + 1] - fh[i]) / DX - q[i] * w;
            dq[i] = -(fq[i + 1] + sl[i + 1] - fq[i] - sr[i]) / DX
                + 0.5 * G * (m.0 + p.0) * (m.1 - p.1) / DX
                - q[i] * u * w;
        }
        (dh, dq)
    }

    /// The same tide with every part of the wave travelling at one speed, √(g·H) for the depth H of the
    /// water at rest (the equations linearised; the second switch off): the tide's front cannot catch
    /// itself up, so it comes as a smooth rise. Here `h` is the level above the bed and may go below it.
    fn rates_linear(&self, h: &[f64], q: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let eta0 = self.mouth();
        let eta = |i: isize| -> f64 {
            if i < 0 {
                eta0
            } else {
                let k = (i as usize).min(N - 1);
                h[k] + self.z[k]
            }
        };
        let qa = |i: isize| -> f64 {
            if i < 0 {
                q[0]
            } else if i as usize >= N {
                q[N - 1]
            } else {
                q[i as usize]
            }
        };
        let depth = |i: usize| (-self.z[i.min(N - 1)]).max(LINEAR_DEPTH);
        let mut fh = vec![0.0; N + 1];
        let mut fe = vec![0.0; N + 1];
        for f in 0..=N {
            let (l, r) = (f as isize - 1, f as isize);
            let a = (G * depth(f)).sqrt();
            fh[f] = 0.5 * (qa(l) + qa(r)) - 0.5 * a * (eta(r) - eta(l));
            fe[f] = 0.5 * (eta(l) + eta(r)) - 0.5 * (qa(r) - qa(l)) / a;
        }
        let mut dh = vec![0.0; N];
        let mut dq = vec![0.0; N];
        for i in 0..N {
            let db = (self.b[(i + 1).min(N - 1)] - self.b[i.saturating_sub(1)])
                / (DX * if i == 0 || i == N - 1 { 1.0 } else { 2.0 });
            dh[i] = -(fh[i + 1] - fh[i]) / DX - q[i] * db / self.b[i];
            dq[i] = -G * depth(i) * (fe[i + 1] - fe[i]) / DX;
        }
        (dh, dq)
    }

    /// The tide's front near Yanguan: where the water ahead of it (up-river) is lowest against the water
    /// behind, over a few cells. Its distance up the estuary (m), the step in level (m) and how steep it
    /// is (the level's rise over 300 m), or None when nothing is rising toward Yanguan.
    pub fn front(&self) -> Option<Front> {
        let lo = ((YANGUAN - 16_000.0) / DX) as usize;
        let hi = ((YANGUAN + 9_000.0) / DX) as usize;
        let mut best = (0.0, lo);
        for i in lo..hi.min(N - 3) {
            let j = self.eta(i - 3) - self.eta(i + 3);
            if j > best.0 {
                best = (j, i);
            }
        }
        if best.0 < 0.08 {
            return None;
        }
        let i = best.1;
        // the level ahead and behind, clear of the smear
        let ahead = self.eta((i + 8).min(N - 1));
        let behind = (i.saturating_sub(8)..i.saturating_sub(1))
            .map(|k| self.eta(k))
            .fold(f64::MIN, f64::max)
            .max(self.eta(i.saturating_sub(8)));
        let h1 = self.h[(i + 8).min(N - 1)].max(0.2);
        let h2 = h1 + (behind - ahead).max(0.0);
        // a bore's Froude number from the depths either side of it (the hydraulic jump's relation)
        let r = h2 / h1;
        let froude = (0.5 * r * (r + 1.0)).sqrt();
        // where in its cells the front sits: weighted by each face's drop
        let mut num = 0.0;
        let mut den = 0.0;
        for k in i.saturating_sub(1)..(i + 3).min(N - 1) {
            let d = (self.eta(k) - self.eta(k + 1)).max(0.0);
            num += d * (k as f64 + 1.0) * DX;
            den += d;
        }
        Some(Front {
            x: if den > 0.0 { num / den } else { Tide::x(i) },
            ahead,
            behind,
            steep: best.0,
            froude,
            depth: h1,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Front {
    pub x: f64,
    pub ahead: f64,
    pub behind: f64,
    pub steep: f64,
    pub froude: f64,
    pub depth: f64,
}

impl Front {
    pub fn height(&self) -> f64 {
        (self.behind - self.ahead).max(0.0)
    }
}

fn minmod(a: f64, b: f64) -> f64 {
    if a * b <= 0.0 {
        0.0
    } else if a.abs() < b.abs() {
        a
    } else {
        b
    }
}

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The moon's ages the library keeps a tide for: one every two days of the month.
pub const AGES: usize = 15;

/// A tide as kept in the library: the moon's age, the switches, the time, and every cell's water.
pub struct Snap {
    pub moon: f64,
    pub on: [bool; 2],
    pub s: f64,
    pub h: Vec<f32>,
    pub q: Vec<f32>,
}

/// Where in the library the tide for these switches and this age is.
pub fn slot(on: [bool; 2], age: usize) -> usize {
    (on[0] as usize * 2 + on[1] as usize) * AGES + age
}

/// The moon's age (0..1) for the library's `age`th tide.
pub fn age_moon(age: usize) -> f64 {
    age as f64 / AGES as f64
}

impl Tide {
    /// The tide a snap kept.
    pub fn from_snap(snap: &Snap) -> Tide {
        let mut t = Tide::new(snap.moon, snap.on);
        t.s = snap.s;
        for i in 0..N {
            t.h[i] = snap.h[i] as f64;
            t.q[i] = snap.q[i] as f64;
        }
        t
    }

    pub fn snap(&self) -> Snap {
        Snap {
            moon: self.moon,
            on: self.on,
            s: self.s,
            h: self.h.iter().map(|&v| v as f32).collect(),
            q: self.q.iter().map(|&v| v as f32).collect(),
        }
    }

    /// The tide `slot` keeps: spun up from rest over two tides, then run on until its front is `ahead`
    /// metres short of Yanguan (or, with no front, until the flood begins to rise there).
    pub fn for_library(moon: f64, on: [bool; 2], ahead: f64) -> Tide {
        let mut t = Tide::new(moon, on);
        t.run(PERIOD * 2.0);
        let mut low = f64::MAX;
        let mut was = t.eta_at(YANGUAN);
        for _ in 0..(PERIOD * 1.5 / 20.0) as usize {
            t.run(20.0);
            if let Some(f) = t.front().filter(|f| f.steep > 0.15) {
                let gap = YANGUAN - f.x;
                if gap > 0.0 && gap <= ahead {
                    return t;
                }
                continue;
            }
            let e = t.eta_at(YANGUAN);
            low = low.min(e);
            // no front: the flood's first rise at Yanguan
            if e - was > 0.0 && e > low + 0.03 && t.front().is_none() {
                return t;
            }
            was = e;
        }
        t
    }
}

/// The library as bytes: "BORE", the cell count and the snaps, little-endian.
pub fn write_library(snaps: &[Snap]) -> Vec<u8> {
    let mut out = b"BORE".to_vec();
    out.extend((N as u32).to_le_bytes());
    out.extend((snaps.len() as u32).to_le_bytes());
    for s in snaps {
        out.extend((s.moon as f32).to_le_bytes());
        out.extend(((s.on[0] as u32) | (s.on[1] as u32) << 1).to_le_bytes());
        out.extend(s.s.to_le_bytes());
        for v in s.h.iter().chain(&s.q) {
            out.extend(v.to_le_bytes());
        }
    }
    out
}

pub fn read_library(b: &[u8]) -> anyhow::Result<Vec<Snap>> {
    anyhow::ensure!(b.len() >= 12 && &b[..4] == b"BORE", "not a tide library");
    let u = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    anyhow::ensure!(u(4) as usize == N, "the library's cells aren't this tide's");
    let count = u(8) as usize;
    let each = 16 + 8 * N;
    anyhow::ensure!(b.len() == 12 + count * each, "the library is cut short");
    Ok((0..count)
        .map(|k| {
            let o = 12 + k * each;
            let on = u(o + 4);
            let mut s = [0u8; 8];
            s.copy_from_slice(&b[o + 8..o + 16]);
            Snap {
                moon: f(o) as f64,
                on: [on & 1 != 0, on & 2 != 0],
                s: f64::from_le_bytes(s),
                h: (0..N).map(|i| f(o + 16 + 4 * i)).collect(),
                q: (0..N).map(|i| f(o + 16 + 4 * (N + i))).collect(),
            }
        })
        .collect())
}
