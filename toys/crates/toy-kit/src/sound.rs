//! A small sound: struck notes (a sine with a touch of FM, a soft attack, an exponential ring) and an
//! optional low bed, mixed on the audio thread. A take keeps the `Bells` handle and calls `strike`;
//! `synth()` gives the shell its `Synth`.
//!
//! Lifted from pbordjadze/busquets (`crates/kit/src/sound.rs`, Nov 3, 2026), where it was written on
//! toy-kit 6bf0613; same author, MIT. busquets' takes call them takes; here a take is any `Toy`.

use crate::Synth;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub freq: f32,
    pub amp: f32,
    /// Seconds for the ring to fall by 60 dB.
    pub decay: f32,
    /// -1 left .. 1 right.
    pub pan: f32,
    /// FM index: 0 a pure sine, 1 a bell, 3 a bright pluck.
    pub bright: f32,
}

#[derive(Default)]
struct Shared {
    queue: Vec<Note>,
    bed: f32,
    bed_freq: f32,
}

/// The take's end: strike notes, set the bed.
#[derive(Clone, Default)]
pub struct Bells(Arc<Mutex<Shared>>);

impl Bells {
    pub fn new() -> Bells {
        Bells::default()
    }
    pub fn strike(&self, n: Note) {
        if let Ok(mut s) = self.0.lock() {
            if s.queue.len() < 32 {
                s.queue.push(n);
            }
        }
    }
    /// A low bed under everything: `level` 0..1 at `freq` Hz.
    pub fn bed(&self, level: f32, freq: f32) {
        if let Ok(mut s) = self.0.lock() {
            s.bed = level;
            s.bed_freq = freq;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(BellSynth {
            shared: self.0.clone(),
            voices: Vec::new(),
            bed: 0.0,
            bed_phase: [0.0; 3],
            bed_freq: 55.0,
        })
    }
}

struct Voice {
    n: Note,
    phase: f64,
    mphase: f64,
    t: f64,
}

struct BellSynth {
    shared: Arc<Mutex<Shared>>,
    voices: Vec<Voice>,
    bed: f32,
    bed_phase: [f64; 3],
    bed_freq: f32,
}

impl Synth for BellSynth {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let (mut target_bed, mut target_freq) = (self.bed, self.bed_freq);
        if let Ok(mut s) = self.shared.try_lock() {
            for n in s.queue.drain(..) {
                if self.voices.len() >= 24 {
                    self.voices.remove(0);
                }
                self.voices.push(Voice {
                    n,
                    phase: 0.0,
                    mphase: 0.0,
                    t: 0.0,
                });
            }
            target_bed = s.bed;
            target_freq = if s.bed_freq > 0.0 {
                s.bed_freq
            } else {
                self.bed_freq
            };
        }
        let dt = 1.0 / rate;
        let tau = std::f64::consts::TAU;
        for frame in out.chunks_mut(channels) {
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in &mut self.voices {
                let env_a = (v.t / 0.004).min(1.0);
                let env = env_a * (-6.9 * v.t / v.n.decay as f64).exp();
                let m = (v.mphase * tau).sin()
                    * v.n.bright as f64
                    * (-3.0 * v.t / v.n.decay as f64).exp();
                let s = ((v.phase * tau) + m).sin() * env * v.n.amp as f64;
                let p = (v.n.pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
                l += s as f32 * p.cos();
                r += s as f32 * p.sin();
                v.phase = (v.phase + v.n.freq as f64 * dt).fract();
                v.mphase = (v.mphase + v.n.freq as f64 * 2.0 * dt).fract();
                v.t += dt;
            }
            self.bed += (target_bed - self.bed) * 0.0002;
            self.bed_freq += (target_freq - self.bed_freq) * 0.0002;
            if self.bed > 1e-4 {
                let mut b = 0.0;
                for (i, k) in [1.0, 1.5, 2.003].iter().enumerate() {
                    self.bed_phase[i] = (self.bed_phase[i] + self.bed_freq as f64 * k * dt).fract();
                    b += (self.bed_phase[i] * tau).sin() / (i as f64 + 1.0);
                }
                let b = (b * 0.05 * self.bed as f64) as f32;
                l += b;
                r += b;
            }
            let (l, r) = (soft(l), soft(r));
            frame[0] = l;
            if channels > 1 {
                frame[1] = r;
            }
        }
        self.voices.retain(|v| v.t < v.n.decay as f64 * 1.2);
    }
}

fn soft(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// A pentatonic degree (0, 1, 2…; negative below) above `root` Hz: the notes a pass can be.
pub fn pentatonic(root: f32, degree: i32) -> f32 {
    const STEPS: [i32; 5] = [0, 2, 4, 7, 9];
    let oct = degree.div_euclid(5);
    let semis = STEPS[degree.rem_euclid(5) as usize] + 12 * oct;
    root * 2f32.powf(semis as f32 / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pentatonic_degrees() {
        assert_eq!(pentatonic(220.0, 0), 220.0);
        assert!((pentatonic(220.0, 5) - 440.0).abs() < 1e-3);
        assert!((pentatonic(220.0, -5) - 110.0).abs() < 1e-3);
        assert!((pentatonic(220.0, 3) - 220.0 * 2f32.powf(7.0 / 12.0)).abs() < 1e-3);
    }

    #[test]
    fn a_struck_note_sounds_and_rings_out() {
        let bells = Bells::new();
        let mut synth = bells.synth();
        let mut out = vec![0.0f32; 2 * 4800];
        synth.render(&mut out, 2, 48000.0);
        assert!(out.iter().all(|s| *s == 0.0), "silent before a strike");
        bells.strike(Note {
            freq: 440.0,
            amp: 0.5,
            decay: 0.2,
            pan: -1.0,
            bright: 1.0,
        });
        synth.render(&mut out, 2, 48000.0);
        let (l, r) = out.chunks(2).fold((0.0f32, 0.0f32), |(l, r), f| {
            (l.max(f[0].abs()), r.max(f[1].abs()))
        });
        assert!(l > 0.1 && l < 1.0, "left {l}");
        assert!(r < 1e-3, "panned hard left, right {r}");
        for _ in 0..10 {
            synth.render(&mut out, 2, 48000.0);
        }
        assert!(out.iter().all(|s| s.abs() < 1e-6), "rung out");
    }
}
