//! The sound: synchrony made audible. Fireflies are silent, and so is the field while it is out of
//! step; but where the lights on a patch of the screen flash together, that patch rings once, a
//! soft glass chime, each patch its own note of a pentatonic scale and its own place left to right.
//! A continent in step is a slow chord; a field falling into step is chimes beginning to sound; a
//! finger's touch rings a high note as its light leaves.
//!
//! The listening (`Ear`, on the main thread) watches the screen in a grid of patches: each patch's
//! glow is its fireflies' brightness, weighted by their light; when it rises through a threshold
//! (and had fallen below a lower one since), the patch chimes. The voices (`Chimes`, the audio
//! thread) are sine partials in a bell's ratios with an exponential decay.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

/// The grid of patches over the screen.
pub const COLS: usize = 3;
pub const ROWS: usize = 4;
/// A patch chimes when its glow rises above ON, and may again once it has fallen below OFF.
const ON: f32 = 0.5;
const OFF: f32 = 0.22;
/// A patch needs at least this much light (the sum of its fireflies' weights) to have a voice, so a
/// lone city's flash is not a patch in step.
const MIN_LIGHT: f32 = 3.0;
/// The scale (A minor pentatonic, two octaves), high notes at the top of the screen.
const NOTES: [f32; 10] = [220.0, 261.63, 293.66, 329.63, 392.0, 440.0, 523.25, 587.33, 659.25, 783.99];

#[derive(Clone, Copy)]
pub struct Strike {
    pub freq: f32,
    pub amp: f32,
    /// -1 left .. 1 right
    pub pan: f32,
}

pub type Queue = Arc<Mutex<Vec<Strike>>>;

/// The listening, on the main thread.
pub struct Ear {
    armed: [bool; COLS * ROWS],
    pub queue: Queue,
    t: f64,
}

impl Ear {
    pub fn new() -> Ear {
        Ear { armed: [true; COLS * ROWS], queue: Arc::new(Mutex::new(Vec::new())), t: 0.0 }
    }

    /// One frame: `cells` is each patch's (sum of weight x glow, sum of weight).
    pub fn hear(&mut self, cells: &[(f32, f32); COLS * ROWS], quiet: bool, t: f64) {
        self.t = t;
        let mut out = Vec::new();
        for (k, &(lw, w)) in cells.iter().enumerate() {
            if w < MIN_LIGHT {
                self.armed[k] = true;
                continue;
            }
            let g = lw / w;
            if self.armed[k] && g > ON {
                self.armed[k] = false;
                let (c, r) = (k % COLS, k / COLS);
                let note = NOTES[(ROWS - 1 - r) * 2 + c];
                let amp = (0.035 + 0.04 * (w / 15.0).min(1.0)) * if quiet { 0.6 } else { 1.0 };
                if std::env::var_os("FIREFLIES_SOUND").is_some() {
                    eprintln!("strike patch {k} glow {g:.2} light {w:.1} at {:.2}", self.t);
                }
                out.push(Strike { freq: note, amp, pan: (c as f32 - (COLS as f32 - 1.0) / 2.0) * 0.6 });
            } else if g < OFF {
                self.armed[k] = true;
            }
        }
        if !out.is_empty() {
            if let Ok(mut q) = self.queue.lock() {
                q.extend(out);
            }
        }
    }

    /// A finger's touch: a high note where it touched (x 0..1 across the screen).
    pub fn touch(&self, x: f32) {
        if let Ok(mut q) = self.queue.lock() {
            q.push(Strike { freq: 1046.5, amp: 0.05, pan: (x * 2.0 - 1.0) * 0.6 });
        }
    }
}

struct Voice {
    s: Strike,
    t: f64,
}

/// The voices, on the audio thread.
pub struct Chimes {
    queue: Queue,
    voices: Vec<Voice>,
}

impl Chimes {
    pub fn new(queue: Queue) -> Chimes {
        Chimes { queue, voices: Vec::new() }
    }
}

/// A glass bell's partials (ratio, level, decay seconds).
const PARTIALS: [(f64, f64, f64); 3] = [(1.0, 1.0, 1.2), (2.76, 0.3, 0.5), (5.40, 0.1, 0.22)];

impl Synth for Chimes {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(mut q) = self.queue.try_lock() {
            for s in q.drain(..) {
                if self.voices.len() >= 24 {
                    self.voices.remove(0);
                }
                self.voices.push(Voice { s, t: 0.0 });
            }
        }
        let dt = 1.0 / rate;
        let frames = out.len() / channels.max(1);
        for v in &mut self.voices {
            let (l, r) = ((1.0 - v.s.pan) * 0.5, (1.0 + v.s.pan) * 0.5);
            for i in 0..frames {
                let t = v.t + i as f64 * dt;
                // a soft attack (8 ms), so it is struck with felt, not a hammer
                let attack = (t / 0.008).min(1.0);
                let mut x = 0.0;
                for &(ratio, level, decay) in &PARTIALS {
                    x += level * (std::f64::consts::TAU * v.s.freq as f64 * ratio * t).sin() * (-t / decay).exp();
                }
                let y = (x * attack * v.s.amp as f64) as f32;
                if channels >= 2 {
                    out[i * channels] += y * l;
                    out[i * channels + 1] += y * r;
                } else {
                    out[i * channels] += y;
                }
            }
            v.t += frames as f64 * dt;
        }
        self.voices.retain(|v| v.t < 4.0);
        for s in out.iter_mut() {
            *s = s.clamp(-0.9, 0.9);
        }
    }
}
