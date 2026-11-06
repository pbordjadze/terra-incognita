//! The turntable: one record, a platter whose speed is its pitch, and the moves a Houston DJ makes on
//! it (SOURCES.md). Everything here runs per sample on the audio thread (`DeckSynth`); the toy talks
//! to it through the `Arc<Mutex<Deck>>` it keeps, briefly, and reads back where the needle is.
//!
//! - **The screw.** The platter runs at `pitch` (1.0 as recorded). Speed and pitch move together, as a
//!   turntable's do, never time-stretched: the record is read faster or slower, that is all. Screw
//!   tapes sit around 0.66–0.75.
//! - **The chop.** Two copies of the same record a beat apart, cut between: the needle goes back
//!   exactly one beat of the song (`chop`), so the phrase says itself again and the groove never loses
//!   its place, at whatever speed. The cut is a short eased crossfade (`CUT_S`) between the two
//!   copies, so it never clicks. Held (`hold`), it keeps going back each time the beat comes round.
//! - **Bring it back.** The platter spun back by hand (`rewind`): two bars back along the record, fast,
//!   with the whine of the music played backward, and dropped there.
//! - **The brake.** The motor off (`stop`): the platter slows under its own weight to a stop, the pitch
//!   falling with it; on again (`start`), it pulls up to speed in a fraction of a second.

use std::sync::{Arc, Mutex};

/// The platter's turns a second at 1.0: 33⅓ rpm.
pub const RPS: f64 = 33.0 + 1.0 / 3.0;
/// How long the cut between the two copies takes (an equal-power crossfade).
pub const CUT_S: f64 = 0.010;
/// The motor's pull: speed gained a second when it starts (1.0 in a quarter second).
const PULL: f64 = 4.0;
/// The platter's slowing a second with the motor off (a Technics' brake, about two thirds of a second).
const BRAKE: f64 = 1.5;
/// How quickly the platter follows the pitch fader (its time constant, seconds).
const FOLLOW_S: f64 = 0.06;
/// A rewind goes back this many beats (two bars of four).
pub const REWIND_BEATS: f64 = 8.0;

/// A record: the song's samples, its beat, and its shape for the ring.
pub struct Song {
    pub name: String,
    pub rate: f64,
    pub l: Vec<f32>,
    pub r: Vec<f32>,
    /// One beat of the song, seconds, and where the first one falls.
    pub beat: f64,
    pub first: f64,
    /// Its loudness around the ring, 0..1, `SHAPE` bins from start to end.
    pub shape: Vec<f32>,
}

pub const SHAPE: usize = 360;

impl Song {
    pub fn new(name: &str, rate: f64, l: Vec<f32>, r: Vec<f32>) -> Song {
        let (beat, first) = crate::tempo::beat(&l, &r, rate);
        let shape = shape(&l, &r);
        Song { name: name.into(), rate, l, r, beat, first, shape }
    }

    pub fn seconds(&self) -> f64 {
        self.l.len() as f64 / self.rate
    }

    /// The two channels at `t` seconds (cubic, between samples; silence off the record).
    fn at(&self, t: f64) -> (f32, f32) {
        let x = t * self.rate;
        let i = x.floor();
        let f = (x - i) as f32;
        let i = i as i64;
        let n = self.l.len() as i64;
        let get = |v: &Vec<f32>, k: i64| if k >= 0 && k < n { v[k as usize] } else { 0.0 };
        let cubic = |v: &Vec<f32>| {
            let (a, b, c, d) = (get(v, i - 1), get(v, i), get(v, i + 1), get(v, i + 2));
            // Catmull-Rom
            b + 0.5 * f * (c - a + f * (2.0 * a - 5.0 * b + 4.0 * c - d + f * (3.0 * (b - c) + d - a)))
        };
        (cubic(&self.l), cubic(&self.r))
    }
}

