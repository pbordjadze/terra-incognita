#!/usr/bin/env python3
"""Screw's ears for a critic who can't hear (painting-company row 250): a played plan's sound
(`<out>/audio.wav`, toy-kit check.rs) as a picture and as numbers.

    python3 tools/toys/screw/listen.py <plan.json> <out-dir>

Writes `<out>/listen.png`: a spectrogram on a log frequency scale (40 Hz to 8 kHz; a voice's
harmonics fall as the record slows, a chop shows as a block said twice, a rewind as a swoosh), the
plan's touches marked under it. Prints the touches and the longest silence. (A speed measured from a
whole song's sound was tried, by its beat and by its pitch against the source, and was not reliable
over seconds: chords transpose into each other, a beat and a half folds like a beat. The speed, the
click-free cut and the beat-exact chop are measured sample by sample in screw's own tests, deck.rs;
this shows what a whole minute sounds like.

    --measure    also prints (row 254, Frost) the numbers a critic who can't hear needs: each touch's
                 first sound and how long after it, the loudness second by second (RMS and peak, dBFS),
                 clicks (a sample-to-sample step far above the sound round it, i.e. a discontinuity),
                 and the tones held over each quarter second as note names (a melody, or a drone).
"""
import json, sys, wave
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont

def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    plan, out = json.load(open(args[0])), Path(args[1])
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
    pic = Image.new('RGB', (W, H + 60), (12, 11, 14))
    pic.paste(Image.fromarray(rgb), (0, 0))
    d = ImageDraw.Draw(pic)
    font = ImageFont.load_default()
    for f in (100, 300, 1000, 3000):
        y = int(np.searchsorted(-rows, -f))
        d.line([(0, y), (12, y)], fill=(160, 160, 170)); d.text((14, y - 6), f'{f} Hz', fill=(160, 160, 170), font=font)
    X = lambda t: int(t / secs * (W - 1))
    for s in range(0, int(secs) + 1, 2):
        d.text((X(s), H + 2), f'{s}s', fill=(120, 120, 130), font=font)
    print(f'{out}: {secs:.1f} s of sound')
    for ev in plan.get('events', []):
        if 'move' in ev:
            continue
        kind = next((k for k in ('tap', 'hold', 'drag', 'open', 'down', 'up', 'shake') if k in ev), '?')
        what = ev.get(kind)
        label = f'{kind} {what}' if isinstance(what, str) and kind != 'open' else kind
        t = ev['t']
        d.line([(X(t), H - 30), (X(t), H + 14)], fill=(250, 220, 120))
        d.text((X(t) + 3, H + 16 + 12 * (len(label) % 3)), label, fill=(250, 220, 120), font=font)
        print(f'  t={t:5.1f} {label}')
    # silence: a stretch of 0.5 s under -50 dBFS
    blk = int(0.1 * R)
    rms = np.array([np.sqrt(np.mean(x[i:i + blk] ** 2)) for i in range(0, len(x) - blk, blk)])
    quiet = rms < 10 ** (-50 / 20)
    run, best = 0, (0, 0)
    for i, q in enumerate(quiet):
        run = run + 1 if q else 0
        if run > best[0]: best = (run, i)
    print(f'  longest silence {best[0] * 0.1:.1f} s, ending {best[1] * 0.1:.1f} s; peak {np.abs(x).max():.2f}')
    if '--measure' in sys.argv:
        measure(plan, x, R)
    pic.save(out / 'listen.png')
    print(f'  {out / "listen.png"}')

NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

def note_name(f):
    m = int(round(69 + 12 * np.log2(f / 440)))
    return f'{NAMES[m % 12]}{m // 12 - 1}'

def measure(plan, x, R):
    db = lambda v: 20 * np.log10(max(v, 1e-9))
    # the envelope: 5 ms RMS
    blk = int(0.005 * R)
    env = np.sqrt(np.convolve(x * x, np.ones(blk) / blk, mode='same'))
    print('  touches, and the first sound after each (a rise 6 dB over the 50 ms before it):')
    for ev in plan.get('events', []):
        kind = next((k for k in ('tap', 'hold', 'down', 'up', 'shake') if k in ev), None)
        if kind is None:
            continue
        i = int(ev['t'] * R)
        if i >= len(x):
            continue
        base = max(env[max(0, i - int(0.05 * R)):i].mean() if i > 0 else 0, 10 ** (-60 / 20))
        after = env[i:i + int(1.5 * R)]
        hit = np.nonzero(after > base * 2)[0]
        if len(hit):
            j = hit[0]
            pk = after[j:j + int(0.3 * R)].max()
            print(f'    t={ev["t"]:5.2f} {kind:5s} sound {1000 * j / R:5.0f} ms after, rising {db(base):5.1f} -> {db(pk):5.1f} dBFS')
        else:
            print(f'    t={ev["t"]:5.2f} {kind:5s} no new sound within 1.5 s (level {db(base):.1f} dBFS)')
    # clicks: a step between samples far beyond what the sound round it does (12x the local mean step,
    # and above -40 dBFS): a discontinuity, which a speaker plays as a click
    d = np.abs(np.diff(x))
    k = int(0.002 * R)
    local = np.convolve(d, np.ones(k) / k, mode='same')
    clicks = np.nonzero((d > 12 * local + 1e-9) & (d > 0.01))[0]
    groups = [c for n, c in enumerate(clicks) if n == 0 or c - clicks[n - 1] > R * 0.01]
    print(f'  clicks: {len(groups)}' + (' at ' + ', '.join(f'{c / R:.2f} s' for c in groups[:12]) if groups else ''))
    # loudness each second
    sec = [x[i:i + R] for i in range(0, len(x) - R // 2, R)]
    print('  each second, RMS / peak dBFS:')
    line = []
    for n, v in enumerate(sec):
        line.append(f'{n:2d}s {db(np.sqrt(np.mean(v * v))):5.1f}/{db(np.abs(v).max()):5.1f}')
        if len(line) == 6:
            print('    ' + '  '.join(line)); line = []
    if line:
        print('    ' + '  '.join(line))
    # tones: each quarter second, the peaks of the spectrum (200 Hz to 2.6 kHz, above the crackle's rings) standing 20 dB over its
    # median, as note names, strongest first
    n = 8192
    win = np.hanning(n)
    f = np.fft.rfftfreq(n, 1 / R)
    band = (f > 200) & (f < 2600)
    print('  tones each quarter second (strongest first; - none):')
    row, notes = [], set()
    for q in range(0, len(x) - n, R // 4):
        s = np.abs(np.fft.rfft(x[q:q + n] * win))
        b = 20 * np.log10(s[band] + 1e-9)
        fb = f[band]
        med = np.median(b)
        pk = [i for i in range(1, len(b) - 1) if b[i] > b[i - 1] and b[i] >= b[i + 1] and b[i] > med + 20 and b[i] > -50 + 20 * np.log10(n / 2)]
        pk = sorted(pk, key=lambda i: -b[i])[:3]
        names = []
        for i in pk:
            nm = note_name(fb[i])
            if nm not in names:
                names.append(nm)
        notes.update(names)
        row.append(f'{q / R:5.2f} ' + (','.join(names) if names else '-'))
        if len(row) == 6:
            print('    ' + ' | '.join(row)); row = []
    if row:
        print('    ' + ' | '.join(row))
    print(f'  distinct notes: {len(notes)} ({" ".join(sorted(notes, key=lambda s: (int(s[-1]) if s[-1].isdigit() else 0, NAMES.index(s[:-1]))))})')

if __name__ == '__main__':
    main()
