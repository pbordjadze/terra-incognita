//! The town and its drivers (painting-company row 255): Braess's paradox to play with.
//!
//! Cars come in at the bottom and leave at the top. Every road has a free speed and lanes; a road's
//! speed falls as its cars crowd it, `v = vf / (1 + k / k0)` with `k` the cars a lane a point and
//! `k0 = C / vf`, so every lane, fast or slow, carries at most `C` cars a second (a lane's capacity is
//! about the same whatever the speed limit, as the traffic literature measures it) and a road's time
//! grows with its load, slowly at first and then steeply near capacity (the volume-delay curve of the
//! traffic engineers; Braess 1968 took the same kind of link cost). Every driver, arriving at a
//! junction, takes the next road whose time as it looks right now, plus the quickest way on from its
//! end, is least, each seen with a little error of their own. No one plans for anyone else, and no
//! rule here knows about bridges: the equilibrium (Wardrop's) and the paradox both fall out.
//!
//! Units: points of the reference phone (393 x 852) from the screen's middle, y down; seconds.
//! A road is two directed links: `2r` from its `a` to its `b`, `2r + 1` back.

/// A lane's capacity, cars a second, whatever the road (drawn: the toy's time runs fast).
pub const CAP: f64 = 1.6;
/// The closest cars come in a lane, nose to nose (points).
pub const GAP: f64 = 7.0;
/// Cars a second coming into town.
pub const DEMAND: f64 = 3.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The road in from the bottom, and out at the top.
    Entry,
    Exit,
    /// A narrow street: quick when empty, two lanes.
    Street,
    /// The long way round: slow, wide, it never fills.
    Long,
    /// A road the reader drew: quick and wide.
    Built,
}

