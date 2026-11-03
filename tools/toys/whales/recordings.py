#!/usr/bin/env python3
"""Builds the whales' songs for the toy (toys/whales/data/*.song) from real recordings.

    pip install numpy scipy
    python3 tools/toys/whales/recordings.py            # fetches the sources, writes toys/whales/data/
    python3 tools/toys/whales/recordings.py --cache D  # keeps the downloads in D (default: /tmp/whale-src)

The sources (toys/whales/SOURCES.md credits them):

- humpback: MBARI's Pacific Sound, the MARS cabled hydrophone in Monterey Bay (891 m deep), the 16 kHz
  decimated archive (CC BY 4.0), 2020-11-28 00:01:40-00:04:40 UTC: a male singing close to the
  hydrophone (found with tools/toys/whales/scout.py). Only those three minutes are range-read.
- blue: NOAA PMEL's Acoustics Program, a Northeast Pacific blue whale's A-B call (nepblue24s10x.wav),
  as NOAA publishes it: played ten times faster than it was sung, which brings its 16 Hz moan into
  hearing (US government work, public domain).

What is done to them, and only this: the band the animal sings in is kept (a humpback 90 Hz-5.5 kHz,
the sped-up blue 50-420 Hz: the rest is the sea's and the ships' noise, and the blue file's clipping),
the steady noise floor is lowered a little (a gentle spectral subtraction, never to silence), the
level evened (a gentle 2:1 compressor, then a loudness with a soft ceiling: the clicks and
sharpest onsets otherwise set the peak and the song sits far under it), the end crossfaded into the start so it loops, and it is resampled (both to 12 kHz) and stored as 4-bit IMA ADPCM, which the toy decodes (src/song.rs). Nothing is added.

The file (`.song`, little-endian): b"SONG", u32 version 1, u32 rate, u32 samples, u32 envelope
frames, f32 envelope rate (frames a second); the envelope (per frame: u8 loudness 0-255, u8 pitch,
the spectral peak's frequency on a log scale 0 = 12 Hz .. 255 = 4 kHz, of the sound as sung, before
any speeding); then ADPCM blocks of 1024 samples, each i16 predictor, u8 step index, u8 0, then 512
bytes of nibbles, the low nibble first.
"""
import argparse, io, os, struct, sys, urllib.request
import numpy as np
from scipy import signal
from scipy.io import wavfile

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "../../../toys/whales/data"))
ENV_RATE = 30.0

MBARI = "https://pacific-sound-16khz.s3.amazonaws.com/2020/11/MARS-20201128T000000Z-16kHz.wav"
NOAA_BLUE = "https://www.pmel.noaa.gov/acoustics/whales/sounds/whalewav/nepblue24s10x.wav"


def fetch(url, cache, name, rng=None):
    os.makedirs(cache, exist_ok=True)
    p = os.path.join(cache, name)
    if not os.path.exists(p):
        h = {"Range": f"bytes={rng[0]}-{rng[1]}"} if rng else {}
        data = urllib.request.urlopen(urllib.request.Request(url, headers=h), timeout=300).read()
        open(p, "wb").write(data)
    return open(p, "rb").read()


