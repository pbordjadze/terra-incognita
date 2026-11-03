# toy-kit's changes

One line an addition, newest first. A consumer pins a tag (`toy-kit-v0.<minor>.<patch>`, docs/kit.md):
a patch never changes a public signature; a minor may, and its lines say how to move.

## v0.2.0 (unreleased)

Additive: every toy builds unchanged, and every toy's plans render the same PNGs and WAVs as before
(`tools/kit/same_renders.sh`).

- **Touches must do something** (`check.rs`): after a plan is played, every finger event (`tap`,
  `hold`, `drag`, `pinch`, `down` with its moves and up) is played again without it in a shadow run on
  the same clock, and compared 0.4 s after it began and ended. One that changed nothing prints
  `WARN: <kind> at t=… did nothing` and is counted on a `touches:` line; `"strict_taps": true` in a
  plan, `KIT_STRICT_TAPS=1` or `--strict-taps` makes it `EXPECT FAILED` and fails the check; an event
  that is meant to do nothing says `"inert": true`. A control shadow says whether the toy is
  deterministic; one that isn't is judged within 80 points of the finger against its own noise
  (`may have done nothing`). On by default; `--no-tap-check` skips it. `check::Options`,
  `check::Report`, `check::play_with`.
- **Places by name** (`check.rs`, `lib.rs`, `globe/`): a plan's points (`tap`, `hold`, `drag`'s ends,
  `pinch`'s fingers, `down`/`move`/`up`) may be a name, `"tap": "whale"`, from the new
  `Toy::anchors(&self) -> Vec<(String, [f64; 2])>` (default: none), or `{"lonlat": [lon, lat]}`,
  through the new `Toy::project(&self, [f64; 2]) -> Option<[f64; 2]>` (default: None; a toy with a
  globe returns `self.globe.point(lonlat)`, the new `Globe::point`). Resolved when the event starts;
  a place that doesn't resolve fails the plan. `check::Place`, `check::Places`, `check::resolve`;
  `Event` gains `inert` and `places`.
- **play-all** (`check.rs`): `<toy>-check play-all <dir or plans…> <out> -j N` plays many plans in
  one process, the GPU made once, on N threads (default: the CPUs); a line a plan (`ok`/`FAIL`), each
  plan's lines in `<out>/<plan>.log`, exit 1 if any failed. `check::play_all`.
- **clip** (`check.rs`, `tools/kit/clip.sh`): `<toy>-check clip <plan> <out.mp4>` renders the plan's
  whole length at 1080×1920, every frame at 30 fps, with its sound, and encodes it H.264 high, yuv420p,
  AAC 48 kHz, +faststart. `check::clip`.
- **Loudness** (`check.rs`): a plan with sound prints `loudness … LUFS integrated, RMS … dBFS, peak …
  dBFS` (ITU-R BS.1770). `check::loudness`.
- **Stretched shots** (`check.rs`): `--stretch` writes `<shot>.stretch.png` beside each shot,
  contrast-stretched (luminance's 0.5th to 99.5th percentile over the whole range), to find faint
  panels and seams. `check::stretch`.
- **Launch safety** (`budget.rs`, `gpu/`, `shell.rs`): `KIT_IPHONE_LIMITS=1` requests the device as an Apple7 iPhone would get
  it (limits from wgpu-hal 26's Metal backend, no feature a phone lacks); a GPU validation error is fatal
  in the headless check (still logged, not fatal, on the device); `KIT_MEMORY_REPORT=<file>` writes the
  GPU's peak bytes; `Gpu::texture` and `Gpu::buffer` count what they make (`budget::kit_bytes`); the
  device's log gains the GPU's limits and the memory available before and after the toy starts. A toy's
  own `device.create_*` still works and is counted only in the GPU's total. `tools/kit/launch_check.py`
  uses it all. No public signature changed.
- The command line: `--stretch`, `--strict-taps`, `--no-tap-check` take no value (the other flags
  still do).

## v0.1.0

The kit as the four toys first stood on it (Nov 2, 2026): the toy's life, the GPU, the globe, the
data, the sound, the device's C ABI and the headless check.
