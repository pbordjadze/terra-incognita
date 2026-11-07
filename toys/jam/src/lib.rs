//! Shortcut (a working title; the crate keeps its first name, `jam`): Braess's paradox to play with
//! (painting-company row 255). A small town at night from above: cars come in at the bottom and leave
//! at the top, round a lake, each driver taking whatever way looks quickest right now (`town.rs`).
//! A car warms from white to red as its trip runs long, so the town's colour is everyone's trip.
//! Draw a road from one road to another and the drivers use it; slash across a road to close it; tap a
//! closed road to open it again; tap a car to see the way it chose; shake for the town's own roads.
//! The obvious bridge across the lake makes everyone slower, and nothing in the code says so.
//! (Ghost Jam, the Nagoya ring, is in `main`'s history at 3032c9a; Salt Flat, the cars on the salt, at
//! the commit before this toy's first.)

mod sound;
pub mod town;

use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};
use town::{dist, Kind, Town};

/// The info mark's ghost fingers' ids start here (toy-kit info.rs).
const GHOST: u64 = 1 << 48;
/// How fast the town's clock runs against the screen's: every rule the same, sooner.
const PACE: f64 = 1.7;
/// Seconds the town has run before it opens, so it opens flowing.
const BEFORE: f64 = 70.0;
/// The lake, an ellipse at the middle (points).
const LAKE: [f64; 2] = [66.0, 178.0];

/// The reading scale: 1 on a phone, up to 1.35 on a large iPad.
fn reading(points: [f64; 2]) -> f64 {
    (points[0].min(points[1]) / 393.0).clamp(1.0, 1.35)
}

struct Layout {
    c: [f64; 2],
    /// points (screen) a town point
    s: f64,
}

fn layout_of(points: [f64; 2]) -> Layout {
    let s = (points[0] / 393.0).min(points[1] / 852.0);
    Layout { c: [points[0] * 0.5, points[1] * 0.5], s: s.max(0.3) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stroke {
    /// Down on a road: drawing a new one from there.
    Build,
    /// Down off the roads: a slash that closes what it crosses.
    Cut,
}

#[derive(Clone, Debug)]
struct Finger {
    id: u64,
    down: [f64; 2],
    at: [f64; 2],
    t: f64,
    kind: Stroke,
    path: Vec<[f64; 2]>,
}

/// One of the reader's changes, kept so the town can be rebuilt as they left it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum Edit {
    Build([f64; 2], [f64; 2]),
    Cut([f64; 2], [f64; 2]),
    Open([f64; 2]),
}

pub struct Jam {
    town: Town,
    ground: Backdrop,
    painter: Painter,
    sound: sound::Engines,
    points: [f64; 2],
    t: f64,
    /// The town's own trip, before anyone touched it (seconds): gold at it, red at a third again.
    usual: f64,
    edits: Vec<Edit>,
    /// Lit windows: where (town points), how bright, and when each goes dark or comes on.
    windows: Vec<([f64; 2], f32, f64)>,
    fingers: Vec<Finger>,
    /// Rings where a finger tapped: where (points), when.
    rings: Vec<([f64; 2], f64)>,
    /// A road tapped once: where (town points) and when; a tap on another road builds between them.
    pin: Option<([f64; 2], f64)>,
    /// Where roads were closed: the road, where it was crossed (town points), when.
    barriers: Vec<(usize, [f64; 2], f64)>,
    /// Roads the info mark's ghost closed, to open again when its demo is done: the road, when.
    lent: Vec<(usize, f64)>,
    /// The last tap on the water: where (points) and when, for its ripple.
    splash: ([f64; 2], f64),
    /// Where slashes fell: from, to (town points), when.
    slashes: Vec<([f64; 2], [f64; 2], f64)>,
    /// Each drawn road's tint: the mean light of the cars on it, eased.
    road_tint: Vec<f64>,
    /// The colour the town's arrivals bring, eased.
    arrive: f64,
    touched: bool,
    still: bool,
    born: Option<f64>,
}

/// A car's light, by how long its trip will take against the town's usual: white when quicker, a warm
/// white at the usual, orange, then red when a third slower.
fn trip_colour(u: f64) -> [f32; 3] {
    let stops: [(f64, [f32; 3]); 6] = [
        (0.94, [0.80, 0.90, 1.00]),
        (1.03, [0.98, 0.95, 0.86]),
        (1.10, [1.00, 0.76, 0.38]),
        (1.17, [1.00, 0.42, 0.16]),
        (1.26, [1.00, 0.18, 0.10]),
        (1.6, [0.80, 0.05, 0.05]),
    ];
    if u <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        if u <= w[1].0 {
            let f = ((u - w[0].0) / (w[1].0 - w[0].0)) as f32;
            let f = f * f * (3.0 - 2.0 * f);
            return [0, 1, 2].map(|k| w[0].1[k] + (w[1].1[k] - w[0].1[k]) * f);
        }
    }
    stops[5].1
}

