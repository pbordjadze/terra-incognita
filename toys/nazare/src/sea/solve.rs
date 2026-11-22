//! A swell's field solved a part at a time (`Field::with_rules` runs every part at once): each part a few
//! milliseconds on a phone, so a browser's one thread never stalls on it (the opening's warming, a new swell
//! asked for on the web, where there is no thread to solve it on). The same steps in the same order as
//! solved in one go, so the same field.

use super::*;

/// Rows of the floor a part of the per-cell work takes.
const ROWS: usize = 24;
/// Rays a part traces.
const RAYS: usize = 96;
/// Cells a part of the search for the nearest source settles.
const SETTLED: usize = 24_000;

/// The search for each sea cell's nearest source over the sea (Dijkstra on the eight neighbours), as far as
/// it has gone: how far each is (cells), which source it is, and the cells still to settle.
struct Nearest {
    dist: Vec<f64>,
    from: Vec<usize>,
    heap: std::collections::BinaryHeap<std::cmp::Reverse<(u64, usize)>>,
}

pub struct Solve {
    period: f64,
    from_deg: f64,
    rules: [bool; 2],
    /// Which step is next, and how far into it (a row, a round).
    step: usize,
    at: usize,
    d: [f64; 2],
    cg0: f64,
    phase: Vec<f64>,
    group: Vec<f64>,
    slow: Vec<f64>,
    slow_g: Vec<f64>,
    speed: Vec<f64>,
    smooth: Vec<f64>,
    walls: Vec<bool>,
    crowd: Vec<f64>,
    /// the next ray's offset across the swell (cells)
    ray: f64,
    height: Vec<f32>,
    tau_back: Vec<f64>,
    height_back: Vec<f32>,
    /// the turned-back train's sources (the sea's cells on the wall's face), and their nearest
    sources: Vec<usize>,
    nearest: Option<Nearest>,
}

/// How far upstream of the floor's middle the rays start, and how far across it they reach (cells).
fn reach(floor: &Floor) -> f64 {
    0.5 * (floor.w as f64).hypot(floor.h as f64) + 2.0
}

impl Solve {
    pub fn new(floor: &Floor, period: f64, from_deg: f64, rules: [bool; 2]) -> Solve {
        let n = floor.w * floor.h;
        Solve {
            period,
            from_deg,
            rules,
            step: 0,
            at: 0,
            d: heading(from_deg),
            cg0: speeds(period, 4000.0).1,
            phase: vec![f64::INFINITY; n],
            group: vec![f64::INFINITY; n],
            slow: vec![0.0; n],
            slow_g: vec![0.0; n],
            speed: vec![0.0; n],
            smooth: Vec::new(),
            walls: Vec::new(),
            crowd: Vec::new(),
            ray: 0.0,
            height: vec![0.0; n],
            tau_back: Vec::new(),
            height_back: Vec::new(),
            sources: Vec::new(),
            nearest: None,
        }
    }

    /// The depth the swell feels: with the shallows not slowing it, the open ocean's everywhere.
    fn felt(&self, floor: &Floor, i: usize) -> f64 {
        if self.rules[0] {
            floor.depth(i)
        } else {
            4000.0
        }
    }

    fn back(&self) -> bool {
        self.rules[1] && reflects(self.period)
    }

    /// The next rows (`ROWS`) of the cells from `self.at`, by `f`; true once the last row is done.
    fn rows(&mut self, floor: &Floor, f: impl Fn(&mut Solve, usize)) -> bool {
        let end = (self.at + ROWS).min(floor.h);
        for i in self.at * floor.w..end * floor.w {
            f(self, i);
        }
        self.at = end;
        end == floor.h
    }

    /// The next part; true once the field is solved (`field`).
    pub fn part(&mut self, floor: &Floor) -> bool {
        if self.step > 15 {
            return true;
        }
        let done = if self.step < 8 {
            self.crests_part(floor)
        } else {
            self.heights_part(floor)
        };
        if done {
            // (with no train turned back, its crests are none and its height nothing)
            self.step = if self.step == 8 && !self.back() {
                let n = floor.w * floor.h;
                (self.tau_back, self.height_back) = (vec![f64::INFINITY; n], vec![0.0; n]);
                14
            } else {
                self.step + 1
            };
            self.at = 0;
        }
        self.step > 15
    }

