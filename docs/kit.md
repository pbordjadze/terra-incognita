# The kit and the shared tooling (the seam)

Published Nov 3, 2026, before any of it is built, so every session that uses the engine can see what is
coming, object, and build against it. The conductor of this work owns `tools/kit/` and this file; the
toys' conductor owns `toys/`, Adrift's owns `adrift/` and `tools/adrift/`. A change in someone else's
directory is agreed with its owner first and lands on their say (their merge, or ours with their
yes). Nothing here changes how any app looks: every migration is checked by rendering the app's plans
before and after and comparing the PNGs.

## 1. The toy kit: one library, other repositories included

`toys/crates/toy-kit` is the library. It stays where it is (moving it would break four toys mid-round)
and becomes consumable from any repository as a git dependency:

```toml
toy-kit = { git = "https://github.com/pbordjadze/terra-incognita", tag = "toy-kit-v0.2.0" }
```

Cargo finds the crate by name anywhere in the repository; the tag is the stable API. Rules:

- **Tags, not branches.** `toy-kit-v0.<minor>.<patch>`: a patch never changes a public signature; a
  minor may, and its line in `toys/crates/toy-kit/CHANGES.md` says how to move. Consumers pin a tag.
- **What is public** is what `lib.rs` re-exports; everything else is `#[doc(hidden)]` or private.
- **No repository paths baked in.** Data comes through `Assets` (a directory, the app bundle, or the
  web's index); a consumer brings its own `manifest.json`.

What the kit carries (✓ exists today, + to be added, each extracted from where it already works):

| Part | Module | Today | From |
| --- | --- | --- | --- |
| The toy's life: new, pointer, update, draw, save/restore, shaken, settings | `lib.rs` (`Toy`, `Ctx`, `Host`) | ✓ | |
| The GPU, textures, the view uniform | `gpu/` | ✓ | |
| The globe by day and night, its hand (drag, pinch, fling), project/unproject | `globe/` | ✓ | adrift-core, Nov 2 |
| Real data: a manifest, files by name, the bundle or a directory | `assets.rs` | ✓ | |
| Sound: a `Synth` on the audio thread | `lib.rs` | ✓ | |
| The device's C ABI, `ios_shell!` | `shell.rs` | ✓ | |
| The headless check (plans, shots, films, WAV) | `check.rs` | ✓ | |
| **Launch safety**: the iPhone's GPU limits requested on Linux (`KIT_IPHONE_LIMITS=1`), so a pipeline the phone refuses fails the check; GPU memory counted as it is allocated; a budget in `toy.json` | `gpu/`, `budget.rs` | + | the two launch crashes |
| **Taps that must do something**: every finger event in a plan must change the frame unless it says `"inert": true` | `check.rs` | + | "plans pass when a tap does nothing" |
| **Places by name**: a plan taps `"at": "name"` or `"lonlat": [lon, lat]`, the toy names its places (`Toy::anchors`) | `check.rs`, `lib.rs` (default: none) | + | "plans hard-code positions" |
| **Many plans, one process**: `check play-all <dir> -j N` (the GPU and data loaded once) | `check.rs` | + | the 51-launch suite |
| **A clip**: `check clip <plan> out.mp4`: 1080×1920, 30 fps, H.264 yuv420p + AAC, the plan's sound mixed in, ready for TikTok | `check.rs` + `tools/kit/clip.sh` | + | `tools/adrift/tiktok.sh` |

Not in the kit (yet): Adrift's sea, journeys, studio and conducting stay in adrift-core. Adrift moving
its globe onto the kit is Adrift's conductor's call, offered, not scheduled.

## 2. Shared tooling (`tools/kit/`, any app)

| Tool | What it does | First users |
| --- | --- | --- |
| `launch_check.py` | one launch check for every engine app's CI: shaders to MSL 2.4 by naga (as wgpu does on iOS); the check run under the iPhone's limits; peak memory (RSS and the GPU's) against the budget; with `--ipa`, every manifest file in the bundle | the four toys (replacing `tools/toys/launch_check.sh`, which stays as a one-line wrapper), then Adrift |
| `artifact.py` | a web build directory into what the claude.ai Artifact tool publishes: the files map, an `index.json` of the data for hosts that serve no binaries, base64 only where a host needs it, the size limits checked | `web-build.sh --artifact` (Adrift), any toy's web build |
| `webgpu_shot.cjs` | a screenshot of a WebGPU page in headless Chromium that is not blank: the flags and adapter that give headless Chromium a working WebGPU on Linux (SwiftShader/Vulkan), waiting for real frames, failing on a blank canvas | Adrift's web build; any web shell |
| `play_all` (in `check.rs`) | see above; `adrift-check` gets the same `play-all -j N` | Adrift's suite, every toy's CI |
| GPU runner summary | `tools/adrift/gpu-runner.sh` reports the adapter wgpu actually used, its kind (discrete, integrated, software) and driver | the homelab runner |

Each lands with its line in `docs/tools.md` (› The engine's apps).

## Order

1. This file (the seam). Messages to the toys', Adrift's and busquets' conductors.
2. New files only, nobody's toes: `tools/kit/webgpu_shot.cjs`, `tools/kit/artifact.py`, `tools/kit/launch_check.py`.
3. The kit's additions, on branch `kit-tooling`, every toy's plans rendered before and after (identical
   PNGs), merged when the toys' conductor says the toys are between rounds.
4. Adrift's side (`play-all`, `--artifact`, the runner's summary, the launch check in `adrift.yml`) on
   Adrift's conductor's yes.
5. The tag `toy-kit-v0.2.0`; busquets and traveling-squadron told how to depend on it.

## Answers (Nov 3)

- **Toys' conductor:** yes, on two conditions. The did-nothing tap check warns by default and fails only
  when a plan opts in. `launch_check.sh` keeps its interface. `toys/ios` stays theirs. Their first wants
  are the iPhone's limits, GPU bytes and the budget (Mold still closes at launch on the phone), plus a
  loudness line and contrast-stretched shots. Merge only when all four toys are green with identical renders.
- **Adrift's conductor:** `play-all` (built from `origin/adrift`, with `play`'s lines unchanged; they
  merge it), `--artifact` and the launch check as a job that doesn't block anything at first: yes. The GPU
  runner was already fixed on main. Nothing is merged into `adrift` or `main` from 06:45 to 07:30 UTC.
- **busquets:** vendors a trimmed copy of the kit today. It will switch to a tagged git dependency once
  the globe is behind a cargo feature (`globe`, default on) and the kit takes its `paint`, `backdrop`,
  `layer`, `sound`, `web` (a generic WebGPU shell) and `check::main_with`, all of which are to be lifted
  into toy-kit.

## The tools

One line a tool as it lands (moved here from `docs/tools.md` on Nov 3).

| Tool | What it does | Run | Users |
| --- | --- | --- | --- |
| Toy kit | the library every toy stands on (the toy's life, GPU, globe, data, sound, the device's ABI, the headless check) | `toys/crates/toy-kit`; from another repository a git dependency pinned to a `toy-kit-v*` tag | the four toys |
| WebGPU shot | a WebGPU page screenshotted by headless Chromium on Linux, not blank (the four flags that composite the canvas); fails on no adapter, a page error or a one-colour canvas | `node tools/kit/webgpu_shot.cjs <dir or url> <out.png> [--size 390x844 --dpr 3 --wait 8 --shots 2,4]` | any web shell |
| Web artifact | a web build made publishable as a claude.ai artifact: other binaries wrapped as base64 JSON in `assets/index.json`, the `files` map in batches within the host's limits | `python3 tools/kit/artifact.py <web-dir>` (writes `artifact.json`) | Adrift's web build |
| Launch check | any engine app's launch as the iPhone would take it: the shaders it builds translated to MSL 2.4 by naga; its short launch plan at the phone's 1170x2532 under an Apple7 iPhone's GPU limits (`KIT_IPHONE_LIMITS=1`), a GPU validation error fatal; peak RSS against the app's budget (`memory_mb`, default 1400 MB) with the GPU's peak bytes beside it; with `--ipa`, every manifest file bundled. Can't run Apple's Metal compiler or the phone's memory pressure | `python3 tools/kit/launch_check.py --check '<cmd> {plan} {out} --size {size} --ppp {ppp}' --manifest m.json [--app-json toy.json] [--ipa f]` | the toys (through the wrapper below), then Adrift |
| Toy launch check | the launch check above with a toy's check binary, manifest and budget (`toy.json`'s `memory_mb` if set) (same CLI as before) | `bash tools/toys/launch_check.sh <toy> [--ipa f]` | `toys.yml` |
| Launch safety in the kit | `KIT_IPHONE_LIMITS=1`: the device requested as on an iPhone; `KIT_MEMORY_REPORT=<f>`: the GPU's peak live bytes (wgpu's counters) and the bytes made through the kit's helpers; GPU errors fatal headless, logged on the device, with the launch's available memory in `toy-log.txt` | `toys/crates/toy-kit/src/budget.rs` | the launch check |
| Toy check: touches | every finger event in a plan played again without it (a shadow run, same clock); one that changes nothing 0.4 s on prints `WARN: tap at t=… did nothing` (fails with `"strict_taps": true` or `KIT_STRICT_TAPS=1`; `"inert": true` exempts an event); a toy that isn't deterministic is judged near the finger against its own noise | opt-in in `<toy>-check play` (`KIT_TAP_CHECK=1` or `--tap-check`; on in `toys.yml`'s check) | every toy's plans |
| Toy check: places by name | a plan touches `"tap": "name"` (the toy's `Toy::anchors`) or `{"lonlat": [lon, lat]}` (`Toy::project`, `Globe::point`), resolved when the event starts; an unresolved place fails the plan | in any plan | toys that name their places |
| Toy check: play-all | many plans in one process, the GPU made once, on N threads; a line a plan, exit 1 if any failed; each plan's lines in `<out>/<plan>.log` | `<toy>-check play-all <dir or plans…> <out> -j N` | toys' CI, Adrift's later |
| Toy check: loudness, stretch | a plan with sound prints its integrated LUFS, RMS and peak; `--stretch` writes `<shot>.stretch.png`, contrast-stretched, to find faint panels and seams | `<toy>-check play … [--stretch]` | the toys' critics |
| Toy clip | a plan rendered at 1080×1920, 30 fps, with its sound, encoded H.264 high yuv420p + AAC 48 kHz, +faststart (ready to post) | `<toy>-check clip <plan> <out.mp4>` (encodes with `bash tools/kit/clip.sh <frames-dir> <out.mp4> [fps]`) | any toy |
| Same renders | every toy's plans played by two sets of check binaries and every PNG and WAV compared byte for byte: a kit change that must not change a look | `bash tools/kit/same_renders.sh render <bin-dir> <out>`, then `compare <a> <b>`; for a toy that differs from itself run to run (Mold), `python3 tools/kit/render_noise.py <a> <b>` against two runs of the base; `bash tools/kit/play_all_speed.sh <toy>` times play-all | kit changes |

## What the kit can't prove

A Linux check can't run Apple's Metal compiler or the phone's memory pressure. The device's
`toy-log.txt` and a run on the phone stay the last word: a round isn't done until it has opened on one.
