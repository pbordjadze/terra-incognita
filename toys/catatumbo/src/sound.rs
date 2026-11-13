//! Thunder, from the bolt itself: every piece of the channel cracks at once when the stroke returns,
//! and each piece's crack reaches you when sound (343 m/s) has crossed the distance from it. Near pieces
//! arrive first, sharp; the far ones later and duller (the air takes the highs), and together they are
//! the rumble, as long as the channel is deep. Under it, rain on the lake.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

/// The speed of sound in warm, wet air, metres a second.
pub const SOUND_MS: f64 = 343.0;

/// One crack: after `delay` seconds, a burst of noise of `amp`, dying in `tau` seconds, its highs
/// kept by `bright` (1 a crack, 0 a rumble), at `pan` (-1 left, 1 right).
#[derive(Clone, Copy, Debug)]
pub struct Pulse {
    pub delay: f64,
    pub amp: f32,
    pub tau: f32,
    pub bright: f32,
    pub pan: f32,
}

#[derive(Default)]
struct Shared {
    queue: Vec<Pulse>,
    rain: f32,
}

#[derive(Clone, Default)]
pub struct Thunder(Arc<Mutex<Shared>>);

impl Thunder {
    pub fn new() -> Thunder {
        Thunder::default()
    }
    pub fn send(&self, pulses: &[Pulse]) {
        if let Ok(mut s) = self.0.lock() {
            let room = 1200usize.saturating_sub(s.queue.len());
            s.queue.extend(pulses.iter().take(room));
        }
    }
    pub fn rain(&self, level: f32) {
        if let Ok(mut s) = self.0.lock() {
            s.rain = level;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Voice {
            shared: self.0.clone(),
            clock: 0.0,
            waiting: Vec::new(),
            live: Vec::new(),
            seed: 0x1234_5678,
            rain: 0.0,
            rain_lp: 0.0,
            rain_hp: 0.0,
            rumble: [0.0; 2],
            out_lp: [0.0; 2],
        })
    }
}

struct Live {
    p: Pulse,
    t: f64,
    lp: [f32; 2],
}

struct Voice {
    shared: Arc<Mutex<Shared>>,
    clock: f64,
    waiting: Vec<(f64, Pulse)>,
    live: Vec<Live>,
    seed: u32,
    rain: f32,
    rain_lp: f32,
    rain_hp: f32,
    rumble: [f32; 2],
    out_lp: [f32; 2],
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
        let mut rain_target = self.rain;
        if let Ok(mut s) = self.shared.lock() {
            for p in s.queue.drain(..) {
                self.waiting.push((self.clock + p.delay, p));
            }
            rain_target = s.rain;
        }
        let dt = 1.0 / rate;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            // pulses whose sound has arrived
            let now = self.clock;
            let mut k = 0;
            while k < self.waiting.len() {
                if self.waiting[k].0 <= now {
                    let (_, p) = self.waiting.swap_remove(k);
                    if self.live.len() < 600 {
                        self.live.push(Live {
                            p,
                            t: 0.0,
                            lp: [0.0; 2],
                        });
                    }
                } else {
                    k += 1;
                }
            }
            let mut l = 0.0f32;
            let mut r = 0.0f32;
            let mut i = 0;
            while i < self.live.len() {
                let n = self.noise();
                let v = &mut self.live[i];
                let t = v.t as f32;
                let env = (t / 0.003).min(1.0) * (-t / v.p.tau).exp();
                // a one-pole low-pass, twice, whose corner falls from ~4 kHz (bright) to ~120 Hz (dull)
                let fc = 120.0 * (35.0f32).powf(v.p.bright);
                let a = 1.0 - (-std::f32::consts::TAU * fc / rate as f32).exp();
                v.lp[0] += a * (n - v.lp[0]);
                v.lp[1] += a * (v.lp[0] - v.lp[1]);
                // the dull ones lose loudness to the filter; give it back so a far roll is heard
                let s = v.lp[1] * env * v.p.amp * (1.0 + 2.5 * (1.0 - v.p.bright));
                let pan = v.p.pan.clamp(-1.0, 1.0);
                l += s * (1.0 - pan) * 0.5;
                r += s * (1.0 + pan) * 0.5;
                v.t += dt;
                if v.t as f32 > v.p.tau * 7.0 {
                    self.live.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            // rain on the lake: high, soft noise
            self.rain += (rain_target - self.rain) * 0.0005;
            let n = self.noise();
            self.rain_lp += 0.08 * (n - self.rain_lp);
            self.rain_hp += 0.5 * ((n - self.rain_lp) - self.rain_hp);
            let hiss = self.rain_hp * self.rain * 0.06;
            // and the storm's own far roll, never quite gone
            let n2 = self.noise();
            self.rumble[0] += 0.0008 * (n2 - self.rumble[0]);
            self.rumble[1] += 0.0008 * (self.rumble[0] - self.rumble[1]);
            let roll = self.rumble[1] * 1.6 * self.rain;
            l += hiss + roll;
            r += hiss * 0.9 + roll;
            // a last gentle low-pass and a soft limit
            self.out_lp[0] += 0.6 * (l - self.out_lp[0]);
            self.out_lp[1] += 0.6 * (r - self.out_lp[1]);
            let (ol, or) = (soft(self.out_lp[0]), soft(self.out_lp[1]));
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
