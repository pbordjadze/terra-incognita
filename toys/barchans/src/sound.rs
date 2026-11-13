//! The desert's sound: the wind, the hiss of sand in saltation as much as is moving, and the dunes'
//! song. A big dune's slip face avalanching hums: the dunes near Tarfaya sing at about 100 Hz
//! (Andreotti 2004), a low drone with its overtones, loud as the avalanche is large.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
struct Levels {
    wind: f32,
    hiss: f32,
    song: f32,
    pitch: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Levels>>);

impl Sound {
    /// The wind (0..1), the sand moving (0..1) and the song (0..1) at `pitch` Hz.
    pub fn set(&self, wind: f32, hiss: f32, song: f32, pitch: f32) {
        if let Ok(mut l) = self.0.lock() {
            *l = Levels {
                wind,
                hiss,
                song,
                pitch,
            };
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Voice {
            shared: self.0.clone(),
            now: Levels {
                pitch: 100.0,
                ..Default::default()
            },
            seed: 0x51DE_5A2D,
            lp: [0.0; 4],
            phase: 0.0,
            wob: 0.0,
            t: 0.0,
        })
    }
}

struct Voice {
    shared: Arc<Mutex<Levels>>,
    now: Levels,
    seed: u32,
    lp: [f32; 4],
    phase: f64,
    wob: f32,
    t: f64,
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
        let target = self.shared.lock().map(|l| *l).unwrap_or_default();
        let frames = out.len() / channels.max(1);
        let dt = 1.0 / rate;
        for f in 0..frames {
            let k = 0.0004;
            self.now.wind += (target.wind - self.now.wind) * k;
            self.now.hiss += (target.hiss - self.now.hiss) * k * 2.0;
            self.now.song += (target.song - self.now.song) * k * 0.6;
            self.now.pitch += (target.pitch - self.now.pitch) * k * 0.2;
            let n = self.noise();
            // the wind: low noise, gusting
            let gust = 0.7 + 0.3 * ((self.t * 0.31).sin() * (self.t * 0.13).sin()) as f32;
            self.lp[0] += 0.006 * (n - self.lp[0]);
            self.lp[1] += 0.006 * (self.lp[0] - self.lp[1]);
            let wind = self.lp[1] * 9.0 * self.now.wind * gust;
            // the hiss of the grains: high noise
            self.lp[2] += 0.5 * (n - self.lp[2]);
            self.lp[3] += 0.05 * (self.lp[2] - self.lp[3]);
            let hiss = (self.lp[2] - self.lp[3]) * 0.12 * self.now.hiss * gust;
            // the song: a drone at the pitch, with a slow waver and its overtones
            self.wob += 0.00002 * (self.noise() - self.wob * 0.01);
            self.phase += (self.now.pitch as f64 * (1.0 + self.wob as f64 * 0.5)) * dt;
            let p = (self.phase * std::f64::consts::TAU) as f32;
            let drone = (p.sin() + 0.45 * (2.0 * p).sin() + 0.2 * (3.0 * p).sin() + 0.1 * (5.0 * p).sin())
                * (0.8 + 0.2 * self.lp[0] * 20.0);
            let song = drone * 0.22 * self.now.song;
            let l = soft(wind + hiss + song);
            let r = soft(wind * 0.95 + hiss * 1.05 + song);
            if channels >= 2 {
                out[f * channels] = l;
                out[f * channels + 1] = r;
                for c in 2..channels {
                    out[f * channels + c] = 0.0;
                }
            } else if channels == 1 {
                out[f] = (l + r) * 0.5;
            }
            self.t += dt;
        }
    }
}

fn soft(x: f32) -> f32 {
    x / (1.0 + x.abs())
}
