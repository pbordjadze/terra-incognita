//! A song's beat, from the record itself: where the sound starts (its onsets, in three bands), the
//! period that gathers them on one phase when the song is folded on it, the beat's level chosen near
//! a Houston record's (about 82 BPM), then that period refined over the whole song, which also gives
//! where the beats fall.

/// Onset frames a second's worth of hop.
const HOP: usize = 256;

/// (one beat in seconds, the first beat's time in seconds). A song too short or too quiet to tell
/// gets 85 BPM from the start.
pub fn beat(l: &[f32], r: &[f32], rate: f64) -> (f64, f64) {
    let o = onsets(l, r, rate);
    let fr = rate / HOP as f64;
    if o.len() < (fr * 4.0) as usize || o.iter().all(|v| *v == 0.0) {
        return (60.0 / 85.0, 0.0);
    }
    // the period that puts the onsets most on one phase, folded in windows of 16 s (so a period a
    // hair off doesn't smear): a dotted period (a beat and a half, a pattern's accident) splits them
    // three ways and loses. Then the beat's level (half, it, twice, four times) a Houston record's.
    let lag = |bpm: f64| 60.0 * fr / bpm;
    let (lo, hi) = (lag(200.0), lag(50.0));
    let win = (16.0 * fr) as usize;
    let mut best = (f64::MIN, lo);
    let mut p = lo;
    while p <= hi {
        let mut score = 0.0;
        for w in o.chunks(win).filter(|w| w.len() * 2 > win) {
            score += sharpness(w, p, 48).0;
        }
        if score > best.0 {
            best = (score, p);
        }
        p += 0.1;
    }
    let prior = |bpm: f64| (-0.5 * ((bpm / 82.0).log2() / 0.45).powi(2)).exp();
    let p0 = [0.5, 1.0, 2.0, 4.0]
        .iter()
        .map(|m| best.1 * m)
        .filter(|p| *p >= lag(200.0) * 0.98 && *p <= lag(50.0) * 1.02)
        .max_by(|a, b| {
            prior(60.0 * fr / a)
                .partial_cmp(&prior(60.0 * fr / b))
                .unwrap()
        })
        .unwrap_or(best.1);
    if std::env::var("SCREW_TEMPO").is_ok() {
        eprintln!(
            "the folds' period: {:.1} BPM; the beat: {:.1} BPM",
            60.0 * fr / best.1,
            60.0 * fr / p0
        );
    }
    // refined: fold the onsets on periods near it and keep the sharpest fold
    let mut top = (f64::MIN, p0, 0.0);
    let steps = 400;
    for i in 0..=steps {
        let p = p0 * (0.97 + 0.06 * i as f64 / steps as f64);
        let (sharp, phase) = sharpness(&o, p, 96);
        if sharp > top.0 {
            top = (sharp, p, phase * p);
        }
    }
    let beat = top.1 / fr;
    // (an onset frame says where a difference rose: the frame after the sound began, half a hop)
    let first = ((top.2 - 0.5) / fr).rem_euclid(beat);
    (beat, first)
}

/// The onsets folded on a period of `p` frames into `bins`: how far the fullest phase (with its
/// neighbours) stands above the mean, and where it is (a share of the period).
fn sharpness(o: &[f32], p: f64, bins: usize) -> (f64, f64) {
    let mut fold = vec![0.0f64; bins];
    for (n, v) in o.iter().enumerate() {
        let ph = (n as f64 / p).fract();
        fold[((ph * bins as f64) as usize).min(bins - 1)] += *v as f64;
    }
    let mean = fold.iter().sum::<f64>() / bins as f64;
    let (mut m, mut at) = (f64::MIN, 0usize);
    for b in 0..bins {
        let s = fold[b] + 0.5 * (fold[(b + 1) % bins] + fold[(b + bins - 1) % bins]);
        if s > m {
            (m, at) = (s, b);
        }
    }
    (m / (2.0 * mean).max(1e-9), (at as f64 + 0.5) / bins as f64)
}

