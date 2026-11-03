# toy-kit's changes

One line an addition, newest first. A consumer pins a tag (`toy-kit-v0.<minor>.<patch>`, docs/kit.md):
a patch never changes a public signature; a minor may, and its lines say how to move.

## v0.2.0 (unreleased)

Additive: every toy builds unchanged, and every toy's plans render the same PNGs and WAVs as before
(`tools/kit/same_renders.sh`). For the library half (features, lifted modules) that was checked on
all seven toys against kit-check d6f17ad: 26 plans, 4569 PNGs and WAVs byte-identical; Mold's 4
plans (it races GPU atomics) within its own run-to-run noise by `tools/kit/render_noise.py`.

- **A git dependency** for other repositories, with its own version (`version = "0.2.0"` in its
  Cargo.toml; the toys keep the workspace's 0.1.0). Its other fields still inherit the toys'
  workspace, which cargo resolves inside a git checkout: `tools/kit/consumer_check.sh` builds a
  throwaway crate outside this repository against a ref.
- **The `globe` feature** (default on): `globe` (`Globe`, its passes, `globe::wgsl`) and
  `gpu::Textures` (the globe's NASA-derived files) build only with it. `default-features = false`
  builds the kit without them; `gpu::Texture`, `gpu::textures::decode`, `View`, the geometry in
  `model` and everything else stay.
- **The `web` feature**: `web::run::<T>(canvas_id, key)`, a WebGPU shell for wasm32 (one canvas,
  pointer events, the sound from the first touch, `save`/`restore` through localStorage, a haptic as
  `navigator.vibrate`). Its dependencies (wasm-bindgen `=0.2.130`, web-sys, js-sys,
  wasm-bindgen-futures, console_error_panic_hook) are wasm32's alone, so a native build is the same
  with or without it; on wasm32 wgpu takes `webgpu`. `check` isn't built on wasm32.
- **From busquets' copy of the kit** (pbordjadze/busquets `crates/kit`, written on toy-kit 6bf0613;
  same author, MIT), lifted as they were: `paint` (`Painter`: soft discs, rings, round-capped strokes
  and glows in one instanced draw a blend; `Shape`, `pm`), `backdrop` (`Backdrop`, `BackdropParams`,
  `PRELUDE`: a full-screen fragment of the toy's own over a camera and eight parameters), `layer`
  (`Layer`: a screen-sized texture that keeps what is painted into it, laid over the frame),
  `sound` (`Bells`, `Note`, `pentatonic`: struck FM notes and a low bed), `web` (above) and
  `check::main_with::<T>(name, &args)` (the check's command line from arguments given, without the
  program's name; `check::main` now calls it with the process's). They use the kit as it now is:
  their textures and buffers made through `wgpu` directly aren't counted in `budget::kit_bytes`, only
  in the GPU's total.

**Moving from a vendored copy** (busquets' `crates/kit`): delete the copy and depend on the tag,

```toml
toy-kit = { git = "https://github.com/pbordjadze/terra-incognita", rev = "85b00e81298a1bd3014e29ca5d9b1cbe80675a25", default-features = false }
# 0.2.0 is that commit; once the tag toy-kit-v0.2.0 exists, tag = "toy-kit-v0.2.0" says the same
# and, in the web crate: features = ["web"]
```

`default-features = false` leaves the globe and its data out. The crate is `toy_kit` (`use kit::…`
becomes `use toy_kit::…`, or `kit = { package = "toy-kit", … }` keeps the old name). Everything the
copy had is here under the same paths and signatures; what it lacked and now gets: `gpu::Texture`,
`gpu::View`, `model`'s geometry, `budget` (the iPhone's limits under `KIT_IPHONE_LIMITS=1`, GPU
errors fatal in the check), the check's places by name, touch check, play-all, clip, loudness and
`--stretch`. Two behaviours differ from the 6bf0613 copy: on a surface (the device, the web) a GPU
validation error is logged through `toy_kit::log` instead of wgpu's default panic (headless it is
still fatal), and `Gpu::new` on a surface logs the adapter's limits there too.

- **Touches must do something** (`check.rs`): after a plan is played, every finger event (`tap`,
  `hold`, `drag`, `pinch`, `down` with its moves and up) is played again without it in a shadow run on
  the same clock, and compared 0.4 s after it began and ended. One that changed nothing prints
  `WARN: <kind> at t=… did nothing` and is counted on a `touches:` line; `"strict_taps": true` in a
  plan, `KIT_STRICT_TAPS=1` or `--strict-taps` makes it `EXPECT FAILED` and fails the check; an event
  that is meant to do nothing says `"inert": true`. A control shadow says whether the toy is
  deterministic; one that isn't is judged within 80 points of the finger against its own noise
  (`may have done nothing`). Opt-in (a plan plays 1.5–3× longer): `KIT_TAP_CHECK=1` or `--tap-check`, set in `toys.yml`; `--no-tap-check` turns it off. `check::Options`,
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
