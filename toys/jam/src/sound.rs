//! The ring heard from its centre (where the experiment's camera stood): 22 small engines, each a
//! buzz whose pitch follows its car's speed and whose voice opens when it accelerates, and the hiss of
//! its tyres. In even flow it is one steady hum; a jam drops a handful to idle and brings back the
//! rising notes of cars pulling away. A tap adds a soft thump. Drawn, not recorded.

use crate::traffic::N;
use std::sync::{Arc, Mutex};
use toy_kit::Synth;

const TABLE: usize = 2048;

#[derive(Clone, Copy, Default)]
struct Car {
    v: f32,
    acc: f32,
    pan: f32,
}

#[derive(Default)]
struct Shared {
    cars: [Car; N],
    thumps: u32,
}

#[derive(Clone, Default)]
pub struct Engines(Arc<Mutex<Shared>>);

impl Engines {
    pub fn new() -> Engines {
        Engines::default()
    }
    /// Each car's speed (m/s), acceleration (m/s²) and pan (-1 left .. 1 right).
    pub fn set(&self, cars: &[(f32, f32, f32)]) {
        if let Ok(mut s) = self.0.try_lock() {
            for (c, &(v, acc, pan)) in s.cars.iter_mut().zip(cars) {
                *c = Car { v, acc, pan };
            }
        }
    }
    pub fn thump(&self) {
        if let Ok(mut s) = self.0.lock() {
            s.thumps += 1;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        // an engine's buzz: the firing note and its overtones, falling off
        let amps = [1.0, 0.75, 0.55, 0.50, 0.32, 0.26, 0.16, 0.12, 0.08, 0.05];
        let table: Vec<f32> = (0..TABLE)
            .map(|i| {
                let x = i as f32 / TABLE as f32 * std::f32::consts::TAU;
                amps.iter().enumerate().map(|(k, a)| a * ((k + 1) as f32 * x + k as f32 * 0.7).sin()).sum::<f32>() * 0.35
            })
            .collect();
        Box::new(Hum {
            shared: self.0.clone(),
            table,
            phase: [0.0; N],
            level: [0.0; N],
            freq: [30.0; N],
            pan: [0.0; N],
            detune: std::array::from_fn(|i| 1.0 + ((i as f32 * 2.399).sin()) * 0.06),
            noise: 0x1234_5678,
            hiss: [0.0; 2],
            thump: 0.0,
            thump_phase: 0.0,
            seen: 0,
            fade: 0.0,
        })
    }
}

struct Hum {
    shared: Arc<Mutex<Shared>>,
    table: Vec<f32>,
    phase: [f32; N],
    level: [f32; N],
    freq: [f32; N],
    pan: [f32; N],
    detune: [f32; N],
    noise: u32,
    hiss: [f32; 2],
    thump: f32,
    thump_phase: f32,
    seen: u32,
    fade: f32,
}

impl Synth for Hum {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let rate = rate as f32;
        let (cars, thumps) = match self.shared.try_lock() {
            Ok(s) => (Some(s.cars), s.thumps),
            Err(_) => (None, self.seen),
        };
        if thumps != self.seen {
            self.seen = thumps;
            self.thump = 1.0;
            self.thump_phase = 0.0;
        }
        let frames = out.len() / channels.max(1);
        // targets for this buffer
        let mut tf = self.freq;
        let mut tl = self.level;
        let mut tp = self.pan;
        let mut speed = 0.0;
        if let Some(cars) = cars {
            for i in 0..N {
                let c = cars[i];
                // the engine's note: idle low, rising with the speed (and a gear's worth more when
                // pulling hard); idling quiet, cruising steady, pulling away out of a stop loud
                let pull = (c.acc.max(0.0) / 1.5).min(1.0);
                tf[i] = (52.0 + 7.5 * c.v + 25.0 * pull) * self.detune[i];
                let launch = pull * (1.0 - (c.v / 9.0).min(1.0));
                tl[i] = 0.30 + 0.035 * c.v + 0.35 * pull + 2.2 * launch;
                tp[i] = c.pan;
                speed += c.v / N as f32;
            }
        }
        let n = frames.max(1) as f32;
        let (f0, l0, p0) = (self.freq, self.level, self.pan);
        let gain = 0.16 / (N as f32).sqrt();
        let hiss_gain = 0.025 * (speed / 8.0).min(1.5);
        for f in 0..frames {
            let u = f as f32 / n;
            self.fade = (self.fade + 1.0 / (rate * 1.5)).min(1.0);
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for i in 0..N {
                let fr = f0[i] + (tf[i] - f0[i]) * u;
                let lv = l0[i] + (tl[i] - l0[i]) * u;
                let pn = p0[i] + (tp[i] - p0[i]) * u;
                self.phase[i] = (self.phase[i] + fr / rate).fract();
                let s = self.table[(self.phase[i] * TABLE as f32) as usize % TABLE] * lv;
                l += s * (0.6 - 0.4 * pn);
                r += s * (0.6 + 0.4 * pn);
            }
            // tyres: noise, softened
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            let w = (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0;
            self.hiss[0] += (w - self.hiss[0]) * 0.08;
            self.hiss[1] += (self.hiss[0] - self.hiss[1]) * 0.08;
            let h = self.hiss[1] * hiss_gain * 6.0;
            // the thump: a low knock with a little body
            let mut th = 0.0;
            if self.thump > 1e-4 {
                self.thump_phase += (70.0 + 60.0 * self.thump) / rate;
                th = (self.thump_phase * std::f32::consts::TAU).sin() * self.thump * 0.22
                    + (self.thump_phase * 3.0 * std::f32::consts::TAU).sin() * self.thump * self.thump * 0.08;
                self.thump *= (-1.0 / (0.09 * rate)).exp();
            }
            let g = gain * self.fade;
            let (lo, ro) = ((l * g + h + th).tanh(), (r * g + h + th).tanh());
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
        self.freq = tf;
        self.level = tl;
        self.pan = tp;
    }
}
