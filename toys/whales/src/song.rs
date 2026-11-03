//! The songs: the bundled recordings (tools/toys/whales/recordings.py writes them as `.song`: a little
//! header, the loudness and pitch of each thirtieth of a second, then 4-bit IMA ADPCM), decoded here,
//! and the choir that plays them on the audio thread, each voice looping its whale's song at the
//! gain and pan the toy sets.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use toy_kit::Synth;

pub struct Song {
    /// Samples a second of `pcm`.
    pub rate: f64,
    pub pcm: Vec<f32>,
    /// Per frame: loudness 0..1 and the sung pitch 0..1 (log, 12 Hz .. 4 kHz).
    pub env: Vec<[f32; 2]>,
    pub env_rate: f64,
}

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143,
    157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552,
    1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487,
    12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

impl Song {
    pub fn parse(b: &[u8]) -> anyhow::Result<Song> {
        anyhow::ensure!(b.len() > 24 && &b[0..4] == b"SONG", "not a song");
        let u = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let rate = u(8) as f64;
        let n = u(12) as usize;
        let frames = u(16) as usize;
        let env_rate = f32::from_le_bytes([b[20], b[21], b[22], b[23]]) as f64;
        let mut at = 24;
        anyhow::ensure!(b.len() >= at + frames * 2, "short envelope");
        let env = (0..frames).map(|i| [b[at + 2 * i] as f32 / 255.0, b[at + 2 * i + 1] as f32 / 255.0]).collect();
        at += frames * 2;
        let mut pcm = Vec::with_capacity(n + 1024);
        while at + 516 <= b.len() && pcm.len() < n {
            let mut pred = i16::from_le_bytes([b[at], b[at + 1]]) as i32;
            let mut idx = b[at + 2] as i32;
            for byte in &b[at + 4..at + 516] {
                for code in [byte & 15, byte >> 4] {
                    let code = code as i32;
                    let step = STEPS[idx as usize];
                    let mut d = step >> 3;
                    if code & 4 != 0 {
                        d += step;
                    }
                    if code & 2 != 0 {
                        d += step >> 1;
                    }
                    if code & 1 != 0 {
                        d += step >> 2;
                    }
                    pred = if code & 8 != 0 { pred - d } else { pred + d }.clamp(-32768, 32767);
                    idx = (idx + INDEX[code as usize]).clamp(0, 88);
                    pcm.push(pred as f32 / 32768.0);
                }
            }
            at += 516;
        }
        pcm.truncate(n);
        anyhow::ensure!(pcm.len() == n, "short song");
        Ok(Song { rate, pcm, env, env_rate })
    }

    pub fn seconds(&self) -> f64 {
        self.pcm.len() as f64 / self.rate
    }

    /// The loudness and pitch at `t` seconds into the song (it loops).
    pub fn env_at(&self, t: f64) -> [f32; 2] {
        let n = self.env.len();
        if n == 0 {
            return [0.0, 0.0];
        }
        let f = (t.rem_euclid(self.seconds()) * self.env_rate).floor() as usize % n;
        self.env[f]
    }
}

/// What the toy tells the audio thread, and what it tells back: each voice's gain and pan (f32 bits)
/// and how far into its song it is (seconds, f64 bits).
#[derive(Default)]
pub struct Mix {
    pub gain: [AtomicU32; 2],
    pub pan: [AtomicU32; 2],
    pub pos: [AtomicU64; 2],
    /// Frames the audio thread has rendered (the toy knows the sound is running when it moves).
    pub frames: AtomicU64,
}

impl Mix {
    pub fn set(&self, i: usize, gain: f32, pan: f32) {
        self.gain[i].store(gain.to_bits(), Ordering::Relaxed);
        self.pan[i].store(pan.to_bits(), Ordering::Relaxed);
    }
    pub fn pos(&self, i: usize) -> f64 {
        f64::from_bits(self.pos[i].load(Ordering::Relaxed))
    }
}

pub struct Choir {
    songs: [Arc<Song>; 2],
    mix: Arc<Mix>,
    pos: [f64; 2],
    g: [f32; 2],
    p: [f32; 2],
}

impl Choir {
    pub fn new(songs: [Arc<Song>; 2], mix: Arc<Mix>, start: [f64; 2]) -> Choir {
        let pos = [start[0] * songs[0].rate, start[1] * songs[1].rate];
        Choir { songs, mix, pos, g: [0.0; 2], p: [0.0; 2] }
    }
}

impl Synth for Choir {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let frames = out.len() / channels.max(1);
        let tg: [f32; 2] = [0, 1].map(|i| f32::from_bits(self.mix.gain[i].load(Ordering::Relaxed)));
        let tp: [f32; 2] = [0, 1].map(|i| f32::from_bits(self.mix.pan[i].load(Ordering::Relaxed)));
        // a gain or a pan moves over about a fifth of a second, never a click
        let k = 1.0 - (-1.0 / (0.2 * rate as f32)).exp();
        for f in 0..frames {
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in 0..2 {
                self.g[v] += (tg[v] - self.g[v]) * k;
                self.p[v] += (tp[v] - self.p[v]) * k;
                let s = &self.songs[v];
                let n = s.pcm.len();
                let i = self.pos[v] as usize % n;
                let fr = (self.pos[v] - self.pos[v].floor()) as f32;
                let x = s.pcm[i] * (1.0 - fr) + s.pcm[(i + 1) % n] * fr;
                self.pos[v] += s.rate / rate;
                if self.pos[v] >= n as f64 {
                    self.pos[v] -= n as f64;
                }
                // equal power
                let a = (self.p[v].clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
                l += x * self.g[v] * a.cos();
                r += x * self.g[v] * a.sin();
            }
            // a soft ceiling above a knee, so two loud whales together never clip (and one is untouched)
            let soft = |x: f32| {
                let a = x.abs();
                if a < 0.6 { x } else { x.signum() * (0.6 + 0.35 * ((a - 0.6) / 0.35).tanh()) }
            };
            let (l, r) = (soft(l), soft(r));
            if channels == 1 {
                out[f] = 0.5 * (l + r);
            } else {
                out[f * channels] = l;
                out[f * channels + 1] = r;
            }
        }
        for v in 0..2 {
            let t = self.pos[v] / self.songs[v].rate;
            self.mix.pos[v].store(t.to_bits(), Ordering::Relaxed);
        }
        self.mix.frames.fetch_add(frames as u64, Ordering::Relaxed);
    }
}