impl Kind {
    pub fn lanes(self) -> f64 {
        match self {
            Kind::Entry | Kind::Exit => 16.0,
            Kind::Street => 2.0,
            Kind::Long => 40.0,
            Kind::Built => 4.0,
        }
    }
    /// Free speed, points a second.
    pub fn vf(self) -> f64 {
        match self {
            Kind::Entry | Kind::Exit => 220.0,
            Kind::Street => 200.0,
            Kind::Long => 40.0,
            Kind::Built => 220.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Road {
    pub a: usize,
    pub b: usize,
    pub pts: Vec<[f64; 2]>,
    cum: Vec<f64>,
    pub len: f64,
    pub kind: Kind,
    /// Cars may enter it.
    pub open: bool,
    /// Still on the map (a drawn road that was cut goes once it is empty).
    pub alive: bool,
    /// When it was built, closed or opened (seconds), for the drawing.
    pub changed: f64,
}

impl Road {
    pub fn new(a: usize, b: usize, pts: Vec<[f64; 2]>, kind: Kind) -> Road {
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            cum.push(cum.last().unwrap() + dist(w[0], w[1]));
        }
        let len = *cum.last().unwrap();
        Road { a, b, pts, cum, len, kind, open: true, alive: true, changed: -99.0 }
    }

    /// The point `u` points along from `a`, and the way it runs there.
    pub fn at(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        let u = u.clamp(0.0, self.len);
        let i = match self.cum.binary_search_by(|c| c.total_cmp(&u)) {
            Ok(i) => i.min(self.pts.len() - 2),
            Err(i) => (i.max(1) - 1).min(self.pts.len() - 2),
        };
        let (p, q) = (self.pts[i], self.pts[i + 1]);
        let l = (self.cum[i + 1] - self.cum[i]).max(1e-9);
        let f = (u - self.cum[i]) / l;
        let d = [(q[0] - p[0]) / l, (q[1] - p[1]) / l];
        ([p[0] + (q[0] - p[0]) * f, p[1] + (q[1] - p[1]) * f], d)
    }

    /// The nearest point on the road to `q`: its distance and how far along.
    pub fn nearest(&self, q: [f64; 2]) -> (f64, f64) {
        let mut best = (f64::MAX, 0.0);
        for i in 0..self.pts.len() - 1 {
            let (p, r) = (self.pts[i], self.pts[i + 1]);
            let d = [r[0] - p[0], r[1] - p[1]];
            let l2 = (d[0] * d[0] + d[1] * d[1]).max(1e-9);
            let f = (((q[0] - p[0]) * d[0] + (q[1] - p[1]) * d[1]) / l2).clamp(0.0, 1.0);
            let x = [p[0] + d[0] * f, p[1] + d[1] * f];
            let e = dist(x, q);
            if e < best.0 {
                best = (e, self.cum[i] + f * l2.sqrt());
            }
        }
        best
    }

    /// The road's pieces before and after `u`.
    fn split(&self, u: f64) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let (m, _) = self.at(u);
        let mut a = vec![];
        let mut b = vec![m];
        for (i, p) in self.pts.iter().enumerate() {
            if self.cum[i] < u - 1e-6 {
                a.push(*p);
            } else if self.cum[i] > u + 1e-6 {
                b.push(*p);
            }
        }
        a.push(m);
        (a, b)
    }
}

#[derive(Clone, Debug)]
pub struct Car {
    pub link: usize,
    /// How far along its link (points).
    pub s: f64,
    pub v: f64,
    /// When it came into town.
    pub born: f64,
    pub seed: u32,
    /// Held at the end of its link: no way on, or no room.
    pub waiting: bool,
    /// How long its trip will take, as it looks now, against the town's usual (eased; set by the toy).
    pub late: f64,
}

pub struct Town {
    pub nodes: Vec<[f64; 2]>,
    pub roads: Vec<Road>,
    pub cars: Vec<Car>,
    pub t: f64,
    /// Where cars come in and go out.
    pub from: usize,
    pub to: usize,
    /// Each link's speed now, as its crowding gives it.
    pub speed: Vec<f64>,
    /// The quickest time on from every node, as it looks now.
    pub ahead: Vec<f64>,
    next_route: f64,
    next_car: f64,
    rng: u32,
    /// Trips finished: when, and how long they took.
    pub trips: Vec<(f64, f64)>,
}

pub fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn hash(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^ (h >> 16)
}

/// The town's first roads, point-symmetric about the lake: on the left a narrow street up from the
/// bottom junction, then the long way round to the top; on the right the long way round first, then a
/// narrow street. Nothing joins the two sides across the lake.
pub fn first_roads() -> (Vec<[f64; 2]>, Vec<Road>) {
    let o = [0.0, 520.0];
    let s0 = [0.0, 300.0];
    let a = [-98.0, 28.0];
    let b = [98.0, -28.0];
    let e0 = [0.0, -300.0];
    let d = [0.0, -520.0];
    let nodes = vec![o, s0, a, b, e0, d];
    // a narrow street: a gentle bow away from the lake
    let street = |p: [f64; 2], q: [f64; 2], bow: f64| -> Vec<[f64; 2]> {
        let m = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
        let dl = dist(p, q);
        let n = [-(q[1] - p[1]) / dl, (q[0] - p[0]) / dl];
        let c = [m[0] + n[0] * bow, m[1] + n[1] * bow];
        bezier(p, c, c, q, 28)
    };
    // the long way round: out past the edge of the screen and back
    let long = |p: [f64; 2], q: [f64; 2], side: f64| -> Vec<[f64; 2]> {
        let dy = (q[1] - p[1]).signum();
        bezier(p, [side * 330.0, p[1] + 40.0 * dy], [side * 330.0, q[1] - 10.0 * dy], q, 60)
    };
    let roads = vec![
        Road::new(0, 1, vec![o, s0], Kind::Entry),
        Road::new(1, 2, street(s0, a, -30.0), Kind::Street),
        Road::new(2, 4, long(a, e0, -1.0), Kind::Long),
        Road::new(1, 3, long(s0, b, 1.0), Kind::Long),
        Road::new(3, 4, street(b, e0, -30.0), Kind::Street),
        Road::new(4, 5, vec![e0, d], Kind::Exit),
    ];
    (nodes, roads)
}

fn bez(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    [w[0] * p0[0] + w[1] * p1[0] + w[2] * p2[0] + w[3] * p3[0], w[0] * p0[1] + w[1] * p1[1] + w[2] * p2[1] + w[3] * p3[1]]
}

fn bezier(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], n: usize) -> Vec<[f64; 2]> {
    (0..=n).map(|i| bez(p0, p1, p2, p3, i as f64 / n as f64)).collect()
}

