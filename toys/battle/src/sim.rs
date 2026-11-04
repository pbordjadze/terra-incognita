//! The battle: about 3,000 bodies of men (a unit is a few dozen), each obeying only local rules, and the
//! ink, a field of channels that carries Carthaginian units the way each stroke was drawn. Ported from
//! the ink-orders spike (painting-company row 206; branch spike-battle-ink-orders, sim.js), its tuned
//! numbers kept: world units are the spike's field pixels (3.78 m).
//!
//! The rules: a unit follows the ink if it is in reach; else it seeks the nearest enemy within a short
//! radius (much further for men the ink carried and let go); bodies take room (overlaps pushed apart, so
//! the deep Roman column really shoves); at contact Lanchester-like attrition, blows on the nearest foe,
//! harder from the flank and rear and far harder on men fleeing; morale drained by enemies behind, on
//! both sides, by blows, by being outnumbered close by, and by friends running; below zero a unit flees
//! for good. The Romans follow the consuls' intent: advance, and press toward the centre. Nothing is
//! scripted.

pub const WX0: f32 = -400.0;
pub const WY0: f32 = -400.0;
pub const WX1: f32 = 1500.0;
pub const WY1: f32 = 1300.0;

const FC: f32 = 8.0;
pub const REACH: f32 = 26.0;
const GC: f32 = 16.0;
const CG: f32 = 40.0;
/// How much of a blow a Carthaginian takes, and of a fright (1: as the spike had it).
const CARTHAGE_TAKES: f32 = 0.5;
const CARTHAGE_NERVE: f32 = 0.6;
/// The room a fallen body takes.
const FALLEN_R: f32 = 1.9;
/// Where the Roman commanders press: the centre.
const RCX: f32 = 520.0;

#[derive(Clone, Copy)]
pub struct Kind {
    pub side: u8,
    pub cav: bool,
    pub speed: f32,
    atk: f32,
    hp: f32,
    morale: f32,
}

/// 0 Roman legion, 1 Roman horse, 2 allied horse, 3 Gauls and Spaniards, 4 Africans, 5 Hasdrubal's
/// horse, 6 Numidians.
pub const KINDS: [Kind; 7] = [
    Kind { side: 0, cav: false, speed: 13.0, atk: 0.11, hp: 1.0, morale: 1.3 },
    Kind { side: 0, cav: true, speed: 34.0, atk: 0.20, hp: 1.6, morale: 0.8 },
    Kind { side: 0, cav: true, speed: 34.0, atk: 0.18, hp: 1.5, morale: 0.8 },
    Kind { side: 1, cav: false, speed: 14.0, atk: 0.12, hp: 1.5, morale: 1.6 },
    Kind { side: 1, cav: false, speed: 15.0, atk: 0.15, hp: 1.2, morale: 1.4 },
    Kind { side: 1, cav: true, speed: 40.0, atk: 0.26, hp: 1.8, morale: 1.4 },
    Kind { side: 1, cav: true, speed: 44.0, atk: 0.14, hp: 1.2, morale: 1.0 },
];

pub const FORMED: u8 = 0;
pub const FLEEING: u8 = 1;
pub const DEAD: u8 = 2;
pub const GONE: u8 = 3;

pub struct Stroke {
    /// x, y, length along
    pub pts: Vec<[f32; 3]>,
    pub len: f32,
    pub born: f32,
    pub done: bool,
    /// units it has carried (for the haptic when it first takes one)
    pub carried: u32,
}

#[derive(Default)]
pub struct Unit {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub fx: f32,
    pub fy: f32,
    hp: f32,
    mor: f32,
    pub kind: u8,
    grp: u16,
    pub state: u8,
    loosed: bool,
    pub on_ink: bool,
    /// fleeing but ringed in: no way out
    pub trapped: bool,
    pub fight: bool,
    hit: f32,
    /// seconds since it fell or fled (for the drawing's easing)
    pub since: f32,
    /// a per-unit random, 0..1
    pub j: f32,
}