/// The song's loudness in `SHAPE` stretches, as a share of its loudest (on a decibel scale, 36 dB deep).
fn shape(l: &[f32], r: &[f32]) -> Vec<f32> {
    let n = l.len().max(1);
    let mut out = vec![0.0f32; SHAPE];
    for (b, o) in out.iter_mut().enumerate() {
        let (a, z) = (b * n / SHAPE, ((b + 1) * n / SHAPE).min(n));
        let mut s = 0.0f64;
        for k in (a..z).step_by(4) {
            s += (l[k] as f64).powi(2) + (r[k] as f64).powi(2);
        }
        let rms = (s / ((z - a) as f64 / 4.0 * 2.0).max(1.0)).sqrt();
        *o = rms as f32;
    }
    let top = out.iter().cloned().fold(1e-6, f32::max);
    for o in out.iter_mut() {
        let db = 20.0 * (*o / top).max(1e-4).log10();
        *o = ((db + 36.0) / 36.0).clamp(0.0, 1.0);
    }
    out
}

struct Rewind {
    from: f64,
    to: f64,
    t: f64,
    dur: f64,
}

/// The deck: the record on the platter, and everything the hand and motor are doing to it.
#[derive(Default)]
pub struct Deck {
    pub song: Option<Arc<Song>>,
    /// Where the needle is on the record, seconds of the song.
    pub pos: f64,
    /// How fast the record is going under the needle now (1.0 as recorded; negative backward).
    pub vel: f64,
    /// The pitch fader: the speed the motor runs the platter at.
    pub pitch: f64,
    pub motor: bool,
    /// The platter's turns (for the picture: its angle), and a nudge back at each chop.
    pub turns: f64,
    /// Chops so far, rewinds so far (the toy notices them change).
    pub chops: u32,
    pub rewinds: u32,
    /// Samples rendered (the toy's clock check: is the sound running?).
    pub rendered: u64,
    /// The other copy, fading out at a cut: where its needle is, and how far the cut has gone (0..1).
    cut: Option<(f64, f64)>,
    /// Held: where the needle goes back from each time (the end of the repeated beat).
    hold: Option<f64>,
    rewind: Option<Rewind>,
    /// The anti-aliasing filter's memory (when the record runs fast), and its gain's.
    lp: [f32; 2],
    gain: f32,
}

impl Deck {
    pub fn new() -> Deck {
        Deck { pitch: 1.0, ..Default::default() }
    }

    pub fn load(&mut self, song: Arc<Song>) {
        self.song = Some(song);
        (self.pos, self.vel, self.motor, self.cut, self.hold, self.rewind) = (0.0, 0.0, false, None, None, None);
    }

    pub fn beat(&self) -> f64 {
        self.song.as_ref().map(|s| s.beat).unwrap_or(0.5)
    }

    /// The motor on: the platter pulls up to the fader's speed.
    pub fn start(&mut self) {
        if let Some(s) = &self.song {
            if self.pos >= s.seconds() - 0.05 {
                self.pos = 0.0;
            }
            self.motor = true;
        }
    }

    /// The motor off: the brake.
    pub fn stop(&mut self) {
        self.motor = false;
        self.hold = None;
    }

    /// The chop: back exactly one beat, cut across.
    pub fn chop(&mut self) {
        if self.song.is_none() || self.rewind.is_some() {
            return;
        }
        let to = (self.pos - self.beat()).max(0.0);
        self.cut_to(to);
        self.chops += 1;
        self.turns -= 0.05;
    }

    /// Held: chop now, and again each time the needle comes back round to here.
    pub fn hold(&mut self, on: bool) {
        if on && self.song.is_some() && self.rewind.is_none() {
            self.hold = Some(self.pos);
            self.chop();
        } else {
            self.hold = None;
        }
    }

    pub fn holding(&self) -> bool {
        self.hold.is_some()
    }

