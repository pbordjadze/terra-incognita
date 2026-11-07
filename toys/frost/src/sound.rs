//! The window's sound (row 254: "a little more musicality"). The pane at −50 °C is a hard glass plate
//! that rings, and the finger plays it (toys/frost/SOUND.md):
//!
//! - **A touch strikes the glass**: a glassy tone, its pitch by the height of the touch on one
//!   pentatonic scale (higher up the pane, higher), ringing long on bare glass and damped to a short,
//!   muffled tick on thick frost.
//! - **The warm finger on ice**: a soft wet sizzle as it melts (by how much ice is under it and how
//!   fast it goes) with small dry snaps as it breaks through ferns; and as it moves the wet glass sings
//!   faintly (a glass harp), its note following the finger's height.
//! - **The regrowth answers**: each crystal that starts on a melt's edge rings a small high ping on the
//!   same scale, in the order the stroke was drawn, a beat after the finger lifts; the frost creeping
//!   in from the frame and the healing holes chime the same way, softer.
//! - Under all, the fine crackle of the freezing (as many clicks as cells freezing, drawn: frost on a
//!   pane is all but silent) and the faint cold air. A shake is a soft wet rush across the glass.
//!
//! Every voice starts with a short ramp (no clicks) and the whole is kept soft: a phone held close.

use std::sync::{Arc, Mutex};
use toy_kit::Synth;

/// The scale: D major pentatonic, as semitones above D.
const SCALE: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
/// D4.
const ROOT: f32 = 293.66;

/// The frequency of a step of the scale (0 = D4, 5 = D5, …).
pub fn note(step: i32) -> f32 {
    let o = step.div_euclid(5);
    let s = SCALE[step.rem_euclid(5) as usize];
    ROOT * 2f32.powf(o as f32 + s / 12.0)
}

/// A height on the pane (0 at the top, 1 at the foot) as a step of the scale: two octaves, the foot
/// the lowest.
pub fn step_at(height: f64) -> i32 {
    ((1.0 - height.clamp(0.0, 1.0)) * 10.0).round() as i32
}

#[derive(Clone, Copy, Debug)]
pub enum Note {
    /// The glass struck: frequency, pan (-1..1), loudness, and how much frost damps it (0..1).
    Strike {
        f: f32,
        pan: f32,
        amp: f32,
        damp: f32,
    },
    /// A crystal starting: a small high ping.
    Ping { f: f32, pan: f32, amp: f32 },
    /// The stove's warmth sweeping the glass.
    Rush,
}

