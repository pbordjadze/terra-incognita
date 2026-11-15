//! Terra's sound: its own bells, and the open toy's sound beside them.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use toy_kit::Synth;

/// Terra's sound: its own bells, and the open toy's sound beside them.
pub struct Mixer {
    pub bells: Box<dyn Synth>,
    pub room: Arc<Mutex<Option<Box<dyn Synth>>>>,
    /// The toy's level (f32 bits), eased toward on the audio thread.
    pub want: Arc<AtomicU32>,
    pub level: f32,
    pub buf: Vec<f32>,
}

impl Synth for Mixer {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        self.bells.render(out, channels, rate);
        if let Ok(mut room) = self.room.try_lock() {
            if let Some(s) = room.as_mut() {
                self.buf.resize(out.len(), 0.0);
                self.buf.fill(0.0);
                s.render(&mut self.buf, channels, rate);
                let want = f32::from_bits(self.want.load(Ordering::Relaxed));
                let step = (1.0 / (0.4 * rate)) as f32;
                for (i, o) in out.iter_mut().enumerate() {
                    if i % channels == 0 {
                        self.level += (want - self.level).clamp(-step, step);
                    }
                    *o += self.buf[i] * self.level;
                }
            } else {
                self.level = 0.0;
            }
        }
    }
}