impl Town {
    pub fn new(seed: u32) -> Town {
        let (nodes, roads) = first_roads();
        let mut t = Town {
            nodes,
            roads,
            cars: Vec::new(),
            t: 0.0,
            from: 0,
            to: 5,
            speed: Vec::new(),
            ahead: Vec::new(),
            next_route: 0.0,
            next_car: 0.0,
            rng: seed | 1,
            trips: Vec::new(),
        };
        t.route();
        t
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f64 / u32::MAX as f64
    }

    pub fn road_of(link: usize) -> usize {
        link / 2
    }

    /// A link's start and end nodes.
    pub fn ends(&self, link: usize) -> (usize, usize) {
        let r = &self.roads[link / 2];
        if link % 2 == 0 { (r.a, r.b) } else { (r.b, r.a) }
    }

    /// Where a car is, and the way it faces.
    pub fn place(&self, c: &Car) -> ([f64; 2], [f64; 2]) {
        let r = &self.roads[c.link / 2];
        if c.link % 2 == 0 {
            r.at(c.s)
        } else {
            let (p, d) = r.at(r.len - c.s);
            (p, [-d[0], -d[1]])
        }
    }

    /// Whether drivers may use a link (the way in is never driven out, nor the way out in).
    fn usable(&self, link: usize) -> bool {
        let r = &self.roads[link / 2];
        r.open && r.alive && !(link % 2 == 1 && matches!(r.kind, Kind::Entry | Kind::Exit))
    }

    /// How long a link takes now, as a driver sees it: its length at the speed its crowd gives it.
    pub fn cost(&self, link: usize) -> f64 {
        self.roads[link / 2].len / self.speed[link].max(1.0)
    }

    /// Every link's speed from its crowd, and every node's quickest time on to the top.
    pub fn route(&mut self) {
        let nl = self.roads.len() * 2;
        let mut n = vec![0usize; nl];
        for c in &self.cars {
            n[c.link] += 1;
        }
        self.speed = (0..nl)
            .map(|l| {
                let r = &self.roads[l / 2];
                let vf = r.kind.vf();
                let k = n[l] as f64 / (r.len.max(1.0) * r.kind.lanes());
                vf / (1.0 + k * vf / CAP)
            })
            .collect();
        // Dijkstra back from the way out
        let nn = self.nodes.len();
        let mut best = vec![f64::INFINITY; nn];
        let mut done = vec![false; nn];
        best[self.to] = 0.0;
        loop {
            let mut u = None;
            for i in 0..nn {
                if !done[i] && best[i].is_finite() && u.is_none_or(|j: usize| best[i] < best[j]) {
                    u = Some(i);
                }
            }
            let Some(u) = u else { break };
            done[u] = true;
            for l in 0..nl {
                if !self.usable(l) {
                    continue;
                }
                let (from, to) = self.ends(l);
                if to == u {
                    let c = best[u] + self.cost(l);
                    if c < best[from] {
                        best[from] = c;
                    }
                }
            }
        }
        self.ahead = best;
    }

    /// The road a driver at the end of `link` takes next, as it looks to them now (None: no way on).
    pub fn choose(&self, link: Option<usize>, node: usize, seed: u32) -> Option<usize> {
        let back = link.map(|l| l ^ 1);
        let mut best: Option<(f64, usize)> = None;
        let mut fallback = None;
        for l in 0..self.roads.len() * 2 {
            if !self.usable(l) || self.ends(l).0 != node {
                continue;
            }
            let on = self.ahead[self.ends(l).1];
            if !on.is_finite() {
                continue;
            }
            // each driver's own error, a few per cent, theirs for each road
            let e = hash(seed ^ (l as u32).wrapping_mul(0x9e37_79b9)) as f64 / u32::MAX as f64;
            let c = (self.cost(l) + on) * (0.88 + 0.24 * e);
            if Some(l) == back {
                fallback = Some(l);
                continue;
            }
            if best.is_none_or(|b| c < b.0) {
                best = Some((c, l));
            }
        }
        best.map(|b| b.1).or(fallback)
    }

