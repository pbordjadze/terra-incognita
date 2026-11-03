//! The fireflies: pulse-coupled oscillators (Mirollo and Strogatz, 1990), one at each patch of the
//! Earth's lights. Each has a phase that climbs at its own pace; at the top it flashes and starts
//! again. A neighbour sees the flash a moment later (a reaction's delay, longer with distance) and
//! is hurried on: the further along its own cycle, the more (a concave charge, the condition under
//! which Mirollo and Strogatz proved such a population falls into step). Just after its own flash a
//! firefly is blind to others (a refractory spell, as the beetles have). Nobody is in charge:
//! neighbours pull neighbours, so step is found locally first and spreads, and the seas, which no
//! flash crosses, keep the continents each to their own beat.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub const EARTH_KM: f64 = 6371.0;

/// The tunings, in one place.
#[derive(Clone, Copy, Debug)]
pub struct Tune {
    /// The mean period, seconds, and how much the fireflies' own periods differ (a fraction).
    pub period: f64,
    pub spread: f64,
    /// A neighbour is within this many km; at most `max_neighbours` of the nearest.
    pub reach_km: f64,
    pub max_neighbours: usize,
    /// How hard a seen flash hurries a firefly (Mirollo–Strogatz's epsilon, per neighbour).
    pub eps: f64,
    /// The concavity of the charge (their b).
    pub b: f64,
    /// The delay before a flash is seen: a reaction, plus seconds a km.
    pub delay: f64,
    pub delay_per_km: f64,
    /// After its flash, blind for this fraction of its cycle.
    pub refractory: f64,
}

impl Default for Tune {
    fn default() -> Tune {
        Tune {
            period: 1.6,
            spread: 0.07,
            reach_km: 260.0,
            max_neighbours: 10,
            eps: 0.035,
            b: 3.0,
            delay: 0.015,
            delay_per_km: 0.00018,
            refractory: 0.2,
        }
    }
}

pub struct Firefly {
    pub lon: f64,
    pub lat: f64,
    pub xyz: [f64; 3],
    /// How much light it is (0..1, from the sheet).
    pub weight: f64,
    /// Its own pace, cycles a second.
    pub freq: f64,
    /// Its phase, 0..1, at the moment `tref` (it climbs linearly from there: `phase_at`).
    pub phase: f64,
    pub tref: f64,
    /// Bumped whenever its phase is set, so a stale prediction of its next flash is ignored.
    ver: u32,
    /// When it last flashed (the sim's seconds).
    pub flashed: f64,
    /// Its neighbours and how long its flash takes to reach each.
    pub near: Vec<(u32, f32)>,
}

