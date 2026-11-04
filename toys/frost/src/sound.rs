//! The ice's sound: a fine crackle, each click a few milliseconds of bright noise with a glassy ring,
//! as many a second as the frost is freezing cells (drawn: frost on a pane is all but silent; this is
//! its growth made audible), panned to where it grows. Under it, the faint cold air outside.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
struct Want {
    /// Clicks a second.
    rate: f32,
    /// -1 left .. 1 right.
    pan: f32,
}

#[derive(Clone, Default)]
pub struct Crackle(Arc<Mutex<Want>>);

impl Crackle {
    pub fn set(&self, rate: f32, pan: f32) {
        if let Ok(mut w) = self.0.lock() {
            w.rate = rate;
            w.pan = pan;
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(CrackleSynth { want: self.0.clone(), rate: 0.0, pan: 0.0, rng: 0x2545_f491, clicks: Vec::new(), air: [0.0; 2], hp: [0.0; 2] })
    }
}

struct Click {
    t: f32,
    len: f32,
    amp: f32,
    pan: f32,
    /// the ring's frequency, Hz, and phase
    f: f32,
    ph: f32,
}

struct CrackleSynth {
    want: Arc<Mutex<Want>>,
    rate: f32,
    pan: f32,
    rng: u32,
    clicks: Vec<Click>,
    air: [f32; 2],
    hp: [f32; 2],
}

impl CrackleSynth {
    fn rand(&mut self) -> f32 {
        // xorshift
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }
}

impl Synth for CrackleSynth {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let (mut want_rate, mut want_pan) = (self.rate, self.pan);
        if let Ok(w) = self.want.try_lock() {
            want_rate = w.rate;
            want_pan = w.pan;
        }
        let dt = 1.0 / rate as f32;
        for frame in out.chunks_mut(channels) {
            self.rate += (want_rate - self.rate) * 0.0015;
            self.pan += (want_pan - self.pan) * 0.0005;
            // a click now? (Poisson at the freezing's rate)
            if self.rand() < self.rate * dt && self.clicks.len() < 48 {
                let a = self.rand();
                let amp = 0.04 + 0.22 * a * a * a;
                let len = 0.0015 + 0.004 * self.rand();
                let pan = (self.pan + (self.rand() - 0.5) * 0.9).clamp(-1.0, 1.0);
                let f = 2600.0 + 5200.0 * self.rand();
                self.clicks.push(Click { t: 0.0, len, amp, pan, f, ph: 0.0 });
            }
            let (mut l, mut r) = (0.0f32, 0.0f32);
            let mut i = 0;
            while i < self.clicks.len() {
                let n = self.rand() * 2.0 - 1.0;
                let c = &mut self.clicks[i];
                let env = (-c.t / c.len).exp();
                c.ph = (c.ph + c.f * dt).fract();
                let ring = (c.ph * std::f32::consts::TAU).sin() * (-c.t / (c.len * 3.0)).exp() * 0.5;
                let s = (n * env + ring) * c.amp;
                let p = (c.pan + 1.0) * std::f32::consts::FRAC_PI_4;
                l += s * p.cos();
                r += s * p.sin();
                c.t += dt;
                if c.t > c.len * 12.0 {
                    self.clicks.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            // the air outside: brown noise, very low
            for (k, side) in [0usize, 1].iter().enumerate() {
                let n = self.rand() * 2.0 - 1.0;
                self.air[*side] = (self.air[*side] + n * 0.02) * 0.995;
                let _ = k;
            }
            l += self.air[0] * 0.035;
            r += self.air[1] * 0.035;
            // a gentle high-pass on the clicks' noise floor (keeps the crackle fine, not thumpy)
            let (hl, hr) = (l - self.hp[0], r - self.hp[1]);
            self.hp[0] += (l - self.hp[0]) * 0.02;
            self.hp[1] += (r - self.hp[1]) * 0.02;
            let (ol, or) = (hl * 0.9 + l * 0.1, hr * 0.9 + r * 0.1);
            frame[0] = ol.clamp(-1.0, 1.0);
            if channels > 1 {
                frame[1] = or.clamp(-1.0, 1.0);
            }
        }
    }
}