    /// Bring it back: spun back two bars by hand, then dropped.
    pub fn rewind(&mut self) {
        if self.song.is_none() || self.rewind.is_some() {
            return;
        }
        self.hold = None;
        self.cut = None;
        let to = (self.pos - REWIND_BEATS * self.beat()).max(0.0);
        let back = self.pos - to;
        if back < 0.05 {
            return;
        }
        let share = back / (REWIND_BEATS * self.beat());
        self.rewind = Some(Rewind { from: self.pos, to, t: 0.0, dur: 0.30 + 0.45 * share });
        self.rewinds += 1;
        self.motor = true;
    }

    pub fn rewinding(&self) -> bool {
        self.rewind.is_some()
    }

    fn cut_to(&mut self, to: f64) {
        self.cut = Some((self.pos, 0.0));
        self.pos = to;
    }

    /// One output sample's step of `dt` seconds: the platter, the needle, the moves.
    fn step(&mut self, dt: f64) {
        let len = self.song.as_ref().map(|s| s.seconds()).unwrap_or(0.0);
        if let Some(r) = &mut self.rewind {
            r.t += dt;
            let u = (r.t / r.dur).min(1.0);
            // the hand's spin back: fast in the middle, eased at both ends
            let ease = (1.0 - (std::f64::consts::PI * u).cos()) * 0.5;
            let p = r.from - (r.from - r.to) * ease;
            self.vel = (p - self.pos) / dt;
            self.pos = p;
            if u >= 1.0 {
                self.pos = r.to;
                self.vel = 0.0;
                self.rewind = None;
            }
        } else {
            if self.motor {
                let d = self.pitch - self.vel;
                let limit = PULL * dt;
                self.vel += (d * (dt / FOLLOW_S).min(1.0)).clamp(-limit, limit);
            } else {
                // the brake: a steady slowing to a stop
                self.vel -= self.vel.signum() * self.vel.abs().min(BRAKE * dt);
            }
            self.pos += self.vel * dt;
            if let Some((p, _)) = &mut self.cut {
                *p += self.vel * dt;
            }
            if let Some(end) = self.hold {
                if self.vel > 0.0 && self.pos >= end {
                    let to = (self.pos - self.beat()).max(0.0);
                    self.cut_to(to);
                    self.chops += 1;
                    self.turns -= 0.05;
                }
            }
        }
        if self.pos >= len {
            self.pos = len;
            self.motor = false;
            self.vel = self.vel.min(0.0);
        }
        if self.pos < 0.0 {
            self.pos = 0.0;
        }
        self.turns += self.vel * dt * RPS / 60.0;
    }

    /// `out` (interleaved, `channels` a frame) at `rate` Hz.
    pub fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let dt = 1.0 / rate;
        let ch = channels.max(1);
        for frame in out.chunks_mut(ch) {
            self.step(dt);
            self.rendered += 1;
            let Some(song) = self.song.clone() else {
                frame.fill(0.0);
                continue;
            };
            let (mut l, mut r) = song.at(self.pos);
            if let Some((p, k)) = self.cut {
                let k2 = k + dt / CUT_S;
                // equal gain (the two copies are the same song a beat apart, mostly in step: equal power
                // would swell them by half), on an eased curve: no corner where the cut begins or ends
                let e = k * k * (3.0 - 2.0 * k);
                let (a, b) = (e as f32, 1.0 - e as f32);
                let (ol, or) = song.at(p);
                l = l * a + ol * b;
                r = r * a + or * b;
                self.cut = if k2 >= 1.0 { None } else { Some((p, k2)) };
            }
            // faster than recorded, the read skips samples: a one-pole low-pass keeps it from aliasing
            let speed = self.vel.abs();
            if speed > 1.05 {
                let fc = (rate * 0.4 / speed).min(rate * 0.45);
                let k = (1.0 - (-2.0 * std::f64::consts::PI * fc / rate).exp()) as f32;
                self.lp[0] += (l - self.lp[0]) * k;
                self.lp[1] += (r - self.lp[1]) * k;
                (l, r) = (self.lp[0], self.lp[1]);
            } else {
                self.lp = [l, r];
            }
            // nearly still, the needle's sound fades (no rumble, no click when it stops)
            let g = ((speed / 0.06).min(1.0)) as f32;
            self.gain += (g - self.gain) * 0.01;
            let (l, r) = (ceiling(l * self.gain), ceiling(r * self.gain));
            frame[0] = l;
            if ch > 1 {
                frame[1] = r;
            }
            for c in frame.iter_mut().skip(2) {
                *c = 0.0;
            }
        }
    }

    /// The deck's clock without the sound (a device whose audio didn't start: the record still turns).
    pub fn advance(&mut self, dt: f64) {
        let n = (dt * 2000.0).round() as usize;
        let rendered = self.rendered;
        let mut buf = vec![0.0f32; n];
        self.render(&mut buf, 1, 2000.0);
        self.rendered = rendered;
    }
}

