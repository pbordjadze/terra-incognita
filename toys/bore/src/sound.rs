//! The bore's sound: heard long before it's seen at Yanguan, a low roar like a train or thunder rolling
//! closer, a hiss where it breaks, a crack where it strikes the wall, and over it the crowd's murmur,
//! rising as it comes. All noise, filtered.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
struct Levels {
    roar: f32,
    hiss: f32,
    crowd: f32,
    slap: f32,
    pan: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Levels>>);

impl Sound {
    /// The roar (0..1), the breaking hiss (0..1), the crowd (0..1), the wall struck (0..1), and where
    /// along left and right the bore is (-1..1).
    pub fn set(&self, roar: f32, hiss: f32, crowd: f32, slap: f32, pan: f32) {
        if let Ok(mut l) = self.0.lock() {
            *l = Levels {
                roar,
                hiss,
                crowd,
                slap,
                pan,
            };
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Voice {
            shared: self.0.clone(),
            now: Levels::default(),
            seed: 0x7_1DE5,
            lp: [0.0; 8],
            t: 0.0,
        })
    }
}

struct Voice {
    shared: Arc<Mutex<Levels>>,
    now: Levels,
    seed: u32,
    lp: [f32; 8],
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
        for f in 0..frames {
            let k = 0.0003;
            self.now.roar += (target.roar - self.now.roar) * k;
            self.now.hiss += (target.hiss - self.now.hiss) * k;
            self.now.crowd += (target.crowd - self.now.crowd) * k * 0.5;
            self.now.slap += (target.slap - self.now.slap) * k * 6.0;
            self.now.pan += (target.pan - self.now.pan) * k;
            let n = self.noise();
            // the roar: deep noise, two poles low, swelling and ebbing
            self.lp[0] += 0.004 * (n - self.lp[0]);
            self.lp[1] += 0.004 * (self.lp[0] - self.lp[1]);
            let surge = 0.75 + 0.25 * ((self.t * 0.9).sin() * (self.t * 0.37).sin()) as f32;
            let roar = self.lp[1] * 22.0 * self.now.roar * surge;
            // the hiss of breaking water: a band of noise
            self.lp[2] += 0.25 * (n - self.lp[2]);
            self.lp[3] += 0.03 * (self.lp[2] - self.lp[3]);
            let hiss = (self.lp[2] - self.lp[3]) * 0.35 * self.now.hiss;
            // the wall struck: a thump and a crash
            self.lp[4] += 0.02 * (n - self.lp[4]);
            let slap = (self.lp[4] * 4.0 + n * 0.15) * self.now.slap;
            // the crowd: voices are a mid band of noise, gently uneven
            self.lp[5] += 0.08 * (n - self.lp[5]);
            self.lp[6] += 0.02 * (self.lp[5] - self.lp[6]);
            let ripple = 0.6 + 0.4 * ((self.t * 2.3).sin() * (self.t * 1.7 + 1.0).sin()) as f32;
            let crowd = (self.lp[5] - self.lp[6]) * 0.18 * self.now.crowd * ripple;
            let (l, r) = (0.5 - 0.35 * self.now.pan, 0.5 + 0.35 * self.now.pan);
            let left = soft((roar + hiss + slap) * l * 2.0 + crowd * 1.1);
            let right = soft((roar + hiss + slap) * r * 2.0 + crowd * 0.9);
            if channels >= 2 {
                out[f * channels] = left;
                out[f * channels + 1] = right;
                for c in 2..channels {
                    out[f * channels + c] = 0.0;
                }
            } else if channels == 1 {
                out[f] = 0.5 * (left + right);
            }
            self.t += 1.0 / rate;
        }
    }
}

fn soft(x: f32) -> f32 {
    x / (1.0 + x.abs())
}