def mbari(cache, start_s, seconds):
    rate, header = 16000, 332
    a = header + int(start_s * rate) * 3
    b = a + int(seconds * rate) * 3 - 1
    raw = fetch(MBARI, cache, f"mars-20201128-{start_s}-{seconds}.raw", (a, b))
    x = np.frombuffer(raw[: len(raw) // 3 * 3], np.uint8).reshape(-1, 3).astype(np.int32)
    v = x[:, 0] | (x[:, 1] << 8) | (x[:, 2] << 16)
    v = np.where(v >= 1 << 23, v - (1 << 24), v)
    return rate, v.astype(np.float64) / (1 << 23)


def band(x, rate, lo, hi):
    sos = signal.butter(4, [lo, hi], btype="bandpass", fs=rate, output="sos")
    return signal.sosfiltfilt(sos, x)


def denoise(x, rate, strength):
    """Lowers the steady floor: each bin less `strength` times its median level, never below 25 %."""
    f, t, Z = signal.stft(x, rate, nperseg=1024, noverlap=768)
    mag = np.abs(Z)
    floor = np.median(mag, axis=1, keepdims=True)
    gain = np.maximum(1.0 - strength * floor / (mag + 1e-12), 0.25)
    gain = signal.convolve2d(gain, np.ones((3, 5)) / 15.0, mode="same", boundary="symm")
    _, y = signal.istft(Z * gain, rate, nperseg=1024, noverlap=768)
    return y[: len(x)]


def loop(x, rate, fade_s):
    n = int(fade_s * rate)
    head, body, tail = x[:n], x[n:-n] if n else x, x[-n:]
    k = np.sin(np.linspace(0, np.pi / 2, n)) ** 2
    joined = tail * (1 - k) + head * k
    return np.concatenate([body, joined]) if n else x


def envelope(x, rate, speed):
    hop = int(rate / ENV_RATE)
    nfft = 1 << int(np.ceil(np.log2(hop * 4)))
    frames = len(x) // hop
    loud, pitch = [], []
    win = np.hanning(nfft)
    xp = np.concatenate([np.zeros(nfft // 2), x, np.zeros(nfft)])
    f = np.fft.rfftfreq(nfft, 1 / rate)
    for i in range(frames):
        seg = xp[i * hop : i * hop + nfft] * win
        s = np.abs(np.fft.rfft(seg)) ** 2
        loud.append(10 * np.log10(s.sum() + 1e-20))
        k = int(np.argmax(s[1:])) + 1
        sung = max(f[k] / speed, 1e-3)
        pitch.append(np.clip(np.log(sung / 12.0) / np.log(4000 / 12.0), 0, 1))
    loud = np.array(loud)
    lo, hi = np.percentile(loud, 20), np.percentile(loud, 99.5)
    loud = np.clip((loud - lo) / (hi - lo + 1e-9), 0, 1)
    return (loud * 255).astype(np.uint8), (np.array(pitch) * 255).astype(np.uint8)


STEPS = [7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97,
         107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724,
         796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026,
         4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500,
         20350, 22385, 24623, 27086, 29794, 32767]
INDEX = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8]


def adpcm(pcm):
    """IMA ADPCM in blocks of 1024 samples (src/song.rs decodes it the same way)."""
    out = io.BytesIO()
    pcm = np.concatenate([pcm, np.zeros((-len(pcm)) % 1024, np.int16)])
    idx = 0
    for b in range(0, len(pcm), 1024):
        blk = pcm[b : b + 1024].astype(int)
        pred = int(blk[0])
        out.write(struct.pack("<hBB", pred, idx, 0))
        nib = []
        for s in blk:
            step = STEPS[idx]
            diff = s - pred
            code = 0
            if diff < 0:
                code, diff = 8, -diff
            d = step >> 3
            if diff >= step:
                code |= 4; diff -= step; d += step
            if diff >= step >> 1:
                code |= 2; diff -= step >> 1; d += step >> 1
            if diff >= step >> 2:
                code |= 1; d += step >> 2
            pred = max(-32768, min(32767, pred - d if code & 8 else pred + d))
            idx = max(0, min(88, idx + INDEX[code]))
            nib.append(code)
        out.write(bytes(nib[i] | (nib[i + 1] << 4) for i in range(0, 1024, 2)))
    return out.getvalue()


def write(name, x, rate, env):
    loud, pitch = env
    pcm = (np.clip(x, -1, 1) * 32767).astype(np.int16)
    head = b"SONG" + struct.pack("<IIIIf", 1, rate, len(pcm), len(loud), ENV_RATE)
    envb = bytes(np.stack([loud, pitch], 1).reshape(-1))
    data = head + envb + adpcm(pcm)
    os.makedirs(OUT, exist_ok=True)
    open(os.path.join(OUT, name), "wb").write(data)
    print(f"{name}: {len(pcm) / rate:.1f} s at {rate} Hz, {len(data) / 1e3:.0f} kB")


def norm(x, peak=0.89):
    return x * peak / (np.max(np.abs(x)) + 1e-12)


def even(x, rate, rms_db, ratio=2.0):
    """Evens the level: a gentle compressor (2:1 above the song's median loudness, on a 50 ms
    envelope), then the whole set to `rms_db` dBFS with a soft ceiling at 0.95. A recording's clicks
    and sharpest onsets otherwise set its peak, and its song sits 20 dB below it."""
    w = max(int(0.05 * rate), 1)
    env = np.sqrt(np.convolve(x * x, np.ones(w) / w, "same")) + 1e-9
    knee = np.median(env)
    gain = np.where(env > knee, (env / knee) ** (1.0 / ratio - 1.0), 1.0)
    gain = np.convolve(gain, np.ones(w) / w, "same")
    y = x * gain
    y *= 10 ** (rms_db / 20) / np.sqrt(np.mean(y * y))
    return 0.95 * np.tanh(y / 0.95)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", default="/tmp/whale-src")
    a = ap.parse_args()

    # the humpback, as sung (speed 1)
    rate, x = mbari(a.cache, 100, 182)
    x = band(x, rate, 90, 5500)
    x = denoise(x, rate, 0.7)
    x = loop(x, rate, 2.0)
    y = signal.resample_poly(x, 3, 4)  # 16 kHz to 12 kHz
    env = envelope(y, 12000, 1.0)
    write("humpback.song", even(y, 12000, -17.0), 12000, env)

    # the blue whale, ten times faster, as NOAA plays it
    rate, b = wavfile.read(io.BytesIO(fetch(NOAA_BLUE, a.cache, "nepblue24s10x.wav")))
    b = b.astype(np.float64) / 32768.0
    b = band(b, rate, 50, 420)
    b = denoise(b, rate, 0.5)
    b = loop(b, rate, 1.0)
    y = signal.resample_poly(b, 3, 2)  # 8 kHz to 12 kHz (played back without images near 4 kHz)
    env = envelope(y, 12000, 10.0)
    write("blue.song", even(y, 12000, -19.0), 12000, env)


if __name__ == "__main__":
    sys.exit(main())
