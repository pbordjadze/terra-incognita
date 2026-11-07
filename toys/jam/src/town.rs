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
/// How long the drivers' memory of the roads beyond the next is (s), and how much they lean on it.
pub const MEMORY: f64 = 12.0;
pub const RECALL: f64 = 0.5;

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
            Kind::Long => 25.0,
            Kind::Built => 4.0,
        }
    }
    /// Free speed, points a second.
    pub fn vf(self) -> f64 {
        match self {
            Kind::Entry | Kind::Exit => 220.0,
            Kind::Street => 200.0,
            Kind::Long => 25.0,
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
    /// Free speed (points a second): its kind's, or a long road's scaled to its length.
    pub vf: f64,
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
        Road {
            a,
            b,
            pts,
            cum,
            len,
            kind,
            vf: kind.vf(),
            open: true,
            alive: true,
            changed: -99.0,
        }
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
    /// The links it has driven, from the way in.
    pub past: Vec<usize>,
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
    /// The same, as the drivers remember it over the last while (eased, `MEMORY` seconds): what they
    /// expect of the roads beyond the next, which they can't see.
    pub usual: Vec<f64>,
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
/// narrow street. Nothing joins the two sides across the lake. `wide` (1 on a phone) swings the long
/// roads further out on a wider screen, their speed scaled with their length so they take the same time.
pub fn first_roads(wide: f64) -> (Vec<[f64; 2]>, Vec<Road>) {
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
    // the long way round, out wide over the dark
    let long = |p: [f64; 2], q: [f64; 2], side: f64| long_at(p, q, side, wide);
    let phone = Road::new(2, 4, long_at(a, e0, -1.0, 1.0), Kind::Long).len;
    let slow = |mut r: Road| {
        r.vf *= r.len / phone;
        r
    };
    let roads = vec![
        Road::new(0, 1, vec![o, s0], Kind::Entry),
        Road::new(1, 2, street(s0, a, -30.0), Kind::Street),
        slow(Road::new(2, 4, long(a, e0, -1.0), Kind::Long)),
        slow(Road::new(1, 3, long(s0, b, 1.0), Kind::Long)),
        Road::new(3, 4, street(b, e0, -30.0), Kind::Street),
        Road::new(4, 5, vec![e0, d], Kind::Exit),
    ];
    (nodes, roads)
}

fn long_at(p: [f64; 2], q: [f64; 2], side: f64, wide: f64) -> Vec<[f64; 2]> {
    let dy = (q[1] - p[1]).signum();
    bezier(
        p,
        [side * 228.0 * wide, p[1] + 40.0 * dy],
        [side * 228.0 * wide, q[1] - 10.0 * dy],
        q,
        60,
    )
}

fn bez(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    [
        w[0] * p0[0] + w[1] * p1[0] + w[2] * p2[0] + w[3] * p3[0],
        w[0] * p0[1] + w[1] * p1[1] + w[2] * p2[1] + w[3] * p3[1],
    ]
}

fn bezier(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], n: usize) -> Vec<[f64; 2]> {
    (0..=n)
        .map(|i| bez(p0, p1, p2, p3, i as f64 / n as f64))
        .collect()
}

impl Town {
    pub fn new(seed: u32) -> Town {
        Town::wide(seed, 1.0)
    }

    /// The town for a screen `wide` times a phone's proportions across (1 to 1.8).
    pub fn wide(seed: u32, wide: f64) -> Town {
        let (nodes, roads) = first_roads(wide.clamp(1.0, 1.8));
        let mut t = Town {
            nodes,
            roads,
            cars: Vec::new(),
            t: 0.0,
            from: 0,
            to: 5,
            speed: Vec::new(),
            ahead: Vec::new(),
            usual: Vec::new(),
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
        if link.is_multiple_of(2) {
            (r.a, r.b)
        } else {
            (r.b, r.a)
        }
    }

    /// Where a car is, and the way it faces.
    pub fn place(&self, c: &Car) -> ([f64; 2], [f64; 2]) {
        let r = &self.roads[c.link / 2];
        if c.link.is_multiple_of(2) {
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
                let vf = r.vf;
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
        // the remembered times ease toward what is seen now; a new node starts from what is seen
        let k = 1.0 - (-0.25 / MEMORY).exp();
        self.usual.resize(nn, f64::NAN);
        for i in 0..nn {
            let (u, b) = (self.usual[i], best[i]);
            self.usual[i] = if !u.is_finite() || !b.is_finite() {
                b
            } else {
                u + (b - u) * k
            };
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
            let end = self.ends(l).1;
            // the next road as it looks now; beyond it, half what is seen now and half what is usual
            let on = if self.ahead[end].is_finite() && self.usual[end].is_finite() {
                (1.0 - RECALL) * self.ahead[end] + RECALL * self.usual[end]
            } else {
                self.ahead[end]
            };
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
        q[link]
            .last()
            .is_none_or(|&j| self.cars[j].s >= self.gap(link))
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
                            c.past.push(c.link);
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
            let first = self
                .choose(None, self.from, 0)
                .filter(|&l| self.room(l, &q));
            if let Some(l) = first {
                let seed = hash((self.t * 1000.0) as u32 ^ self.rng);
                self.cars.push(Car {
                    link: l,
                    s: 0.0,
                    v: self.roads[l / 2].vf * 0.7,
                    born: self.t,
                    seed,
                    waiting: false,
                    late: 1.0,
                    past: Vec::new(),
                });
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

    /// How long car `i`'s way takes now, end to end: the roads it came by and the one it is on at the
    /// speeds their crowds give them now, and the quickest way on. Not what it has suffered, but how
    /// good its way is at this moment: what a driver setting out now on the same way would expect.
    pub fn way(&self, i: usize) -> f64 {
        let c = &self.cars[i];
        let behind: f64 = c.past.iter().map(|&l| self.cost(l)).sum();
        let on = self.ahead.get(self.ends(c.link).1).copied().unwrap_or(0.0);
        behind + self.cost(c.link) + if on.is_finite() { on } else { 60.0 }
    }

    /// The mean trip of those finished in the last `secs` seconds.
    pub fn mean_trip(&self, secs: f64) -> Option<f64> {
        let r: Vec<f64> = self
            .trips
            .iter()
            .filter(|(at, _)| *at > self.t - secs)
            .map(|x| x.1)
            .collect();
        (!r.is_empty()).then(|| r.iter().sum::<f64>() / r.len() as f64)
    }

    /// The nearest road to `q` within `reach`: (road, distance along, distance away).
    pub fn road_near(
        &self,
        q: [f64; 2],
        reach: f64,
        with_closed: bool,
    ) -> Option<(usize, f64, f64)> {
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
            x.vf = old.vf;
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
        // where it crosses other roads it meets them at a junction
        let mut cross: Vec<(f64, [f64; 2])> = vec![];
        for r in self.roads.iter().filter(|r| r.alive) {
            for w in r.pts.windows(2) {
                if let Some((f, x)) = meet(na, nb, w[0], w[1]) {
                    let along = f * dist(na, nb);
                    if along > snap
                        && along < dist(na, nb) - snap
                        && cross.iter().all(|c| dist(c.1, x) > snap)
                    {
                        cross.push((f, x));
                    }
                }
            }
        }
        cross.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut chain = vec![a];
        for (_, x) in cross {
            if let Some((r, u, _)) = self.road_near(x, 1.0, true) {
                let n = self.node_on(r, u, 6.0);
                if !chain.contains(&n) && n != b {
                    chain.push(n);
                }
            }
        }
        chain.push(b);
        for w in chain.windows(2) {
            let (na, nb) = (self.nodes[w[0]], self.nodes[w[1]]);
            let steps = (dist(na, nb) / 20.0).ceil().max(1.0) as usize;
            let pts = (0..=steps).map(|i| {
                let f = i as f64 / steps as f64;
                [na[0] + (nb[0] - na[0]) * f, na[1] + (nb[1] - na[1]) * f]
            });
            let mut r = Road::new(w[0], w[1], pts.collect(), Kind::Built);
            r.changed = self.t;
            self.roads.push(r);
        }
        self.route();
        Some(self.roads.len() - 1)
    }

    /// Closes every road the stroke from `p` to `q` crosses (not the ways in and out). Returns them,
    /// each with where the stroke crossed it.
    pub fn cut(&mut self, p: [f64; 2], q: [f64; 2]) -> Vec<(usize, [f64; 2])> {
        let mut hit = vec![];
        for (i, r) in self.roads.iter().enumerate() {
            if !r.alive || !r.open || matches!(r.kind, Kind::Entry | Kind::Exit) {
                continue;
            }
            if let Some(w) = r.pts.windows(2).find(|w| crosses(p, q, w[0], w[1])) {
                hit.push((i, [(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0]));
            }
        }
        for &(i, _) in &hit {
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

    /// Drawn roads that were cut leave once they are empty (but not those in `keep`).
    pub fn tidy(&mut self, keep: &[usize]) {
        let mut n = vec![0usize; self.roads.len()];
        for c in &self.cars {
            n[c.link / 2] += 1;
        }
        for (i, r) in self.roads.iter_mut().enumerate() {
            if r.kind == Kind::Built
                && !r.open
                && r.alive
                && n[i] == 0
                && self.t - r.changed > 1.2
                && !keep.contains(&i)
            {
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

/// Where segment `p`-`q` meets segment `a`-`b`: how far along `p`-`q` (0..1) and the point.
fn meet(p: [f64; 2], q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Option<(f64, [f64; 2])> {
    let r = [q[0] - p[0], q[1] - p[1]];
    let s = [b[0] - a[0], b[1] - a[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-9 {
        return None;
    }
    let ap = [a[0] - p[0], a[1] - p[1]];
    let t = (ap[0] * s[1] - ap[1] * s[0]) / den;
    let u = (ap[0] * r[1] - ap[1] * r[0]) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u))
        .then(|| (t, [p[0] + r[0] * t, p[1] + r[1] * t]))
}

fn crosses(p: [f64; 2], q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
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
        assert!(
            after > before * 1.2,
            "the bridge slows everyone: {before:.1} -> {after:.1}"
        );
        assert!(
            again < after * 0.85,
            "cutting it helps: {after:.1} -> {again:.1}"
        );
    }
}

#[cfg(test)]
mod settle {
    use super::*;

    /// After the bridge is closed the town settles back: how long the cars' ways take comes back to
    /// within a tenth of the old within 16 s and stays there.
    #[test]
    fn the_town_settles_after_a_cut() {
        let mut t = Town::new(0x5107_c0de);
        t.step(70.0);
        let eta = |t: &Town| {
            (0..t.cars.len()).map(|i| t.way(i)).sum::<f64>() / t.cars.len().max(1) as f64
        };
        let base = eta(&t);
        let (a, b) = (t.nodes[2], t.nodes[3]);
        t.build(a, b, 10.0);
        t.step(40.0);
        t.cut([0.0, -34.0], [0.0, 34.0]);
        let mut line = String::new();
        let mut worst = 0.0f64;
        for i in 0..40 {
            t.step(2.0);
            let e = eta(&t) / base;
            line += &format!("{e:.2} ");
            if i >= 8 {
                worst = worst.max((e - 1.0).abs());
            }
        }
        eprintln!("{line}");
        assert!(worst < 0.1, "still sloshing: {line}");
    }
}