    /// A part of the arriving train's crests and the rays (steps 0 to 7); true once its step is done.
    fn crests_part(&mut self, floor: &Floor) -> bool {
        match self.step {
            0 => self.rows(floor, |s, i| s.speeds_at(floor, i)),
            1 => {
                let (c0, cg0) = speeds(self.period, 4000.0);
                seed(floor, self.d, floor.cell_m / c0, &mut self.phase);
                seed(floor, self.d, floor.cell_m / cg0, &mut self.group);
                true
            }
            2 => {
                sweep(floor, &self.slow, &mut self.phase, 1);
                self.at += 1;
                self.at == ROUNDS
            }
            3 => {
                // the sets' timing needs less care than the crests'
                sweep(floor, &self.slow_g, &mut self.group, 1);
                true
            }
            // the first arrival kinks where a faster path overtakes: a real crest there is a blend of both
            // arrivals, diffraction rounding the corner over about a wavelength
            4 => {
                smooth_sea(floor, &mut self.phase, SMOOTH);
                true
            }
            5 => {
                smooth_sea(floor, &mut self.group, SMOOTH);
                true
            }
            6 => {
                self.smooth_speeds(floor);
                true
            }
            7 => self.rays(floor),
            _ => true,
        }
    }

    /// A part of the trains' heights and the train turned back (steps 8 to 15); true once its step is done.
    fn heights_part(&mut self, floor: &Floor) -> bool {
        match self.step {
            8 => self.rows(floor, |s, i| s.height_at(floor, i)),
            9 => {
                self.sources(floor);
                true
            }
            10 => {
                sweep(floor, &self.slow, &mut self.tau_back, 1);
                self.at += 1;
                self.at == ROUNDS
            }
            11 => self.nearest_part(floor),
            12 => self.rows(floor, |s, i| s.back_height_at(floor, i)),
            13 => {
                let mut hb: Vec<f64> = self.height_back.iter().map(|&v| v as f64).collect();
                blur(&mut hb, floor.w, floor.h);
                for (i, v) in hb.iter().enumerate() {
                    self.height_back[i] = if floor.sea(i) && !self.walls[i] {
                        *v as f32
                    } else {
                        0.0
                    };
                }
                true
            }
            14 => {
                smooth_sea(floor, &mut self.tau_back, SMOOTH);
                true
            }
            15 => self.rows(floor, |s, i| s.spent_at(floor, i)),
            _ => true,
        }
    }

    /// The rays see the floor smoothed over about a wavelength: a swell does not feel a single rock. And
    /// the wall, where the canyon turns the swell back.
    fn smooth_speeds(&mut self, floor: &Floor) {
        let mut smooth = self.speed.clone();
        blur(&mut smooth, floor.w, floor.h);
        blur(&mut smooth, floor.w, floor.h);
        for (i, s) in smooth.iter_mut().enumerate() {
            if !floor.sea(i) {
                *s = 0.0;
            }
        }
        self.smooth = smooth;
        let back = self.back();
        self.walls = (0..floor.w * floor.h)
            .map(|i| back && floor.canyon[i])
            .collect();
        self.crowd = vec![0.0; floor.w * floor.h];
        self.ray = -reach(floor);
    }

    /// How the rays crowd, `RAYS` of them a part; true once all are traced. Each ray is the swell's own: it
    /// starts upstream of the floor, RAY apart across the swell, and turns toward the slower water as it
    /// goes (dv/ds = −(∇c − (∇c·v) v) / c), depositing once a STEP, so open water holds 1 / (RAY · STEP) a
    /// cell; more where rays converge. The wall ends it.
    fn rays(&mut self, floor: &Floor) -> bool {
        let d = self.d;
        let across = [-d[1], d[0]];
        let centre = [0.5 * floor.w as f64, 0.5 * floor.h as f64];
        let reach = reach(floor);
        for _ in 0..RAYS {
            if self.ray >= reach {
                break;
            }
            let s = self.ray;
            // from a line across the swell, upstream of the floor, straight in until it is over the sea
            let mut p = [
                centre[0] + across[0] * s - d[0] * reach,
                centre[1] + across[1] * s - d[1] * reach,
            ];
            for _ in 0..(4.0 * reach / STEP) as usize {
                if at(floor, &self.smooth, p).is_some() {
                    trace(floor, &self.smooth, &self.walls, &mut self.crowd, p, d);
                    break;
                }
                p = [p[0] + d[0] * STEP, p[1] + d[1] * STEP];
            }
            self.ray += RAY;
        }
        if self.ray < reach {
            return false;
        }
        blur(&mut self.crowd, floor.w, floor.h);
        true
    }