    /// The way a car means to go from where it is: the nodes ahead, as it looks now.
    pub fn plan(&self, i: usize) -> Vec<usize> {
        let c = &self.cars[i];
        let mut link = c.link;
        let mut out = vec![];
        for _ in 0..40 {
            let node = self.ends(link).1;
            out.push(link);
            if node == self.to {
                break;
            }
            match self.choose(Some(link), node, c.seed) {
                Some(l) => link = l,
                None => break,
            }
        }
        out
    }

    /// The cars on each link, front first.
    fn queues(&self) -> Vec<Vec<usize>> {
        let mut q = vec![Vec::new(); self.roads.len() * 2];
        for (i, c) in self.cars.iter().enumerate() {
            q[c.link].push(i);
        }
        for v in q.iter_mut() {
            v.sort_by(|&a, &b| self.cars[b].s.total_cmp(&self.cars[a].s));
        }
        q
    }

    fn gap(&self, link: usize) -> f64 {
        GAP / self.roads[link / 2].kind.lanes().min(3.0)
    }

    /// Room at the start of a link.
    fn room(&self, link: usize, q: &[Vec<usize>]) -> bool {
        q[link].last().is_none_or(|&j| self.cars[j].s >= self.gap(link))
    }

    pub fn step(&mut self, dt: f64) {
        let mut left = dt;
        while left > 1e-9 {
            let h = left.min(1.0 / 60.0);
            self.tick(h);
            left -= h;
        }
    }

    fn tick(&mut self, dt: f64) {
        self.t += dt;
        if self.t >= self.next_route {
            self.next_route = self.t + 0.25;
            self.route();
        }
        let mut q = self.queues();
        let mut gone = vec![];
        for link in 0..q.len() {
            let len = self.roads[link / 2].len;
            let v_l = self.speed[link];
            let gap = self.gap(link);
            let mut limit = f64::INFINITY;
            let members = q[link].clone();
            for &i in &members {
                let c = &mut self.cars[i];
                // speed toward the road's, gently
                let want = v_l;
                let a = if want > c.v { 160.0 } else { 400.0 };
                c.v += (want - c.v).clamp(-a * dt, a * dt);
                let mut s = c.s + c.v * dt;
                if s > limit {
                    s = limit.max(c.s);
                    c.v = ((s - c.s) / dt).min(c.v);
                }
                c.s = s;
                limit = c.s - gap;
            }
            // the front car at the end of its link goes on, if it can
            if let Some(&i) = members.first() {
                if self.cars[i].s >= len {
                    let node = self.ends(link).1;
                    if node == self.to {
                        gone.push(i);
                        self.trips.push((self.t, self.t - self.cars[i].born));
                        continue;
                    }
                    let seed = self.cars[i].seed ^ (node as u32).wrapping_mul(0x85eb_ca6b);
                    match self.choose(Some(link), node, seed) {
                        Some(next) if self.room(next, &q) => {
                            let c = &mut self.cars[i];
                            c.s = (c.s - len).min(gap * 0.5);
                            c.link = next;
                            c.waiting = false;
                            q[link].remove(0);
                            q[next].push(i);
                        }
                        _ => {
                            let c = &mut self.cars[i];
                            c.s = len;
                            c.v = 0.0;
                            c.waiting = true;
                        }
                    }
                }
            }
        }
        gone.sort_unstable();
        for i in gone.into_iter().rev() {
            self.cars.swap_remove(i);
        }
        if self.trips.len() > 4000 {
            self.trips.drain(..2000);
        }
        // cars coming in
        while self.t >= self.next_car {
            let gap = 1.0 / DEMAND;
            self.next_car += gap * (0.45 + 1.1 * self.rand());
            let q = self.queues();
            let first = self.choose(None, self.from, 0).filter(|&l| self.room(l, &q));
            if let Some(l) = first {
                let seed = hash((self.t * 1000.0) as u32 ^ self.rng);
                self.cars.push(Car { link: l, s: 0.0, v: self.roads[l / 2].kind.vf() * 0.7, born: self.t, seed, waiting: false, late: 1.0 });
            }
            if self.next_car < self.t - 1.0 {
                self.next_car = self.t;
            }
        }
    }

