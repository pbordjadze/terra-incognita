//! The night heard: each step the walker takes is a footfall, louder the longer the step, so short
//! steps patter, long ones land one by one, and a try it doesn't take makes no sound (a walker refusing
//! every step is heard standing in silence). Under it a little wind and far crickets; at dawn the
//! crickets stop and a few birds start. Drawn from code, nothing recorded.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default)]
struct Shared {
    /// Steps since the synth last looked: (length, pan).
    steps: Vec<(f32, f32)>,
    stride: f32,
    dawn: f32,
    light: f32,
}

#[derive(Clone, Default)]
pub struct Night(Arc<Mutex<Shared>>);

impl Night {
    pub fn new() -> Night {
        Night::default()
    }
    pub fn set(&self, steps: &[(f32, f32)], stride: f32, dawn: f32, light: f32) {
        if let Ok(mut s) = self.0.try_lock() {
            if s.steps.len() < 256 {
                s.steps.extend_from_slice(steps);
            }
            s.stride = stride;
            s.dawn = dawn;
            s.light = light;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Ears {
            shared: self.0.clone(),
            noise: 0x2468_ace1,
            falls: Vec::new(),
            pending: Vec::new(),
            wind: [0.0; 3],
            cricket: 0.0,
            chirp_t: 0.0,
            birds: Vec::new(),
            bird_wait: 1.0,
            dawn: 0.0,
            light: 1.0,
            fade: 0.0,
            clock: 0.0,
        })
    }
}

/// One footfall: a short soft knock of filtered noise with a little low body.
struct Fall {
    at: f32,
    amp: f32,
    pan: f32,
    lp: [f32; 2],
    body: f32,
    tone: f32,
}

struct Bird {
    age: f32,
    phase: f32,
    f0: f32,
    pan: f32,
    amp: f32,
}

struct Ears {
    shared: Arc<Mutex<Shared>>,
    noise: u32,
    falls: Vec<Fall>,
    pending: Vec<(f32, f32)>,
    wind: [f32; 3],
    cricket: f32,
    chirp_t: f32,
    birds: Vec<Bird>,
    bird_wait: f32,
    dawn: f32,
    light: f32,
    fade: f32,
    clock: f32,
}

impl Ears {
    fn white(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

impl Synth for Ears {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let rate = rate as f32;
        let frames = out.len() / channels.max(1);
        let dt = 1.0 / rate;
        let mut stride = 0.03;
        if let Ok(mut s) = self.shared.try_lock() {
            self.pending.append(&mut s.steps);
            stride = s.stride.max(0.004);
            self.dawn = s.dawn;
            self.light = s.light;
        }
        // spread this buffer's steps across it, as they were spread across the frame
        let span = frames as f32 * dt;
        let n = self.pending.len().min(48);
        let pending: Vec<(f32, f32)> = self.pending.drain(..).take(n).collect();
        for (k, (len, pan)) in pending.into_iter().enumerate() {
            if self.falls.len() >= 48 {
                break;
            }
            // a step's loudness by its length: a shuffle under a hundredth of the screen, a footfall at a tenth
            let amp = (len / 0.12).sqrt().clamp(0.05, 1.0) * 0.5;
            let tone =
                (0.25 + 0.5 * (len / stride.max(len)).min(1.0)) * (0.8 + 0.4 * self.white().abs());
            self.falls.push(Fall {
                at: -(k as f32 + 0.5) / n.max(1) as f32 * span,
                amp,
                pan: pan.clamp(-1.0, 1.0) * 0.6,
                lp: [0.0; 2],
                body: 0.0,
                tone,
            });
        }
        self.pending.clear();
        let night = (1.0 - self.dawn) * self.light;
        for f in 0..frames {
            self.clock += dt;
            self.fade = (self.fade + dt / 1.5).min(1.0);
            let w = self.white();
            // wind: brown-ish noise, slowly breathing
            self.wind[0] += (w - self.wind[0]) * 0.02;
            self.wind[1] += (self.wind[0] - self.wind[1]) * 0.02;
            let gust = 0.6 + 0.4 * (self.clock * 0.21).sin() * (self.clock * 0.07 + 1.0).sin();
            let wind = self.wind[1] * 0.10 * gust * self.light;
            // far crickets: a high pulse train in bursts, by night
            self.chirp_t += dt;
            let burst = (self.chirp_t % 0.9) < 0.22;
            self.cricket = (self.cricket + 4300.0 * dt).fract();
            let pulse = if burst {
                (0.5 + 0.5 * (self.chirp_t * 2.0 * std::f32::consts::PI * 28.0).sin()).powi(3)
            } else {
                0.0
            };
            let cr = (self.cricket * std::f32::consts::TAU).sin() * pulse * 0.010 * night;
            // footfalls
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for fa in self.falls.iter_mut() {
                fa.at += dt;
                if fa.at < 0.0 {
                    continue;
                }
                let env = (fa.at / 0.002).min(1.0) * (-fa.at / 0.028).exp();
                let a = 0.06 + 0.25 * fa.tone;
                fa.lp[0] += (w - fa.lp[0]) * a;
                fa.lp[1] += (fa.lp[0] - fa.lp[1]) * a;
                fa.body += dt * (90.0 + 60.0 * fa.tone);
                let thud = (fa.body * std::f32::consts::TAU).sin() * (-fa.at / 0.04).exp() * 0.5;
                let v = (fa.lp[1] * 2.2 + thud) * env * fa.amp;
                l += v * (0.6 - 0.4 * fa.pan);
                r += v * (0.6 + 0.4 * fa.pan);
            }
            // birds at dawn: short falling whistles
            let mut bl = 0.0;
            let mut br = 0.0;
            for b in self.birds.iter_mut() {
                b.age += dt;
                if b.age < 0.0 {
                    continue;
                }
                let f = b.f0
                    * (1.0 - 0.25 * (b.age / 0.12).min(1.0))
                    * (1.0 + 0.04 * (b.age * 90.0).sin());
                b.phase = (b.phase + f * dt).fract();
                let env = (b.age / 0.01).min(1.0) * (-b.age / 0.05).exp();
                let v = (b.phase * std::f32::consts::TAU).sin() * env * b.amp;
                bl += v * (0.6 - 0.4 * b.pan);
                br += v * (0.6 + 0.4 * b.pan);
            }
            if f == 0 {
                self.falls.retain(|fa| fa.at < 0.25);
                self.birds.retain(|b| b.age < 0.4);
            }
            let g = self.fade * self.light;
            let lo = ((l + bl) * g + wind + cr).tanh();
            let ro = ((r + br) * g + wind + cr).tanh();
            if channels >= 2 {
                out[f * channels] = lo;
                out[f * channels + 1] = ro;
                for c in 2..channels {
                    out[f * channels + c] = 0.0;
                }
            } else if channels == 1 {
                out[f] = 0.5 * (lo + ro);
            }
        }
        // the birds' next call
        if self.dawn > 0.5 && self.light > 0.3 {
            self.bird_wait -= span;
            if self.bird_wait <= 0.0 && self.birds.len() < 6 {
                let x = self.white();
                let calls = 2 + ((x.abs() * 3.0) as usize);
                let f0 = 2600.0 + 900.0 * x;
                let pan = self.white() * 0.7;
                for k in 0..calls {
                    self.birds.push(Bird {
                        age: -(k as f32) * 0.13,
                        phase: 0.0,
                        f0: f0 * (1.0 - 0.03 * k as f32),
                        pan,
                        amp: 0.05,
                    });
                }
                self.bird_wait = 1.2 + 2.5 * self.white().abs();
            }
        }
    }
}
