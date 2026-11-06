#!/usr/bin/env python3
"""Screw's ears for a critic who can't hear (painting-company row 250): a played plan's sound
(`<out>/audio.wav`, toy-kit check.rs) as a picture and as numbers.

    python3 tools/toys/screw/listen.py <plan.json> <out-dir> [--beat <s>]

Writes `<out>/listen.png`: a spectrogram on a log frequency scale (40 Hz to 8 kHz; a voice's
harmonics fall as the record slows, a chop shows as a block said twice, a rewind as a swoosh), the
plan's touches marked under it, and the record's speed measured from the sound alone (the beat's
period in each 3 s, against the song's own beat, `--beat`, by default the practice record's 0.789 s).
Prints the speed it heard at each touch and the longest silence while the record should play. The
click-free cut and the beat-exact chop are proved sample by sample in screw's own tests (deck.rs);
this shows what a whole minute sounds like.
"""
import json, sys, wave
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont

def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    plan, out = json.load(open(args[0])), Path(args[1])
    beat = float(sys.argv[sys.argv.index('--beat') + 1]) if '--beat' in sys.argv else 60 / 76
    w = wave.open(str(out / 'audio.wav'))
    R = w.getframerate()
    x = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768
    x = x.reshape(-1, w.getnchannels()).mean(axis=1)
    # the spectrogram
    n, hop = 4096, 480
    win = np.hanning(n).astype(np.float32)
    frames = max(1, (len(x) - n) // hop)
    spec = np.empty((frames, n // 2 + 1), np.float32)
    for i in range(frames):
        spec[i] = np.abs(np.fft.rfft(x[i * hop:i * hop + n] * win))
    freqs = np.fft.rfftfreq(n, 1 / R)
    H, W = 420, min(1800, frames)
    rows = np.geomspace(40, 8000, H)[::-1]
    idx = np.searchsorted(freqs, rows)
    cols = np.linspace(0, frames - 1, W).astype(int)
    db = 20 * np.log10(spec[cols][:, idx].T + 1e-6)
    img = np.clip((db - (db.max() - 70)) / 70, 0, 1)
    rgb = np.stack([img ** 0.9 * 255, img ** 1.6 * 200, img ** 0.7 * 255 * 0.9], -1).astype(np.uint8)
    secs = len(x) / R
    # the speed heard: each 3 s window's onset autocorrelation, near the song's beat at 0.56..1.15 speed (a two-beat period at 1.0 is a one-beat one at 0.5: the scale stops above that)
    env_hop = 256
    e = np.array([np.sum(x[i:i + env_hop] ** 2) for i in range(0, len(x) - env_hop, env_hop)])
    o = np.maximum(np.diff(np.log(e + 1e-8)), 0)
    fr = R / env_hop
    speeds = []
    for c in np.arange(1.5, secs - 1.5, 0.5):
        a, z = int((c - 1.5) * fr), int((c + 1.5) * fr)
        seg = o[a:z] - o[a:z].mean()
        if np.abs(seg).sum() < 1e-3:
            speeds.append((c, 0.0)); continue
        lags = np.arange(int(beat / 1.15 * fr), int(beat / 0.56 * fr))
        ac = np.array([np.dot(seg[:-l], seg[l:]) for l in lags])
        speeds.append((c, beat / (lags[np.argmax(ac)] / fr)))
    pic = Image.new('RGB', (W, H + 150), (12, 11, 14))
    pic.paste(Image.fromarray(rgb), (0, 0))
    d = ImageDraw.Draw(pic)
    font = ImageFont.load_default()
    for f in (100, 300, 1000, 3000):
        y = int(np.searchsorted(-rows, -f))
        d.line([(0, y), (12, y)], fill=(160, 160, 170)); d.text((14, y - 6), f'{f} Hz', fill=(160, 160, 170), font=font)
    X = lambda t: int(t / secs * (W - 1))
    for s in range(0, int(secs) + 1, 2):
        d.text((X(s), H + 2), f'{s}s', fill=(120, 120, 130), font=font)
    ys = lambda v: H + 140 - int((v - 0.4) / 0.8 * 110)
    for v in (0.5, 0.7, 1.0):
        d.line([(0, ys(v)), (W, ys(v))], fill=(50, 45, 60)); d.text((2, ys(v) - 11), f'{v}x', fill=(110, 100, 130), font=font)
    pts = [(X(c), ys(min(max(v, 0.4), 1.2))) for c, v in speeds if v > 0]
    if len(pts) > 1:
        d.line(pts, fill=(230, 170, 250), width=2)
    print(f'{out}: {secs:.1f} s of sound')
    for ev in plan.get('events', []):
        kind = next((k for k in ('tap', 'hold', 'drag', 'open', 'down', 'up') if k in ev), '?')
        what = ev.get(kind)
        label = f'{kind} {what}' if isinstance(what, str) and kind != 'open' else kind
        t = ev['t']
        d.line([(X(t), H - 30), (X(t), H + 14)], fill=(250, 220, 120))
        d.text((X(t) + 3, H + 16 + 12 * (len(label) % 3)), label, fill=(250, 220, 120), font=font)
        near = [v for c, v in speeds if abs(c - (t + 2.0)) < 0.3]
        print(f'  t={t:5.1f} {label:<18} speed heard 2 s after: {near[0]:.2f}x' if near else f'  t={t:5.1f} {label}')
    # silence: a stretch of 0.5 s under -50 dBFS
    blk = int(0.1 * R)
    rms = np.array([np.sqrt(np.mean(x[i:i + blk] ** 2)) for i in range(0, len(x) - blk, blk)])
    quiet = rms < 10 ** (-50 / 20)
    run, best = 0, (0, 0)
    for i, q in enumerate(quiet):
        run = run + 1 if q else 0
        if run > best[0]: best = (run, i)
    print(f'  longest silence {best[0] * 0.1:.1f} s, ending {best[1] * 0.1:.1f} s; peak {np.abs(x).max():.2f}')
    pic.save(out / 'listen.png')
    print(f'  {out / "listen.png"}')

if __name__ == '__main__':
    main()