#[derive(Default)]
struct Want {
    /// Crackle clicks a second, and where.
    rate: f32,
    pan: f32,
    /// The melt under the finger: the sizzle's loudness (0..1), the snaps a second, where.
    sizzle: f32,
    snaps: f32,
    touch_pan: f32,
    /// The wet glass singing: its frequency and loudness (0..1).
    sing_f: f32,
    sing: f32,
    notes: Vec<Note>,
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
    /// The finger: how loud the melt is, how many snaps a second, the glass's song, where.
    pub fn touch(&self, sizzle: f32, snaps: f32, sing_f: f32, sing: f32, pan: f32) {
        if let Ok(mut w) = self.0.lock() {
            w.sizzle = sizzle;
            w.snaps = snaps;
            w.sing_f = sing_f;
            w.sing = sing;
            w.touch_pan = pan;
        }
    }
    pub fn play(&self, n: Note) {
        if let Ok(mut w) = self.0.lock() {
            if w.notes.len() < 64 {
                w.notes.push(n);
            }
        }
    }
    pub fn synth(&self) -> Box<dyn Synth> {
        Box::new(CrackleSynth::new(self.0.clone()))
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

/// A ringing glass voice: a few modes, each a decaying phasor (re, im) turned by (c, s)·r a sample.
struct Glass {
    modes: Vec<[f32; 5]>,
    t: f32,
    attack: f32,
    pan: f32,
    /// a fingertip's or a grain's tick at the start: noise, this loud, this long
    tick: f32,
    tick_len: f32,
    /// its loudness now, gliding to duck_to
    gain: f32,
    duck_to: f32,
}

impl Glass {
    /// `partials`: (ratio, loudness, decay seconds); `beat`: the first mode's twin, a little detuned.
    fn new(
        f: f32,
        amp: f32,
        pan: f32,
        partials: &[(f32, f32, f32)],
        beat: f32,
        rate: f32,
        tick: f32,
    ) -> Glass {
        let mut modes = Vec::new();
        let mut add = |hz: f32, a: f32, decay: f32| {
            if hz < rate * 0.45 && a > 0.0 {
                let w = std::f32::consts::TAU * hz / rate;
                let r = (-1.0 / (decay * rate)).exp();
                modes.push([a * amp, 0.0, w.cos() * r, w.sin() * r, 0.0]);
            }
        };
        for (i, &(k, a, d)) in partials.iter().enumerate() {
            if i == 0 && beat > 0.0 {
                add(f * k, a * 0.6, d);
                add(f * k + beat, a * 0.4, d * 0.9);
            } else {
                add(f * k, a, d);
            }
        }
        Glass {
            modes,
            t: 0.0,
            attack: 0.002,
            pan,
            tick: tick * amp,
            tick_len: 0.004,
            gain: 1.0,
            duck_to: 1.0,
        }
    }
    fn sample(&mut self, dt: f32, noise: f32) -> f32 {
        let mut s = 0.0;
        for m in &mut self.modes {
            // (re, im) ← (re, im)·(c + i s)
            let (re, im) = (m[0], m[1]);
            m[0] = re * m[2] - im * m[3];
            m[1] = re * m[3] + im * m[2];
            s += m[1];
        }
        let ramp = (self.t / self.attack).min(1.0);
        let tick = if self.t < self.tick_len * 6.0 {
            noise * self.tick * (-self.t / self.tick_len).exp()
        } else {
            0.0
        };
        self.t += dt;
        self.gain += (self.duck_to - self.gain) * (dt / 0.03).min(1.0);
        (s * ramp * ramp * (3.0 - 2.0 * ramp) + tick * ramp) * self.gain
    }
    /// Let it down to `k` of its loudness over the next 30 ms (a duck, never a step).
    fn duck(&mut self, k: f32) {
        self.duck_to *= k;
    }
    fn done(&self) -> bool {
        self.t > 0.05
            && self
                .modes
                .iter()
                .all(|m| (m[0].abs() + m[1].abs()) * self.gain < 2e-5)
    }
}

struct CrackleSynth {
    want: Arc<Mutex<Want>>,
    rate: f32,
    pan: f32,
    rng: u32,
    clicks: Vec<Click>,
    glass: Vec<Glass>,
    air: [f32; 2],
    hp: [f32; 2],
    // the finger: the sizzle's level, its filter's state and bubbles; the song's phase and level
    sizzle: f32,
    want_sizzle: f32,
    snaps: f32,
    touch_pan: f32,
    sz: [f32; 2],
    bubble: f32,
    sing_f: f32,
    want_sing_f: f32,
    sing: f32,
    want_sing: f32,
    sing_ph: f32,
    // the rush: its age (negative: none) and filter
    rush: f32,
    rush_lp: [f32; 2],
    /// the shake's notes still to come: (seconds into the rush, note)
    cascade: Vec<(f32, Note)>,
}

impl CrackleSynth {
    fn new(want: Arc<Mutex<Want>>) -> Self {
        CrackleSynth {
            want,
            rate: 0.0,
            pan: 0.0,
            rng: 0x2545_f491,
            clicks: Vec::new(),
            glass: Vec::new(),
            air: [0.0; 2],
            hp: [0.0; 2],
            sizzle: 0.0,
            want_sizzle: 0.0,
            snaps: 0.0,
            touch_pan: 0.0,
            sz: [0.0; 2],
            bubble: 0.0,
            sing_f: 0.0,
            want_sing_f: 0.0,
            sing: 0.0,
            want_sing: 0.0,
            sing_ph: 0.0,
            rush: -1.0,
            rush_lp: [0.0; 2],
            cascade: Vec::new(),
        }
    }
    fn rand(&mut self) -> f32 {
        // xorshift
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }
    fn start(&mut self, n: Note, rate: f32) {
        if self.glass.len() >= 28 {
            // the quietest goes
            if let Some(i) = (0..self.glass.len()).min_by(|&a, &b| {
                let e = |g: &Glass| g.modes.iter().map(|m| m[0].abs() + m[1].abs()).sum::<f32>();
                e(&self.glass[a]).total_cmp(&e(&self.glass[b]))
            }) {
                self.glass.swap_remove(i);
            }
        }
        match n {
            Note::Strike { f, pan, amp, damp } => {
                // a new touch stands clear: what still rings is let down (a quick run of taps is a
                // run of notes, not a pad)
                for g in &mut self.glass {
                    g.duck(0.35);
                }
                // a pane: its fundamental and its inharmonic upper modes, the frost taking the ring
                let k = 1.0 - 0.85 * damp;
                let partials = [
                    (1.0, 1.0, 0.6 * k),
                    (2.01, 0.42, 0.34 * k * k),
                    (2.93, 0.2, 0.2 * k * k),
                    (4.16, 0.12 * k, 0.1 * k),
                    (5.43, 0.06 * k, 0.06 * k),
                ];
                self.glass.push(Glass::new(
                    f,
                    amp,
                    pan,
                    &partials,
                    0.9,
                    rate,
                    0.6 + 1.6 * damp,
                ));
            }
            Note::Ping { f, pan, amp } => {
                for g in &mut self.glass {
                    g.duck(0.8);
                }
                let partials = [(1.0, 1.0, 0.32), (2.0, 0.22, 0.12), (4.0, 0.06, 0.05)];
                self.glass
                    .push(Glass::new(f, amp, pan, &partials, 1.7, rate, 0.25));
            }
            Note::Rush => {
                self.rush = 0.0;
                // the frost letting go: a cascade down the scale from the top of the pane to its
                // foot as the warmth sweeps it, each a little softer, the last ringing on
                self.cascade.clear();
                for i in 0..14 {
                    let at = 0.11 * i as f32 + 0.02 * self.rand();
                    let step = 15 - i as i32 - if self.rand() < 0.3 { 1 } else { 0 };
                    let pan = -0.6 + 1.2 * i as f32 / 13.0;
                    let amp = 0.10 * (1.0 - 0.5 * i as f32 / 13.0);
                    self.cascade.push((
                        at,
                        Note::Ping {
                            f: note(step),
                            pan,
                            amp,
                        },
                    ));
                }
            }
        }
    }
}

fn place(s: f32, pan: f32) -> (f32, f32) {
    let p = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (s * p.cos(), s * p.sin())
}

impl Synth for CrackleSynth {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let (mut want_rate, mut want_pan) = (self.rate, self.pan);
        let mut notes = Vec::new();
        if let Ok(mut w) = self.want.try_lock() {
            want_rate = w.rate;
            want_pan = w.pan;
            self.want_sizzle = w.sizzle;
            self.snaps = w.snaps;
            self.touch_pan = w.touch_pan;
            self.want_sing = w.sing;
            if w.sing_f > 0.0 {
                self.want_sing_f = w.sing_f;
            }
            notes = std::mem::take(&mut w.notes);
        }
        let r = rate as f32;
        for n in notes {
            self.start(n, r);
        }
        let dt = 1.0 / r;
        for frame in out.chunks_mut(channels) {
            self.rate += (want_rate - self.rate) * 0.0015;
            self.pan += (want_pan - self.pan) * 0.0005;
            // a click now? (Poisson at the freezing's rate)
            if self.rand() < self.rate * dt && self.clicks.len() < 48 {
                let a = self.rand();
                let amp = 0.012 + 0.07 * a * a * a;
                let len = 0.0015 + 0.004 * self.rand();
                let pan = (self.pan + (self.rand() - 0.5) * 0.9).clamp(-1.0, 1.0);
                let f = 2600.0 + 5200.0 * self.rand();
                self.clicks.push(Click {
                    t: 0.0,
                    len,
                    amp,
                    pan,
                    f,
                    ph: 0.0,
                });
            }
            // a snap where the finger breaks through ice: lower, drier, sharper than the crackle
            if self.rand() < self.snaps * dt && self.clicks.len() < 48 {
                let a = self.rand();
                let pan = (self.touch_pan + (self.rand() - 0.5) * 0.3).clamp(-1.0, 1.0);
                let f = 1100.0 + 1800.0 * self.rand();
                self.clicks.push(Click {
                    t: 0.0,
                    len: 0.0025 + 0.003 * a,
                    amp: 0.07 + 0.08 * a,
                    pan,
                    f,
                    ph: 0.0,
                });
            }
            let (mut l, mut rr) = (0.0f32, 0.0f32);
            let mut i = 0;
            while i < self.clicks.len() {
                let n = self.rand() * 2.0 - 1.0;
                let c = &mut self.clicks[i];
                // (a quarter-millisecond rise: a crackle, not a click in the speaker)
                let rise = (c.t / 0.00025).min(1.0);
                let env = (-c.t / c.len).exp() * rise;
                c.ph = (c.ph + c.f * dt).fract();
                let ring = (c.ph * std::f32::consts::TAU).sin()
                    * (-c.t / (c.len * 3.0)).exp()
                    * 0.5
                    * rise;
                let (a, b) = place((n * env + ring) * c.amp, c.pan);
                l += a;
                rr += b;
                c.t += dt;
                if c.t > c.len * 12.0 {
                    self.clicks.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            // the air outside: brown noise, very low
            for side in 0..2 {
                let n = self.rand() * 2.0 - 1.0;
                self.air[side] = (self.air[side] + n * 0.02) * 0.995;
            }
            l += self.air[0] * 0.008;
            rr += self.air[1] * 0.008;
            // a gentle high-pass on the clicks and the air (keeps the crackle fine, not thumpy)
            let (hl, hr) = (l - self.hp[0], rr - self.hp[1]);
            self.hp[0] += (l - self.hp[0]) * 0.02;
            self.hp[1] += (rr - self.hp[1]) * 0.02;
            let (mut ol, mut or) = (hl * 0.9 + l * 0.1, hr * 0.9 + rr * 0.1);

            // the melt: noise, band-passed high, in small wet bubbles
            // (rising in 25 ms, dying away over a quarter second: the water stops, not a cut)
            let k = if self.want_sizzle > self.sizzle {
                0.0008
            } else {
                0.0001
            };
            self.sizzle += (self.want_sizzle - self.sizzle) * k;
            if self.sizzle > 1e-4 {
                if self.rand() < 90.0 * dt {
                    self.bubble = self.bubble.max(0.4 + 0.6 * self.rand());
                }
                self.bubble *= 0.9985;
                let n = self.rand() * 2.0 - 1.0;
                // (two one-pole stages: a band round 3–5 kHz)
                self.sz[0] += (n - self.sz[0]) * 0.45;
                let hi = self.sz[0] - self.sz[1];
                self.sz[1] += (self.sz[0] - self.sz[1]) * 0.25;
                let s = hi * self.sizzle * (0.25 + 0.75 * self.bubble) * 0.10;
                let (a, b) = place(s, self.touch_pan);
                ol += a;
                or += b;
            }
            // the wet glass singing under the moving finger (a glass harp's tone, gliding to its note)
            // (rising in 50 ms, let go over half a second: the glass stops singing, not a switch)
            let k = if self.want_sing > self.sing {
                0.0004
            } else {
                0.00004
            };
            self.sing += (self.want_sing - self.sing) * k;
            if self.want_sing_f > 0.0 {
                if self.sing_f == 0.0 {
                    self.sing_f = self.want_sing_f;
                }
                self.sing_f += (self.want_sing_f - self.sing_f) * 0.0012;
            }
            if self.sing > 1e-4 {
                self.sing_ph = (self.sing_ph + self.sing_f * dt).fract();
                let p = self.sing_ph * std::f32::consts::TAU;
                let s =
                    (p.sin() + 0.18 * (2.0 * p).sin() + 0.05 * (3.0 * p).sin()) * self.sing * 0.06;
                let (a, b) = place(s, self.touch_pan);
                ol += a;
                or += b;
            }
            // the glass's voices
            let mut i = 0;
            while i < self.glass.len() {
                let n = self.rand() * 2.0 - 1.0;
                let g = &mut self.glass[i];
                let s = g.sample(dt, n);
                let (a, b) = place(s, g.pan);
                ol += a;
                or += b;
                if g.done() {
                    self.glass.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            // the rush: a wet sweep across the glass, low to bright and away, 1.6 s
            if self.rush >= 0.0 {
                let t = self.rush;
                while let Some(k) = self.cascade.iter().position(|c| c.0 <= t) {
                    let (_, n) = self.cascade.swap_remove(k);
                    self.start(n, r);
                }
                // (a soft swell and a long taper, darker than before: water, under the notes)
                let a = (t / 0.35).min(1.0);
                let env = a * a * (3.0 - 2.0 * a) * (-(t - 0.35).max(0.0) / 0.7).exp();
                let cut = 0.015 + 0.08 * (t / 0.8).min(1.0);
                for side in 0..2 {
                    let n = self.rand() * 2.0 - 1.0;
                    self.rush_lp[side] += (n - self.rush_lp[side]) * cut;
                }
                let k = env * 0.22;
                let sweep = (t / 0.9).min(1.0) * 2.0 - 1.0;
                let (a, _) = place(self.rush_lp[0] * k, sweep);
                let (_, b) = place(self.rush_lp[1] * k, sweep);
                ol += a;
                or += b;
                self.rush += dt;
                if t > 4.5 {
                    self.rush = -1.0;
                }
            }
            // a soft limit (never a hard clip)
            let soft = |x: f32| {
                if x.abs() < 0.6 {
                    x
                } else {
                    x.signum() * (0.6 + 0.4 * ((x.abs() - 0.6) / 0.4).tanh())
                }
            };
            frame[0] = soft(ol);
            if channels > 1 {
                frame[1] = soft(or);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 48000.0;

    fn render(c: &Crackle, s: &mut Box<dyn Synth>, secs: f64) -> Vec<f32> {
        let mut out = vec![0.0; (secs * R) as usize * 2];
        for chunk in out.chunks_mut(960) {
            s.render(chunk, 2, R);
        }
        let _ = c;
        out.chunks(2).map(|f| (f[0] + f[1]) * 0.5).collect()
    }

    /// The strongest frequency in a stretch (a plain DFT over a band, fine enough for a test).
    fn peak_hz(x: &[f32], lo: f32, hi: f32) -> f32 {
        let mut best = (0.0, lo);
        let mut f = lo;
        while f < hi {
            let w = std::f32::consts::TAU * f / R as f32;
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (i, v) in x.iter().enumerate() {
                re += v * (w * i as f32).cos();
                im += v * (w * i as f32).sin();
            }
            let m = re * re + im * im;
            if m > best.0 {
                best = (m, f);
            }
            f += 1.0;
        }
        best.1
    }

    #[test]
    fn a_strike_rings_at_the_height_s_note_without_a_click() {
        for (h, want) in [
            (0.95, note(step_at(0.95))),
            (0.5, note(step_at(0.5))),
            (0.05, note(step_at(0.05))),
        ] {
            let c = Crackle::default();
            let mut s = c.synth();
            c.play(Note::Strike {
                f: want,
                pan: 0.0,
                amp: 0.25,
                damp: 0.0,
            });
            let x = render(&c, &mut s, 1.0);
            let got = peak_hz(&x[2400..2400 + 9600], want * 0.8, want * 1.2);
            // the onset rises, not steps: the level in its first 0.1 ms against its loudest 1 ms
            let lvl = |a: usize, b: usize| x[a..b].iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let first = lvl(0, 5);
            let loud = (0..20)
                .map(|k| lvl(k * 48, k * 48 + 48))
                .fold(0.0f32, f32::max);
            let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            println!("measured: strike at height {h}: wants {want:.1} Hz, rings at {got:.1} Hz; its first 0.1 ms at {:.0}% of its loudest; peak {peak:.2}", 100.0 * first / loud);
            assert!(first < loud * 0.1);
            assert!((got - want).abs() < want * 0.01 + 2.0, "{got} vs {want}");
            assert!(peak < 0.5);
        }
    }

    #[test]
    fn frost_damps_the_strike() {
        let ring = |damp: f32| {
            let c = Crackle::default();
            let mut s = c.synth();
            c.play(Note::Strike {
                f: note(5),
                pan: 0.0,
                amp: 0.25,
                damp,
            });
            let x = render(&c, &mut s, 1.2);
            let tail = &x[(0.4 * R) as usize..(0.5 * R) as usize];
            (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt()
        };
        let (bare, frost) = (ring(0.0), ring(1.0));
        println!(
            "measured: strike's ring 0.4 s on: bare glass {:.1} dBFS, under thick frost {:.1} dBFS",
            20.0 * bare.log10(),
            20.0 * frost.log10()
        );
        assert!(bare > frost * 6.0);
    }

    #[test]
    fn the_scale_is_pentatonic_and_rises_up_the_pane() {
        let notes: Vec<f32> = (0..=10).map(note).collect();
        println!(
            "measured: the scale, foot to top: {:?}",
            notes.iter().map(|f| format!("{f:.0}")).collect::<Vec<_>>()
        );
        assert!(step_at(0.0) > step_at(1.0));
        assert!((note(5) / note(0) - 2.0).abs() < 1e-3);
    }

    #[test]
    fn many_pings_stay_soft() {
        let c = Crackle::default();
        let mut s = c.synth();
        // a hard stroke and its answer: a strike, the finger's sizzle and song at their loudest, then
        // eighteen pings 0.08 s apart over it, and a shake
        c.play(Note::Strike {
            f: note(3),
            pan: 0.0,
            amp: 0.22,
            damp: 0.0,
        });
        c.play(Note::Rush);
        c.touch(1.0, 60.0, note(4), 1.0, 0.0);
        let mut x = render(&c, &mut s, 0.6);
        for k in 0..18 {
            c.play(Note::Ping {
                f: note(5 + k % 7),
                pan: 0.0,
                amp: 0.14,
            });
            x.extend(render(&c, &mut s, 0.08));
        }
        x.extend(render(&c, &mut s, 1.0));
        let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        println!("measured: a strike, a rush, the sizzle, snaps and song at their loudest, eighteen pings: peak {peak:.2}");
        assert!(peak < 0.6);
    }
}