/// Something due: a flash seen by `to` (ver None), or `to`'s own flash if its version still holds,
/// or (force) a finger's ring of light reaching it, which makes it flash.
#[derive(PartialEq)]
struct Pulse {
    at: f64,
    to: u32,
    ver: Option<u32>,
    force: bool,
}
impl Eq for Pulse {}
impl Ord for Pulse {
    fn cmp(&self, o: &Self) -> Ordering {
        o.at.partial_cmp(&self.at).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Pulse {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

pub struct Swarm {
    pub flies: Vec<Firefly>,
    pub tune: Tune,
    pub t: f64,
    pulses: BinaryHeap<Pulse>,
    rng: u64,
}

/// A small deterministic random source (xorshift).
pub fn rand(s: &mut u64) -> f64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

fn xyz(lon: f64, lat: f64) -> [f64; 3] {
    let (la, lo) = (lat.to_radians(), lon.to_radians());
    [la.cos() * lo.sin(), la.sin(), la.cos() * lo.cos()]
}

fn km(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    d.acos() * EARTH_KM
}

impl Swarm {
    /// From the built data (tools/toys/fireflies/build.py: u32 count, then lon, lat, weight as f32).
    pub fn from_bytes(bytes: &[u8], tune: Tune, seed: u64) -> anyhow::Result<Swarm> {
        anyhow::ensure!(bytes.len() >= 4, "fireflies.bin: too short");
        let n = u32::from_le_bytes(bytes[0..4].try_into()?) as usize;
        anyhow::ensure!(
            bytes.len() >= 4 + n * 12,
            "fireflies.bin: {n} promised, fewer there"
        );
        let f = |i: usize| f32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as f64;
        let places: Vec<(f64, f64, f64)> = (0..n)
            .map(|i| (f(4 + i * 12), f(8 + i * 12), f(12 + i * 12)))
            .collect();
        Ok(Swarm::new(&places, tune, seed))
    }

    pub fn new(places: &[(f64, f64, f64)], tune: Tune, seed: u64) -> Swarm {
        let mut rng = seed.max(1);
        let mut flies: Vec<Firefly> = places
            .iter()
            .map(|&(lon, lat, weight)| {
                // a period drawn about the mean (the sum of three uniforms: near a bell)
                let g = (rand(&mut rng) + rand(&mut rng) + rand(&mut rng) - 1.5) * 2.0;
                let period = tune.period * (1.0 + tune.spread * g);
                Firefly {
                    lon,
                    lat,
                    xyz: xyz(lon, lat),
                    weight,
                    freq: 1.0 / period,
                    phase: rand(&mut rng),
                    tref: 0.0,
                    ver: 0,
                    flashed: -10.0,
                    near: Vec::new(),
                }
            })
            .collect();
        // neighbours: a grid on the sphere's cube of cells the reach's size
        let cell = 2.0 * (tune.reach_km / EARTH_KM / 2.0).sin();
        let key = |p: [f64; 3]| {
            (
                (p[0] / cell).floor() as i32,
                (p[1] / cell).floor() as i32,
                (p[2] / cell).floor() as i32,
            )
        };
        let mut grid: std::collections::HashMap<(i32, i32, i32), Vec<u32>> = Default::default();
        for (i, fl) in flies.iter().enumerate() {
            grid.entry(key(fl.xyz)).or_default().push(i as u32);
        }
        for i in 0..flies.len() {
            let p = flies[i].xyz;
            let k = key(p);
            let mut cand: Vec<(f64, u32)> = Vec::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(v) = grid.get(&(k.0 + dx, k.1 + dy, k.2 + dz)) {
                            for &j in v {
                                if j as usize != i {
                                    let d = km(p, flies[j as usize].xyz);
                                    if d <= tune.reach_km {
                                        cand.push((d, j));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            cand.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            cand.truncate(tune.max_neighbours);
            flies[i].near = cand
                .iter()
                .map(|&(d, j)| (j, (tune.delay + d * tune.delay_per_km) as f32))
                .collect();
        }
        let mut s = Swarm {
            flies,
            tune,
            t: 0.0,
            pulses: BinaryHeap::new(),
            rng,
        };
        for i in 0..s.flies.len() {
            s.predict(i);
        }
        s
    }

    /// Its phase at time `t`.
    pub fn phase_at(&self, i: usize, t: f64) -> f64 {
        let f = &self.flies[i];
        (f.phase + f.freq * (t - f.tref)).min(1.0)
    }

    /// Sets a phase now and predicts the flash it climbs to.
    fn set(&mut self, i: usize, phase: f64, t: f64) {
        let f = &mut self.flies[i];
        f.phase = phase;
        f.tref = t;
        f.ver = f.ver.wrapping_add(1);
        self.predict(i);
    }

    fn predict(&mut self, i: usize) {
        let f = &self.flies[i];
        self.pulses.push(Pulse {
            at: f.tref + (1.0 - f.phase) / f.freq,
            to: i as u32,
            ver: Some(f.ver),
            force: false,
        });
    }

    /// The charge for a phase (Mirollo–Strogatz's f: concave, 0 at 0, 1 at 1) and back.
    fn charge(&self, phase: f64) -> f64 {
        let b = self.tune.b;
        (1.0 + (b.exp() - 1.0) * phase).ln() / b
    }
    fn phase_of(&self, x: f64) -> f64 {
        let b = self.tune.b;
        ((b * x).exp() - 1.0) / (b.exp() - 1.0)
    }

    fn flash(&mut self, i: usize, at: f64) {
        self.flies[i].flashed = at;
        for k in 0..self.flies[i].near.len() {
            let (j, d) = self.flies[i].near[k];
            self.pulses.push(Pulse {
                at: at + d as f64,
                to: j,
                ver: None,
                force: false,
            });
        }
        self.set(i, 0.0, at);
    }

    /// Time moves `dt` seconds: every flash, own or seen, at its own moment, in order.
    pub fn step(&mut self, dt: f64) {
        let end = self.t + dt;
        while let Some(p) = self.pulses.peek() {
            if p.at > end {
                break;
            }
            let p = self.pulses.pop().unwrap();
            let i = p.to as usize;
            let now = p.at;
            if p.force {
                if (now - self.flies[i].flashed) * self.flies[i].freq >= self.tune.refractory {
                    self.flash(i, now);
                }
                continue;
            }
            match p.ver {
                Some(v) => {
                    if v == self.flies[i].ver {
                        self.flash(i, now);
                    }
                }
                None => {
                    let f = &self.flies[i];
                    if (now - f.flashed) * f.freq < self.tune.refractory {
                        continue;
                    }
                    let x = self.charge(self.phase_at(i, now)) + self.tune.eps;
                    if x >= 1.0 {
                        self.flash(i, now);
                    } else {
                        let ph = self.phase_of(x);
                        self.set(i, ph, now);
                    }
                }
            }
        }
        self.t = end;
    }

    /// How bright a firefly is now, 0..1: a quick rise and a slower fall after its flash.
    pub fn glow(&self, i: usize, soft: bool) -> f32 {
        let s = self.t - self.flies[i].flashed;
        let (rise, fall) = if soft { (0.12, 0.45) } else { (0.06, 0.24) };
        if s < 0.0 {
            0.0
        } else if s < rise {
            (s / rise) as f32
        } else {
            (-(s - rise) / fall).exp() as f32
        }
    }

    /// The fireflies within `r_km` of a place, nearest first.
    pub fn around(&self, at: [f64; 3], r_km: f64) -> Vec<(f64, usize)> {
        let mut v: Vec<(f64, usize)> = self
            .flies
            .iter()
            .enumerate()
            .map(|(i, f)| (km(at, f.xyz), i))
            .filter(|(d, _)| *d <= r_km)
            .collect();
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        v
    }

    /// A finger's touch: the firefly nearest flashes now, those within `r_km` of it are hurried on to
    /// the brink by different amounts (a ragged sparkle under the finger, out of step with the field
    /// round them, which pulls them back), and a ring of light leaves the finger at `speed_kms`: each
    /// firefly it reaches, out to `reach_km`, flashes as it arrives, so even a touch on the dark sea
    /// is answered by the nearest coast a beat later. When nothing is within reach, the ring carries
    /// on to the nearest dozen. Returns how many answered and when the last of them will (seconds).
    pub fn touch(&mut self, at: [f64; 3], r_km: f64, reach_km: f64, speed_kms: f64) -> (usize, f64) {
        let t = self.t;
        let mut all: Vec<(f64, usize)> = self.flies.iter().enumerate().map(|(i, f)| (km(at, f.xyz), i)).collect();
        all.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut last: f64 = 0.0;
        let mut n = 0;
        for (k, &(d, i)) in all.iter().enumerate() {
            if d <= r_km {
                if k == 0 {
                    self.flash(i, t);
                } else {
                    let r = 0.82 + 0.17 * rand(&mut self.rng);
                    self.set(i, r, t);
                }
            } else if d <= reach_km || k < 12 {
                let delay = d / speed_kms;
                self.pulses.push(Pulse { at: t + delay, to: i as u32, ver: None, force: true });
                last = last.max(delay);
            } else {
                break;
            }
            n += 1;
        }
        (n, last)
    }

    /// How in step the fireflies are, 0 (scattered) to 1 (as one), the Kuramoto order parameter of
    /// those `pick` lets in.
    pub fn order(&self, pick: impl Fn(&Firefly) -> bool) -> f64 {
        let (mut c, mut s, mut n) = (0.0, 0.0, 0.0);
        for (i, _) in self.flies.iter().enumerate().filter(|(_, f)| pick(f)) {
            let a = self.phase_at(i, self.t) * std::f64::consts::TAU;
            c += a.cos();
            s += a.sin();
            n += 1.0;
        }
        if n == 0.0 {
            0.0
        } else {
            (c * c + s * s).sqrt() / n
        }
    }

    /// The phases, to keep (a firefly's step is the toy's whole state).
    pub fn phases(&self) -> Vec<f32> {
        (0..self.flies.len())
            .map(|i| self.phase_at(i, self.t) as f32)
            .collect()
    }
    pub fn set_phases(&mut self, p: &[f32]) {
        if p.len() == self.flies.len() {
            self.pulses.clear();
            let t = self.t;
            for (i, &q) in p.iter().enumerate() {
                self.set(i, (q as f64).clamp(0.0, 0.999), t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of fireflies 60 km apart, as along a coast.
    fn line(n: usize) -> Vec<(f64, f64, f64)> {
        (0..n).map(|i| (i as f64 * 0.54, 40.0, 1.0)).collect()
    }

    #[test]
    fn neighbours_fall_into_step() {
        let mut s = Swarm::new(&line(40), Tune::default(), 7);
        let r0 = s.order(|_| true);
        for _ in 0..(60 * 60) {
            s.step(1.0 / 60.0);
        }
        let r1 = s.order(|_| true);
        assert!(r1 > 0.6 && r1 > r0 + 0.4, "order {r0:.2} -> {r1:.2}");
    }

    #[test]
    fn a_touch_on_the_sea_reaches_the_nearest_lights() {
        let mut s = Swarm::new(&line(10), Tune::default(), 5);
        // 20 degrees west of the line's start, over open water: nothing within reach
        let (n, last) = s.touch(xyz(-20.0, 40.0), 50.0, 300.0, 2000.0);
        assert_eq!(n, 10);
        assert!(last > 0.8 && last < 1.2, "the ring reaches them in {last:.2} s");
        s.step(1.3);
        assert!(s.flies.iter().all(|f| f.flashed > 0.5), "every one flashed as the ring arrived");
    }

    #[test]
    fn seas_keep_continents_apart() {
        let mut places = line(20);
        places.extend(line(20).into_iter().map(|(lo, la, w)| (lo + 60.0, la, w)));
        let s = Swarm::new(&places, Tune::default(), 3);
        assert!(s.flies[..20]
            .iter()
            .all(|f| f.near.iter().all(|&(j, _)| j < 20)));
    }

    #[test]
    fn a_nudge_scatters_a_patch() {
        let mut s = Swarm::new(&line(40), Tune::default(), 7);
        for _ in 0..(60 * 60) {
            s.step(1.0 / 60.0);
        }
        let (n, _) = s.touch(xyz(10.0, 40.0), 200.0, 400.0, 2000.0);
        assert!(n >= 5);
        assert!(s.order(|_| true) < 0.95);
    }
}
