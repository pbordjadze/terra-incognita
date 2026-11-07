//! A practice record for the checks (and anyone without a song): synthesized here, nobody's music.
//! 76 BPM, sixteen bars: a long 808-style kick, a clap on two and four, hats on the eighths, a bass
//! line, an electric piano's chords, and a voice (a buzz through three vowel formants) singing a short
//! phrase in two bars of every four, from the first, so slowing it down drops something voice-like, as the screw does.

const BPM: f64 = 76.0;
const BARS: usize = 16;
pub const RATE: f64 = 44_100.0;

/// (rate, left, right).
pub fn record() -> (f64, Vec<f32>, Vec<f32>) {
    let beat = 60.0 / BPM;
    let n = (BARS as f64 * 4.0 * beat * RATE) as usize + (RATE * 1.5) as usize;
    let mut l = vec![0.0f32; n];
    let mut r = vec![0.0f32; n];
    let tau = std::f64::consts::TAU;
    let mut seed = 0x9E37_79B9u32;
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f64 / u32::MAX as f64) * 2.0 - 1.0
    };
    // the harmony: a bar each of Am7, Fmaj7, Dm7, E7 (roots in Hz)
    let roots = [110.0, 87.31, 73.42, 82.41];
    let chords: [[f64; 4]; 4] = [
        [220.0, 261.63, 329.63, 392.0],
        [174.61, 220.0, 261.63, 329.63],
        [146.83, 174.61, 220.0, 261.63],
        [164.81, 207.65, 246.94, 293.66],
    ];
    // the voice's phrase (a pitch for each eighth note of a bar, 0 for rest) and its vowels
    let phrase = [440.0, 0.0, 392.0, 329.63, 0.0, 329.63, 293.66, 261.63];
    let vowels = [
        [730.0, 1090.0, 2440.0],
        [570.0, 840.0, 2410.0],
        [300.0, 870.0, 2240.0],
    ];
    let mut bp = [[0.0f64; 2]; 3];
    let mut voice_phase = 0.0f64;
    let mut bass_phase = 0.0f64;
    for k in 0..n {
        let t = k as f64 / RATE;
        let b = (t / beat).floor();
        let ph = t - b * beat;
        let bar = (b as usize / 4) % 4;
        let in_bar = b as usize % 4;
        let on = (b as usize) < BARS * 4;
        let mut mono = 0.0f64;
        let (mut sl, mut sr) = (0.0f64, 0.0f64);
        if on {
            // the kick on one and three (and an extra on the "and" of three), long and low
            let kick_at = if in_bar == 0 || in_bar == 2 {
                Some(ph)
            } else if in_bar == 3 && ph > beat * 0.5 {
                Some(ph - beat * 0.5)
            } else {
                None
            };
            if let Some(p) = kick_at {
                let f = 48.0 + 110.0 * (-p * 28.0).exp();
                mono += 0.75 * (-p * 3.2).exp() * (tau * f * p).sin();
            }
            // the clap on two and four
            if in_bar % 2 == 1 {
                let e = (-ph * 22.0).exp()
                    + 0.5
                        * (-(ph - 0.012).max(0.0) * 30.0).exp()
                        * if ph > 0.012 { 1.0 } else { 0.0 };
                let c = 0.28 * e * noise();
                sl += c;
                sr += c;
            }
            // hats on the eighths, a little to the right
            let e8 = t % (beat / 2.0);
            let h = 0.07 * (-e8 * 70.0).exp() * noise();
            sl += h * 0.7;
            sr += h;
            // the bass: the root, an eighth before each beat it slides
            let f = roots[bar]
                * if in_bar == 3 && ph > beat * 0.5 {
                    1.5
                } else {
                    1.0
                };
            bass_phase += tau * f / RATE;
            mono += 0.22 * bass_phase.sin() * (1.0 - (-ph * 40.0).exp());
            // the piano's chord on each bar's one and the "and" of two
            let hit = if in_bar == 0 {
                Some(ph)
            } else if in_bar >= 1 && t - (b - in_bar as f64 + 1.5) * beat >= 0.0 {
                Some(t - (b - in_bar as f64 + 1.5) * beat)
            } else {
                None
            };
            if let Some(p) = hit {
                for (i, &cf) in chords[bar].iter().enumerate() {
                    let tone =
                        (tau * cf * p).sin() + 0.3 * (tau * 2.0 * cf * p).sin() * (-p * 6.0).exp();
                    let v = 0.035 * tone * (-p * 1.4).exp();
                    if i % 2 == 0 {
                        sl += v;
                        sr += v * 0.6;
                    } else {
                        sl += v * 0.6;
                        sr += v;
                    }
                }
            }
            // the voice, from the first bar: two bars of every four
            let vb = b as usize / 4;
            if vb.is_multiple_of(2) || vb % 4 == 3 {
                let eighth = ((t / (beat / 2.0)).floor() as usize) % 8;
                let p8 = t % (beat / 2.0);
                let f0 = phrase[eighth];
                if f0 > 0.0 {
                    let vib = 1.0 + 0.012 * (tau * 5.5 * t).sin();
                    voice_phase += f0 * vib / RATE;
                    let saw = 2.0 * voice_phase.fract() - 1.0;
                    let env =
                        (p8 / 0.03).min(1.0) * (1.0 - ((p8 - beat * 0.42) / 0.04).clamp(0.0, 1.0));
                    let vw = vowels[eighth % 3];
                    let mut sum = 0.0;
                    for (j, &fc) in vw.iter().enumerate() {
                        // a two-pole resonator at the formant
                        let w = tau * fc / RATE;
                        let rr = (-std::f64::consts::PI * 90.0 / RATE).exp();
                        let y =
                            (1.0 - rr) * saw + 2.0 * rr * w.cos() * bp[j][0] - rr * rr * bp[j][1];
                        bp[j][1] = bp[j][0];
                        bp[j][0] = y;
                        sum += y * [1.0, 0.6, 0.25][j];
                    }
                    mono += 0.5 * sum * env;
                }
            }
        }
        l[k] = (mono + sl).tanh() as f32 * 0.85;
        r[k] = (mono + sr).tanh() as f32 * 0.85;
    }
    (RATE, l, r)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_practice_record_has_its_own_beat() {
        let (rate, l, r) = super::record();
        let (beat, _) = crate::tempo::beat(&l, &r, rate);
        assert!(
            (beat / (60.0 / super::BPM) - 1.0).abs() < 0.002,
            "found {beat}"
        );
        assert!(l.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    }
}
