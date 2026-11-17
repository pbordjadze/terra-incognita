//! Barrier fences on the forest floor (metres): the island's own along the road, and the ones a finger
//! draws. A crab that walks into one turns along it, the way that ends sooner, and walks on round its
//! end; with the rule off it only presses against it, and the crabs pile up there.

/// A crab touches a fence this close (metres) and is held this far off it.
const TOUCH: f32 = 0.1;
const HOLD: f32 = 0.09;
/// The fences' grid cell (metres) and the most fence a finger can lay (segments).
const CELL: f32 = 0.5;
const MOST: usize = 900;

#[derive(Clone, Copy)]
struct Seg {
    a: [f32; 2],
    b: [f32; 2],
    /// how far along its fence `a` is, and the fence's length (metres)
    at: f32,
    line: usize,
}

#[derive(Default)]
pub struct Fences {
    pub lines: Vec<Vec<[f32; 2]>>,
    lengths: Vec<f32>,
    segs: Vec<Seg>,
    /// the grid's cells, each the segments near it, and its origin and size (cells)
    cells: Vec<Vec<u32>>,
    origin: [f32; 2],
    dims: [usize; 2],
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

impl Fences {
    pub fn add_line(&mut self, pts: Vec<[f32; 2]>) {
        self.lines.push(pts);
        self.rebuild();
    }

    /// Starts a fence the finger is laying at `p`.
    pub fn begin(&mut self, p: [f32; 2]) {
        self.lines.push(vec![p]);
    }

    /// The finger's fence reaches `p`: a post every 15 cm or so.
    pub fn extend(&mut self, p: [f32; 2]) {
        let Some(line) = self.lines.last_mut() else {
            return;
        };
        let last = *line.last().unwrap_or(&p);
        let d = sub(p, last);
        if dot(d, d) < 0.15 * 0.15 {
            return;
        }
        line.push(p);
        // the oldest drawn fence comes down when there is too much (the first two are the island's)
        while self.lines.iter().map(|l| l.len()).sum::<usize>() > MOST && self.lines.len() > 3 {
            self.lines.remove(2);
        }
        self.rebuild();
    }

    /// Drops a fence of one post (a tap, not a drag).
    pub fn end(&mut self) {
        if self.lines.last().is_some_and(|l| l.len() < 2) {
            self.lines.pop();
        }
        self.rebuild();
    }

    fn rebuild(&mut self) {
        self.segs.clear();
        self.lengths.clear();
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for (li, line) in self.lines.iter().enumerate() {
            let mut at = 0.0;
            for w in line.windows(2) {
                self.segs.push(Seg {
                    a: w[0],
                    b: w[1],
                    at,
                    line: li,
                });
                at += dot(sub(w[1], w[0]), sub(w[1], w[0])).sqrt();
                for p in w {
                    for k in 0..2 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
            }
            self.lengths.push(at);
        }
        self.cells.clear();
        if self.segs.is_empty() {
            self.dims = [0, 0];
            return;
        }
        self.origin = [lo[0] - CELL, lo[1] - CELL];
        self.dims = [
            ((hi[0] - lo[0]) / CELL) as usize + 3,
            ((hi[1] - lo[1]) / CELL) as usize + 3,
        ];
        self.cells = vec![Vec::new(); self.dims[0] * self.dims[1]];
        for (i, s) in self.segs.iter().enumerate() {
            let x0 = ((s.a[0].min(s.b[0]) - TOUCH - self.origin[0]) / CELL) as usize;
            let x1 = ((s.a[0].max(s.b[0]) + TOUCH - self.origin[0]) / CELL) as usize;
            let y0 = ((s.a[1].min(s.b[1]) - TOUCH - self.origin[1]) / CELL) as usize;
            let y1 = ((s.a[1].max(s.b[1]) + TOUCH - self.origin[1]) / CELL) as usize;
            for y in y0..=y1.min(self.dims[1] - 1) {
                for x in x0..=x1.min(self.dims[0] - 1) {
                    self.cells[y * self.dims[0] + x].push(i as u32);
                }
            }
        }
    }

    /// A crab at `p` walking at `v` meets the fences: it is held off them and, if it `follows`, turns
    /// along the one it touches (`along` keeps which way). Whether it touched one.
    pub fn touch(
        &self,
        p: &mut [f32; 2],
        v: &mut [f32; 2],
        along: &mut f32,
        follows: bool,
    ) -> bool {
        if self.dims[0] == 0 {
            return false;
        }
        let x = (p[0] - self.origin[0]) / CELL;
        let y = (p[1] - self.origin[1]) / CELL;
        if x < 0.0 || y < 0.0 || x as usize >= self.dims[0] || y as usize >= self.dims[1] {
            return false;
        }
        let mut hit = false;
        for &i in &self.cells[y as usize * self.dims[0] + x as usize] {
            let s = self.segs[i as usize];
            let ab = sub(s.b, s.a);
            let len2 = dot(ab, ab).max(1e-8);
            let u = (dot(sub(*p, s.a), ab) / len2).clamp(0.0, 1.0);
            let q = [s.a[0] + ab[0] * u, s.a[1] + ab[1] * u];
            let d = sub(*p, q);
            let l = dot(d, d).sqrt();
            if l >= TOUCH || l < 1e-6 {
                continue;
            }
            let n = [d[0] / l, d[1] / l];
            *p = [q[0] + n[0] * HOLD.max(l), q[1] + n[1] * HOLD.max(l)];
            let into = dot(*v, n);
            if into >= 0.0 {
                continue;
            }
            hit = true;
            if follows {
                let len = len2.sqrt();
                let t = [ab[0] / len, ab[1] / len];
                if *along == 0.0 {
                    let pos = s.at + u * len;
                    *along = if pos < self.lengths[s.line] * 0.5 {
                        -1.0
                    } else {
                        1.0
                    };
                }
                let speed = dot(*v, *v).sqrt();
                *v = [t[0] * *along * speed, t[1] * *along * speed];
            } else {
                *v = [(v[0] - n[0] * into) * 0.1, (v[1] - n[1] * into) * 0.1];
            }
        }
        hit
    }
}