/// A soft ceiling: untouched to 0.9, then bent smoothly toward 1.0 (a loud master read between its
/// samples overshoots a little).
fn ceiling(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.9 {
        x
    } else {
        x.signum() * (0.9 + 0.1 * ((a - 0.9) / 0.1).tanh())
    }
}

/// The deck as the shell's sound.
pub struct DeckSynth(pub Arc<Mutex<Deck>>);

impl toy_kit::Synth for DeckSynth {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        if let Ok(mut d) = self.0.lock() {
            d.render(out, channels, rate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 48_000.0;

    /// A record: a tone, and a click on every beat.
    fn record(freq: f64, bpm: f64, seconds: f64, clicks: bool) -> Arc<Song> {
        let rate = 44_100.0;
        let n = (seconds * rate) as usize;
        let beat = 60.0 / bpm;
        let mut l = vec![0.0f32; n];
        for (k, s) in l.iter_mut().enumerate() {
            let t = k as f64 / rate;
            *s = (0.3 * (2.0 * std::f64::consts::PI * freq * t).sin()) as f32;
            if clicks {
                let ph = (t + 1e-9) % beat;
                if ph < 0.002 {
                    *s += 0.6 * (1.0 - ph / 0.002) as f32;
                }
            }
        }
        Arc::new(Song { name: "test".into(), rate, r: l.clone(), l, beat, first: 0.0, shape: vec![0.5; SHAPE] })
    }

    fn deck(song: Arc<Song>) -> Deck {
        let mut d = Deck::new();
        d.load(song);
        d.start();
        d
    }

    fn run(d: &mut Deck, seconds: f64) -> Vec<f32> {
        let mut out = vec![0.0f32; (seconds * R) as usize * 2];
        d.render(&mut out, 2, R);
        out.iter().step_by(2).cloned().collect()
    }

    /// The tone's frequency by its rising zero crossings.
    fn freq(x: &[f32]) -> f64 {
        let ups: Vec<f64> = x.windows(2).enumerate().filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0).map(|(i, w)| i as f64 + (-w[0] / (w[1] - w[0])) as f64).collect();
        (ups.len() - 1) as f64 / ((ups[ups.len() - 1] - ups[0]) / R)
    }

    #[test]
    fn the_screw_slows_and_lowers_together() {
        let mut d = deck(record(440.0, 90.0, 20.0, false));
        run(&mut d, 1.0);
        assert!((freq(&run(&mut d, 1.0)) - 440.0).abs() < 1.0);
        d.pitch = 0.7;
        run(&mut d, 0.5);
        let p0 = d.pos;
        let f = freq(&run(&mut d, 2.0));
        let moved = d.pos - p0;
        assert!((f - 308.0).abs() < 1.0, "the tone at 0.7: {f}");
        assert!((moved - 1.4).abs() < 0.005, "the song moved {moved} s in 2 s at 0.7");
    }

