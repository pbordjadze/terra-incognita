//! What a bat detector by the cave mouth would let you hear: the bats' calls, too high for the ear,
//! brought down to clicks (as a heterodyne detector does), thickening into a crackle as the column
//! builds and spattering when a swerve runs through; under them the crickets of a Texas summer dusk,
//! and the air past the hawk when it dives.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
pub struct Mix {
    /// clicks a second
    pub clicks: f32,
    /// the hawk's speed, metres a second
    pub hawk: f32,
    /// 0 at the start of dusk, 1 at its deepest (the crickets come up)
    pub dusk: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Mix>>);

impl Sound {
    pub fn set(&self, m: Mix) {
        if let Ok(mut s) = self.0.lock() {
            *s = m;
        }
    }

    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Detector {
            mix: self.0.clone(),
            now: Mix::default(),
            clicks: Vec::with_capacity(96),
            seed: 0x9E37_79B9,
            owed: 0.0,
            t: 0.0,
            lp: [0.0; 2],
            cricket: 0.0,
        })
    }
}

struct Click {
    phase: f32,
    freq: f32,
    amp: f32,
    left: f32,
    right: f32,
    age: f32,
    decay: f32,
}

struct Detector {
    mix: Arc<Mutex<Mix>>,
    now: Mix,
    clicks: Vec<Click>,
    seed: u32,
    owed: f32,
    t: f64,
    lp: [f32; 2],
    cricket: f32,
}

impl Detector {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
}

impl Synth for Detector {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(m) = self.mix.try_lock() {
            self.now = *m;
        }
        let dt = 1.0 / rate as f32;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            self.t += dt as f64;
            // new clicks, at random (a Poisson stream at the mix's rate)
            self.owed += self.now.clicks.min(900.0) * dt;
            while self.owed > 0.0 && self.rand() < self.owed {
                self.owed -= 1.0;
                if self.clicks.len() < 96 {
                    let pan = self.rand();
                    let amp = 0.03 + 0.12 * self.rand() * self.rand();
                    let freq = 1700.0 + 2300.0 * self.rand();
                    let decay = 1.0 / (0.0012 + 0.0022 * self.rand());
                    self.clicks.push(Click {
                        phase: 0.0,
                        freq,
                        amp,
                        left: (1.0 - pan).sqrt(),
                        right: pan.sqrt(),
                        age: 0.0,
                        decay,
                    });
                }
            }
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for c in &mut self.clicks {
                // a soft start, then an exponential ring
                let env = (c.age * 4000.0).min(1.0) * (-c.age * c.decay).exp();
                let s = (c.phase * std::f32::consts::TAU).sin() * env * c.amp;
                l += s * c.left;
                r += s * c.right;
                c.phase = (c.phase + c.freq * dt).fract();
                c.age += dt;
            }
            self.clicks.retain(|c| c.age * c.decay < 9.0);
            // crickets: a chirp of three pulses, about twice a second, soft, coming up with the dark
            let tc = (self.t * 2.1).fract() as f32;
            let pulse = ((tc * 3.0 / 0.33).fract() < 0.5 && tc < 0.33) as u8 as f32;
            self.cricket += (pulse - self.cricket) * (dt * 400.0).min(1.0);
            let cr = ((self.t * 4400.0).fract() as f32 * std::f32::consts::TAU).sin()
                * self.cricket
                * (0.004 + 0.012 * self.now.dusk);
            // the air past the hawk: noise, low-passed more as it slows
            let n = self.rand() * 2.0 - 1.0;
            let speed = (self.now.hawk / 30.0).clamp(0.0, 1.0);
            let k = 0.02 + 0.25 * speed;
            self.lp[0] += (n - self.lp[0]) * k;
            self.lp[1] += (self.lp[0] - self.lp[1]) * k;
            let air = self.lp[1] * speed * speed * 0.9;
            let (ol, or) = ((l + cr * 0.8 + air).tanh(), (r + cr + air).tanh());
            for ch in 0..channels {
                out[f * channels + ch] = if ch % 2 == 0 { ol } else { or };
            }
        }
    }
}
