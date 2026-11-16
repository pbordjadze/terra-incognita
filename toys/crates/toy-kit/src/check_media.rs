//! The check's files: a shot as PNG, the sound as WAV, its loudness, and a shot stretched for looking.

use std::path::Path;

pub fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> anyhow::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    image::save_buffer(path, rgba, w, h, image::ExtendedColorType::Rgba8)?;
    Ok(())
}

pub fn write_wav(path: &Path, samples: &[f32], channels: u16, rate: u32) -> anyhow::Result<()> {
    let mut b = Vec::with_capacity(44 + samples.len() * 2);
    let data = (samples.len() * 2) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, b)?;
    Ok(())
}

/// The loudness of interleaved stereo at 48 kHz: integrated LUFS (ITU-R BS.1770-4: K-weighted,
/// 400 ms blocks, gated at -70 LUFS and 10 LU below), RMS and peak in dBFS. Silence is -inf.
pub fn loudness(samples: &[f32], rate: f64) -> (f64, f64, f64) {
    let db = |x: f64| {
        if x > 0.0 {
            20.0 * x.log10()
        } else {
            f64::NEG_INFINITY
        }
    };
    let frames = samples.len() / 2;
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())) as f64;
    let rms = if frames > 0 {
        (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
    } else {
        0.0
    };
    // the K-weighting's two biquads (BS.1770's coefficients at 48 kHz)
    let (b1, a1) = (
        [1.53512485958697, -2.69169618940638, 1.19839281085285],
        [-1.69065929318241, 0.73248077421585],
    );
    let (b2, a2) = ([1.0, -2.0, 1.0], [-1.99004745483398, 0.99007225036621]);
    let mut sq = vec![0.0f64; frames];
    for ch in 0..2 {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        let (mut u1, mut u2, mut z1, mut z2) = (0.0, 0.0, 0.0, 0.0);
        for (i, s) in sq.iter_mut().enumerate() {
            let x = samples[i * 2 + ch] as f64;
            let y = b1[0] * x + b1[1] * x1 + b1[2] * x2 - a1[0] * y1 - a1[1] * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            let z = b2[0] * y + b2[1] * u1 + b2[2] * u2 - a2[0] * z1 - a2[1] * z2;
            (u2, u1, z2, z1) = (u1, y, z1, z);
            *s += z * z;
        }
    }
    let block = (0.4 * rate) as usize;
    let hop = (0.1 * rate) as usize;
    let mut blocks = Vec::new();
    let mut i = 0;
    while block > 0 && i + block <= frames {
        blocks.push(sq[i..i + block].iter().sum::<f64>() / block as f64);
        i += hop;
    }
    let lufs = |z: f64| -0.691 + 10.0 * z.log10();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let abs: Vec<f64> = blocks.into_iter().filter(|z| lufs(*z) > -70.0).collect();
    let integrated = if abs.is_empty() {
        f64::NEG_INFINITY
    } else {
        let rel = lufs(mean(&abs)) - 10.0;
        let gated: Vec<f64> = abs.iter().copied().filter(|z| lufs(*z) > rel).collect();
        if gated.is_empty() {
            f64::NEG_INFINITY
        } else {
            lufs(mean(&gated))
        }
    };
    (integrated, db(rms), db(peak))
}

/// A contrast-stretched copy of a frame: each channel's 0.5th to 99.5th percentile of luminance
/// spread over the whole range, so a faint panel or a seam shows.
pub fn stretch(rgba: &[u8]) -> Vec<u8> {
    let mut hist = [0usize; 256];
    for p in rgba.as_chunks::<4>().0 {
        hist[((p[0] as u32 * 54 + p[1] as u32 * 183 + p[2] as u32 * 19) >> 8) as usize] += 1;
    }
    let n = rgba.len() / 4;
    let pct = |q: f64| {
        let mut acc = 0;
        for (v, c) in hist.iter().enumerate() {
            acc += c;
            if acc as f64 >= q * n as f64 {
                return v as f64;
            }
        }
        255.0
    };
    let (lo, hi) = (pct(0.005), pct(0.995));
    let span = (hi - lo).max(1.0);
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let s = |c: u8| ((c as f64 - lo) / span * 255.0).round().clamp(0.0, 255.0) as u8;
            [s(p[0]), s(p[1]), s(p[2]), 255]
        })
        .collect()
}
