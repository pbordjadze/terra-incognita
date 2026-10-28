"""Does `metal.py twin` notice when a Metal file and its twin part ways? Each layer's shader is changed
the way a session might change one of the two and forget the other (a colour, an opacity, a width, a
pace), and compared with its twin unchanged: every change must come out APART, and the shader as it
is must agree. Slow (headless Chromium, a minute or two) and so not in CI: run it when a layer's
tolerances in layers.py are changed, or a shader's look moves a long way.

    python3 tools/metal/tests/twin_drift.py
"""
import json, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import metal  # noqa: E402
import layers  # noqa: E402

DRIFTS = {
    'ocean': [
        ("the streamlines' blue", 'over(acc, float3(0.20, 0.42, 0.62), 0.22 * core', 'over(acc, float3(0.20, 0.42, 0.66), 0.22 * core'),
        ("the streamlines' strength", '0.22 * core * running * water', '0.19 * core * running * water'),
        ("the sheen's tightness", 'pow(max(dot(n, h), 0.0), 18.0)', 'pow(max(dot(n, h), 0.0), 12.0)'),
        ("a line's width", 'float lineW = 0.75 * uPx * reading', 'float lineW = 0.9 * uPx * reading'),
    ],
    'clouds': [
        ("a layer's opacity", '0.0, 2.8, 5.2, 0.004, 0.14, 4.0, 0.80,', '0.0, 2.8, 5.2, 0.004, 0.14, 4.0, 0.76,'),
        ("a layer's lit colour", 'float3(0.99, 0.988, 0.98)', 'float3(0.98, 0.988, 0.98)'),
        ('the westerlies', 'float2(-0.10, 0.19)', 'float2(-0.10, 0.21)'),
        ("the shadows' strength", 'float strength = U.tone.x * mix', 'float strength = 0.8 * U.tone.x * mix'),
    ],
    'globe': [
        ('the open sea', 'G_OPEN = float3(0xC2, 0xE4, 0xF4)', 'G_OPEN = float3(0xC2, 0xE4, 0xF8)'),
        ("the coast's width", '(0.55 - 0.35 * pale) * stroke(d, 0.6 * scale', '(0.55 - 0.35 * pale) * stroke(d, 0.9 * scale'),
        ("the limb's air", 'g = mix(g, G_ATMOS, 0.55 * pow(1 - z, 4.0));', 'g = mix(g, G_ATMOS, 0.40 * pow(1 - z, 4.0));'),
        ("the land's relief", 'gland = groundTex.sample(linearS, uv, gradient2d(dx, dy)).rgb * (1 + 0.20 * hill);',
         'gland = groundTex.sample(linearS, uv, gradient2d(dx, dy)).rgb * (1 + 0.25 * hill);'),
    ],
}
# Where the comparison's sight ends, kept to say so: the land's relief made stronger lightens the lit
# slopes and darkens the shaded as much (no lean), and the twin's derivatives already set lines a
# little apart; one cloud layer's light a hundredth redder is under the clouds' own noise, which a
# twentieth of a pixel stirs. The picture shows both.
MISSED = {"the land's relief", "a layer's lit colour"}


class Options:
    device, style, month, days, time, scale, lean, apart = 'phone', 'ground', 10, 0.4, 12.0, None, None, None

    def __init__(self, layer, at):
        self.layer, self.at = layer, at


VIEWS = ('earth', 'descent')                            # (a drift counts as caught at either)


def verdict(layer_name, shader, at):
    """The twin comparison's verdict (0 agree, 1 apart) with the layer's shader taken from `shader`."""
    layer = layers.LAYERS[layer_name]
    original = layer.shader
    layer.shader = shader
    try:
        with open('/dev/null', 'w') as quiet:
            stdout, sys.stdout = sys.stdout, quiet
            try:
                return layers.cmd_twin(Options(layer_name, at))
            finally:
                sys.stdout = stdout
    finally:
        layer.shader = original


def main():
    failures = 0
    for name, drifts in DRIFTS.items():
        layer = layers.LAYERS[name]
        src = Path(layer.shader).read_text()
        ok = all(verdict(name, layer.shader, at) == 0 for at in VIEWS)
        print(f'{name}: as it is, {"agrees" if ok else "APART"} ({" and ".join(VIEWS)})')
        failures += not ok
        for what, old, new in drifts:
            if old not in src:
                print(f'  {what}: the shader has changed; update this drift')
                failures += 1
                continue
            with tempfile.TemporaryDirectory() as d:
                p = Path(d) / Path(layer.shader).name
                p.write_text(src.replace(old, new, 1))
                caught = any(verdict(name, p, at) == 1 for at in VIEWS)
            known = what in MISSED
            print(f'  {what} changed in the Metal alone: {"caught" if caught else "missed, as known" if known else "MISSED"}')
            failures += not caught and not known
    print('every drift caught' if not failures else f'{failures} not as they should be')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