impl Jam {
    fn layout(&self) -> Layout {
        layout_of(self.points)
    }

    fn to_screen(&self, p: [f64; 2]) -> [f64; 2] {
        let l = self.layout();
        [l.c[0] + l.s * p[0], l.c[1] + l.s * p[1]]
    }

    fn to_town(&self, q: [f64; 2]) -> [f64; 2] {
        let l = self.layout();
        [(q[0] - l.c[0]) / l.s, (q[1] - l.c[1]) / l.s]
    }

    /// How near a road a finger must land, in town points.
    fn reach(&self) -> f64 {
        24.0 * reading(self.points) / self.layout().s
    }

    fn apply(&mut self, e: &Edit) -> bool {
        let reach = self.reach();
        match *e {
            Edit::Build(p, q) => self.town.build(p, q, reach).is_some(),
            Edit::Cut(p, q) => {
                let hit = self.town.cut(p, q);
                for &(r, at) in &hit {
                    self.barriers.push((r, at, self.town.t));
                }
                !hit.is_empty()
            }
            Edit::Open(p) => match self.town.road_near(p, reach, true) {
                Some((r, _, _)) if !self.town.roads[r].open => {
                    self.town.reopen(r);
                    true
                }
                _ => false,
            },
        }
    }

    fn fresh(&mut self) {
        let wide = (self.points[0] / self.points[1]) / (393.0 / 852.0);
        let mut town = Town::wide(0x5107_c0de, wide * 0.95);
        // the usual trip: what the drivers expect, on average, over the town's last half minute alone
        let (mut sum, mut n) = (0.0, 0);
        let mut run = 0.0;
        while run < BEFORE {
            town.step(0.5);
            run += 0.5;
            if run > BEFORE - 30.0 {
                for i in 0..town.cars.len() {
                    sum += town.way(i);
                    n += 1;
                }
            }
        }
        self.usual = if n > 0 { sum / n as f64 } else { 25.0 };
        for i in 0..town.cars.len() {
            town.cars[i].late = town.way(i) / self.usual;
        }
        self.town = town;
        self.edits.clear();
        self.pin = None;
        self.barriers.clear();
        self.lent.clear();
        self.road_tint.clear();
        self.arrive = 1.0;
    }

    /// A car's place on its lane (drive on the right), and the way it faces.
    fn lane_place(&self, c: &town::Car) -> ([f64; 2], [f64; 2]) {
        lane_place(&self.town, c)
    }

    /// The two junctions across the lake: the shortcut a newcomer will want.
    fn shores(&self) -> ([f64; 2], [f64; 2]) {
        (self.town.nodes[2], self.town.nodes[3])
    }

    /// Whether the invitation shows: the town as it was, nothing pinned, no finger down.
    fn inviting(&self) -> bool {
        self.edits.is_empty() && self.pin.is_none()
    }

    /// The newest open road the reader drew, if any.
    fn bridge(&self) -> Option<usize> {
        self.town.roads.iter().enumerate().rev().find(|(_, r)| r.kind == Kind::Built && r.open && r.alive).map(|(i, _)| i)
    }
}

