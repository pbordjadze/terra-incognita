//! Swell's sound: a soft low plunk where a finger drops into the sea, and a glass chime wherever a
//! wave reaches a coast (its note by the coast's latitude on a pentatonic scale, its loudness by the
//! wave's height there, panned to where the coast is on screen, hushed behind the globe).

use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub enum Note {
    Plunk { amp: f64, pan: f64, freq: f64 },
    Chime { freq: f64, amp: f64, pan: f64 },
}

pub type Queue = Arc<Mutex<Vec<Note>>>;

struct Voice {
    freq: f64,
    phase: [f64; 3],
    amp: f64,
    t: f64,
    decay: f64,
    pan: f64,
    plunk: bool,
    lp: f64,
    seed: u32,
}

pub struct Chimes {
    queue: Queue,
    voices: Vec<Voice>,
}

impl Chimes {
    pub fn new(queue: Queue) -> Chimes {
        Chimes {
            queue,
            voices: Vec::new(),
        }
    }
}

const PARTIALS: [(f64, f64); 3] = [(1.0, 1.0), (2.756, 0.28), (5.40, 0.09)];

impl toy_kit::Synth for Chimes {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(mut q) = self.queue.try_lock() {
            for n in q.drain(..) {
                if self.voices.len() >= 28 {
                    // the oldest gives way
                    let k = self
                        .voices
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.t.total_cmp(&b.1.t))
                        .map(|(k, _)| k)
                        .unwrap_or(0);
                    self.voices.remove(k);
                }
                self.voices.push(match n {
                    Note::Plunk { amp, pan, freq } => Voice {
                        freq,
                        phase: [0.0; 3],
                        amp,
                        t: 0.0,
                        decay: 0.22,
                        pan,
                        plunk: true,
                        lp: 0.0,
                        seed: 7,
                    },
                    Note::Chime { freq, amp, pan } => Voice {
                        freq,
                        phase: [0.0; 3],
                        amp,
                        t: 0.0,
                        decay: 1.5,
                        pan,
                        plunk: false,
                        lp: 0.0,
                        seed: (freq * 13.0) as u32 | 1,
                    },
                });
            }
        }
        let frames = out.len() / channels.max(1);
        let dt = 1.0 / rate;
        let tau = 2.0 * std::f64::consts::PI;
        for v in &mut self.voices {
            let (gl, gr) = ((1.0 - v.pan) * 0.5, (1.0 + v.pan) * 0.5);
            for f in 0..frames {
                let attack = (v.t / 0.004).min(1.0);
                let env = attack * (-v.t / v.decay).exp() * v.amp;
                let mut s = 0.0;
                if v.plunk {
                    // a drop: a falling tone and its octave
                    let fr = v.freq * (1.0 + 1.2 * (-v.t / 0.03).exp());
                    v.phase[0] = (v.phase[0] + fr * dt) % 1.0;
                    v.phase[1] = (v.phase[1] + 2.0 * fr * dt) % 1.0;
                    s = (tau * v.phase[0]).sin() + 0.25 * (tau * v.phase[1]).sin();
                } else {
                    for (p, &(ratio, a)) in PARTIALS.iter().enumerate() {
                        v.phase[p] = (v.phase[p] + v.freq * ratio * dt) % 1.0;
                        // the upper partials die sooner, as glass's do
                        s += a * (tau * v.phase[p]).sin() * (-v.t * ratio * 0.6).exp();
                    }
                    // the wash: soft noise swelling in over 0.15 s and out over 0.8
                    v.seed = v.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let w = (v.seed >> 8) as f64 / (1u32 << 24) as f64 * 2.0 - 1.0;
                    v.lp += (w - v.lp) * 0.06;
                    let swell = (v.t / 0.15).min(1.0) * (-v.t / 0.8).exp();
                    s += 0.9 * v.lp * swell;
                }
                let s = s * env * 0.2;
                if channels >= 2 {
                    out[f * channels] += (s * gl * 2.0) as f32;
                    out[f * channels + 1] += (s * gr * 2.0) as f32;
                } else {
                    out[f * channels] += s as f32;
                }
                v.t += dt;
            }
        }
        self.voices.retain(|v| v.t < v.decay * 7.0);
        for v in out.iter_mut() {
            *v = v.tanh();
        }
    }
}
