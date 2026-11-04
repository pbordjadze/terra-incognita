//! The river: a centreline that migrates by the Howard–Knutson (1984) curvature model, as Sylvester,
//! Durkin and Covault (2019, meanderpy) write it, and the oxbow lakes it leaves.
//!
//! The channel is a line of nodes, upstream first, about half a width apart, in metres on the sheet
//! (x right, y down, the sheet's centre at 0). Each step every node moves sideways by
//!
//!   R1(s) = omega * R0(s) + gamma * sum_j R0(s - xi_j) exp(-alpha xi_j) / sum_j exp(-alpha xi_j),
//!   R0 = kl * W * C(s),
//!
//! C the line's curvature, xi the distance upstream: a bank erodes where the flow bends, and more where
//! the bend upstream sends the fast water against it, so loops grow and travel down the valley. Then the
//! line is resampled at an even spacing and smoothed a little (the critic's warning: a jagged line or one
//! that crosses itself looks broken). Where two parts of the line, far apart along it, come within a
//! width of each other the neck is cut: the loop between them leaves the river as an oxbow lake.
//! The two ends are held (they are off the screen).

pub const W: f64 = 260.0; // the channel's width, m (the Beni's is 200-400 m here)
const DS: f64 = W * 0.5; // node spacing, m
const KL: f64 = 60.0; // migration rate constant, m a year (meanderpy's default)
const CF: f64 = 0.022; // friction factor
const DEPTH: f64 = 16.0; // m
const OMEGA: f64 = -1.0;
const GAMMA: f64 = 2.5;
const CUT: f64 = W * 1.0; // a neck narrower than this (centreline to centreline: the banks touch) is cut
const HELD: usize = 3; // nodes held at each end
/// The meander belt on the sheet (the young forest between the old terraces and pastures): its axis's
/// x and half its width, m. Banks outside it (older, cohesive ground) give way a quarter as fast.
/// Nodes apart along the river before two parts of it can be a neck (5 widths).
const GAP: usize = (W * 5.0 / DS) as usize;

pub const BELT_X: f64 = 1100.0;
pub const BELT_HALF: f64 = 4600.0;

/// How easily the bank at `p` erodes, 0.25..1.
pub fn erodible(p: [f64; 2]) -> f64 {
    let out = ((p[0] - BELT_X).abs() - BELT_HALF).max(0.0);
    0.25 + 0.75 * (-out / 1200.0).exp()
}

#[derive(Clone)]
pub struct Oxbow {
    pub line: Vec<[f64; 2]>,
    /// The year it was cut off (or abandoned, for a whole old course).
    pub born: f64,
    /// A whole course left by a new one, not a cut loop.
    pub course: bool,
}

pub struct River {
    pub line: Vec<[f64; 2]>,
    pub oxbows: Vec<Oxbow>,
    /// Years since the start.
    pub year: f64,
    /// The years a cutoff happened (the toy flashes them), as (year, where).
    pub cuts: Vec<(f64, [f64; 2])>,
    rng: u64,
    r0: Vec<f64>,
    r1: Vec<f64>,
}

pub fn rand(s: &mut u64) -> f64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

impl River {
    pub fn new(line: Vec<[f64; 2]>, seed: u64) -> River {
        let mut r = River { line, oxbows: Vec::new(), year: 0.0, cuts: Vec::new(), rng: seed | 1, r0: Vec::new(), r1: Vec::new() };
        r.line = resample(&r.line, DS);
        r
    }

    /// A new course: the old one is left as an abandoned channel.
    pub fn relay(&mut self, line: Vec<[f64; 2]>) {
        let old = std::mem::take(&mut self.line);
        if old.len() > 4 {
            self.oxbows.push(Oxbow { line: old, born: self.year, course: true });
        }
        self.line = resample(&line, DS);
    }

    pub fn rng(&mut self) -> f64 {
        rand(&mut self.rng)
    }

    /// Moves the river on by `dt` years (in steps no longer than `max_dt`).
    pub fn advance(&mut self, dt: f64) {
        let n = (dt / 0.5).ceil().max(1.0) as usize;
        for _ in 0..n {
            self.step(dt / n as f64);
        }
    }

