//! Murmuration's sound (the word is the sound), fed by what the simulation hears (`Ears`).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use toy_kit::Synth;

/// What the sound hears, from the simulation (levels as f32 bits).
#[derive(Default)]
pub(crate) struct Ears {
    pub rush: AtomicU32,
    pub murmur: AtomicU32,
    pub wings: AtomicU32,
    pub rate: AtomicU32,
    pub phase: AtomicU32,
}

pub(crate) fn put(a: &AtomicU32, v: f32) {
    a.store(v.to_bits(), Ordering::Relaxed)
}
pub(crate) fn get(a: &AtomicU32) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

/// The sound of a murmuration (the word is the sound): the hush of thousands of wings far off, the
/// rush of the near ones passing, and, from behind your own bird, its wingbeats.
pub(crate) struct Wind {
    ears: Arc<Ears>,
    seed: u32,
    pink: [f32; 3],
    bp: [[f32; 2]; 3],
    lv: [f32; 3],
    phase: f32,
}

impl Wind {
    pub(crate) fn new(ears: Arc<Ears>) -> Wind {
        Wind {
            ears,
            seed: 0x1234_5678,
            pink: [0.0; 3],
            bp: [[0.0; 2]; 3],
            lv: [0.0; 3],
            phase: 0.0,
        }
    }
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One state-variable band-pass step: returns the band.
fn svf(s: &mut [f32; 2], x: f32, f: f32, q: f32) -> f32 {
    let low = s[0] + f * s[1];
    let high = x - low - q * s[1];
    let band = f * high + s[1];
    s[0] = low;
    s[1] = band;
    band
}

impl Synth for Wind {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let target = [
            get(&self.ears.murmur),
            get(&self.ears.rush),
            get(&self.ears.wings),
        ];
        let beat = get(&self.ears.rate).max(1.0);
        // Your bird's wingbeat, heard in step with the one seen.
        self.phase = get(&self.ears.phase) / std::f32::consts::TAU;
        let fr = |hz: f32| 2.0 * (std::f32::consts::PI * hz / rate as f32).sin();
        let (f_low, f_mid, f_hi) = (fr(420.0), fr(1500.0), fr(3200.0));
        for frame in out.chunks_mut(channels) {
            for k in 0..3 {
                self.lv[k] += (target[k] - self.lv[k]) * 0.0004;
            }
            let w = self.noise();
            // Pinkish: a few one-pole filters summed.
            self.pink[0] = 0.997 * self.pink[0] + 0.03 * w;
            self.pink[1] = 0.96 * self.pink[1] + 0.12 * w;
            self.pink[2] = 0.6 * self.pink[2] + 0.4 * w;
            let pink = self.pink[0] + self.pink[1] + self.pink[2] * 0.5;
            let murmur = svf(&mut self.bp[0], pink, f_low, 1.2) * self.lv[0] * 0.5;
            let rush = svf(&mut self.bp[1], w, f_mid, 1.4) * self.lv[1] * 0.4;
            self.phase = (self.phase + beat / rate as f32) % 1.0;
            let flutter = (0.5 - 0.5 * (self.phase * std::f32::consts::TAU).cos()).powi(3);
            let wings = svf(&mut self.bp[2], w, f_hi, 1.6) * flutter * self.lv[2] * 0.12;
            let s = (murmur + rush + wings).clamp(-1.0, 1.0);
            for c in frame.iter_mut() {
                *c = s;
            }
        }
    }
}
