//! The huddle's sound: the wind over the ice, in gusts, as strong as it blows; and now and then an
//! emperor's call, a run of short buzzing syllables in two voices a little apart (the bird sings
//! with both sides of its syrinx at once, so the two beat against each other). A call comes when a
//! penguin presses back in, and when a finger touches one.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default)]
struct Shared {
    wind: f32,
    calls: Vec<(f32, f32)>,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Shared>>);

impl Sound {
    /// The wind's speed, metres a second.
    pub fn wind(&self, speed: f32) {
        if let Ok(mut s) = self.0.lock() {
            s.wind = speed;
        }
    }

    /// A call, from `pan` (-1 left .. 1 right), `near` (0 far .. 1 close) loud.
    pub fn call(&self, pan: f32, near: f32) {
        if let Ok(mut s) = self.0.lock() {
            if s.calls.len() < 4 {
                s.calls.push((pan, near));
            }
        }
    }

    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Ice {
            shared: self.0.clone(),
            wind: 0.0,
            seed: 0x51CE_0001,
            lp: [0.0; 3],
            gust: 0.0,
            t: 0.0,
            voices: Vec::new(),
        })
    }
}

struct Voice {
    f0: f32,
    phase: [f32; 2],
    age: f32,
    syllables: u32,
    left: f32,
    right: f32,
    amp: f32,
}

struct Ice {
    shared: Arc<Mutex<Shared>>,
    wind: f32,
    seed: u32,
    lp: [f32; 3],
    gust: f32,
    t: f32,
    voices: Vec<Voice>,
}

impl Ice {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }

    fn take(&mut self) {
        let Ok(mut s) = self.shared.try_lock() else {
            return;
        };
        self.wind = s.wind;
        let calls = std::mem::take(&mut s.calls);
        drop(s);
        for (pan, near) in calls {
            let p = (pan * 0.5 + 0.5).clamp(0.0, 1.0);
            let f0 = 420.0 + 140.0 * self.rand();
            let syllables = 3 + (self.rand() * 4.0) as u32;
            self.voices.push(Voice {
                f0,
                phase: [0.0; 2],
                age: 0.0,
                syllables,
                left: (1.0 - p).sqrt(),
                right: p.sqrt(),
                amp: 0.04 + 0.1 * near,
            });
        }
    }
}

/// One syllable: 0.26 s of sound then 0.09 s of quiet; its envelope at time `t` into it.
fn syllable(t: f32) -> f32 {
    let on = 0.26;
    if t > on {
        return 0.0;
    }
    let a = (t / 0.03).min(1.0);
    let r = ((on - t) / 0.05).min(1.0);
    a * r
}

impl Synth for Ice {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        self.take();
        let dt = 1.0 / rate as f32;
        let frames = out.len() / channels.max(1);
        let tau = std::f32::consts::TAU;
        for f in 0..frames {
            self.t += dt;
            // the wind: noise through low-passes, its loudness drifting in gusts
            let n = self.rand() * 2.0 - 1.0;
            let speed = (self.wind / 12.0).clamp(0.05, 1.2);
            self.gust += ((0.6 + 0.4 * (self.t * 0.31).sin() * (self.t * 0.13 + 2.0).sin())
                - self.gust)
                * dt
                * 0.5;
            let k = 0.015 + 0.05 * speed * self.gust;
            self.lp[0] += (n - self.lp[0]) * k;
            self.lp[1] += (self.lp[0] - self.lp[1]) * k;
            self.lp[2] += (n - self.lp[2]) * (0.2 * speed);
            let hiss = (n - self.lp[2]) * 0.015 * speed * self.gust;
            let roar = self.lp[1] * 2.2 * speed * self.gust;
            let (mut l, mut r) = (roar + hiss, roar * 0.92 + hiss * 1.1);
            // the calls
            for v in &mut self.voices {
                let per = 0.35;
                let k = (v.age / per) as u32;
                if k < v.syllables {
                    let ts = v.age - k as f32 * per;
                    // each syllable glides down a little; the last is longer and lower
                    let glide =
                        1.0 - 0.06 * ts / 0.26 - if k + 1 == v.syllables { 0.05 } else { 0.0 };
                    let e = syllable(ts) * v.amp;
                    let mut s = 0.0;
                    for (vi, ratio) in [1.0f32, 1.035].iter().enumerate() {
                        let fr = v.f0 * ratio * glide;
                        v.phase[vi] = (v.phase[vi] + fr * dt).fract();
                        let ph = v.phase[vi] * tau;
                        // a buzzy voice: a few harmonics
                        s += ph.sin()
                            + 0.6 * (2.0 * ph).sin()
                            + 0.35 * (3.0 * ph).sin()
                            + 0.2 * (4.0 * ph).sin();
                    }
                    l += s * e * v.left * 0.5;
                    r += s * e * v.right * 0.5;
                }
                v.age += dt;
            }
            self.voices
                .retain(|v| v.age < v.syllables as f32 * 0.35 + 0.1);
            let (ol, or) = (l.tanh(), r.tanh());
            for ch in 0..channels {
                out[f * channels + ch] = if ch % 2 == 0 { ol } else { or };
            }
        }
    }
}
