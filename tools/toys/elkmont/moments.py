"""Picks a played film's telling moments for a critic: the brightest frame of each burst and the darkest
between, from the frames' mean light (a burst's flashes last a fifth of a second, so a plan's fixed shot
times miss them whenever the swarm's timing moves).

    python3 tools/toys/elkmont/moments.py <out-dir> [from-seconds] [every-seconds]

<out-dir> is the check's (with film/NNNN.png); from and every are the plan's film settings (default 70 and
1/30). Copies the picks to <out-dir>/moments/ as <t>-<kind>.png and prints them.
"""

import shutil
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def main() -> None:
    out = Path(sys.argv[1])
    start = float(sys.argv[2]) if len(sys.argv) > 2 else 70.0
    every = float(sys.argv[3]) if len(sys.argv) > 3 else 1 / 30
    frames = sorted((out / "film").glob("*.png"))
    light = np.array([np.asarray(Image.open(f).convert("L").resize((90, 195))).mean() for f in frames])
    base = np.median(light)
    picks = {}
    # bursts: runs of frames well above the median; the brightest frame of each
    hot = light > base + 4.0
    i = 0
    while i < len(frames):
        if hot[i]:
            j = i
            while j < len(frames) and hot[j]:
                j += 1
            k = i + int(np.argmax(light[i:j]))
            picks[k] = "burst"
            i = j
        else:
            i += 1
    picks[int(np.argmin(light))] = "dark"
    dest = out / "moments"
    dest.mkdir(exist_ok=True)
    for k, kind in sorted(picks.items()):
        t = start + k * every
        name = f"{t:06.2f}-{kind}.png"
        shutil.copy(frames[k], dest / name)
        print(dest / name, f"light {light[k]:.1f}")


if __name__ == "__main__":
    main()
