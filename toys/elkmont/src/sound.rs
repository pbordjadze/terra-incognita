//! The night's sound at Elkmont: the Little River running over its stones below the trail, low and
//! steady, with the small rise and fall of moving water, and now and then a gray treefrog's trill far
//! off in the woods. The fireflies are silent. Drawn from noise, not recorded.

use toy_kit::Synth;

pub struct Night {
    seed: u32,
    /// The river's two filters (a low roar and a brighter rush), their states.
    low: [f32; 2],
    rush: [f32; 2],
    /// The slow swell of the water.
    swell: f32,
    swell_target: f32,
    t: f64,
    /// The next frog's trill: when it starts (s), how long, its pitch (Hz), its loudness, its side.
    frog_at: f64,
    frog: Option<(f64, f64, f32, f32, f32)>,
    gain: f32,
}

impl Night {
    pub fn new() -> Night {
        Night {
            seed: 0x1234_5679,
            low: [0.0; 2],
            rush: [0.0; 2],
            swell: 0.6,
            swell_target: 0.6,
            t: 0.0,
            frog_at: 9.0,
            frog: None,
            gain: 0.0,
        }
    }

    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn rnd(&mut self) -> f32 {
        self.noise() * 0.5 + 0.5
    }
}

impl Default for Night {
    fn default() -> Self {
        Night::new()
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

impl Synth for Night {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let dt = 1.0 / rate;
        let f_low = (2.0 * (std::f64::consts::PI * 220.0 / rate).sin()) as f32;
        let f_rush = (2.0 * (std::f64::consts::PI * 1500.0 / rate).sin()) as f32;
        for frame in out.chunks_mut(channels) {
            self.t += dt;
            // fade in over the first two seconds
            self.gain = (self.gain + dt as f32 * 0.5).min(1.0);
            if self.rnd() < 0.00004 {
                self.swell_target = 0.45 + 0.4 * self.rnd();
            }
            self.swell += (self.swell_target - self.swell) * 0.00002;
            let n = self.noise();
            let low = svf(&mut self.low, n, f_low, 0.9);
            let rush = svf(&mut self.rush, n, f_rush, 1.2);
            let river = (low * 0.55 + rush * 0.12) * self.swell * 0.5;
            // a frog: a trill of short pulses, about 40 a second, far off
            if self.frog.is_none() && self.t >= self.frog_at {
                let pitch = 2000.0 + 500.0 * self.rnd();
                let side = self.rnd() * 2.0 - 1.0;
                self.frog = Some((
                    self.t,
                    0.5 + 0.4 * self.rnd() as f64,
                    pitch,
                    0.03 + 0.03 * self.rnd(),
                    side,
                ));
                self.frog_at = self.t + 6.0 + 18.0 * self.rnd() as f64;
            }
            let mut frog = 0.0;
            let mut side = 0.0;
            if let Some((t0, len, pitch, loud, s)) = self.frog {
                let u = self.t - t0;
                if u > len {
                    self.frog = None;
                } else {
                    let env = ((u / 0.08).min(1.0) * ((len - u) / 0.1).min(1.0)) as f32;
                    let pulse = (((u * 41.0).fract() as f32) * std::f32::consts::PI)
                        .sin()
                        .powi(3);
                    frog = (std::f64::consts::TAU * pitch as f64 * u).sin() as f32
                        * pulse
                        * env
                        * loud;
                    side = s;
                }
            }
            let l = (river + frog * (1.0 - side) * 0.7) * self.gain;
            let r = (river + frog * (1.0 + side) * 0.7) * self.gain;
            if channels >= 2 {
                frame[0] = l;
                frame[1] = r;
                for c in frame.iter_mut().skip(2) {
                    *c = 0.0;
                }
            } else {
                frame[0] = (l + r) * 0.5;
            }
        }
    }
}
