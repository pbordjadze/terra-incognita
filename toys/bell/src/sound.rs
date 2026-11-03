//! The bell's voice: every ringing strike's modes as the ear at the screen's centre hears them, each
//! degree's singlets split by the Earth's turning (degrees 2..6; the higher ones die too soon to beat),
//! the eastward singlets a little to the right, the westward to the left. A soft knock at each strike.

use crate::modes::{self, MODES};
use std::sync::{Arc, Mutex};

/// What the toy tells the ear: where it listens, and the strikes it has not heard yet.
#[derive(Default)]
pub struct Ear {
    pub listener: [f64; 3],
    pub new: Vec<([f64; 3], f64)>,
}

struct Partial {
    mode: usize,
    m: i32,
    // the oscillator (a turning unit vector) and its step a sample
    re: f64,
    im: f64,
    rot: (f64, f64),
    rate_set: f64,
    env: f64,
    decay_k: f64,
    // the amplitudes of cos and sin, now and wanted
    c: f64,
    s: f64,
    tc: f64,
    ts: f64,
    pan: f64,
}

struct Ringing {
    pos: [f64; 3],
    strength: f64,
    partials: Vec<Partial>,
    knock: f64,
    age: f64,
}

pub struct Voice {
    ear: Arc<Mutex<Ear>>,
    ringing: Vec<Ringing>,
    listener: [f64; 3],
    seed: u32,
    lp: f64,
    aimed: [f64; 3],
    /// the level follower: strikes piled on strikes are held near one loudness
    level: f64,
}

pub const MAX_RINGING: usize = 8;

fn polar(p: [f64; 3]) -> (f64, f64) {
    // colatitude's cosine (y is north) and longitude
    (p[1].clamp(-1.0, 1.0), p[0].atan2(p[2]))
}

impl Voice {
    pub fn new(ear: Arc<Mutex<Ear>>) -> Voice {
        Voice { ear, ringing: Vec::new(), listener: [0.0, 0.0, 1.0], seed: 12345, lp: 0.0, aimed: [9.0; 3], level: 0.0 }
    }