pub struct Sim {
    seed: u32,
    pub units: Vec<Unit>,
    pub strokes: Vec<Stroke>,
    pub started: bool,
    pub time: f32,
    /// [x, y, side] of every unit that fell, in order (the drawing stains the paper with them)
    pub deaths: Vec<[f32; 3]>,
    group_ink: [i32; 64],
    river: Vec<[f32; 2]>,
    // the ink field
    fw: usize,
    fh: usize,
    f_dist: Vec<f32>,
    f_t: Vec<[f32; 2]>,
    f_p: Vec<[f32; 2]>,
    f_s: Vec<f32>,
    f_id: Vec<i32>,
    // bins
    gw: usize,
    gh: usize,
    head: Vec<i32>,
    next: Vec<i32>,
    cw: usize,
    ch: usize,
    coarse: Vec<i32>,
    dmg: Vec<f32>,
}

pub struct Ink {
    pub tx: f32,
    pub ty: f32,
    pub dx: f32,
    pub dy: f32,
    pub d: f32,
    pub left: f32,
    pub k: i32,
}

impl Sim {
    pub fn new(river: Vec<[f32; 2]>) -> Sim {
        let fw = ((WX1 - WX0) / FC).ceil() as usize + 1;
        let fh = ((WY1 - WY0) / FC).ceil() as usize + 1;
        let gw = ((WX1 - WX0) / GC).ceil() as usize + 2;
        let gh = ((WY1 - WY0) / GC).ceil() as usize + 2;
        let cw = ((WX1 - WX0) / CG).ceil() as usize;
        let ch = ((WY1 - WY0) / CG).ceil() as usize;
        let mut river = river;
        river.sort_by(|a, b| a[1].partial_cmp(&b[1]).unwrap());
        let mut s = Sim {
            seed: 216,
            units: Vec::with_capacity(3200),
            strokes: vec![],
            started: false,
            time: 0.0,
            deaths: vec![],
            group_ink: [-1; 64],
            river,
            fw,
            fh,
            f_dist: vec![1e9; fw * fh],
            f_t: vec![[0.0; 2]; fw * fh],
            f_p: vec![[0.0; 2]; fw * fh],
            f_s: vec![0.0; fw * fh],
            f_id: vec![-1; fw * fh],
            gw,
            gh,
            head: vec![-1; gw * gh],
            next: vec![],
            cw,
            ch,
            coarse: vec![-1; cw * ch * 2],
            dmg: vec![],
        };
        s.deploy();
        s
    }

    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        self.seed as f32 / 4294967296.0
    }

    fn add(&mut self, k: u8, x: f32, y: f32, g: u16) {
        let kd = KINDS[k as usize];
        let (r1, r2, r3, r4) = (self.rand(), self.rand(), self.rand(), self.rand());
        self.units.push(Unit {
            // ranks are men, not a grid: a little give in every place
            x: x + (r1 - 0.5) * 2.6,
            y: y + (r2 - 0.5) * 2.6,
            fy: if kd.side == 1 { -1.0 } else { 1.0 },
            hp: kd.hp,
            mor: kd.morale * (0.85 + r3 * 0.3),
            kind: k,
            grp: g,
            j: r4,
            ..Default::default()
        });
    }

    fn block(&mut self, k: u8, g: u16, x0: f32, y0: f32, cols: u32, rows: u32, sp: f32) {
        for r in 0..rows {
            for c in 0..cols {
                self.add(k, x0 + c as f32 * sp, y0 + r as f32 * sp, g);
            }
        }
    }

    fn deploy(&mut self) {
        let mut g = 0u16;
        // Romans, facing down the screen: maniples in three lines, checkered; depth over breadth
        let (mx0, mw, sp, gap, n) = (352.0, 5u32, 4.6, 7.0, 12u32);
        let mpx = mw as f32 * sp + gap;
        for m in 0..n {
            let x = mx0 + m as f32 * mpx;
            self.block(0, g, x, 330.0, mw, 10, sp);
            if m < n - 1 {
                self.block(0, g, x + mpx / 2.0, 278.0, mw, 11, sp);
            }
            self.block(0, g, x, 236.0, mw, 7, sp);
        }
        g += 1;
        self.block(1, g, 262.0, 352.0, 5, 14, 5.2);
        g += 1;
        self.block(2, g, 724.0, 352.0, 8, 15, 5.2);
        g += 1;
        // Hannibal's crescent, bowed toward the Romans
        for r in 0..8 {
            for c in 0..66 {
                let u = (c as f32 / 65.0) * 2.0 - 1.0;
                let x = 365.0 + c as f32 * 4.6 + (r % 2) as f32 * 2.3;
                self.add(3, x, 540.0 - 46.0 * (u * std::f32::consts::FRAC_PI_2).cos() + r as f32 * 4.6, g);
            }
        }
        g += 1;
        self.block(4, g, 318.0, 560.0, 7, 23, 4.6);
        g += 1;
        self.block(4, g, 678.0, 560.0, 7, 23, 4.6);
        g += 1;
        self.block(5, g, 256.0, 560.0, 7, 34, 5.2);
        g += 1;
        self.block(6, g, 750.0, 570.0, 10, 15, 5.4);
        self.next = vec![-1; self.units.len()];
        self.dmg = vec![0.0; self.units.len()];
    }

    // ---------- the ink ----------

    fn stamp(&mut self, k: i32, a: [f32; 3], b: [f32; 3]) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (tx, ty) = (dx / l, dy / l);
        let i0 = (((a[0].min(b[0]) - REACH - WX0) / FC).floor().max(0.0)) as usize;
        let i1 = (((a[0].max(b[0]) + REACH - WX0) / FC).ceil() as usize).min(self.fw - 1);
        let j0 = (((a[1].min(b[1]) - REACH - WY0) / FC).floor().max(0.0)) as usize;
        let j1 = (((a[1].max(b[1]) + REACH - WY0) / FC).ceil() as usize).min(self.fh - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let (cx, cy) = (WX0 + i as f32 * FC, WY0 + j as f32 * FC);
                let t = ((cx - a[0]) * tx + (cy - a[1]) * ty).clamp(0.0, l);
                let (px, py) = (a[0] + tx * t, a[1] + ty * t);
                let d = ((cx - px).powi(2) + (cy - py).powi(2)).sqrt();
                let c = j * self.fw + i;
                // the newer ink lies on top where channels cross
                if d < REACH && (d < self.f_dist[c] - 0.01 || (self.f_id[c] != k && self.f_id[c] >= 0 && d < REACH * 0.55)) {
                    self.f_dist[c] = d;
                    self.f_t[c] = [tx, ty];
                    self.f_p[c] = [px, py];
                    self.f_s[c] = a[2] + t;
                    self.f_id[c] = k;
                }
            }
        }
    }

    pub fn begin_stroke(&mut self, x: f32, y: f32) -> usize {
        self.started = true;
        self.strokes.push(Stroke { pts: vec![[x, y, 0.0]], len: 0.0, born: self.time, done: false, carried: 0 });
        self.strokes.len() - 1
    }

    /// The whole body (group) of Carthage's nearest formed man within `r` of (x, y) goes to stroke `k`.
    pub fn claim(&mut self, k: usize, x: f32, y: f32, r: f32) {
        let mut best = (r * r, None);
        for u in self.units.iter().filter(|u| u.state == FORMED && KINDS[u.kind as usize].side == 1) {
            let d2 = (u.x - x).powi(2) + (u.y - y).powi(2);
            if d2 < best.0 {
                best = (d2, Some(u.grp as usize));
            }
        }
        if let Some(g) = best.1 {
            self.group_ink[g] = k as i32;
        }
    }

    pub fn extend_stroke(&mut self, k: usize, x: f32, y: f32) {
        let p = *self.strokes[k].pts.last().unwrap();
        let d = ((x - p[0]).powi(2) + (y - p[1]).powi(2)).sqrt();
        if d < 5.0 {
            return;
        }
        // a little smoothing, as a brush would
        let q = [p[0] * 0.35 + x * 0.65, p[1] * 0.35 + y * 0.65];
        let dd = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt();
        let st = &mut self.strokes[k];
        let n = [q[0], q[1], st.len + dd];
        st.len += dd;
        st.pts.push(n);
        self.stamp(k as i32, p, n);
    }

    pub fn end_stroke(&mut self, k: usize) {
        self.strokes[k].done = true;
    }

    pub fn ink_at(&self, x: f32, y: f32) -> Option<Ink> {
        let i = ((x - WX0) / FC).round();
        let j = ((y - WY0) / FC).round();
        if i < 0.0 || j < 0.0 || i as usize >= self.fw || j as usize >= self.fh {
            return None;
        }
        let c = j as usize * self.fw + i as usize;
        if self.f_id[c] < 0 {
            return None;
        }
        let (dx, dy) = (self.f_p[c][0] - x, self.f_p[c][1] - y);
        let d = (dx * dx + dy * dy).sqrt();
        if d > REACH {
            return None;
        }
        let st = &self.strokes[self.f_id[c] as usize];
        Some(Ink { tx: self.f_t[c][0], ty: self.f_t[c][1], dx, dy, d, left: st.len - self.f_s[c], k: self.f_id[c] })
    }

    pub fn river_x(&self, y: f32) -> f32 {
        let r = &self.river;
        if r.is_empty() {
            return -999.0;
        }
        let k = r.partition_point(|p| p[1] < y);
        if k == 0 {
            return r[0][0];
        }
        if k >= r.len() {
            return r[r.len() - 1][0];
        }
        let (a, b) = (r[k - 1], r[k]);
        let t = (y - a[1]) / (b[1] - a[1]).max(1e-3);
        a[0] + t * (b[0] - a[0])
    }

    // ---------- the rules ----------

    fn bin(&mut self) {
        self.head.fill(-1);
        self.coarse.fill(-1);
        for (i, u) in self.units.iter().enumerate() {
            if u.state == FORMED {
                let cx = ((u.x - WX0) / CG).floor();
                let cy = ((u.y - WY0) / CG).floor();
                if cx >= 0.0 && cy >= 0.0 && (cx as usize) < self.cw && (cy as usize) < self.ch {
                    self.coarse[(cy as usize * self.cw + cx as usize) * 2 + KINDS[u.kind as usize].side as usize] = i as i32;
                }
            }
        }
        // the fallen are binned too: they lie where they fell and the living cannot walk through
        // them, so the dead pile into a mass the fight closes round, not over
        for (i, u) in self.units.iter().enumerate() {
            self.next[i] = -1;
            if u.state <= DEAD {
                if let Some(c) = self.cell(u.x, u.y) {
                    self.next[i] = self.head[c];
                    self.head[c] = i as i32;
                }
            }
        }
    }

    fn cell(&self, x: f32, y: f32) -> Option<usize> {
        let gx = ((x - WX0) / GC).floor() as i64 + 1;
        let gy = ((y - WY0) / GC).floor() as i64 + 1;
        if gx < 0 || gy < 0 || gx >= self.gw as i64 || gy >= self.gh as i64 {
            return None;
        }
        Some(gy as usize * self.gw + gx as usize)
    }

    pub fn step(&mut self, dt: f32) {
        for u in self.units.iter_mut() {
            if u.state != FORMED {
                u.since += dt;
            }
        }
        if !self.started {
            return;
        }
        self.time += dt;
        self.bin();
        let n = self.units.len();
        self.dmg.iter_mut().for_each(|d| *d = 0.0);
        for i in 0..n {
            if self.units[i].state >= DEAD {
                continue;
            }
            let ui = &self.units[i];
            let kd = KINDS[ui.kind as usize];
            let (side, cav) = (kd.side, kd.cav);
            let (x, y) = (ui.x, ui.y);
            let gx = ((x - WX0) / GC).floor() as i64 + 1;
            let gy = ((y - WY0) / GC).floor() as i64 + 1;
            let (mut ne, mut nd) = (-1i64, 1e9f32);
            let (mut nf, mut nfd) = (-1i64, 1e9f32);
            let (ax, ay, an, mut fnn, mut en) = (0.0f32, 0.0f32, 0, 0, 0);
            let (mut rx, mut ry, mut rear, mut flee) = (0.0f32, 0.0f32, 0, 0);
            let look = if cav { 3 } else { 2 };
            for jy in gy - look..=gy + look {
                for jx in gx - look..=gx + look {
                    if jx < 0 || jy < 0 || jx >= self.gw as i64 || jy >= self.gh as i64 {
                        continue;
                    }
                    let mut j = self.head[jy as usize * self.gw + jx as usize];
                    while j >= 0 {
                        let ju = j as usize;
                        if ju != i && self.units[ju].state < DEAD {
                            let uj = &self.units[ju];
                            let (dx, dy) = (uj.x - x, uj.y - y);
                            let d2 = dx * dx + dy * dy;
                            if KINDS[uj.kind as usize].side == side {
                                if d2 < 196.0 {
                                    if uj.state == FLEEING {
                                        flee += 1;
                                    } else {
                                        // (the spike counted every formed friend here, so its
                                        // keep-with-friends rule never ran; kept, as tuned)
                                        fnn += 1;
                                    }
                                }
                            } else if uj.state == FLEEING && ui.state == FORMED {
                                // men running close at hand: cut down once no formed foe is near
                                // (no pursuit across the sheet)
                                if d2 < 400.0 && d2 < nfd {
                                    nfd = d2;
                                    nf = j as i64;
                                }
                            } else {
                                if d2 < nd {
                                    nd = d2;
                                    ne = j as i64;
                                }
                                if d2 < 300.0 {
                                    let d = d2.sqrt().max(1e-3);
                                    rx += dx / d;
                                    ry += dy / d;
                                    en += 1;
                                    if (dx * ui.fx + dy * ui.fy) / d < -0.6 {
                                        rear += 1;
                                    }
                                }
                            }
                        }
                        j = self.next[ju];
                    }
                }
            }
            if ne < 0 && ui.state == FORMED {
                // nobody at hand: a coarser look for the nearest formed enemy further off
                let r: i64 = if cav {
                    if side == 1 {
                        if ui.loosed { 8 } else { 2 }
                    } else {
                        4
                    }
                } else if side == 1 && ui.loosed {
                    5
                } else {
                    1
                };
                let cx = ((x - WX0) / CG).floor() as i64;
                let cy = ((y - WY0) / CG).floor() as i64;
                let o = 1 - side as usize;
                for jy in cy - r..=cy + r {
                    for jx in cx - r..=cx + r {
                        if jx < 0 || jy < 0 || jx >= self.cw as i64 || jy >= self.ch as i64 {
                            continue;
                        }
                        let j = self.coarse[(jy as usize * self.cw + jx as usize) * 2 + o];
                        if j < 0 {
                            continue;
                        }
                        let uj = &self.units[j as usize];
                        let d2 = (uj.x - x).powi(2) + (uj.y - y).powi(2);
                        if d2 < nd {
                            nd = d2;
                            ne = j as i64;
                        }
                    }
                }
            }
            if ne < 0 && nf >= 0 {
                ne = nf;
                nd = nfd;
            }
            let nd = nd.sqrt();
            let (mut tx, mut ty, mut sp) = (0.0f32, 0.0f32, kd.speed);
            let mut on_ink = false;
            let mut fight = false;
            let state = ui.state;
            let loosed0 = ui.loosed;
            let grp = ui.grp as usize;
            let (fxi, fyi) = (ui.fx, ui.fy);
            let mut loosed = loosed0;
            let mut new_state = state;
            let trapped0 = ui.trapped;
            let mut trapped = false;
            let mut mor = ui.mor;
            let hit = ui.hit;
            let mut taken: Option<i32> = None;
            if state == FLEEING {
                let (mut hx, mut hy) = (-rx, -ry);
                let hl = (hx * hx + hy * hy).sqrt();
                if hl > 0.0 {
                    hx /= hl;
                    hy /= hl;
                }
                // ringed in, there is nowhere to run: men press together where they stand, so the
                // pocket stays a mass; with a way out they run for home, slowing as the fight falls
                // behind, a short trail that stops (the spike's flight wandered the whole sheet)
                let ringed = en >= 3 && (rx * rx + ry * ry).sqrt() / (en as f32) < 0.6;
                trapped = ringed || (trapped0 && en > 0);
                if trapped {
                    tx = 0.0;
                    ty = 0.0;
                } else {
                    tx = hx * 0.8;
                    ty = hy * 0.8 + if side == 1 { 0.6 } else { -0.6 };
                    sp *= if cav { 1.1 } else { 0.9 } * (-ui.since / 5.0).exp();
                }
                mor = mor.min(0.0);
            } else {
                let contact = ne >= 0 && nd < if cav { 8.0 } else { 6.4 };
                let ink = if side == 1 { self.ink_at(x, y) } else { None };
                if contact {
                    fight = true;
                    let e = ne as usize;
                    let ue = &self.units[e];
                    tx = (ue.x - x) / nd * 0.5;
                    ty = (ue.y - y) / nd * 0.5;
                    if let Some(k) = &ink {
                        tx += k.tx * 0.6;
                        ty += k.ty * 0.6;
                    }
                    // Lanchester at the point of contact: harder from the flank or rear
                    let ek = KINDS[ue.kind as usize];
                    let fdot = ((x - ue.x) * ue.fx + (y - ue.y) * ue.fy) / nd.max(1e-3);
                    let mut a = kd.atk
                        * if fdot < -0.3 { 2.2 } else if fdot < 0.3 { 1.5 } else { 1.0 }
                        * if ue.state == FLEEING { 3.0 } else { 1.0 };
                    if cav && !ek.cav {
                        a *= 0.8;
                    }
                    // Hannibal's veterans gave far less than they took (the accounts: some 6,000 of his
                    // fell against nearly ten times as many Romans)
                    if ek.side == 1 {
                        a *= CARTHAGE_TAKES;
                    }
                    let r = self.rand();
                    self.dmg[e] += a * (0.6 + r * 0.8) * dt;
                } else if let Some(k) = ink.as_ref().filter(|k| k.left > 6.0 && !(ne >= 0 && nd < if cav { 22.0 } else { 12.0 })) {
                    // carried: the stroke's direction, and drawn back toward its line
                    on_ink = true;
                    let pull = (k.d / REACH).min(1.0) * 0.9;
                    let fade = (k.left / 30.0).min(1.0);
                    let dd = k.d.max(1e-3);
                    tx = k.tx * fade + k.dx / dd * pull;
                    ty = k.ty * fade + k.dy / dd * pull;
                    sp *= 1.15;
                    taken = Some(k.k);
                    loosed = true;
                } else if ne >= 0
                    && nd < if side == 1 {
                        if loosed { if cav { 320.0 } else { 200.0 } } else if cav { 60.0 } else { 26.0 }
                    } else if cav {
                        140.0
                    } else {
                        40.0
                    }
                {
                    let ue = &self.units[ne as usize];
                    tx = (ue.x - x) / nd;
                    ty = (ue.y - y) / nd;
                } else if side == 0 {
                    // the consuls' intent: advance, and press toward the centre
                    if cav {
                        tx = 0.0;
                        ty = 0.6;
                    } else {
                        tx = (RCX - x) * 0.004;
                        ty = 1.0;
                    }
                } else if self.group_ink[grp] >= 0 && !loosed {
                    // a body some of whose men the ink has taken goes to the channel too
                    let st = &self.strokes[self.group_ink[grp] as usize];
                    let (mut bd, mut bx, mut by) = (1e9f32, 0.0, 0.0);
                    for p in st.pts.iter().step_by(2) {
                        let d = (p[0] - x).powi(2) + (p[1] - y).powi(2);
                        if d < bd && st.len - p[2] > 20.0 {
                            bd = d;
                            bx = p[0];
                            by = p[1];
                        }
                    }
                    if bd < 1e9 {
                        tx = bx - x;
                        ty = by - y;
                    } else if an > 0 {
                        tx = ax / an as f32 / kd.speed;
                        ty = ay / an as f32 / kd.speed;
                    }
                } else if an > 0 {
                    tx = ax / an as f32 / kd.speed;
                    ty = ay / an as f32 / kd.speed;
                }
                // morale: an enemy behind, or on both sides, breaks men; friends running are catching
                let surround = en >= 3 && (rx * rx + ry * ry).sqrt() / (en as f32) < 0.45;
                let terms = [
                    if rear >= 3 { 0.1 + rear as f32 * 0.07 } else { 0.0 },
                    if surround { 0.5 } else { 0.0 },
                    (flee.min(5)) as f32 * 0.006,
                    hit / dt.max(1e-4) * 0.1,
                    (en as f32 - fnn as f32 - 2.0).max(0.0) * 0.012,
                ];
                let mut dm: f32 = terms.iter().sum();
                if side == 1 {
                    dm *= CARTHAGE_NERVE;
                }
                if en == 0 && flee == 0 {
                    dm = -0.03;
                }
                mor = (kd.morale * 1.15).min(mor - dm * dt);
                if mor < 0.0 {
                    new_state = FLEEING;
                }
            }
            if let Some(k) = taken {
                self.group_ink[grp] = k;
                self.strokes[k as usize].carried += 1;
            }
            // steer
            let u = &mut self.units[i];
            u.loosed = loosed;
            u.on_ink = on_ink;
            u.trapped = trapped;
            u.fight = fight;
            u.mor = mor;
            if new_state != u.state {
                u.state = new_state;
                u.since = 0.0;
            }
            let tl = (tx * tx + ty * ty).sqrt();
            let (mut dvx, mut dvy) = (0.0, 0.0);
            if tl > 0.02 {
                let m = tl.min(1.0);
                dvx = tx / tl * sp * m;
                dvy = ty / tl * sp * m;
            }
            let acc = if cav { 2.5 } else { 3.5 };
            let k = (acc * dt).min(1.0);
            u.vx += (dvx - u.vx) * k;
            u.vy += (dvy - u.vy) * k;
            // face slowly toward the way they go or the foe they fight
            let (mut wx, mut wy) = (u.vx, u.vy);
            if fight && ne >= 0 {
                let (ex, ey) = (self.units[ne as usize].x, self.units[ne as usize].y);
                wx = ex - x;
                wy = ey - y;
            }
            let u = &mut self.units[i];
            let wl = (wx * wx + wy * wy).sqrt();
            if wl > 2.0 {
                let r = ((if fight { 2.5 } else { 1.2 }) * dt).min(1.0);
                u.fx = fxi + (wx / wl - fxi) * r;
                u.fy = fyi + (wy / wl - fyi) * r;
                let fl = (u.fx * u.fx + u.fy * u.fy).sqrt().max(1e-4);
                u.fx /= fl;
                u.fy /= fl;
            }
        }
        for i in 0..n {
            let u = &mut self.units[i];
            if u.state >= DEAD {
                continue;
            }
            u.hit = self.dmg[i];
            u.hp -= self.dmg[i];
            if u.hp <= 0.0 {
                u.state = DEAD;
                u.since = 0.0;
                self.deaths.push([u.x, u.y, KINDS[u.kind as usize].side as f32]);
                continue;
            }
            u.x += u.vx * dt;
            u.y += u.vy * dt;
            if u.x < WX0 + 10.0 || u.x > WX1 - 10.0 || u.y < WY0 + 10.0 || u.y > WY1 - 10.0 {
                u.state = GONE;
            }
        }
        // bodies take room: overlap is pushed apart, so depth presses
        self.bin();
        for _ in 0..2 {
            for i in 0..n {
                if self.units[i].state >= DEAD {
                    continue;
                }
                let ri = if KINDS[self.units[i].kind as usize].cav { 2.9 } else { 2.25 };
                let gx = ((self.units[i].x - WX0) / GC).floor() as i64 + 1;
                let gy = ((self.units[i].y - WY0) / GC).floor() as i64 + 1;
                for jy in gy - 1..=gy + 1 {
                    for jx in gx - 1..=gx + 1 {
                        if jx < 0 || jy < 0 || jx >= self.gw as i64 || jy >= self.gh as i64 {
                            continue;
                        }
                        let mut j = self.head[jy as usize * self.gw + jx as usize];
                        while j >= 0 {
                            let ju = j as usize;
                            if ju != i && self.units[ju].state == DEAD {
                                // the fallen do not move; the living step round them
                                let rr = ri + FALLEN_R;
                                let dx = self.units[ju].x - self.units[i].x;
                                let dy = self.units[ju].y - self.units[i].y;
                                let d2 = dx * dx + dy * dy;
                                if d2 < rr * rr && d2 > 1e-6 {
                                    let d = d2.sqrt();
                                    let o = (rr - d) / d * 0.5;
                                    self.units[i].x -= dx * o;
                                    self.units[i].y -= dy * o;
                                }
                            } else if ju > i && self.units[ju].state < DEAD {
                                let rr = ri + if KINDS[self.units[ju].kind as usize].cav { 2.9 } else { 2.25 };
                                let dx = self.units[ju].x - self.units[i].x;
                                let dy = self.units[ju].y - self.units[i].y;
                                let d2 = dx * dx + dy * dy;
                                if d2 < rr * rr && d2 > 1e-6 {
                                    let d = d2.sqrt();
                                    let o = (rr - d) * 0.5 / d * 0.8;
                                    self.units[i].x -= dx * o;
                                    self.units[i].y -= dy * o;
                                    self.units[ju].x += dx * o;
                                    self.units[ju].y += dy * o;
                                }
                            }
                            j = self.next[ju];
                        }
                    }
                }
            }
        }
        // the river is not crossed in battle
        for i in 0..n {
            if self.units[i].state < DEAD {
                let rx = self.river_x(self.units[i].y);
                let u = &mut self.units[i];
                if u.x < rx + 6.0 {
                    u.x = rx + 6.0;
                    u.vx = u.vx.max(0.0);
                }
            }
        }
    }

    /// An army broken: every man of `side` still formed turns to run.
    pub fn rout(&mut self, side: u8) {
        for u in self.units.iter_mut() {
            if u.state == FORMED && KINDS[u.kind as usize].side == side {
                u.state = FLEEING;
                u.since = 0.0;
                u.trapped = false;
            }
        }
    }

    /// Per side: formed, fleeing, fallen, gone off the field.
    pub fn census(&self) -> [[u32; 4]; 2] {
        let mut c = [[0u32; 4]; 2];
        for u in &self.units {
            let s = KINDS[u.kind as usize].side as usize;
            c[s][match u.state {
                FORMED => 0,
                FLEEING => 1,
                DEAD => 2,
                _ => 3,
            }] += 1;
        }
        c
    }
}