    /// The train the wall turns back starts: its sources are the sea's cells on the wall's face, each
    /// starting when the arriving crest reaches it and as tall as the arriving wave there; from them its
    /// crests are a second eikonal over the sea (`sweep`, next), and each cell takes its nearest source's
    /// height (as the arriving wave there grew to it, shoaling on from the face), halving over SPREAD_M.
    fn sources(&mut self, floor: &Floor) {
        let (w, h) = (floor.w, floor.h);
        let n = w * h;
        let wall = &self.walls;
        self.sources = (0..n)
            .filter(|&i| {
                !wall[i] && self.slow[i] > 0.0 && self.phase[i].is_finite() && self.height[i] > 0.0
            })
            .filter(|&i| on_face(wall, w, h, i))
            .collect();
        self.tau_back = vec![f64::INFINITY; n];
        for &i in &self.sources {
            self.tau_back[i] = self.phase[i];
        }
        let mut nb = Nearest {
            dist: vec![f64::INFINITY; n],
            from: vec![usize::MAX; n],
            heap: std::collections::BinaryHeap::new(),
        };
        for &i in &self.sources {
            nb.dist[i] = 0.0;
            nb.from[i] = i;
            nb.heap.push(std::cmp::Reverse((0u64, i)));
        }
        self.nearest = Some(nb);
        self.height_back = vec![0.0; n];
    }

    /// The search for each sea cell's nearest source, `SETTLED` cells a part; true once done.
    fn nearest_part(&mut self, floor: &Floor) -> bool {
        let Some(nb) = self.nearest.as_mut() else {
            return true;
        };
        let (w, h) = (floor.w as i64, floor.h as i64);
        for _ in 0..SETTLED {
            let Some(std::cmp::Reverse((dk, i))) = nb.heap.pop() else {
                return true;
            };
            let di = f64::from_bits(dk);
            if di > nb.dist[i] {
                continue;
            }
            let (x, y) = ((i % floor.w) as i64, (i / floor.w) as i64);
            for (dx, dy) in [
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = (ny * w + nx) as usize;
                if self.slow[j] <= 0.0 {
                    continue;
                }
                let nd = di
                    + if dx * dy != 0 {
                        std::f64::consts::SQRT_2
                    } else {
                        1.0
                    };
                if nd < nb.dist[j] {
                    nb.dist[j] = nd;
                    nb.from[j] = nb.from[i];
                    nb.heap.push(std::cmp::Reverse((nd.to_bits(), j)));
                }
            }
        }
        nb.heap.is_empty()
    }

    /// The turned-back train's height at a cell, from its nearest source's, spreading.
    fn back_height_at(&mut self, floor: &Floor, i: usize) {
        let Some(nb) = &self.nearest else { return };
        let src = nb.from[i];
        if src == usize::MAX || !self.tau_back[i].is_finite() {
            return;
        }
        let spread = (1.0 + nb.dist[i] * floor.cell_m / SPREAD_M).sqrt().recip();
        self.height_back[i] = (self.height[src] as f64 * spread * self.shoal(floor, i)
            / self.shoal(floor, src)) as f32;
    }

    /// A cell's speeds and slownesses.
    fn speeds_at(&mut self, floor: &Floor, i: usize) {
        if !floor.sea(i) {
            return;
        }
        let felt = self.felt(floor, i);
        let (c, cg) = speeds(self.period, felt);
        self.speed[i] = c / floor.cell_m;
        if self.back() && floor.canyon[i] {
            // the rays meet the wall as a wall, not as deep water to bend away from before they
            // reach it: they see it as the shelf at its rim
            self.speed[i] =
                speeds(self.period, if self.rules[0] { RIM_M } else { felt }).0 / floor.cell_m;
        } else {
            self.slow[i] = floor.cell_m / c;
            self.slow_g[i] = floor.cell_m / cg;
        }
    }

    /// How the swell grows as it slows (shoaling, through the group speed).
    fn shoal(&self, floor: &Floor, i: usize) -> f64 {
        (self.cg0 / speeds(self.period, self.felt(floor, i)).1).sqrt()
    }

    /// The arriving train's height at a cell.
    fn height_at(&mut self, floor: &Floor, i: usize) {
        if floor.sea(i) && self.phase[i].is_finite() {
            let grow = if self.rules[0] {
                (self.crowd[i] * RAY * STEP).sqrt().min(REFRACT_MAX)
            } else {
                // nothing bends: the train keeps its own height
                (self.crowd[i] * RAY * STEP).min(1.0)
            };
            self.height[i] = (H0 * self.shoal(floor, i) * grow) as f32;
        }
    }

    /// Inside where they break the waves are broken: what is left of them is held to the depth.
    fn spent_at(&mut self, floor: &Floor, i: usize) {
        let most = (SPENT * self.felt(floor, i).min(floor.depth(i))) as f32;
        let both = self.height[i] + self.height_back[i];
        if both > most {
            self.height[i] *= most / both;
            self.height_back[i] *= most / both;
        }
    }

    /// The solved field (once `part` is done).
    pub fn field(self) -> Field {
        Field {
            period: self.period,
            from_deg: self.from_deg,
            tau: self.phase,
            tau_back: self.tau_back,
            tau_group: self.group,
            height: self.height,
            height_back: self.height_back,
            rules: self.rules,
        }
    }
}