impl Toy for Jam {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let ground = Backdrop::new(&ctx.gpu, "town", GROUND_WGSL);
        let painter = Painter::new(&ctx.gpu);
        toy_kit::step("town: the evening before");
        let mut toy = Jam {
            town: Town::new(1),
            ground,
            painter,
            sound: sound::Engines::new(),
            points: ctx.points(),
            t: 0.0,
            usual: 25.0,
            edits: Vec::new(),
            windows: Vec::new(),
            fingers: Vec::new(),
            rings: Vec::new(),
            pin: None,
            barriers: Vec::new(),
            lent: Vec::new(),
            splash: ([0.0, 0.0], -99.0),
            slashes: Vec::new(),
            road_tint: Default::default(),
            arrive: 1.0,
            touched: false,
            still: ctx.reduce_motion,
            born: None,
        };
        toy.fresh();
        toy.windows = windows(&toy.town);
        Ok(toy)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        let was = self.points[0] / self.points[1];
        self.points = ctx.points();
        // a screen of other proportions (turned, or split): the town laid out for it, the reader's
        // changes made again
        if ((self.points[0] / self.points[1]) / was - 1.0).abs() > 0.05 {
            let edits = std::mem::take(&mut self.edits);
            self.fresh();
            for e in edits {
                if self.apply(&e) {
                    self.edits.push(e);
                }
            }
            self.windows = windows(&self.town);
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let q = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.touched = true;
                let w = self.to_town(q);
                let kind = if self.town.road_near(w, self.reach(), true).is_some() { Stroke::Build } else { Stroke::Cut };
                self.fingers.retain(|f| f.id != e.id);
                self.fingers.push(Finger { id: e.id, down: q, at: q, t: e.t, kind, path: vec![q] });
            }
            Phase::Move => {
                if let Some(f) = self.fingers.iter_mut().find(|f| f.id == e.id) {
                    f.at = q;
                    if dist(*f.path.last().unwrap(), q) > 3.0 {
                        f.path.push(q);
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some(k) = self.fingers.iter().position(|f| f.id == e.id) else { return };
                let f = self.fingers.remove(k);
                if e.phase == Phase::Cancel {
                    return;
                }
                let moved = dist(f.down, q);
                let (a, b) = (self.to_town(f.down), self.to_town(q));
                if moved < 12.0 {
                    // a tap: while the invitation shows, a touch on it (a shore's ring, the dotted road,
                    // the water) builds the road it shows; a closed road opens; a road pins a start, and
                    // a second road builds to it; the water ripples
                    let (sa, sb) = self.shores();
                    let wet = (a[0] / LAKE[0]).hypot(a[1] / LAKE[1]) < 1.0;
                    let near_invite = seg_dist(a, sa, sb) < 34.0 / self.layout().s.min(1.0);
                    if self.inviting() && (wet || near_invite) {
                        let edit = Edit::Build(sa, sb);
                        if self.apply(&edit) {
                            self.edits.push(edit);
                            self.sound.thump();
                            ctx.haptic();
                        }
                        if wet {
                            self.splash = (q, self.t);
                        }
                        self.rings.push((q, self.t));
                        return;
                    }
                    let edit = Edit::Open(a);
                    if self.apply(&edit) {
                        self.edits.push(edit);
                        self.pin = None;
                        self.sound.thump();
                        ctx.haptic();
                    } else if self.town.road_near(a, self.reach(), false).is_some() {
                        match self.pin {
                            Some((p, _)) if dist(p, a) > 40.0 / self.layout().s => {
                                let edit = Edit::Build(p, a);
                                if self.apply(&edit) {
                                    self.edits.push(edit);
                                    self.sound.thump();
                                    ctx.haptic();
                                }
                                self.pin = None;
                            }
                            Some(_) => self.pin = None,
                            None => {
                                let at = self.town.road_near(a, self.reach(), false).map(|(r, u, _)| self.town.roads[r].at(u).0).unwrap_or(a);
                                self.pin = Some((at, self.t));
                                ctx.haptic();
                            }
                        }
                    } else {
                        self.pin = None;
                        if (a[0] / LAKE[0]).hypot(a[1] / LAKE[1]) < 1.0 {
                            self.splash = (q, self.t);
                        }
                    }
                    self.rings.push((q, self.t));
                    return;
                }
                let reach = self.reach();
                let ends_on_road = self.town.road_near(b, reach, true).is_some();
                if f.kind == Stroke::Build && ends_on_road {
                    let edit = Edit::Build(a, b);
                    if self.apply(&edit) {
                        self.edits.push(edit);
                        self.sound.thump();
                        ctx.haptic();
                    }
                } else {
                    // a slash: every road it crosses, along the finger's own path
                    let mut path = f.path.clone();
                    path.push(q);
                    let mut any = false;
                    let had = self.barriers.len();
                    for w in path.windows(2) {
                        let edit = Edit::Cut(self.to_town(w[0]), self.to_town(w[1]));
                        if self.apply(&edit) {
                            self.edits.push(edit);
                            any = true;
                        }
                    }
                    if any {
                        self.sound.thump();
                        ctx.haptic();
                    }
                    // the info mark's ghost shows the slash, and gives the road back after
                    if e.id >= GHOST {
                        for &(r, _, _) in &self.barriers[had..] {
                            self.lent.push((r, self.t + 3.0));
                        }
                    }
                    self.slashes.push((a, b, self.t));
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.points = ctx.points();
        self.t = t;
        if self.born.is_none() {
            self.born = Some(t);
        }
        let had = self.town.trips.len();
        self.town.step(dt * PACE);
        let keep: Vec<usize> = self.lent.iter().map(|x| x.0).collect();
        self.town.tidy(&keep);
        // each car's light eases toward how long its trip will take
        // slow to redden and quick to cool, so a moment's worsening (the few seconds a closed road's
        // cars take to go round) doesn't flash, and a lasting jam or a relief both show
        let (warm, cool) = (1.0 - (-dt * PACE * 0.45).exp(), 1.0 - (-dt * PACE * 3.0).exp());
        for i in 0..self.town.cars.len() {
            let want = self.town.way(i) / self.usual;
            let c = &mut self.town.cars[i];
            c.late += (want - c.late) * if want > c.late { warm } else { cool };
        }
        let new = self.town.trips.len().saturating_sub(had);
        for &(_, trip) in &self.town.trips[self.town.trips.len() - new..] {
            self.arrive += (trip / self.usual - self.arrive) * 0.06;
        }
        // each drawn road wears the colour of the trips of the cars on it: white while it helps, red
        // once the crowd it brought makes every way slower
        let mut tint = vec![(0.0f64, 0usize); self.town.roads.len()];
        for c in &self.town.cars {
            tint[c.link / 2].0 += c.late;
            tint[c.link / 2].1 += 1;
        }
        for (i, r) in self.town.roads.iter().enumerate() {
            if r.kind == Kind::Built {
                let want = if tint[i].1 > 0 { tint[i].0 / tint[i].1 as f64 } else { 1.0 };
                if self.road_tint.len() <= i {
                    self.road_tint.resize(i + 1, 1.0);
                }
                let e = &mut self.road_tint[i];
                *e += (want - *e) * 0.05;
            }
        }
        self.rings.retain(|(_, at)| t - at < 1.2);
        self.slashes.retain(|(_, _, at)| t - at < 0.8);
        let due: Vec<usize> = self.lent.iter().filter(|(_, at)| t >= *at).map(|x| x.0).collect();
        self.lent.retain(|(_, at)| t < *at);
        for r in due {
            if self.town.roads.get(r).is_some_and(|x| !x.open && x.alive) {
                let at = self.town.roads[r].at(self.town.roads[r].len * 0.5).0;
                let edit = Edit::Open(at);
                if self.apply(&edit) {
                    self.edits.push(edit);
                }
            }
        }
        if self.pin.is_some_and(|(_, at)| t - at > 8.0) {
            self.pin = None;
        }
        let town = &self.town;
        self.barriers.retain(|&(r, _, _)| town.roads.get(r).is_some_and(|x| !x.open && x.alive));
        // the engines heard: the cars nearest the middle of the screen
        let l = self.layout();
        let mut near: Vec<(f64, (f32, f32, f32))> = self
            .town
            .cars
            .iter()
            .map(|c| {
                let p = self.town.place(c).0;
                let d = p[0].hypot(p[1]);
                let pan = (p[0] * l.s / (self.points[0] * 0.5)).clamp(-1.0, 1.0) as f32;
                (d, ((c.v / 9.0) as f32, 0.0, pan))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let cars: Vec<(f32, f32, f32)> = near.iter().take(sound::VOICES).map(|x| x.1).collect();
        self.sound.set(&cars);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp;
        let size = [ctx.width, ctx.height];
        let l = self.layout();
        let m = (l.s * k) as f32; // pixels a town point
        let px = |p: [f64; 2]| -> [f32; 2] { [((l.c[0] + l.s * p[0]) * k) as f32, ((l.c[1] + l.s * p[1]) * k) as f32] };
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let tm = self.t;

        let arrive = trip_colour(self.arrive);
        let params = BackdropParams {
            time: tm as f32,
            scale: m,
            center: [(self.splash.0[0] * k) as f32, (self.splash.0[1] * k) as f32],
            p: [(l.c[0] * k) as f32, (l.c[1] * k) as f32, LAKE[0] as f32, LAKE[1] as f32, arrive[0], arrive[1], arrive[2], (tm - self.splash.1) as f32],
        };
        self.ground.render(gpu, &mut enc, target, size, &params);

        let reach = self.reach();
        let p = &mut self.painter;
        let town = &self.town;
        // the windows, a few going dark or coming on as the evening goes
        for &(w, bright, phase) in &self.windows {
            let on = if self.still { 1.0 } else { (((tm + phase) / 37.0).fract() < 0.92) as i32 as f32 };
            if on > 0.0 {
                p.glow(px(w), px(w), 2.0 * m, 5.0 * m, [1.0, 0.72, 0.40, 0.05 * bright]);
                p.disc(px(w), 0.75 * m, 0.4 * m, [1.0, 0.80, 0.52, 0.55 * bright]);
            }
        }
        // the roads: their verge, their asphalt, a closed one dim and broken
        for pass in 0..3 {
            for (ri, r) in town.roads.iter().enumerate() {
                if !r.alive {
                    continue;
                }
                let (w, asphalt) = match r.kind {
                    Kind::Street => (3.6, [0.150, 0.150, 0.165]),
                    Kind::Long => (4.6, [0.105, 0.115, 0.125]),
                    Kind::Entry | Kind::Exit => (4.8, [0.130, 0.135, 0.150]),
                    Kind::Built => (5.0, [0.36, 0.36, 0.37]),
                };
                let since = (town.t - r.changed).max(0.0);
                // a new road is laid from its start, quickly
                let laid = if r.kind == Kind::Built && r.open { (since / 0.45).min(1.0) } else { 1.0 };
                let fade = if r.open {
                    1.0
                } else if r.kind == Kind::Built {
                    (1.0 - since / 1.2).max(0.0) * 0.5
                } else {
                    0.42
                } as f32;
                let upto = r.len * laid;
                let mut u = 0.0;
                while u < upto {
                    let v = (u + 6.0).min(upto);
                    let dash = !r.open && ((u / 9.0) as i64) % 2 == 1;
                    if !dash {
                        let (a, b) = (px(r.at(u).0), px(r.at(v).0));
                        match pass {
                            0 => p.line(a, b, (w + 1.6) as f32 * m, 1.6 * m, [0.0, 0.0, 0.01, 0.45 * fade]),
                            1 => {
                                let mut col = asphalt;
                                if r.kind == Kind::Built {
                                    if let Some(&tl) = self.road_tint.get(ri) {
                                        let c = trip_colour(tl);
                                        let k = (0.35 * ((tl - 1.0) / 0.25).clamp(0.0, 1.0) + 0.25) as f32;
                                        col = [0, 1, 2].map(|j| col[j] * (1.0 - k) + c[j] * k);
                                        if r.open && tl > 1.05 {
                                            p.glow(a, b, w as f32 * m, 5.0 * m, [c[0], c[1], c[2], 0.10 * ((tl - 1.05) / 0.2).clamp(0.0, 1.0) as f32]);
                                        }
                                    }
                                }
                                p.line(a, b, w as f32 * m, 0.7 * m, [col[0], col[1], col[2], fade])
                            }
                            _ => {
                                let (wa, dir) = r.at(u);
                                let wet = (wa[0] / LAKE[0]).hypot(wa[1] / LAKE[1]) < 1.08;
                                if r.kind == Kind::Built && r.open && wet {
                                    // over the water: a bridge's two rails, pale
                                    for side in [-1.0, 1.0] {
                                        let o = [-dir[1] * side * (w + 0.9), dir[0] * side * (w + 0.9)];
                                        let (ra, rb) = (r.at(u).0, r.at(v).0);
                                        p.line(px([ra[0] + o[0], ra[1] + o[1]]), px([rb[0] + o[0], rb[1] + o[1]]), 0.55 * m, 0.4 * m, [0.62, 0.64, 0.68, 0.85]);
                                    }
                                }
                                if r.kind != Kind::Street && r.open && ((u / 14.0) as i64) % 2 == 0 {
                                    // the centre line, faint
                                    p.line(a, b, 0.35 * m, 0.4 * m, [0.55, 0.52, 0.42, 0.12]);
                                }
                            }
                        }
                    }
                    u = v;
                }
                // street lamps along the town's streets
                if pass == 2 && r.kind == Kind::Street && r.open {
                    let mut u = 10.0;
                    while u < r.len - 6.0 {
                        let (q, d) = r.at(u);
                        let side = if ((u / 34.0) as i64) % 2 == 0 { 1.0 } else { -1.0 };
                        let lamp = [q[0] - d[1] * side * (w + 3.0), q[1] + d[0] * side * (w + 3.0)];
                        p.glow(px(lamp), px(lamp), 9.0 * m, 9.0 * m, [1.0, 0.80, 0.50, 0.035]);
                        p.disc(px(lamp), 0.9 * m, 0.5 * m, [1.0, 0.86, 0.62, 0.8]);
                        u += 34.0;
                    }
                }
            }
        }
        p.render(gpu, &mut enc, target, size, None);

        // the cars: a small light each, coloured by its trip so far
        for c in &town.cars {
            let (q, d) = lane_place(town, c);
            let col = trip_colour(c.late);
            let head = [q[0] + d[0] * 1.6, q[1] + d[1] * 1.6];
            let tail = [q[0] - d[0] * 1.6, q[1] - d[1] * 1.6];
            // its light on the road round it, its headlights ahead, itself
            p.glow(px(q), px(q), 3.0 * m, 9.0 * m, [col[0], col[1], col[2], 0.16]);
            let ahead = [q[0] + d[0] * 11.0, q[1] + d[1] * 11.0];
            p.glow(px(head), px(ahead), 1.4 * m, 4.0 * m, [1.0, 0.95, 0.85, 0.06]);
            p.line(px(tail), px(head), 1.9 * m, 0.9 * m, [col[0], col[1], col[2], 1.0]);
            p.disc(px(q), 0.9 * m, 0.6 * m, [1.0, 1.0, 1.0, 0.45]);
        }

        // the fingers: a road being drawn, a slash
        let kk = k as f32;
        let to_town = |q: [f64; 2]| [(q[0] - l.c[0]) / l.s, (q[1] - l.c[1]) / l.s];
        for f in &self.fingers {
            let a = ((tm - f.t) / 0.15).min(1.0) as f32;
            match f.kind {
                Stroke::Build => {
                    let s = town.road_near(to_town(f.down), reach, true).map(|(r, u, _)| town.roads[r].at(u).0);
                    let from = s.map(px).unwrap_or([(f.down[0] * k) as f32, (f.down[1] * k) as f32]);
                    let end = town.road_near(to_town(f.at), reach, true).map(|(r, u, _)| town.roads[r].at(u).0);
                    let to = end.map(px).unwrap_or([(f.at[0] * k) as f32, (f.at[1] * k) as f32]);
                    p.line(from, to, 3.3 * m, 1.2 * m, [0.85, 0.88, 0.95, 0.30 * a]);
                    p.disc(from, 4.0 * m, 1.0 * m, [0.95, 0.97, 1.0, 0.8 * a]);
                    if end.is_some() {
                        p.disc(to, 4.0 * m, 1.0 * m, [0.95, 0.97, 1.0, 0.8 * a]);
                    } else {
                        p.ring(to, 5.0 * m, 0.8 * m, 0.8 * m, [0.95, 0.97, 1.0, 0.45 * a]);
                    }
                }
                Stroke::Cut => {
                    for w in f.path.windows(2) {
                        p.line([w[0][0] as f32 * kk, w[0][1] as f32 * kk], [w[1][0] as f32 * kk, w[1][1] as f32 * kk], 1.2 * m, 1.0 * m, [1.0, 0.55, 0.45, 0.55 * a]);
                    }
                }
            }
        }
        for &(a, b, at) in &self.slashes {
            let f = (1.0 - (tm - at) / 0.8).clamp(0.0, 1.0) as f32;
            p.line(px(a), px(b), 1.2 * m, 1.0 * m, [1.0, 0.55, 0.45, 0.5 * f * f]);
        }
        for &(q, at) in &self.rings {
            let f = ((tm - at) / 1.2).clamp(0.0, 1.0) as f32;
            p.glow_ring([q[0] as f32 * kk, q[1] as f32 * kk], (6.0 + 14.0 * f) * m, 0.7 * m, 1.2 * m, [1.0, 0.97, 0.92, 0.45 * (1.0 - f)]);
        }
        // closed roads: a barrier where they were cut
        for &(r, at, when) in &self.barriers {
            let road = &town.roads[r];
            let (_, u) = road.nearest(at);
            let (q, d) = road.at(u);
            let come = ((town.t - when) / 0.25).clamp(0.0, 1.0) as f32;
            let n = [-d[1] * 7.5, d[0] * 7.5];
            let (e0, e1) = ([q[0] - n[0], q[1] - n[1]], [q[0] + n[0], q[1] + n[1]]);
            p.line(px(e0), px(e1), 2.0 * m, 0.6 * m, [0.05, 0.03, 0.03, 0.9 * come]);
            for j in 0..4 {
                let (f0, f1) = (j as f64 / 4.0, (j as f64 + 1.0) / 4.0);
                let a0 = [e0[0] + (e1[0] - e0[0]) * f0, e0[1] + (e1[1] - e0[1]) * f0];
                let a1 = [e0[0] + (e1[0] - e0[0]) * f1, e0[1] + (e1[1] - e0[1]) * f1];
                let col = if j % 2 == 0 { [1.0, 0.30, 0.20, come] } else { [0.95, 0.92, 0.88, come] };
                p.line(px(a0), px(a1), 1.3 * m, 0.4 * m, col);
            }
            p.glow(px(q), px(q), 4.0 * m, 9.0 * m, [1.0, 0.3, 0.2, 0.12 * come]);
        }
        // a pinned start: a light on the road, waiting for the other end, and every other road a faint
        // glow where it could go
        if let Some((w, at)) = self.pin {
            let age = tm - at;
            let lit = (0.5 + 0.5 * (age * 3.0).cos()) as f32 * ((age / 0.3).min(1.0) as f32);
            for r in town.roads.iter().filter(|r| r.alive && r.open) {
                let mut u = 0.0;
                while u < r.len {
                    let v = (u + 8.0).min(r.len);
                    let (a0, a1) = (r.at(u).0, r.at(v).0);
                    if dist(a0, w) > 30.0 {
                        p.glow(px(a0), px(a1), 3.0 * m, 5.0 * m, [0.80, 0.88, 1.0, 0.10 * lit]);
                    }
                    u = v;
                }
            }
            let breathe = if self.still { 1.0 } else { 0.75 + 0.25 * (age * 4.0).sin() as f32 };
            p.glow(px(w), px(w), 5.0 * m, 12.0 * m, [0.85, 0.92, 1.0, 0.25 * breathe]);
            p.disc(px(w), 4.0 * m, 1.0 * m, [0.95, 0.97, 1.0, 0.9]);
            p.glow_ring(px(w), (8.0 + 3.0 * breathe) * m, 0.6 * m, 1.2 * m, [0.95, 0.97, 1.0, 0.5]);
        }
        // the invitation: until the reader changes a road, a road of light keeps drawing itself across
        // the lake from shore to shore
        let since = tm - self.born.unwrap_or(tm);
        if self.edits.is_empty() && self.pin.is_none() && self.fingers.is_empty() {
            let (a, b) = (town.nodes[2], town.nodes[3]);
            let come = 1.0f32;
            let cycle = 3.2;
            let ph = if self.still { 1.0 } else { (since / cycle + 0.45).fract() };
            // the drawing finger goes over in the first 40% of the cycle, the line it leaves fades after
            let reach = (ph / 0.4).min(1.0);
            let fade = if ph < 0.4 { 1.0 } else { 1.0 - 0.6 * ((ph - 0.4) / 0.6) } as f32;
            let n = 16;
            for i in 0..=n {
                let f = i as f64 / n as f64;
                if f > reach {
                    break;
                }
                let q = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                p.disc(px(q), 1.5 * m, 0.7 * m, [0.92, 0.95, 1.0, 0.55 * come * fade]);
            }
            for end in [a, b] {
                p.glow_ring(px(end), 7.0 * m, 0.6 * m, 1.4 * m, [0.92, 0.95, 1.0, 0.35 * come]);
            }
            if ph < 0.45 {
                let f = reach;
                let q = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                p.glow(px(q), px(q), 3.0 * m, 9.0 * m, [0.92, 0.95, 1.0, 0.5 * come]);
                p.disc(px(q), 2.4 * m, 0.8 * m, [1.0, 1.0, 1.0, 0.9 * come]);
            }
        }
        p.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        self.fresh();
        ctx.haptic();
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.still = ctx.reduce_motion;
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.sound.synth())
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&self.edits).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(edits) = serde_json::from_str::<Vec<Edit>>(saved) {
            self.fresh();
            for e in edits {
                if self.apply(&e) {
                    self.edits.push(e);
                }
            }
            self.town.step(20.0);
        }
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let (a, b) = self.shores();
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        let mut out = vec![
            ("west".into(), self.to_screen(a)),
            ("east".into(), self.to_screen(b)),
            ("lake".into(), self.to_screen(mid)),
            ("slash-a".into(), self.to_screen([mid[0] - 10.0, mid[1] - 34.0])),
            ("slash-b".into(), self.to_screen([mid[0] + 10.0, mid[1] + 34.0])),
            ("centre".into(), self.layout().c),
        ];
        // a car on screen, partway
        if let Some(c) = self.town.cars.iter().find(|c| {
            let p = self.town.place(c).0;
            p[1].abs() < 200.0 && c.v > 5.0
        }) {
            out.push(("car".into(), self.to_screen(self.lane_place(c).0)));
        }
        // the narrow streets' and the long roads' middles
        for (name, r) in [("west-street", 1), ("west-long", 2), ("east-long", 3), ("east-street", 4)] {
            if let Some(road) = self.town.roads.get(r) {
                out.push((name.into(), self.to_screen(road.at(road.len * 0.5).0)));
            }
        }
        out
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        use toy_kit::info::{Demo, Stroke as S};
        let (a, b) = self.shores();
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        if self.bridge().is_some() {
            return Some(Demo::new(
                "Slash across a road to close it. Draw from road to road to build one.",
                vec![S::drag(0.5, 0.5, self.to_screen([mid[0] - 12.0, mid[1] - 40.0]), self.to_screen([mid[0] + 12.0, mid[1] + 40.0]))],
            ));
        }
        Some(Demo::new("Draw from road to road to build one. Slash across a road to close it.", vec![S::drag(0.5, 1.4, self.to_screen(a), self.to_screen(b))]))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "Every driver takes the way that looks quickest now.",
                "A road slows as it fills. The long roads round never fill.",
                "A car glows white when its way is quick, red when slow. Shake for the old roads.",
            ],
            "Real: the rules, and that a new road can slow everyone (Braess, 1968). Made up: the town and its cars.",
        ))
    }

