//! The Hoh's sound: the hush of a wet forest (a low soft noise, brighter as the years run faster) and
//! water dripping off moss; now and then a varied thrush, one long buzzy whistle on one pitch, the next
//! on another; and a giant coming down, the crack of it going, then the thud through the floor. Drawn
//! from noise and sines, not recorded.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

#[derive(Default, Clone, Copy)]
pub struct Mix {
    /// Years a second, over the resting pace (0 at rest).
    pub hurry: f32,
    /// Giants that have started down, and that have landed (running counts), and the last one's height (m).
    pub cracks: u32,
    pub thuds: u32,
    pub size: f32,
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
        Box::new(Forest {
            mix: self.0.clone(),
            now: Mix::default(),
            seed: 0x4f1b_c0de,
            hush: [[0.0; 2]; 2],
            low: [[0.0; 2]; 2],
            drips: Vec::with_capacity(16),
            drip_owed: 0.0,
            thrush: None,
            thrush_in: 3.0,
            crack: 0.0,
            crack_f: [[0.0; 2]; 2],
            thud: 0.0,
            thud_phase: 0.0,
            thud_f: [0.0; 2],
            heard: (0, 0),
            started: false,
            level: 0.0,
        })
    }
}

struct Drip {
    phase: f32,
    freq: f32,
    amp: f32,
    pan: f32,
    age: f32,
}

struct Thrush {
    phase: f32,
    freq: f32,
    age: f32,
    len: f32,
    pan: f32,
}

struct Forest {
    mix: Arc<Mutex<Mix>>,
    now: Mix,
    seed: u32,
    hush: [[f32; 2]; 2],
    low: [[f32; 2]; 2],
    drips: Vec<Drip>,
    drip_owed: f32,
    thrush: Option<Thrush>,
    thrush_in: f32,
    /// Seconds left of a crack, and its filter.
    crack: f32,
    crack_f: [[f32; 2]; 2],
    /// Seconds into a thud (0 none), its sine's phase, its noise's filter.
    thud: f32,
    thud_phase: f32,
    thud_f: [f32; 2],
    heard: (u32, u32),
    started: bool,
    level: f32,
}

impl Forest {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// A state-variable filter: (band, low).
fn svf(s: &mut [f32; 2], x: f32, f: f32, q: f32) -> (f32, f32) {
    let low = s[1] + f * s[0];
    let high = x - low - q * s[0];
    let band = f * high + s[0];
    s[0] = band;
    s[1] = low;
    (band, low)
}

/// The varied thrush's pitches (Hz): each song one of them, held.
const THRUSH: [f32; 5] = [2900.0, 3400.0, 3900.0, 4500.0, 5100.0];

impl Synth for Forest {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(m) = self.mix.try_lock() {
            self.now = *m;
        }
        let m = self.now;
        if !self.started {
            // what happened before the sound began isn't heard again
            self.heard = (m.cracks, m.thuds);
            self.started = true;
        }
        if m.cracks > self.heard.0 {
            self.heard.0 = m.cracks;
            self.crack = 0.5;
        }
        if m.thuds > self.heard.1 {
            self.heard.1 = m.thuds;
            self.thud = 1e-4;
        }
        let rate = rate as f32;
        let dt = 1.0 / rate;
        let fq = |hz: f32| (2.0 * (std::f32::consts::PI * hz / rate).sin()).min(1.0);
        let hurry = (m.hurry / 60.0).clamp(0.0, 1.0);
        let (f_hush, f_low) = (fq(700.0 + 1500.0 * hurry), fq(160.0));
        let (f_crack, f_thud) = (fq(2600.0), fq(90.0));
        let size = (m.size / 60.0).clamp(0.4, 1.2);
        let target = 0.05 + 0.10 * hurry;
        let frames = out.len() / channels.max(1);
        for f in 0..frames {
            self.level += (target - self.level) * 0.0003;
            let n = [self.rand() * 2.0 - 1.0, self.rand() * 2.0 - 1.0];
            let mut s = [0.0f32; 2];
            for c in 0..2 {
                let (band, _) = svf(&mut self.hush[c], n[c], f_hush, 1.3);
                let (_, low) = svf(&mut self.low[c], n[c], f_low, 1.0);
                s[c] = self.level * (band * 0.7 + low * 0.9);
            }
            let d = self.drips(dt, hurry);
            let t = self.thrush(dt);
            let g = self.fall(n, dt, f_crack, f_thud, size);
            for c in 0..2 {
                s[c] += d[c] + t[c] + g[c];
            }
            for c in 0..channels {
                out[f * channels + c] = s[c.min(1)].clamp(-1.0, 1.0);
            }
        }
    }
}

