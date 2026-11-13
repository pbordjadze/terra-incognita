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

/// A stroke's cracks gathered to at most `most`, those that arrive together merged (their noise adds in
/// power), the inaudible dropped: the ear can't tell a thousand cracks from a hundred and forty, and
/// the audio thread can't afford them.
pub fn gather(mut pulses: Vec<Pulse>, most: usize) -> Vec<Pulse> {
    pulses.retain(|p| p.amp > 0.003);
    if pulses.len() <= most {
        return pulses;
    }
    pulses.sort_by(|a, b| a.delay.total_cmp(&b.delay));
    let per = pulses.len().div_ceil(most);
    pulses
        .chunks(per)
        .map(|g| {
            let n = g.len() as f32;
            Pulse {
                delay: g.iter().map(|p| p.delay).sum::<f64>() / n as f64,
                amp: g.iter().map(|p| p.amp * p.amp).sum::<f32>().sqrt(),
                tau: g.iter().map(|p| p.tau).sum::<f32>() / n,
                bright: g.iter().map(|p| p.bright).sum::<f32>() / n,
                pan: g.iter().map(|p| p.pan).sum::<f32>() / n,
            }
        })
        .collect()
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

/// A crack sounding: its filter's coefficient, its gains left and right, its envelope and how it falls
/// a sample, all worked out once when it arrives.
struct Live {
    a: f32,
    gain: [f32; 2],
    env: f32,
    fall: f32,
    rise: f32,
    lp: [f32; 2],
}

impl Live {
    fn new(p: &Pulse, rate: f64) -> Live {
        // a one-pole low-pass, twice, whose corner falls from ~4 kHz (bright) to ~120 Hz (dull)
        let fc = 120.0 * (35.0f32).powf(p.bright);
        let a = 1.0 - (-std::f32::consts::TAU * fc / rate as f32).exp();
        // the dull ones lose loudness to the filter; give it back so a far roll is heard
        let s = p.amp * (1.0 + 2.5 * (1.0 - p.bright));
        let pan = p.pan.clamp(-1.0, 1.0);
        Live {
            a,
            gain: [s * (1.0 - pan) * 0.5, s * (1.0 + pan) * 0.5],
            env: 0.0,
            fall: (-1.0 / (p.tau as f64 * rate)).exp() as f32,
            rise: (1.0 / (0.003 * rate)) as f32,
            lp: [0.0; 2],
        }
    }
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
            if !s.queue.is_empty() {
                for p in s.queue.drain(..) {
                    self.waiting.push((self.clock + p.delay, p));
                }
                // latest first, so the next to arrive is always at the end
                self.waiting.sort_by(|a, b| b.0.total_cmp(&a.0));
            }
            rain_target = s.rain;
        }
        let dt = 1.0 / rate;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            // pulses whose sound has arrived
            let now = self.clock;
            while self.waiting.last().is_some_and(|w| w.0 <= now) {
                if let Some((_, p)) = self.waiting.pop() {
                    if self.live.len() < 400 {
                        self.live.push(Live::new(&p, rate));
                    }
                }
            }
            let mut l = 0.0f32;
            let mut r = 0.0f32;
            let mut i = 0;
            while i < self.live.len() {
                let n = self.noise();
                let v = &mut self.live[i];
                // a 3 ms rise, then the fall (`rise` steps it up to 1, `fall` down from there)
                v.env = if v.rise > 0.0 {
                    let e = v.env + 1.0 / v.rise;
                    if e >= 1.0 {
                        v.rise = 0.0;
                    }
                    e.min(1.0)
                } else {
                    v.env * v.fall
                };
                v.lp[0] += v.a * (n - v.lp[0]);
                v.lp[1] += v.a * (v.lp[0] - v.lp[1]);
                let s = v.lp[1] * v.env;
                l += s * v.gain[0];
                r += s * v.gain[1];
                if v.rise == 0.0 && v.env < 0.001 {
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
