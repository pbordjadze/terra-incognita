//! Nazaré's sound: the surf. Each wave that breaks at Praia do Norte comes as a crash, a low thump
//! and a long wash of white noise, louder the taller the wave broke (the sets loudest); under it the
//! coast's own steady roar.

use std::sync::{Arc, Mutex};

/// The crashes asked for (each a loudness, 0..1), and the roar's level.
#[derive(Default)]
pub struct Asked {
    pub crashes: Vec<f64>,
    pub roar: f64,
}

pub type Queue = Arc<Mutex<Asked>>;

struct Crash {
    amp: f64,
    t: f64,
    lp: [f64; 2],
    thump: f64,
}

pub struct Surf {
    queue: Queue,
    crashes: Vec<Crash>,
    roar: f64,
    roar_lp: [f64; 2],
    seed: u32,
}

impl Surf {
    pub fn new(queue: Queue) -> Surf {
        Surf {
            queue,
            crashes: Vec::new(),
            roar: 0.0,
            roar_lp: [0.0; 2],
            seed: 0x51ab,
        }
    }

    fn noise(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f64 / (1u32 << 24) as f64 * 2.0 - 1.0
    }
}

impl toy_kit::Synth for Surf {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let mut want_roar = self.roar;
        if let Ok(mut q) = self.queue.try_lock() {
            for amp in q.crashes.drain(..) {
                if self.crashes.len() >= 6 {
                    self.crashes.remove(0);
                }
                self.crashes.push(Crash {
                    amp,
                    t: 0.0,
                    lp: [0.0; 2],
                    thump: 0.0,
                });
            }
            want_roar = q.roar;
        }
        let frames = out.len() / channels.max(1);
        let dt = 1.0 / rate;
        // the filters' strength from their corner frequencies, whatever the rate
        let k = |hz: f64| 1.0 - (-std::f64::consts::TAU * hz * dt).exp();
        let (k_roar, k_wash, k_low) = (k(380.0), k(900.0), k(160.0));
        for f in 0..frames {
            self.roar += (want_roar - self.roar) * 2e-5;
            let n = self.noise();
            self.roar_lp[0] += (n - self.roar_lp[0]) * k_roar;
            self.roar_lp[1] += (self.roar_lp[0] - self.roar_lp[1]) * k_roar;
            let mut s = self.roar_lp[1] * self.roar * 0.9;
            for c in self.crashes.iter_mut() {
                let attack = (c.t / 0.25).min(1.0);
                let env = attack * attack * (-c.t / 2.4).exp() * c.amp;
                // the wash, a little brighter at first, then lower as it runs up the beach
                let bright = k_wash * (0.3 + 0.7 * (-c.t / 0.9).exp()) + k_low * 0.2;
                c.lp[0] += (n - c.lp[0]) * bright;
                c.lp[1] += (c.lp[0] - c.lp[1]) * bright;
                // the thump: the lip landing, a falling low tone
                c.thump += (48.0 + 30.0 * (-c.t / 0.08).exp()) * dt;
                let thump = (std::f64::consts::TAU * c.thump).sin() * (-c.t / 0.35).exp();
                s += 2.2 * c.lp[1] * env + 0.5 * thump * c.amp * attack;
                c.t += dt;
            }
            let s = (s * 1.5).tanh() as f32;
            for ch in 0..channels {
                out[f * channels + ch] += s;
            }
        }
        self.crashes.retain(|c| c.t < 12.0);
    }
}