impl Forest {
    /// Water dripping off the moss, more as the years run faster.
    fn drips(&mut self, dt: f32, hurry: f32) -> [f32; 2] {
        let mut s = [0.0f32; 2];
        self.drip_owed += (1.6 + 4.0 * hurry) * dt;
        if self.drip_owed >= 1.0 {
            self.drip_owed -= 1.0;
            if self.drips.len() < 14 {
                let freq = 1400.0 + 2400.0 * self.rand();
                let amp = 0.015 + 0.03 * self.rand();
                let pan = self.rand();
                self.drips.push(Drip {
                    phase: 0.0,
                    freq,
                    amp,
                    pan,
                    age: 0.0,
                });
            }
        }
        for d in &mut self.drips {
            // a drop's note falls a little as it rings
            let env = (-d.age / 0.035).exp();
            let v = (d.phase * std::f32::consts::TAU).sin() * d.amp * env;
            s[0] += v * (1.0 - d.pan);
            s[1] += v * d.pan;
            d.phase += d.freq * (1.0 - d.age * 2.0) * dt;
            d.age += dt;
        }
        self.drips.retain(|d| d.age < 0.2);
        s
    }

    /// Now and then a varied thrush: one long buzzy whistle on one pitch.
    fn thrush(&mut self, dt: f32) -> [f32; 2] {
        let mut s = [0.0f32; 2];
        self.thrush_in -= dt;
        if self.thrush.is_none() && self.thrush_in <= 0.0 {
            let i = (self.rand() * THRUSH.len() as f32) as usize % THRUSH.len();
            let len = 1.4 + 0.8 * self.rand();
            let pan = 0.2 + 0.6 * self.rand();
            self.thrush = Some(Thrush {
                phase: 0.0,
                freq: THRUSH[i],
                age: 0.0,
                len,
                pan,
            });
            self.thrush_in = 7.0 + 9.0 * self.rand();
        }
        if let Some(t) = self.thrush.as_mut() {
            let env = (t.age / 0.18).min(1.0) * ((t.len - t.age) / 0.6).clamp(0.0, 1.0);
            // buzzy: the whistle beaten at some 70 Hz, a faint octave over it
            let buzz = 0.6 + 0.4 * (t.age * 70.0 * std::f32::consts::TAU).sin();
            let ph = t.phase * std::f32::consts::TAU;
            let v = (ph.sin() + 0.12 * (2.0 * ph).sin()) * env * buzz * 0.035;
            s[0] += v * (1.0 - t.pan);
            s[1] += v * t.pan;
            t.phase += t.freq * dt;
            t.age += dt;
            if t.age >= t.len {
                self.thrush = None;
            }
        }
        s
    }

    /// A giant going: the crack, splintering, then the thud of it landing.
    fn fall(&mut self, n: [f32; 2], dt: f32, f_crack: f32, f_thud: f32, size: f32) -> [f32; 2] {
        let mut s = [0.0f32; 2];
        if self.crack > 0.0 {
            let k = self.crack / 0.5;
            let splinter = if self.rand() < 0.004 { 1.0 } else { 0.0 };
            for c in 0..2 {
                let (band, _) = svf(&mut self.crack_f[c], n[c] + splinter * 4.0, f_crack, 0.5);
                s[c] += band * k * k * 0.25 * size;
            }
            self.crack -= dt;
        }
        // and the thud of it landing, felt more than heard
        if self.thud > 0.0 {
            let env = (-self.thud / 0.55).exp() * (self.thud / 0.01).min(1.0);
            let (_, low) = svf(&mut self.thud_f, n[0], f_thud, 0.7);
            let sine = (self.thud_phase * std::f32::consts::TAU).sin();
            let v = (sine * 0.5 + low * 1.4) * env * 0.5 * size;
            s[0] += v;
            s[1] += v;
            self.thud_phase += (48.0 - 10.0 * self.thud) * dt;
            self.thud += dt;
            if self.thud > 3.0 {
                self.thud = 0.0;
            }
        }
        s
    }
}
