#!/usr/bin/env python3
"""Scouts MBARI's Pacific Sound hydrophone (Monterey Bay, 16 kHz, CC BY 4.0) for humpback song.

    python3 tools/toys/whales/scout.py 2018-12-01 6 14 [minutes-per-probe=2]

Range-reads `minutes` of each hour between the two UTC hours of that day (the daily files are 24-bit
mono, 4 GB; only the probes are fetched) and prints, for each probe, how much of its energy lies in
humpback song's band (150-2000 Hz) against the band below it (ships, wind), and how tonal it is. The
best probe is the one to listen to; recordings.py cuts the clip the toy uses.
"""
import sys
import urllib.request

import numpy as np

BASE = "https://pacific-sound-16khz.s3.amazonaws.com"
RATE, HEADER = 16000, 332


def read(day, start_s, seconds):
    y, m, d = day.split("-")
    url = f"{BASE}/{y}/{m}/MARS-{y}{m}{d}T000000Z-16kHz.wav"
    a = HEADER + int(start_s * RATE) * 3
    b = a + int(seconds * RATE) * 3 - 1
    req = urllib.request.Request(url, headers={"Range": f"bytes={a}-{b}"})
    raw = urllib.request.urlopen(req, timeout=120).read()
    x = np.frombuffer(raw[: len(raw) // 3 * 3], np.uint8).reshape(-1, 3).astype(np.int32)
    v = x[:, 0] | (x[:, 1] << 8) | (x[:, 2] << 16)
    v = np.where(v >= 1 << 23, v - (1 << 24), v)
    return v.astype(np.float32) / (1 << 23)


def score(x):
    n = 4096
    frames = np.lib.stride_tricks.sliding_window_view(x, n)[:: n // 2] * np.hanning(n)
    s = np.abs(np.fft.rfft(frames, axis=1)) ** 2
    f = np.fft.rfftfreq(n, 1 / RATE)
    song = s[:, (f > 150) & (f < 2000)].sum(1)
    low = s[:, (f > 20) & (f < 150)].sum(1)
    ratio = np.median(song / (low + 1e-12))
    band = s[:, (f > 150) & (f < 2000)]
    tonal = np.mean(band.max(1) / (band.mean(1) + 1e-12))
    return ratio, tonal, np.percentile(song, 90) / (np.percentile(song, 10) + 1e-12)


if __name__ == "__main__":
    day, h0, h1 = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    mins = float(sys.argv[4]) if len(sys.argv) > 4 else 2
    for h in range(h0, h1):
        x = read(day, h * 3600, mins * 60)
        r, t, dyn = score(x)
        print(f"{day} {h:02d}:00Z  band {r:7.2f}  tonal {t:6.1f}  swing {dyn:7.1f}", flush=True)