    fn card_at_top(&self, _ctx: &Ctx) -> bool {
        true
    }

    fn origins(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "In 1968 Dietrich Braess showed that adding a road can make every driver's trip slower, when each chooses for themselves.",
                "Wardrop (1952): selfish drivers settle where no one gains by switching. That is not where everyone does best.",
                "A road's time grows with its load: the traffic engineers' volume-delay curves, and a lane carries about as many cars an hour whatever its speed.",
            ],
            "Drawn: the town, the lake, the cars, and how fast time runs. Real: the drivers' rule and the roads' slowing.",
        ))
    }
}


/// A car's place on its lane (drive on the right), and the way it faces.
/// How far `p` is from the segment `a`-`b`.
fn seg_dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l2 = (d[0] * d[0] + d[1] * d[1]).max(1e-9);
    let f = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0);
    dist(p, [a[0] + d[0] * f, a[1] + d[1] * f])
}

fn lane_place(town: &Town, c: &town::Car) -> ([f64; 2], [f64; 2]) {
    let (p, d) = town.place(c);
    let r = &town.roads[c.link / 2];
    let lanes = match r.kind {
        Kind::Street => 1u32,
        _ => 2,
    };
    let lane = (c.seed % lanes) as f64;
    let side = 1.8 + 3.0 * lane;
    let right = [-d[1], d[0]];
    ([p[0] + right[0] * side, p[1] + right[1] * side], d)
}

