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

/// What the toy tells the audio thread, and what it tells back: each voice's gain, pan, how long its
/// sound takes to arrive (seconds) and how muffled it arrives (a low-pass's corner, Hz), all f32 bits;
/// and how far into its song each voice is (seconds, f64 bits).
pub struct Mix {
    pub gain: [AtomicU32; 2],
    pub pan: [AtomicU32; 2],
    pub delay: [AtomicU32; 2],
    pub corner: [AtomicU32; 2],
    pub pos: [AtomicU64; 2],
    /// Frames the audio thread has rendered (the toy knows the sound is running when it moves).
    pub frames: AtomicU64,
    /// Counts the hydrophone's placings: a new one jumps to the new delay (dipping out and back),
    /// any other change of delay glides.
    pub moved: AtomicU32,
}

impl Default for Mix {
    fn default() -> Mix {
        let f = |x: f32| AtomicU32::new(x.to_bits());
        Mix {
            gain: [f(0.0), f(0.0)],
            pan: [f(0.0), f(0.0)],
            delay: [f(0.0), f(0.0)],
            corner: [f(20_000.0), f(20_000.0)],
            pos: Default::default(),
            frames: AtomicU64::new(0),
            moved: AtomicU32::new(0),
        }
    }
}

impl Mix {
    pub fn set(&self, i: usize, gain: f32, pan: f32) {
        self.gain[i].store(gain.to_bits(), Ordering::Relaxed);
        self.pan[i].store(pan.to_bits(), Ordering::Relaxed);
    }
    /// Where the voice is heard from: `delay` seconds late, muffled above `corner` Hz.
    pub fn place(&self, i: usize, delay: f32, corner: f32) {
        self.delay[i].store(delay.to_bits(), Ordering::Relaxed);
        self.corner[i].store(corner.to_bits(), Ordering::Relaxed);
    }
    pub fn pos(&self, i: usize) -> f64 {
        f64::from_bits(self.pos[i].load(Ordering::Relaxed))
    }
}

/// The longest a song may arrive late (seconds): the rings' life.
pub const MAX_DELAY: f64 = 7.5;

pub struct Choir {
    songs: [Arc<Song>; 2],
    mix: Arc<Mix>,
    pos: [f64; 2],
    g: [f32; 2],
    p: [f32; 2],
    /// Each voice's recent past at the output's rate (a ring buffer), the delay read from it now and
    /// the one it is going to, the dip that hides a jump between them, and the low-pass's state.
    past: [Vec<f32>; 2],
    head: usize,
    delay: [f64; 2],
    dip: [f32; 2],
    lp: [f32; 2],
    corner: [f32; 2],
    /// How much of the past has been written (a delay never reads before the first sound), and the
    /// placing whose delay the voices are at.
    written: usize,
    moved: u32,
    jumping: bool,
}

impl Choir {
    pub fn new(songs: [Arc<Song>; 2], mix: Arc<Mix>, start: [f64; 2]) -> Choir {
        let pos = [start[0] * songs[0].rate, start[1] * songs[1].rate];
        // the past is kept at up to 48 kHz; a faster output keeps a shorter one (MAX_DELAY is then less)
        let n = (MAX_DELAY * 48_000.0) as usize + 4096;
        Choir { songs, mix, pos, g: [0.0; 2], p: [0.0; 2], past: [vec![0.0; n], vec![0.0; n]], head: 0, delay: [0.0; 2], dip: [1.0; 2], lp: [0.0; 2], corner: [20_000.0; 2], written: 0, moved: 0, jumping: false }
    }
}

impl Synth for Choir {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let frames = out.len() / channels.max(1);
        let ld = |a: &AtomicU32| f32::from_bits(a.load(Ordering::Relaxed));
        let tg: [f32; 2] = [0, 1].map(|i| ld(&self.mix.gain[i]));
        let tp: [f32; 2] = [0, 1].map(|i| ld(&self.mix.pan[i]));
        let td: [f64; 2] = [0, 1].map(|i| ld(&self.mix.delay[i]) as f64);
        let tc: [f32; 2] = [0, 1].map(|i| ld(&self.mix.corner[i]));
        let n = self.past[0].len();
        let max_d = ((n - 2) as f64 / rate).min(MAX_DELAY);
        // a gain or a pan moves over about a fifth of a second, never a click
        let k = 1.0 - (-1.0 / (0.2 * rate as f32)).exp();
        // a dip of 60 ms each way hides a jump in the delay (the hydrophone moved)
        let dk = 1.0 / (0.06 * rate as f32);
        let moved = self.mix.moved.load(Ordering::Relaxed);
        if moved != self.moved {
            self.moved = moved;
            self.jumping = true;
        }
        for f in 0..frames {
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in 0..2 {
                self.g[v] += (tg[v] - self.g[v]) * k;
                self.p[v] += (tp[v] - self.p[v]) * k;
                self.corner[v] += (tc[v] - self.corner[v]) * k;
                let s = &self.songs[v];
                let len = s.pcm.len();
                let i = self.pos[v] as usize % len;
                let fr = (self.pos[v] - self.pos[v].floor()) as f32;
                let x = s.pcm[i] * (1.0 - fr) + s.pcm[(i + 1) % len] * fr;
                self.pos[v] += s.rate / rate;
                if self.pos[v] >= len as f64 {
                    self.pos[v] -= len as f64;
                }
                // the song into its past, and out again as late as it arrives where it is heard
                self.past[v][self.head] = x;
                // a jump (the hydrophone moved) dips out and back; a drift (the whale swims, the
                // camera's scale changes) glides, a twentieth of a second each second
                let want = td[v].clamp(0.0, max_d);
                if self.jumping && (want - self.delay[v]).abs() > 0.02 {
                    self.dip[v] -= dk;
                    if self.dip[v] <= 0.0 {
                        self.dip[v] = 0.0;
                        self.delay[v] = want;
                    }
                } else {
                    let slew = 0.2 / rate;
                    self.delay[v] += (want - self.delay[v]).clamp(-slew, slew);
                    self.dip[v] = (self.dip[v] + dk).min(1.0);
                }
                let back = ((self.delay[v] * rate).round() as usize).min(self.written);
                let y = self.past[v][(self.head + n - back.min(n - 1)) % n];
                // the farther, the more muffled: a one-pole low-pass
                let a = 1.0 - (-std::f32::consts::TAU * self.corner[v] / rate as f32).exp();
                self.lp[v] += a * (y - self.lp[v]);
                let y = self.lp[v] * self.dip[v];
                // equal power
                let pa = (self.p[v].clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
                l += y * self.g[v] * pa.cos();
                r += y * self.g[v] * pa.sin();
            }
            self.head = (self.head + 1) % n;
            self.written = (self.written + 1).min(n - 1);
            if self.jumping && self.dip[0] >= 1.0 && self.dip[1] >= 1.0 {
                self.jumping = false;
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
