"""How much each switch changes its toy: plays a toy twice from the same start, once with every rule on and
once with rule i switched off, and prints the mean difference between the two runs' frames 1, 2, 4 and 8
seconds after the switch (0-255, the whole frame). A switch that barely moves these numbers does nothing a
reader could see, and doesn't belong on the switches panel (the switches mark, docs/kit.md).

    python3 tools/toys/switch_ab.py <toy> <rule count> [--at T] [--size 390x844] [--then '[events]']

Run from the repository's root after `cargo build --release` in toys/ and `tools/toys/assets.py <toy>`.
"""

import argparse
import json
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

AFTER = [1, 2, 4, 8]


def play(toy: str, plan: dict, out: Path, size: str) -> None:
    p = out.with_suffix(".json")
    p.write_text(json.dumps(plan))
    exe = Path("toys/target/release") / f"{toy}-check"
    subprocess.run(
        [str(exe), "play", str(p), str(out), "--size", size, "--ppp", "1", "--no-tap-check"],
        check=True,
        capture_output=True,
        cwd=".",
    )


def diff(a: Path, b: Path) -> float:
    x, y = Image.open(a).convert("RGB"), Image.open(b).convert("RGB")
    return sum(ImageStat.Stat(ImageChops.difference(x, y)).mean) / 3


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("toy")
    ap.add_argument("rules", type=int)
    ap.add_argument("--at", type=float, default=2.0)
    ap.add_argument("--size", default="390x844")
    ap.add_argument("--keep", help="a folder to keep the runs' frames in, and a sheet of them")
    ap.add_argument("--then", default="[]", help="events after the switch, JSON (a touch, say)")
    a = ap.parse_args()
    shots = [{"t": a.at + s, "name": f"s{s}"} for s in AFTER]
    base = {"seconds": a.at + max(AFTER) + 0.5, "fps": 20, "shots": shots}
    with tempfile.TemporaryDirectory() as d:
        root = Path(a.keep or d)
        root.mkdir(parents=True, exist_ok=True)
        open_only = [{"t": a.at - 0.4, "tap": "switches"}]
        then = json.loads(a.then)
        play(a.toy, {**base, "events": open_only + then}, root / "on", a.size)
        for i in range(a.rules):
            ev = open_only + [{"t": a.at, "switch": [i, False]}] + then
            play(a.toy, {**base, "events": ev}, root / f"off{i}", a.size)
            assert (root / "on" / "s1.png").exists(), "no shots"
            row = [diff(root / "on" / f"s{s}.png", root / f"off{i}" / f"s{s}.png") for s in AFTER]
            print(a.toy, i, " ".join(f"{v:5.1f}" for v in row))
        if a.keep:
            sheet(root, a.rules)


def sheet(root: Path, rules: int) -> None:
    """Every run's frames side by side: a row a run (all on, then each rule off), a column a moment."""
    runs = ["on"] + [f"off{i}" for i in range(rules)]
    first = Image.open(root / "on" / "s1.png")
    w, h = first.size[0] // 2, first.size[1] // 2
    out = Image.new("RGB", (w * len(AFTER), h * len(runs)))
    for r, run in enumerate(runs):
        for c, s in enumerate(AFTER):
            out.paste(Image.open(root / run / f"s{s}.png").convert("RGB").resize((w, h)), (c * w, r * h))
    out.save(root / "sheet.png")


if __name__ == "__main__":
    main()