    fn strike(&mut self, pos: [f64; 3], strength: f64) {
        let mut partials = Vec::new();
        for (i, &(l, _, _)) in MODES.iter().enumerate() {
            let ms: Vec<i32> = if (2..=6).contains(&l) { (-(l as i32)..=l as i32).collect() } else { vec![0] };
            for m in ms {
                partials.push(Partial {
                    mode: i,
                    m,
                    re: 1.0,
                    im: 0.0,
                    rot: (1.0, 0.0),
                    rate_set: 0.0,
                    env: 1.0,
                    decay_k: 1.0,
                    c: 0.0,
                    s: 0.0,
                    tc: 0.0,
                    ts: 0.0,
                    pan: (m as f64 / 6.0).clamp(-0.5, 0.5),
                });
            }
        }
        if self.ringing.len() >= MAX_RINGING {
            // the quietest gives way
            let quiet = self
                .ringing
                .iter()
                .enumerate()
                .map(|(k, r)| (k, r.partials.iter().map(|p| p.env * (p.tc.abs() + p.ts.abs())).sum::<f64>()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(k, _)| k)
                .unwrap_or(0);
            self.ringing.remove(quiet);
        }
        let mut r = Ringing { pos, strength, partials, knock: 1.0, age: 0.0 };
        Self::aim(&mut r, self.listener);
        for p in &mut r.partials {
            (p.c, p.s) = (p.tc, p.ts);
        }
        self.ringing.push(r);
    }

    /// The amplitudes each singlet has at the listener (the addition theorem, singlet by singlet).
    fn aim(r: &mut Ringing, listener: [f64; 3]) {
        let (xs, ps) = polar(r.pos);
        let (xr, pr) = polar(listener);
        let dphi = pr - ps;
        let cosg = (r.pos[0] * listener[0] + r.pos[1] * listener[1] + r.pos[2] * listener[2]).clamp(-1.0, 1.0);
        let mut pl = [0.0; 13];
        modes::legendre(cosg, &mut pl);
        let mut cache: Vec<Option<(Vec<f64>, Vec<f64>)>> = vec![None; MODES.len()];
        for p in &mut r.partials {
            let l = MODES[p.mode].0 as usize;
            let w = modes::weight(p.mode) * r.strength;
            if p.m == 0 && !(2..=6).contains(&l) {
                (p.tc, p.ts) = (w * pl[l], 0.0);
                continue;
            }
            let (a, b) = cache[p.mode].get_or_insert_with(|| (modes::schmidt(l, xs), modes::schmidt(l, xr)));
            let k = p.m.unsigned_abs() as usize;
            let prod = a[k] * b[k];
            if p.m == 0 {
                (p.tc, p.ts) = (w * prod, 0.0);
            } else {
                // cos(m Δφ) cos(ωt) split into its two travelling halves, each at its own pitch
                let ph = p.m as f64 * dphi;
                (p.tc, p.ts) = (0.5 * w * prod * ph.cos(), 0.5 * w * prod * ph.sin());
            }
        }
    }
}

impl toy_kit::Synth for Voice {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let got = self.ear.try_lock().ok().map(|mut ear| (ear.listener, ear.new.drain(..).collect::<Vec<_>>()));
        if let Some((listener, new)) = got {
            self.listener = listener;
            for (p, s) in new {
                self.strike(p, s);
            }
        }
        let listener = self.listener;
        let moved = (0..3).map(|k| (listener[k] - self.aimed[k]).abs()).sum::<f64>() > 1e-3;
        if moved {
            self.aimed = listener;
        }
        for r in &mut self.ringing {
            if moved {
                Self::aim(r, listener);
            }
            for p in &mut r.partials {
                if p.rate_set != rate {
                    let (f, tau) = modes::heard(p.mode);
                    let f = f + p.m as f64 * modes::split(p.mode);
                    let w = 2.0 * std::f64::consts::PI * f / rate;
                    p.rot = (w.cos(), w.sin());
                    p.decay_k = (-1.0 / (tau * rate)).exp();
                    p.rate_set = rate;
                }
            }
        }
        let frames = out.len() / channels.max(1);
        let gain = 0.16;
        for r in &mut self.ringing {
            let n = frames as f64;
            for p in &mut r.partials {
                if p.env < 1e-4 {
                    continue;
                }
                let (dc, ds) = ((p.tc - p.c) / n, (p.ts - p.s) / n);
                let (gl, gr) = ((1.0 - p.pan) * gain, (1.0 + p.pan) * gain);
                for f in 0..frames {
                    let (re, im) = (p.re * p.rot.0 - p.im * p.rot.1, p.re * p.rot.1 + p.im * p.rot.0);
                    p.re = re;
                    p.im = im;
                    p.env *= p.decay_k;
                    p.c += dc;
                    p.s += ds;
                    let v = p.env * (p.c * re + p.s * im);
                    if channels >= 2 {
                        out[f * channels] += (v * gl) as f32;
                        out[f * channels + 1] += (v * gr) as f32;
                    } else {
                        out[f * channels] += (v * gain) as f32;
                    }
                }
                let norm = 1.0 / (p.re * p.re + p.im * p.im).sqrt();
                p.re *= norm;
                p.im *= norm;
            }
            // the strike: a few milliseconds of soft noise and a bright, inharmonic tink (the hammer
            // on the bell), rising in 3 ms and gone in about 60
            if r.knock > 1e-3 {
                let k = (-1.0 / (0.05 * rate)).exp();
                const TINK: [(f64, f64); 4] = [(1187.0, 0.5), (1733.0, 0.35), (2411.0, 0.25), (3167.0, 0.15)];
                for f in 0..frames {
                    self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let w = (self.seed >> 8) as f64 / (1u32 << 24) as f64 * 2.0 - 1.0;
                    self.lp += (w - self.lp) * 0.08;
                    let age = r.age;
                    r.age += 1.0 / rate;
                    let rise = (age / 0.003).min(1.0);
                    let tink: f64 = TINK.iter().map(|&(fr, a)| a * (2.0 * std::f64::consts::PI * fr * age).sin() * (-age * fr / 60.0).exp()).sum();
                    let v = (rise * r.knock * r.strength * (0.5 * self.lp * (-age / 0.012).exp() + 0.22 * tink)) as f32;
                    r.knock *= k;
                    for c in 0..channels {
                        out[f * channels + c] += v;
                    }
                }
            }
        }
        self.ringing.retain(|r| r.knock > 1e-3 || r.partials.iter().any(|p| p.env > 2e-3));
        // a slow level follower over the whole: piled strikes stay near one loudness
        let peak = out.iter().fold(0f32, |m, v| m.max(v.abs())) as f64;
        let k = if peak > self.level { 0.3 } else { 0.02 };
        self.level += (peak - self.level) * k;
        let g = (0.45 / self.level.max(0.45)) as f32;
        for v in out.iter_mut() {
            *v = (*v * g).tanh();
        }
    }
}