    #[test]
    fn a_chop_goes_back_one_beat_and_keeps_the_groove() {
        let bpm = 90.0;
        let beat = 60.0 / bpm;
        let song = record(0.0, bpm, 20.0, true);
        for &pitch in &[1.0, 0.7] {
            let mut d = deck(song.clone());
            d.pitch = pitch;
            run(&mut d, 1.0);
            let mut out = run(&mut d, 1.37);
            let before = d.pos;
            d.chop();
            assert!((before - d.pos - beat).abs() < 1e-9);
            out.extend(run(&mut d, 0.4));
            d.chop();
            out.extend(run(&mut d, 3.0));
            // the clicks heard: every one a beat (at this speed) after the last, across both cuts
            let mut at = Vec::new();
            let mut i = 0;
            while i < out.len() {
                if out[i] > 0.45 {
                    at.push(i as f64 / R);
                    i += (0.05 * R) as usize;
                } else {
                    i += 1;
                }
            }
            let want = beat / pitch;
            for w in at.windows(2) {
                let gap = w[1] - w[0];
                assert!((gap - want).abs() < 0.0015, "pitch {pitch}: clicks {gap:.4} s apart, want {want:.4}: {at:?}");
            }
            assert!(at.len() >= 4);
        }
    }

    #[test]
    fn the_cut_never_clicks() {
        let mut d = deck(record(220.0, 87.0, 20.0, false));
        run(&mut d, 1.0);
        let steady = run(&mut d, 0.5);
        let jerk = |x: &[f32]| x.windows(3).map(|w| (w[2] - 2.0 * w[1] + w[0]).abs()).fold(0.0f32, f32::max);
        let calm = jerk(&steady);
        for k in 0..12 {
            run(&mut d, 0.13 + 0.07 * k as f64);
            d.chop();
            let cut = run(&mut d, 0.05);
            assert!(jerk(&cut) < calm * 1.6, "cut {k}: {} against {calm}", jerk(&cut));
        }
    }

    #[test]
    fn held_it_keeps_saying_the_last_beat() {
        let mut d = deck(record(330.0, 80.0, 30.0, false));
        run(&mut d, 3.0);
        let end = d.pos;
        d.hold(true);
        run(&mut d, 4.0);
        assert!(d.pos <= end + 0.01 && d.pos >= end - d.beat() - 0.01, "the needle stays in the last beat: {} vs {end}", d.pos);
        assert!(d.chops >= 5);
        d.hold(false);
        run(&mut d, 2.0);
        assert!(d.pos > end + 1.0);
    }

    #[test]
    fn a_rewind_drops_two_bars_back() {
        let mut d = deck(record(330.0, 80.0, 30.0, false));
        run(&mut d, 10.0);
        let from = d.pos;
        let turns = d.turns;
        d.rewind();
        let mut back = false;
        let mut steps = 0;
        while d.rewinding() && steps < 4000 {
            run(&mut d, 0.0005);
            back |= d.vel < -3.0;
            steps += 1;
        }
        assert!(back, "it spun back fast");
        assert!(d.turns < turns, "the record turned backward");
        assert!(!d.rewinding());
        let landed = from - 8.0 * 0.75;
        assert!((d.pos - landed).abs() < 0.01, "landed at {} for {landed}", d.pos);
        run(&mut d, 1.0);
        assert!((d.vel - 1.0).abs() < 1e-3, "and plays on");
    }

    #[test]
    fn the_brake_slows_it_to_a_stop() {
        let mut d = deck(record(330.0, 80.0, 30.0, false));
        run(&mut d, 2.0);
        d.stop();
        let mut last = d.vel;
        for _ in 0..20 {
            run(&mut d, 0.05);
            assert!(d.vel <= last + 1e-9);
            last = d.vel;
        }
        assert_eq!(d.vel, 0.0);
        let quiet = run(&mut d, 0.2);
        assert!(quiet.iter().all(|s| s.abs() < 1e-3));
        d.start();
        run(&mut d, 0.6);
        assert!((d.vel - 1.0).abs() < 1e-3);
    }
}
