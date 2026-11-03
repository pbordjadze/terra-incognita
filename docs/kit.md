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

## What the kit can't prove

A Linux check can't run Apple's Metal compiler or the phone's memory pressure. The device's
`toy-log.txt` and a run on the phone stay the last word: a round isn't done until it has opened on one.