/// The town's lit windows: houses along both sides of the narrow streets, and the town at the top and
/// the houses at the bottom where the roads come in and go out; none on the lake or a road.
fn windows(town: &Town) -> Vec<([f64; 2], f32, f64)> {
    let mut h: u32 = 0x2545_f491;
    let mut rnd = move || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        h as f64 / u32::MAX as f64
    };
    let mut out = vec![];
    let clear = |q: [f64; 2]| {
        let lake = (q[0] / (LAKE[0] + 8.0)).hypot(q[1] / (LAKE[1] + 8.0)) > 1.0;
        lake && town.roads.iter().all(|r| r.nearest(q).0 > 7.0)
    };
    for r in town.roads.iter().filter(|r| r.kind == Kind::Street) {
        let mut u = 4.0;
        while u < r.len - 4.0 {
            let (q, d) = r.at(u);
            for side in [-1.0, 1.0] {
                if rnd() < 0.75 {
                    let off = 10.0 + 22.0 * rnd() * rnd();
                    let w = [q[0] - d[1] * side * off, q[1] + d[0] * side * off];
                    if clear(w) {
                        out.push((w, (0.35 + 0.65 * rnd()) as f32, rnd() * 400.0));
                    }
                }
            }
            u += 5.0 + 6.0 * rnd();
        }
    }
    // the town at the top, the houses at the bottom
    for (c, n, spread) in [([0.0, -390.0], 260, [150.0, 70.0]), ([0.0, 400.0], 120, [130.0, 50.0])] {
        for _ in 0..n {
            let g = [rnd() + rnd() + rnd() - 1.5, rnd() + rnd() + rnd() - 1.5];
            let w = [c[0] + g[0] * spread[0], c[1] + g[1] * spread[1]];
            if clear(w) {
                out.push((w, (0.35 + 0.65 * rnd()) as f32, rnd() * 400.0));
            }
        }
    }
    out
}

