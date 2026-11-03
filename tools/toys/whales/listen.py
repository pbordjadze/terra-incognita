#!/usr/bin/env python3
"""Looks at a played plan's sound: its spectrogram as a PNG and its loudness every half second.

    pip install numpy scipy matplotlib
    python3 tools/toys/whales/listen.py toys/build/whales/out/touch/audio.wav [out.png] [--top 4000]

The check (whales-check play) writes audio.wav when a plan has "audio": true. Read the PNG to see
the songs (the humpback's harmonic sweeps, the blue whale's low moan), dropouts (a dark column),
images or tones (a flat line that is neither whale), and how a touch changes the mix; the printed
levels (dBFS, both channels averaged) say whether a tap went quiet, and by how much the land's
silence holds. Used through rounds 1-3 to hear the toy without a speaker.
"""
import sys
import numpy as np
from scipy.io import wavfile
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        sys.exit(__doc__)
    top = 4000
    if "--top" in sys.argv:
        top = float(sys.argv[sys.argv.index("--top") + 1])
        args = [a for a in args if a != str(sys.argv[sys.argv.index("--top") + 1])]
    wav = args[0]
    out = args[1] if len(args) > 1 else wav.rsplit(".", 1)[0] + "-spectrogram.png"
    rate, x = wavfile.read(wav)
    x = x.astype(float) / 32768.0
    m = x.mean(1) if x.ndim == 2 else x
    levels = []
    for a in np.arange(0.0, len(m) / rate, 0.5):
        seg = m[int(a * rate) : int((a + 0.5) * rate)]
        levels.append(f"{a:.1f}:{20 * np.log10(np.sqrt((seg ** 2).mean()) + 1e-9):.0f}")
    print("dBFS every 0.5 s  " + " ".join(levels))
    print(f"peak {20 * np.log10(np.abs(x).max() + 1e-9):.1f} dBFS")
    plt.figure(figsize=(16, 4))
    plt.specgram(m + 1e-9, NFFT=2048, Fs=rate, noverlap=1536, cmap="magma", vmin=-140)
    plt.ylim(0, top)
    plt.xlabel("seconds")
    plt.ylabel("Hz")
    plt.tight_layout()
    plt.savefig(out, dpi=60)
    print(f"spectrogram {out}")


if __name__ == "__main__":
    main()
