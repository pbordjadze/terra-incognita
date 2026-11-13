//! The crowd's sound: a stadium's murmur under everything, and a roar that rises where the wave is,
//! as loud as the number standing, heard from where it is round the bowl.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
pub struct Mix {
    /// the share of the crowd standing (0..1)
    pub standing: f32,
    /// where the standing are, left to right (-1..1), and how near the camera (0 far .. 1 near)
    pub pan: f32,
    pub near: f32,
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
        Box::new(Roar {
            mix: self.0.clone(),
            now: Mix::default(),
            level: 0.0,
            seed: 0xA27E_C401,
            bands: [[0.0; 4]; 2],
            t: 0.0,
        })
    }
}

struct Roar {
    mix: Arc<Mutex<Mix>>,
    now: Mix,
    level: f32,
    seed: u32,
    /// two channels of filter state: a low murmur's and the roar's band
    bands: [[f32; 4]; 2],
    t: f32,
}

impl Roar {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
    }
}

impl Synth for Roar {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(m) = self.mix.try_lock() {
            self.now = *m;
        }
        let dt = 1.0 / rate as f32;
        let frames = out.len() / channels.max(1);
        // the roar follows the standing share, up fast, down slower
        let want = (self.now.standing * 60.0).sqrt().min(1.0) * (0.55 + 0.45 * self.now.near);
        let p = (self.now.pan * 0.5 + 0.5).clamp(0.0, 1.0);
        let (gl, gr) = ((1.0 - p).sqrt() * 1.2, p.sqrt() * 1.2);
        for f in 0..frames {
            self.t += dt;
            let k = if want > self.level { 3.0 } else { 0.8 };
            self.level += (want - self.level) * (dt * k).min(1.0);
            let mut o = [0.0f32; 2];
            for (ch, b) in [0usize, 1].iter().enumerate() {
                let n = self.rand();
                let s = &mut self.bands[*b];
                // the murmur: deep, soft, always
                s[0] += (n - s[0]) * 0.02;
                s[1] += (s[0] - s[1]) * 0.02;
                // the roar: a voiced band, about 300 Hz to 2 kHz
                s[2] += (n - s[2]) * 0.22;
                s[3] += (s[2] - s[3]) * 0.22;
                let band = s[2] - s[3] * 0.6;
                let murmur = s[1] * 3.0 * (0.5 + 0.2 * (self.t * 0.3 + ch as f32).sin());
                let g = if ch == 0 { gl } else { gr };
                o[ch] = murmur * 0.25 + band * self.level * 0.55 * g;
            }
            for ch in 0..channels {
                out[f * channels + ch] = o[ch % 2].tanh();
            }
        }
    }
}