/// The town at night from above: dark fields and woods, the lake at the middle, the town's warm light at
/// the top and the evening's at the bottom (steady: only the cars and the reader's roads change colour). `p0.xy` the centre (px),
/// `p0.zw` the lake's radii (town points), `p1.xyz` the arrivals' colour; `u.scale` pixels a town point.
const GROUND_WGSL: &str = r#"
fn fbm(p: vec2<f32>) -> f32 {
  var a = 0.0; var w = 0.5; var q = p;
  for (var i = 0; i < 4; i = i + 1) { a = a + w * noise2(q); q = q * 2.03 + vec2<f32>(17.1, 9.3); w = w * 0.5; }
  return a;
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let m = (px - u.p0.xy) / u.scale;           // town points from the middle
  let pxm = 1.0 / u.scale;
  var col = vec3<f32>(0.060, 0.075, 0.085);
  // the land: fields and woods
  let field = fbm(m * 0.012 + vec2<f32>(4.0, 2.0));
  col = col * (0.82 + 0.35 * field) * (0.96 + 0.06 * noise2(m * 0.35));
  let wood = smoothstep(0.52, 0.66, fbm(m * 0.03 + vec2<f32>(1.3, 7.1)));
  col = mix(col, vec3<f32>(0.035, 0.055, 0.050), 0.7 * wood);
  // the lake: still, dark, a shimmer of the town's light on it
  let e = length(m / u.p0.zw);
  let shore = (e - 1.0) * min(u.p0.z, u.p0.w);
  let water = 1.0 - smoothstep(-0.5 - pxm, 0.5 + pxm, shore);
  let ripple = 0.5 + 0.5 * sin(m.y * 0.55 + 0.6 * sin(m.x * 0.21 + u.time * 0.4) + u.time * 0.7);
  var lake = vec3<f32>(0.020, 0.040, 0.075) * (0.85 + 0.3 * (1.0 - e));
  lake = lake + vec3<f32>(1.0, 0.9, 0.75) * 0.03 * ripple * smoothstep(0.2, 1.0, e);
  // a tap on the water: rings going out from it (u.center the tap in px, p1.w the seconds since)
  let since = u.p1.w;
  if (since < 3.0) {
    let r = length(px - u.center) / u.scale;
    let front = since * 60.0;
    let ring = exp(-pow((r - front) / 3.0, 2.0)) + 0.6 * exp(-pow((r - front * 0.6) / 3.0, 2.0));
    lake = lake + vec3<f32>(0.10, 0.14, 0.20) * ring * (1.0 - since / 3.0);
  }
  col = mix(col, lake, water);
  // the shore's edge
  col = col + vec3<f32>(0.05, 0.07, 0.07) * (1.0 - smoothstep(0.0, 2.5, abs(shore))) * 0.6;
  // the town's light at the top, in the arrivals' colour; the evening's at the bottom
  let top = exp(-max(m.y + 400.0, 0.0) / 200.0) * exp(-pow(m.x / 220.0, 2.0));
  col = col + vec3<f32>(1.0, 0.86, 0.62) * 0.12 * top;
  let bottom = exp(-max(420.0 - m.y, 0.0) / 160.0) * exp(-pow(m.x / 200.0, 2.0));
  col = col + vec3<f32>(0.55, 0.65, 0.85) * 0.07 * bottom;
  // the dark coming in at the edges
  let uv = px / u.size;
  let v = length((uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, u.size.y / u.size.x));
  col = col * (1.0 - 0.45 * smoothstep(0.35, 1.1, v));
  return vec4<f32>(col, 1.0);
}
"#;

toy_kit::ios_shell!(Jam);
