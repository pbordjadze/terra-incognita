//! Moon Road's sound: the lake lapping at the beach, a little louder in a gust, and a soft glassy
//! note for each facet the moon paints near you (the nearer, the louder; across the road, a step of
//! a pentatonic scale), so the painting is heard as it goes.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Clone, Copy, Debug)]
pub struct Chime {
    pub freq: f32,
    pub amp: f32,
    pub pan: f32,
}

#[derive(Default)]
struct Shared {
    queue: Vec<Chime>,
    wind: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Shared>>);

impl Sound {
    pub fn send(&self, c: &[Chime]) {
        if let Ok(mut s) = self.0.lock() {
            let room = 24usize.saturating_sub(s.queue.len());
            s.queue.extend(c.iter().take(room));
        }
    }
    /// How gusty it is, 0..1: the lapping's level.
    pub fn wind(&self, w: f32) {
        if let Ok(mut s) = self.0.lock() {
            s.wind = w;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Voice {
            shared: self.0.clone(),
            clock: 0.0,
            live: Vec::new(),
            seed: 0x1234_5679,
            lp: [0.0; 3],
            wind: 0.0,
        })
    }
}

struct Live {
    c: Chime,
    t: f32,
    phase: f32,
}

struct Voice {
    shared: Arc<Mutex<Shared>>,
    clock: f64,
    live: Vec<Live>,
    seed: u32,
    lp: [f32; 3],
    wind: f32,
}

impl Voice {
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

const DECAY_S: f32 = 1.6;

impl Synth for Voice {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let mut wind = self.wind;
        if let Ok(mut s) = self.shared.lock() {
            for c in s.queue.drain(..) {
                if self.live.len() >= 20 {
                    self.live.remove(0);
                }
                self.live.push(Live {
                    c,
                    t: 0.0,
                    phase: 0.0,
                });
            }
            wind = s.wind;
        }
        let dt = 1.0 / rate as f32;
        let tau = std::f32::consts::TAU;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in &mut self.live {
                let env = (v.t / 0.006).min(1.0) * (-6.9 * v.t / DECAY_S).exp();
                // a sine with its octave and twelfth faint: glass more than bell
                let p = v.phase * tau;
                let s = (p.sin()
                    + 0.18 * (2.0 * p).sin()
                    + 0.06 * (3.0 * p).sin() * (-v.t * 4.0).exp())
                    * env
                    * v.c.amp;
                l += s * (1.0 - v.c.pan) * 0.5;
                r += s * (1.0 + v.c.pan) * 0.5;
                v.phase = (v.phase + v.c.freq * dt).fract();
                v.t += dt;
            }
            self.live.retain(|v| v.t < DECAY_S * 1.3);
            // the lapping: low noise that swells as each small wave reaches the beach
            self.wind += (wind - self.wind) * 0.00005;
            let n = self.noise();
            let a = 0.03 + 0.02 * self.wind;
            self.lp[0] += a * (n - self.lp[0]);
            self.lp[1] += a * (self.lp[0] - self.lp[1]);
            let ph = (self.clock / 2.1).fract() as f32;
            let lap = 0.25 + 0.75 * (ph * tau).sin().max(0.0).powi(3);
            let n2 = self.noise();
            self.lp[2] += 0.25 * (n2 - self.lp[2]);
            let water = self.lp[1] * 1.6 * lap * (0.6 + 0.6 * self.wind)
                + self.lp[2] * 0.012 * lap * (0.3 + self.wind);
            l += water;
            r += water * 0.92;
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
            self.clock += dt as f64;
        }
    }
}

fn soft(x: f32) -> f32 {
    x / (1.0 + x.abs())
}
