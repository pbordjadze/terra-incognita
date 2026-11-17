//! The bay's sound: the water a finger stirs (a soft swish as loud as the stirring), raindrops and a fish's
//! dart as small splashes, and the coquí frogs in the mangroves, calling "co-quí" from all round, far and near.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

/// One splash: a burst of noise after `delay`, `amp`, dying in `tau`, its highs kept by `bright`
/// (0 dull .. 1 crisp), at `pan` (-1 left .. 1 right).
#[derive(Clone, Copy, Debug)]
pub struct Splash {
    pub delay: f32,
    pub amp: f32,
    pub tau: f32,
    pub bright: f32,
    pub pan: f32,
}

#[derive(Default)]
struct Shared {
    queue: Vec<Splash>,
    swish: f32,
    pan: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Shared>>);

impl Sound {
    pub fn send(&self, splashes: &[Splash]) {
        if let Ok(mut s) = self.0.lock() {
            let room = 200usize.saturating_sub(s.queue.len());
            s.queue.extend(splashes.iter().take(room));
        }
    }
    /// The water being stirred now, 0..1, and where (-1 left .. 1 right).
    pub fn swish(&self, level: f32, pan: f32) {
        if let Ok(mut s) = self.0.lock() {
            s.swish = level;
            s.pan = pan;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Voice {
            shared: self.0.clone(),
            clock: 0.0,
            waiting: Vec::with_capacity(256),
            live: Vec::with_capacity(64),
            seed: 0x1234_5679,
            swish: 0.0,
            pan: 0.0,
            water: [0.0; 3],
            frogs: Vec::with_capacity(16),
            next_frog: 0.6,
        })
    }
}

struct Live {
    k: Splash,
    t: f32,
    lp: f32,
    hp: f32,
}

/// A coquí's call: "co", a short low note, then "quí", higher and rising.
struct Frog {
    t: f32,
    amp: f32,
    pan: f32,
    pitch: f32,
    phase: f32,
}

struct Voice {
    shared: Arc<Mutex<Shared>>,
    clock: f64,
    waiting: Vec<(f64, Splash)>,
    live: Vec<Live>,
    seed: u32,
    swish: f32,
    pan: f32,
    water: [f32; 3],
    frogs: Vec<Frog>,
    next_frog: f64,
}

impl Voice {
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn take(&mut self) {
        // never wait on the main thread here: a busy lock leaves the queue for the next buffer
        if let Ok(mut s) = self.shared.try_lock() {
            for k in s.queue.drain(..) {
                self.waiting.push((self.clock + k.delay as f64, k));
            }
            self.swish += (s.swish - self.swish) * 0.25;
            self.pan = s.pan;
        }
    }

    fn start_due(&mut self) {
        let now = self.clock;
        let mut i = 0;
        while i < self.waiting.len() {
            if self.waiting[i].0 <= now {
                let (_, k) = self.waiting.swap_remove(i);
                if self.live.len() < 60 && self.live.len() < self.live.capacity() {
                    self.live.push(Live {
                        k,
                        t: 0.0,
                        lp: 0.0,
                        hp: 0.0,
                    });
                }
            } else {
                i += 1;
            }
        }
        if now >= self.next_frog && self.frogs.len() < self.frogs.capacity() {
            let near = (self.noise() * 0.5 + 0.5).powi(3);
            let pan = self.noise() * 0.9;
            let pitch = 1.0 + self.noise() * 0.06;
            self.frogs.push(Frog {
                t: 0.0,
                amp: 0.012 + 0.05 * near,
                pan,
                pitch,
                phase: 0.0,
            });
            // a chorus: calls a second or two apart, sometimes a pair close together
            self.next_frog = now + 0.35 + 1.6 * (self.noise() * 0.5 + 0.5) as f64;
        }
    }

    fn splashes(&mut self, rate: f32, dt: f32) -> (f32, f32) {
        let (mut l, mut r) = (0.0f32, 0.0f32);
        let mut i = 0;
        while i < self.live.len() {
            let n = self.noise();
            let v = &mut self.live[i];
            let env = (v.t / 0.002).min(1.0) * (-v.t / v.k.tau).exp();
            let fc = 250.0 * (20.0f32).powf(v.k.bright);
            let a = 1.0 - (-std::f32::consts::TAU * fc / rate).exp();
            v.lp += a * (n - v.lp);
            v.hp += 0.03 * (v.lp - v.hp);
            let s = (v.lp - v.hp) * env * v.k.amp;
            l += s * (1.0 - v.k.pan) * 0.5;
            r += s * (1.0 + v.k.pan) * 0.5;
            v.t += dt;
            if v.t > v.k.tau * 7.0 {
                self.live.swap_remove(i);
            } else {
                i += 1;
            }
        }
        (l, r)
    }

    fn frogs(&mut self, dt: f32) -> (f32, f32) {
        let (mut l, mut r) = (0.0f32, 0.0f32);
        for f in &mut self.frogs {
            // co: 0..0.09 s at ~1.15 kHz; quí: 0.13..0.27 s rising 1.9 to 2.2 kHz
            let (hz, env) = if f.t < 0.09 {
                (
                    1150.0,
                    (f.t / 0.01).min(1.0) * ((0.09 - f.t) / 0.02).min(1.0),
                )
            } else if f.t > 0.13 && f.t < 0.27 {
                let u = (f.t - 0.13) / 0.14;
                (
                    1900.0 + 300.0 * u,
                    (u / 0.08).min(1.0) * ((1.0 - u) / 0.15).min(1.0),
                )
            } else {
                (1500.0, 0.0)
            };
            f.phase = (f.phase + hz * f.pitch * dt).fract();
            let s = (f.phase * std::f32::consts::TAU).sin() * env * f.amp;
            l += s * (1.0 - f.pan) * 0.5;
            r += s * (1.0 + f.pan) * 0.5;
            f.t += dt;
        }
        self.frogs.retain(|f| f.t < 0.3);
        (l, r)
    }
}

impl Synth for Voice {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        self.take();
        let dt = 1.0 / rate;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            self.start_due();
            let (sl, sr) = self.splashes(rate as f32, dt as f32);
            let (fl, fr) = self.frogs(dt as f32);
            // the stirred water: noise, band-limited, as loud as the stirring
            let n = self.noise();
            self.water[0] += 0.08 * (n - self.water[0]);
            self.water[1] += 0.08 * (self.water[0] - self.water[1]);
            self.water[2] += 0.004 * (self.water[1] - self.water[2]);
            let w = (self.water[1] - self.water[2]) * self.swish * 0.9;
            let l = sl + fl + w * (1.0 - self.pan) * 0.5;
            let r = sr + fr + w * (1.0 + self.pan) * 0.5;
            let (ol, or) = (soft(l), soft(r));
            if channels >= 2 {
                out[f * channels] = ol;
                out[f * channels + 1] = or;
                for c in 2..channels {
                    out[f * channels + c] = 0.0;
                }
            } else if channels == 1 {
                out[f] = (ol + or) * 0.5;
            }
            self.clock += dt;
        }
    }
}

fn soft(x: f32) -> f32 {
    x / (1.0 + x.abs())
}
