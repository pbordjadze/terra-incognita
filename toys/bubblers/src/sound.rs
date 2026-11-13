//! The beach's sound: each ball set down a tiny dry tick, a bolting crowd a rustle of legs on sand,
//! the surf a long way off breaking and drawing back, and the tide's wash when it comes in.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

/// One sound: a burst of noise after `delay`, `amp`, dying in `tau`, its highs kept by `bright`
/// (0 dull .. 1 crisp), at `pan` (-1 left .. 1 right).
#[derive(Clone, Copy, Debug)]
pub struct Tick {
    pub delay: f32,
    pub amp: f32,
    pub tau: f32,
    pub bright: f32,
    pub pan: f32,
}

#[derive(Default)]
struct Shared {
    queue: Vec<Tick>,
    wash: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Shared>>);

impl Sound {
    pub fn send(&self, ticks: &[Tick]) {
        if let Ok(mut s) = self.0.lock() {
            let room = 400usize.saturating_sub(s.queue.len());
            s.queue.extend(ticks.iter().take(room));
        }
    }
    /// The tide's water over the sand, 0..1.
    pub fn wash(&self, level: f32) {
        if let Ok(mut s) = self.0.lock() {
            s.wash = level;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Voice {
            shared: self.0.clone(),
            clock: 0.0,
            waiting: Vec::new(),
            live: Vec::new(),
            seed: 0x2545_F491,
            wash: 0.0,
            surf: [0.0; 3],
        })
    }
}

struct Live {
    k: Tick,
    t: f32,
    lp: f32,
    hp: f32,
}

struct Voice {
    shared: Arc<Mutex<Shared>>,
    clock: f64,
    waiting: Vec<(f64, Tick)>,
    live: Vec<Live>,
    seed: u32,
    wash: f32,
    surf: [f32; 3],
}

impl Voice {
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

impl Synth for Voice {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let mut wash = self.wash;
        if let Ok(mut s) = self.shared.lock() {
            for k in s.queue.drain(..) {
                self.waiting.push((self.clock + k.delay as f64, k));
            }
            wash = s.wash;
        }
        let dt = 1.0 / rate;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            let now = self.clock;
            let mut i = 0;
            while i < self.waiting.len() {
                if self.waiting[i].0 <= now {
                    let (_, k) = self.waiting.swap_remove(i);
                    if self.live.len() < 120 {
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
            let (mut l, mut r) = (0.0f32, 0.0f32);
            let mut i = 0;
            while i < self.live.len() {
                let n = self.noise();
                let v = &mut self.live[i];
                let env = (v.t / 0.0008).min(1.0) * (-v.t / v.k.tau).exp();
                let fc = 300.0 * (25.0f32).powf(v.k.bright);
                let a = 1.0 - (-std::f32::consts::TAU * fc / rate as f32).exp();
                v.lp += a * (n - v.lp);
                // a little high-pass: a tick has no boom
                v.hp += 0.02 * (v.lp - v.hp);
                let s = (v.lp - v.hp) * env * v.k.amp;
                l += s * (1.0 - v.k.pan) * 0.5;
                r += s * (1.0 + v.k.pan) * 0.5;
                v.t += dt as f32;
                if v.t > v.k.tau * 7.0 {
                    self.live.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            // the surf, far off: low noise swelling every eight seconds or so
            let n = self.noise();
            self.surf[0] += 0.004 * (n - self.surf[0]);
            self.surf[1] += 0.004 * (self.surf[0] - self.surf[1]);
            let ph = (self.clock / 8.3).fract() as f32;
            let swell = 0.35 + 0.65 * (ph * std::f32::consts::TAU).sin().max(0.0).powi(2);
            let surf = self.surf[1] * 2.2 * swell;
            // the wash over the sand: brighter, nearer
            self.wash += (wash - self.wash) * 0.0003;
            let n2 = self.noise();
            self.surf[2] += 0.12 * (n2 - self.surf[2]);
            let water = self.surf[2] * self.wash * 0.18;
            l += surf + water;
            r += surf * 0.9 + water * 1.05;
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
