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
/// A patch chimes when the fireflies bright in it right now outnumber its own usual (their running
/// mean over a few seconds) by ON_TIMES times plus ON_MORE, at least MIN_BRIGHT of them; it may again
/// once they fall under OFF_TIMES times the usual. The usual is the same in step or not (a firefly is
/// bright the same share of its cycle either way): only the peaks differ, so this hears a region in
/// step at any zoom, whatever else shares its patch.
const ON_TIMES: f32 = 3.0;
const ON_MORE: f32 = 3.0;
/// And never two chimes closer than this (seconds), whichever patches: a few at a time, not a shower.
const GAP_S: f64 = 0.6;
const OFF_TIMES: f32 = 1.3;
const MIN_BRIGHT: u32 = 5;
const USUAL_S: f64 = 4.0;
/// A patch rings at most RUN beats in a row, then rests REST beats.
const RUN: u32 = 3;
const REST: u32 = 4;
/// A patch needs at least this much light (the sum of its fireflies' weights) to have a voice, so a
/// lone city's flash is not a patch in step.
const MIN_LIGHT: f32 = 2.0;
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
    /// Each patch's strikes in a row, and the beats it still rests (so a lone patch in step is a
    /// few chimes and a breath, not a metronome), and where it is in its little tune.
    run: [u32; COLS * ROWS],
    rest: [u32; COLS * ROWS],
    step: [usize; COLS * ROWS],
    usual: [f32; COLS * ROWS],
    heard: bool,
    start: f64,
    last: f64,
    pub queue: Queue,
    t: f64,
}

impl Ear {
    pub fn new() -> Ear {
        Ear { armed: [true; COLS * ROWS], run: [0; COLS * ROWS], rest: [0; COLS * ROWS], step: [0; COLS * ROWS], usual: [0.0; COLS * ROWS], heard: false, start: 0.0, last: -1.0, queue: Arc::new(Mutex::new(Vec::new())), t: 0.0 }
    }

    /// One frame: `cells` is each patch's (sum of weight x glow, sum of weight, how many fireflies
    /// are bright now).
    pub fn hear(&mut self, cells: &[(f32, f32, u32); COLS * ROWS], quiet: bool, t: f64) {
        let dt = (t - self.t).clamp(0.0, 0.1);
        self.t = t;
        // (warming up, the usual is the plain mean of every frame so far; and nothing chimes until it
        // has heard a couple of seconds)
        if !self.heard {
            self.heard = true;
            self.start = t;
        }
        let age = t - self.start;
        let a = (1.0 - (-dt / USUAL_S).exp()).max(dt / (age + dt).max(1e-6)) as f32;
        let warm = age >= 2.0;
        let mut out = Vec::new();
        for (k, &(_, w, nb)) in cells.iter().enumerate() {
            let usual = self.usual[k];
            self.usual[k] += (nb as f32 - usual) * a;
            if w < MIN_LIGHT {
                self.armed[k] = true;
                continue;
            }
            let n = nb as f32;
            let g = n;
            if warm && self.armed[k] && nb >= MIN_BRIGHT && n > ON_TIMES * usual + ON_MORE && t - self.last >= GAP_S {
                self.last = t;
                self.armed[k] = false;
                if self.rest[k] > 0 {
                    self.rest[k] -= 1;
                    continue;
                }
                self.run[k] += 1;
                if self.run[k] >= RUN {
                    self.run[k] = 0;
                    self.rest[k] = REST;
                }
                let (c, r) = (k % COLS, k / COLS);
                // its note wanders among three neighbours of the scale: up, back, down, back
                const TUNE: [i32; 4] = [0, 1, 0, -1];
                self.step[k] = (self.step[k] + 1) % TUNE.len();
                let i = ((ROWS - 1 - r) * 2 + c) as i32 + TUNE[self.step[k]];
                let note = NOTES[i.clamp(0, NOTES.len() as i32 - 1) as usize];
                // the middle of the screen sings, the edges are quieter
                let edge = if c == COLS / 2 { 1.0 } else { 0.55 };
                let amp = edge * (0.07 + 0.08 * (w / 15.0).min(1.0)) * if quiet { 0.6 } else { 1.0 };
                if std::env::var_os("FIREFLIES_SOUND").is_some() {
                    eprintln!("strike patch {k} bright {g} usual {usual:.1} light {w:.1} at {:.2}", self.t);
                }
                out.push(Strike { freq: note, amp, pan: (c as f32 - (COLS as f32 - 1.0) / 2.0) * 0.45 });
            } else if n < OFF_TIMES * usual + 1.0 {
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