    /// How long car `i`'s whole trip will take, as it looks now: its time so far, the rest of its road
    /// at that road's speed, and the quickest way on from its end.
    pub fn eta(&self, i: usize) -> f64 {
        let c = &self.cars[i];
        let r = &self.roads[c.link / 2];
        let rest = (r.len - c.s).max(0.0) / self.speed.get(c.link).copied().unwrap_or(1.0).max(1.0);
        let on = self.ahead.get(self.ends(c.link).1).copied().unwrap_or(0.0);
        (self.t - c.born) + rest + if on.is_finite() { on } else { 60.0 }
    }

    /// The mean trip of those finished in the last `secs` seconds.
    pub fn mean_trip(&self, secs: f64) -> Option<f64> {
        let r: Vec<f64> = self.trips.iter().filter(|(at, _)| *at > self.t - secs).map(|x| x.1).collect();
        (!r.is_empty()).then(|| r.iter().sum::<f64>() / r.len() as f64)
    }

    /// The nearest road to `q` within `reach`: (road, distance along, distance away).
    pub fn road_near(&self, q: [f64; 2], reach: f64, with_closed: bool) -> Option<(usize, f64, f64)> {
        let mut best: Option<(usize, f64, f64)> = None;
        for (i, r) in self.roads.iter().enumerate() {
            if !r.alive || (!r.open && !with_closed) {
                continue;
            }
            let (d, u) = r.nearest(q);
            if d < reach && best.is_none_or(|b| d < b.2) {
                best = Some((i, u, d));
            }
        }
        best
    }

    /// A node at `u` along road `r`: an end if it is within `snap`, else the road split there.
    pub fn node_on(&mut self, r: usize, u: f64, snap: f64) -> usize {
        let len = self.roads[r].len;
        if u < snap {
            return self.roads[r].a;
        }
        if u > len - snap {
            return self.roads[r].b;
        }
        let (pa, pb) = self.roads[r].split(u);
        let m = *pa.last().unwrap();
        let n = self.nodes.len();
        self.nodes.push(m);
        let old = self.roads[r].clone();
        let mut first = Road::new(old.a, n, pa, old.kind);
        let mut second = Road::new(n, old.b, pb, old.kind);
        for x in [&mut first, &mut second] {
            x.open = old.open;
            x.changed = old.changed;
        }
        let nr = self.roads.len();
        let u0 = first.len;
        self.roads[r] = first;
        self.roads.push(second);
        // the cars on it keep their places
        for c in self.cars.iter_mut() {
            if c.link == 2 * r {
                if c.s >= u0 {
                    c.link = 2 * nr;
                    c.s -= u0;
                }
            } else if c.link == 2 * r + 1 {
                let tail = old.len - u0;
                if c.s < tail {
                    c.link = 2 * nr + 1;
                } else {
                    c.s -= tail;
                }
            }
        }
        self.route();
        n
    }

