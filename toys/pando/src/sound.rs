//! The sound of aspens in wind: a bright hiss, like rain beginning, that swells as a gust comes
//! through; nearer one stem, the hiss breaks into the patter of single leaves ticking against each
//! other; at one leaf, only its own soft flutter; under the lens, almost nothing. Drawn from noise,
//! not recorded.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
pub struct Mix {
    /// The wind where we are, m/s.
    pub wind: f32,
    /// How near: 0 the grove, 1 one stem, 2 one leaf, 3 its skin (fractional in a dive).
    pub near: f32,
    /// One leaf's twist rate (rad/s), for its own flutter.
    pub flutter: f32,
}

#[derive(Clone, Default)]
pub struct Sound(Arc<Mutex<Mix>>);

impl Sound {
    pub fn set(&self, m: Mix) {
        if let Ok(mut s) = self.0.lock() {
            *s = m;
        }
    }

    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(Rustle {
            mix: self.0.clone(),
            now: Mix::default(),
            seed: 0x2545_f491,
            hiss: [[0.0; 2]; 2],
            lo: [[0.0; 2]; 2],
            flap: [0.0; 2],
            level: 0.0,
            ticks: Vec::with_capacity(64),
            owed: 0.0,
        })
    }
}

struct Tick {
    phase: f32,
    freq: f32,
    amp: f32,
    pan: f32,
    age: f32,
}

struct Rustle {
    mix: Arc<Mutex<Mix>>,
    now: Mix,
    seed: u32,
    hiss: [[f32; 2]; 2],
    lo: [[f32; 2]; 2],
    flap: [f32; 2],
    level: f32,
    ticks: Vec<Tick>,
    owed: f32,
}

impl Rustle {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// A state-variable filter's band pass.
fn svf(s: &mut [f32; 2], x: f32, f: f32, q: f32) -> f32 {
    let low = s[1] + f * s[0];
    let high = x - low - q * s[0];
    let band = f * high + s[0];
    s[0] = band;
    s[1] = low;
    band
}

impl Synth for Rustle {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(m) = self.mix.try_lock() {
            self.now = *m;
        }
        let m = self.now;
        let dt = 1.0 / rate as f32;
        let fq = |hz: f32| (2.0 * (std::f32::consts::PI * hz / rate as f32).sin()).min(1.0);
        let (f_hiss, f_air, f_lo) = (fq(3400.0), fq(1300.0), fq(240.0));
        let w = m.wind.max(0.0);
        // how loud the leaves are: grows with the wind, fades as we go in past one leaf
        let close = (1.0 - (m.near - 1.5).max(0.0) / 1.5).clamp(0.05, 1.0);
        let target = (w / 7.0).min(1.0).powf(1.3) * 0.22 * close + 0.012;
        // ticks of leaf on leaf near one stem
        let tick_rate = if m.near > 0.5 && m.near < 2.2 {
            w * w * 6.0 * (1.0 - (m.near - 1.0).abs()).max(0.0)
        } else {
            0.0
        };
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            self.level += (target - self.level) * 0.0004;
            let n1 = self.rand() * 2.0 - 1.0;
            let n2 = self.rand() * 2.0 - 1.0;
            let mut s = [0.0f32; 2];
            for (c, n) in [n1, n2].into_iter().enumerate() {
                // the grove's hiss: brighter near a stem, broader far off
                let bright = svf(&mut self.hiss[c], n, f_hiss, 0.9);
                let air = svf(&mut self.lo[c], n * 0.5, f_air, 1.2);
                let mixk = (m.near / 1.2).clamp(0.0, 1.0);
                s[c] = self.level * (bright * (0.5 + 0.5 * mixk) + air * 0.6 * (1.0 - mixk));
            }
            // one leaf's own flutter: a soft low flap as it turns through edge-on
            if m.near > 1.4 {
                let flap = svf(&mut self.flap, n1, f_lo, 2.0) * (m.flutter.abs() / 30.0).min(1.0);
                let k = (1.0 - (m.near - 2.0).abs()).max(0.0) * 0.25;
                s[0] += flap * k;
                s[1] += flap * k;
            }
            self.owed += tick_rate * dt;
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                if self.ticks.len() < 60 {
                    let freq = 2500.0 + 3500.0 * self.rand();
                    let amp = 0.03 + 0.06 * self.rand();
                    let pan = self.rand();
                    self.ticks.push(Tick {
                        phase: 0.0,
                        freq,
                        amp,
                        pan,
                        age: 0.0,
                    });
                }
            }
            for t in &mut self.ticks {
                let env = (-t.age / 0.004).exp();
                let v = (t.phase * std::f32::consts::TAU).sin() * t.amp * env;
                s[0] += v * (1.0 - t.pan);
                s[1] += v * t.pan;
                t.phase += t.freq * dt;
                t.age += dt;
            }
            self.ticks.retain(|t| t.age < 0.03);
            for c in 0..channels {
                out[f * channels + c] = s[c.min(1)].clamp(-1.0, 1.0);
            }
        }
    }
}