/// How much sound begins in each hop: the rise of log energy in three bands (the kick's, the voice's
/// and the hats'), less its own local mean.
fn onsets(l: &[f32], r: &[f32], rate: f64) -> Vec<f32> {
    let n = l.len().min(r.len()) / HOP;
    let pole = |hz: f64| (-2.0 * std::f64::consts::PI * hz / rate).exp() as f32;
    let (a1, a2) = (pole(150.0), pole(2000.0));
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    let mut e = vec![[0.0f32; 3]; n];
    for (f, ef) in e.iter_mut().enumerate() {
        for k in f * HOP..(f + 1) * HOP {
            let x = (l[k] + r[k]) * 0.5;
            s1 = x + (s1 - x) * a1;
            s2 = x + (s2 - x) * a2;
            let (low, high) = (s1, x - s2);
            let mid = s2 - s1;
            ef[0] += low * low;
            ef[1] += mid * mid;
            ef[2] += high * high;
        }
    }
    let w = [1.0f32, 0.7, 0.5];
    let mut o = vec![0.0f32; n];
    for f in 1..n {
        for b in 0..3 {
            let d = (1e-7 + e[f][b]).ln() - (1e-7 + e[f - 1][b]).ln();
            o[f] += w[b] * d.max(0.0);
        }
    }
    // less the local mean (half a second each side), never below zero
    let half = (rate / HOP as f64 * 0.5) as usize;
    let mut sum = vec![0.0f64; n + 1];
    for f in 0..n {
        sum[f + 1] = sum[f] + o[f] as f64;
    }
    (0..n)
        .map(|f| {
            let (a, z) = (f.saturating_sub(half), (f + half + 1).min(n));
            (o[f] - ((sum[z] - sum[a]) / (z - a) as f64) as f32).max(0.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A drum pattern at `bpm`: a kick on 1 and 3, a snare on 2 and 4, hats on the eighths, and a
    /// held bass note under it, starting `offset` seconds in.
    pub fn groove(bpm: f64, seconds: f64, offset: f64) -> Vec<f32> {
        let rate = 44_100.0;
        let n = (seconds * rate) as usize;
        let beat = 60.0 / bpm;
        let mut x = vec![0.0f32; n];
        let mut seed = 7u32;
        let mut noise = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
        };
        for (k, s) in x.iter_mut().enumerate() {
            let t = k as f64 / rate - offset;
            *s = 0.05 * (2.0 * std::f64::consts::PI * 55.0 * t).sin() as f32;
            if t < 0.0 {
                continue;
            }
            let b = (t / beat).floor() as i64;
            let ph = t - b as f64 * beat;
            if b % 2 == 0 {
                let f = 50.0 + 90.0 * (-ph * 30.0).exp();
                *s +=
                    (0.8 * (-ph * 9.0).exp() * (2.0 * std::f64::consts::PI * f * ph).sin()) as f32;
            } else {
                *s += (0.4 * (-ph * 18.0).exp()) as f32 * noise();
            }
            let e = t % (beat / 2.0);
            *s += (0.12 * (-e * 60.0).exp()) as f32 * noise();
        }
        x
    }

    #[test]
    fn finds_the_beat_of_a_groove() {
        for &(bpm, off) in &[(72.0, 0.31), (84.0, 0.0), (95.5, 0.12), (150.0, 0.2)] {
            let x = groove(bpm, 40.0, off);
            let (b, first) = beat(&x, &x, 44_100.0);
            let want = if bpm > 110.0 { 120.0 / bpm } else { 60.0 / bpm };
            assert!(
                (b / want - 1.0).abs() < 0.002,
                "{bpm} BPM: a beat of {b:.4} s, want {want:.4}"
            );
            let off_by = ((first - off).rem_euclid(60.0 / bpm) + 0.5 * 60.0 / bpm)
                .rem_euclid(60.0 / bpm)
                - 0.5 * 60.0 / bpm;
            assert!(
                off_by.abs() < 0.015 || bpm > 110.0,
                "{bpm} BPM: the first beat at {first:.3}, want {off}"
            );
        }
    }
}