    /// A new road from `p` to `q`, both on roads (within `reach`). Returns its index.
    pub fn build(&mut self, p: [f64; 2], q: [f64; 2], reach: f64) -> Option<usize> {
        let (ra, ua, _) = self.road_near(p, reach, true)?;
        let (rb, ub, _) = self.road_near(q, reach, true)?;
        let snap = 22.0;
        let pa = self.roads[ra].at(ua).0;
        let pb = self.roads[rb].at(ub).0;
        if dist(pa, pb) < 40.0 {
            return None;
        }
        let a = self.node_on(ra, ua, snap);
        // the first split may have moved the second road's index or its distance along
        let (rb, ub, _) = self.road_near(pb, 1.0, true).unwrap_or((rb, ub, 0.0));
        let b = self.node_on(rb, ub, snap);
        if a == b {
            return None;
        }
        let (na, nb) = (self.nodes[a], self.nodes[b]);
        let steps = (dist(na, nb) / 20.0).ceil().max(1.0) as usize;
        let pts = (0..=steps).map(|i| {
            let f = i as f64 / steps as f64;
            [na[0] + (nb[0] - na[0]) * f, na[1] + (nb[1] - na[1]) * f]
        });
        let mut r = Road::new(a, b, pts.collect(), Kind::Built);
        r.changed = self.t;
        self.roads.push(r);
        self.route();
        Some(self.roads.len() - 1)
    }

    /// Closes every road the stroke from `p` to `q` crosses (not the ways in and out). Returns them.
    pub fn cut(&mut self, p: [f64; 2], q: [f64; 2]) -> Vec<usize> {
        let mut hit = vec![];
        for (i, r) in self.roads.iter().enumerate() {
            if !r.alive || !r.open || matches!(r.kind, Kind::Entry | Kind::Exit) {
                continue;
            }
            if r.pts.windows(2).any(|w| crosses(p, q, w[0], w[1])) {
                hit.push(i);
            }
        }
        for &i in &hit {
            self.roads[i].open = false;
            self.roads[i].changed = self.t;
        }
        if !hit.is_empty() {
            self.route();
        }
        hit
    }

    /// Opens a closed road again.
    pub fn reopen(&mut self, r: usize) {
        self.roads[r].open = true;
        self.roads[r].changed = self.t;
        self.route();
    }

    /// Drawn roads that were cut leave once they are empty.
    pub fn tidy(&mut self) {
        let mut n = vec![0usize; self.roads.len()];
        for c in &self.cars {
            n[c.link / 2] += 1;
        }
        for (i, r) in self.roads.iter_mut().enumerate() {
            if r.kind == Kind::Built && !r.open && r.alive && n[i] == 0 && self.t - r.changed > 1.2 {
                r.alive = false;
            }
        }
    }

    /// The cars on each road (both ways).
    pub fn load(&self) -> Vec<usize> {
        let mut n = vec![0usize; self.roads.len()];
        for c in &self.cars {
            n[c.link / 2] += 1;
        }
        n
    }
}

fn crosses(p: [f64; 2], q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (d1, d2, d3, d4) = (o(p, q, a), o(p, q, b), o(a, b, p), o(a, b, q));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(t: &mut Town, secs: f64) -> f64 {
        t.step(secs);
        t.mean_trip(30.0).unwrap()
    }

    #[test]
    /// The paradox, measured: the bridge across the lake slows the mean trip, and closing it again
    /// brings it back. Nothing in the drivers' rule mentions bridges.
    fn a_bridge_slows_everyone() {
        let mut t = Town::new(7);
        let before = settle(&mut t, 120.0);
        let n0 = t.cars.len();
        let a = t.nodes[2];
        let b = t.nodes[3];
        t.build(a, b, 10.0).expect("bridge");
        let after = settle(&mut t, 120.0);
        let n1 = t.cars.len();
        let load = t.load();
        eprintln!("before {before:.1}s {n0} cars; after {after:.1}s {n1} cars; load {load:?}");
        let br = t.roads.len() - 1;
        t.cut(
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0 - 20.0],
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0 + 20.0],
        );
        assert!(!t.roads[br].open);
        let again = settle(&mut t, 150.0);
        eprintln!("cut again {again:.1}s {} cars", t.cars.len());
        assert!(after > before * 1.2, "the bridge slows everyone: {before:.1} -> {after:.1}");
        assert!(again < after * 0.85, "cutting it helps: {after:.1} -> {again:.1}");
    }
}

