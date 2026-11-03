#!/usr/bin/env python3
"""Listens to a played plan's sound (painting-company row 204): the longest silent stretch, where
the sound sits left to right, and the chimes' level, so a toy that goes quiet when left alone, or
leans to one side, is caught without a speaker.

    python3 tools/toys/fireflies/silence.py <out-dir>/audio.wav [--from S] [--max-silence S]

Silent is below -60 dBFS over a 50 ms window. Exits 1 when the longest silent stretch after
--from (default 3 s, the field's first unsynchronised seconds) is longer than --max-silence
(default 8 s). Balance is (R - L) / (R + L) of the windows' RMS: 0 centred, -1 all left.
"""
import argparse, sys, wave

import numpy as np


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('wav')
    ap.add_argument('--from', dest='start', type=float, default=3.0)
    ap.add_argument('--max-silence', type=float, default=8.0)
    a = ap.parse_args()
    w = wave.open(a.wav)
    rate, ch, n = w.getframerate(), w.getnchannels(), w.getnframes()
    x = np.frombuffer(w.readframes(n), dtype=np.int16).astype(np.float64).reshape(-1, ch) / 32768.0
    win = int(rate * 0.05)
    k = len(x) // win
    seg = x[: k * win].reshape(k, win, ch)
    rms = np.sqrt((seg ** 2).mean(axis=1))  # k x ch
    level = rms.mean(axis=1)
    loud = level > 10 ** (-60 / 20)
    t0 = int(a.start / 0.05)
    longest, run, at, run_start = 0, 0, 0.0, 0
    for i in range(t0, k):
        if loud[i]:
            run = 0
        else:
            if run == 0:
                run_start = i
            run += 1
            if run > longest:
                longest, at = run, run_start * 0.05
    longest_s = longest * 0.05
    bal = 0.0
    if ch == 2:
        l, r = rms[:, 0].sum(), rms[:, 1].sum()
        bal = (r - l) / max(r + l, 1e-12)
    peak = np.abs(x).max()
    sounding = loud[t0:].mean() if k > t0 else 0.0
    print(f'{a.wav}: {n / rate:.1f} s, peak {peak:.2f}, sounding {sounding:.0%} of the time after {a.start:.0f} s, '
          f'longest silence {longest_s:.1f} s (from {at:.1f} s), balance {bal:+.2f}')
    if longest_s > a.max_silence:
        print(f'FAIL: silent for {longest_s:.1f} s (more than {a.max_silence:.0f} s)')
        sys.exit(1)


if __name__ == '__main__':
    main()