    fn step(&mut self, dt: f64) {
        let n = self.line.len();
        if n < 8 {
            self.year += dt;
            return;
        }
        let p = &self.line;
        // tangents, arclength steps, curvature (central differences)
        let mut ds = vec![DS; n];
        for i in 1..n {
            ds[i] = dist(p[i], p[i - 1]).max(1.0);
        }
        self.r0.resize(n, 0.0);
        self.r1.resize(n, 0.0);
        let mut tang = vec![[0.0, 0.0]; n];
        for i in 0..n {
            let a = p[i.saturating_sub(1)];
            let b = p[(i + 1).min(n - 1)];
            let d = [b[0] - a[0], b[1] - a[1]];
            let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-6);
            tang[i] = [d[0] / l, d[1] / l];
        }
        for i in 1..n - 1 {
            // the turn of the tangent over the arc between the neighbours
            let (a, b) = (tang[i - 1], tang[i + 1]);
            let cross = a[0] * b[1] - a[1] * b[0];
            let dot = a[0] * b[0] + a[1] * b[1];
            let ang = cross.atan2(dot);
            let c = ang / (ds[i] + ds[i + 1]);
            self.r0[i] = KL * W * c;
        }
        self.r0[0] = self.r0[1];
        self.r0[n - 1] = self.r0[n - 2];
        let alpha = 2.0 * CF / DEPTH;
        for i in 0..n {
            let (mut num, mut den) = (0.0, 0.0);
            let mut xi = 0.0;
            let mut j = i;
            loop {
                let g = (-alpha * xi).exp();
                if g < 0.01 {
                    break;
                }
                num += self.r0[j] * g;
                den += g;
                if j == 0 {
                    break;
                }
                xi += ds[j];
                j -= 1;
            }
            self.r1[i] = OMEGA * self.r0[i] + GAMMA * num / den.max(1e-9);
        }
        // move each node along its normal (the outer bank of a left turn is to the right)
        let mut q = self.line.clone();
        for i in HELD..n.saturating_sub(HELD) {
            let t = tang[i];
            let nrm = [-t[1], t[0]];
            let m = (self.r1[i] * dt * erodible(p[i])).clamp(-DS * 0.3, DS * 0.3);
            q[i] = [p[i][0] - nrm[0] * m, p[i][1] - nrm[1] * m];
        }
        // a whisper of noise: the bank is never quite even
        for i in HELD..n.saturating_sub(HELD) {
            let t = tang[i];
            let e = (rand(&mut self.rng) - 0.5) * 0.8 * dt;
            q[i][0] += -t[1] * e;
            q[i][1] += t[0] * e;
        }
        smooth(&mut q, 0.12);
        self.line = resample(&q, DS);
        self.year += dt;
        self.cutoffs();
        self.erase_crossed();
    }

    /// Where the river now runs over a lake, the lake is gone (the channel took it): its line is cut
    /// there, and pieces too short to read are dropped.
    fn erase_crossed(&mut self) {
        if self.oxbows.is_empty() {
            return;
        }
        let cell = W;
        let mut grid: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();
        for p in &self.line {
            grid.insert(((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64));
        }
        let near = |q: [f64; 2], line: &[[f64; 2]]| {
            let (cx, cy) = ((q[0] / cell).floor() as i64, (q[1] / cell).floor() as i64);
            (-1..=1).any(|dx| (-1..=1).any(|dy| grid.contains(&(cx + dx, cy + dy)))) && line.iter().any(|p| dist(*p, q) < W * 0.75)
        };
        let mut out = Vec::with_capacity(self.oxbows.len());
        for o in self.oxbows.drain(..) {
            let mut piece: Vec<[f64; 2]> = Vec::new();
            let mut cut = false;
            for &q in &o.line {
                if near(q, &self.line) {
                    cut = true;
                    if piece.len() >= 5 {
                        out.push(Oxbow { line: std::mem::take(&mut piece), born: o.born, course: o.course });
                    }
                    piece.clear();
                } else {
                    piece.push(q);
                }
            }
            if !cut {
                out.push(Oxbow { line: piece, born: o.born, course: o.course });
            } else if piece.len() >= 5 {
                out.push(Oxbow { line: piece, born: o.born, course: o.course });
            }
        }
        self.oxbows = out;
    }

    fn cutoffs(&mut self) {
        loop {
            let n = self.line.len();
            let cell = CUT;
            let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
            for (i, p) in self.line.iter().enumerate() {
                grid.entry(((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64)).or_default().push(i);
            }
            // the first (most upstream) neck: i < j, far apart along the line, close on the ground
            let gap = GAP;
            let mut found: Option<(usize, usize)> = None;
            'outer: for i in 0..n {
                let p = self.line[i];
                let (cx, cy) = ((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64);
                let mut best: Option<usize> = None;
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if let Some(v) = grid.get(&(cx + dx, cy + dy)) {
                            for &j in v {
                                if j > i + gap && dist(p, self.line[j]) < CUT {
                                    best = Some(best.map_or(j, |b: usize| b.max(j)));
                                }
                            }
                        }
                    }
                }
                if let Some(j) = best {
                    found = Some((i, j));
                    break 'outer;
                }
            }
            let Some((i, j)) = found else { return };
            let loop_line: Vec<[f64; 2]> = self.line[i..=j].to_vec();
            let at = [(self.line[i][0] + self.line[j][0]) * 0.5, (self.line[i][1] + self.line[j][1]) * 0.5];
            let mut new_line = self.line[..=i].to_vec();
            new_line.extend_from_slice(&self.line[j..]);
            self.line = resample(&new_line, DS);
            smooth(&mut self.line, 0.25);
            self.oxbows.push(Oxbow { line: loop_line, born: self.year, course: false });
            self.cuts.push((self.year, at));
        }
    }

    /// The necks closing: where two parts of the river far apart along it are within 2.5 widths, the
    /// midpoint and how close (0 at 2.5 widths, 1 at the cut). One a neck (its closest pair).
    pub fn necks(&self) -> Vec<([f64; 2], f64)> {
        let n = self.line.len();
        let reach = W * 2.5;
        let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
        for (i, p) in self.line.iter().enumerate() {
            grid.entry(((p[0] / reach).floor() as i64, (p[1] / reach).floor() as i64)).or_default().push(i);
        }
        let mut best: Vec<(usize, usize, f64)> = Vec::new();
        for i in 0..n {
            let p = self.line[i];
            let (cx, cy) = ((p[0] / reach).floor() as i64, (p[1] / reach).floor() as i64);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(v) = grid.get(&(cx + dx, cy + dy)) {
                        for &j in v {
                            if j > i + GAP {
                                let d = dist(p, self.line[j]);
                                if d < reach {
                                    // the same neck if both ends are near one already found
                                    match best.iter_mut().find(|b| b.0.abs_diff(i) < GAP && b.1.abs_diff(j) < GAP) {
                                        Some(b) if b.2 <= d => {}
                                        Some(b) => *b = (i, j, d),
                                        None => best.push((i, j, d)),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        best.iter()
            .map(|&(i, j, d)| {
                let (a, b) = (self.line[i], self.line[j]);
                ([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5], ((reach - d) / (reach - CUT)).clamp(0.0, 1.0))
            })
            .collect()
    }

    /// A finger's nudge: the bank near `at` pushed toward it (up to `amount` m, fading over 1.2 km of
    /// river either side), so a bend starts there.
    pub fn nudge(&mut self, at: [f64; 2], amount: f64) {
        let n = self.line.len();
        if n < 8 {
            return;
        }
        let Some((k, d)) = self.line.iter().enumerate().map(|(i, p)| (i, dist(*p, at))).min_by(|a, b| a.1.total_cmp(&b.1)) else { return };
        if d > W * 6.0 || k < HELD || k >= n - HELD {
            return;
        }
        let dir = [(at[0] - self.line[k][0]) / d.max(1.0), (at[1] - self.line[k][1]) / d.max(1.0)];
        let push = amount.min(d);
        for i in HELD..n - HELD {
            let s = (i as f64 - k as f64) * DS;
            let g = (-(s * s) / (2.0 * 600.0 * 600.0)).exp();
            self.line[i][0] += dir[0] * push * g;
            self.line[i][1] += dir[1] * push * g;
        }
        self.line = resample(&self.line, DS);
    }

    /// The channel's length, m.
    pub fn length(&self) -> f64 {
        self.line.windows(2).map(|w| dist(w[0], w[1])).sum()
    }
}

pub fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Laplacian smoothing, the ends held.
pub fn smooth(p: &mut [[f64; 2]], k: f64) {
    let n = p.len();
    if n < 3 {
        return;
    }
    let o = p.to_vec();
    for i in 1..n - 1 {
        for c in 0..2 {
            p[i][c] = o[i][c] + k * (0.5 * (o[i - 1][c] + o[i + 1][c]) - o[i][c]);
        }
    }
}

/// The line again at an even spacing along its length (the last point kept).
pub fn resample(p: &[[f64; 2]], ds: f64) -> Vec<[f64; 2]> {
    if p.len() < 2 {
        return p.to_vec();
    }
    let mut out = vec![p[0]];
    let mut carry = 0.0; // distance walked since the last output point
    for w in p.windows(2) {
        let l = dist(w[0], w[1]);
        if l <= 0.0 {
            continue;
        }
        let mut s = ds - carry;
        while s <= l {
            let t = s / l;
            out.push([w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t]);
            s += ds;
        }
        carry = l - (s - ds);
    }
    let last = *p.last().unwrap();
    if dist(*out.last().unwrap(), last) > ds * 0.3 {
        out.push(last);
    } else {
        *out.last_mut().unwrap() = last;
    }
    out
}
