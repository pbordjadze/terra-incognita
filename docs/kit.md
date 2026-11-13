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
toy-kit = { git = "https://github.com/pbordjadze/terra-incognita", rev = "85b00e81298a1bd3014e29ca5d9b1cbe80675a25" }   # 0.2.0
# once the tag exists: tag = "toy-kit-v0.2.0"
```

In a claude.ai cloud session cargo fetches the git dependency only with `CARGO_NET_GIT_FETCH_WITH_CLI=true`
(the proxy; found by busquets, Nov 3). busquets depends on it so since b6f3375 (no globe; `web` in its web crate).
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
| A host holding several toys: the `hosted` feature (each toy's own C ABI left out), `ios_shell!(host ..)`, `Ctx::take_haptic` | `lib.rs` | ✓ | Terra, Nov 4 |
| VoiceOver buttons a toy names on screen (`Toy::buttons`, `press`; `toy_buttons`) | `lib.rs`, `toys/ios` | ✓ | Terra, Nov 4 |
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
5. 0.2.0 is main's 85b00e81 (Nov 3). The tag `toy-kit-v0.2.0` is still to be made: this session's git
   access pushes branches, not tags (`git tag -a toy-kit-v0.2.0 85b00e81 && git push origin toy-kit-v0.2.0`,
   by anyone who may). Until then a consumer pins the commit (`rev`), which never moves.

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
  into toy-kit. Done on branch `kit-lib` (toy-kit 0.2.0, untagged): the features, the five modules and
  `main_with`; a consumer outside the repository builds against it (`tools/kit/consumer_check.sh`), and
  every toy's plans rendered the same (Mold within its own run-to-run noise).

## The tools

One line a tool as it lands (moved here from `docs/tools.md` on Nov 3).

| Tool | What it does | Run | Users |
| --- | --- | --- | --- |
| Toy kit | the library every toy stands on (the toy's life, GPU, globe, data, sound, the device's ABI, the headless check) | `toys/crates/toy-kit`; from another repository a git dependency pinned to a `toy-kit-v*` tag | the four toys |
| Kit features | `globe` (default on): the globe and `gpu::Textures`, which name Terra's NASA-derived files; `default-features = false` builds the kit without them. `web`: the WebGPU shell `web::run` (wasm32 only; its dependencies wasm32's alone) | `toy-kit = { git = "https://github.com/pbordjadze/terra-incognita", tag = "toy-kit-v0.2.0", default-features = false, features = ["web"] }` | busquets |
| Kit: paint, backdrop, layer, sound, web | soft 2D shapes in one instanced draw (`Painter`), a full-screen fragment of a toy's own (`Backdrop`; `Backdrop::textured` gives it one picture to read, as Kanto's plate), a texture that keeps what is painted (`Layer`), struck FM notes and a bed (`sound::Bells`), a WebGPU shell; `check::main_with` takes the check's arguments given. Lifted from busquets' copy of the kit (same author, MIT) | `toy_kit::paint`, `backdrop`, `layer`, `sound`, `web` | busquets' takes |
| Kit: field | a grid of four numbers a cell written on the CPU (a height, a sand state), uploaded as Rgba16Float (`field::half`) and drawn by a full-screen fragment of the toy's own, the backdrop's shape with `fld(cell)` beside it | `toy_kit::field::Field::new(gpu, label, [w, h], wgsl, blend)`, `.upload(gpu, &cells)`, `.render(...)` | Bubblers, Barchans |
| Barchans' opening field | the dune rule run from scattered heaps for 600 sweeps, written as `toys/barchans/data/opening.bin` (too long to run at launch) with a hill-shaded preview | `cd toys && cargo run --release -p barchans --example opening -- barchans/data/opening.bin [sweeps]` | Barchans |
| Kit consumer check | a throwaway crate outside this repository built against toy-kit at a git ref (by default this checkout's branch, no globe), touching the public API: proves the git dependency and its workspace inheritance resolve; `--wasm` builds it for wasm32 with `web` | `bash tools/kit/consumer_check.sh [<git-url>] [--branch b \| --tag t] [--globe] [--wasm]` | the kit's tags, before busquets moves |
| WebGPU shot | a WebGPU page screenshotted by headless Chromium on Linux, not blank (the four flags that composite the canvas); fails on no adapter, a page error or a one-colour canvas | `node tools/kit/webgpu_shot.cjs <dir or url> <out.png> [--size 390x844 --dpr 3 --wait 8 --shots 2,4]` | any web shell |
| Web artifact | a web build made publishable as a claude.ai artifact: other binaries wrapped as base64 JSON in `assets/index.json`, the `files` map in batches within the host's limits | `python3 tools/kit/artifact.py <web-dir>` (writes `artifact.json`) | Adrift's web build |
| Launch check | any engine app's launch as the iPhone would take it: the shaders it builds translated to MSL 2.4 by naga; its short launch plan at the phone's 1170x2532 under an Apple7 iPhone's GPU limits (`KIT_IPHONE_LIMITS=1`), a GPU validation error fatal; peak RSS against the app's budget (`memory_mb`, default 1400 MB) with the GPU's peak bytes beside it; with `--ipa`, every manifest file bundled. Can't run Apple's Metal compiler or the phone's memory pressure | `python3 tools/kit/launch_check.py --check '<cmd> {plan} {out} --size {size} --ppp {ppp}' --manifest m.json [--app-json toy.json] [--ipa f]` | the toys (through the wrapper below), then Adrift |
| Simulator launch | a toy opened in the iOS Simulator with Metal's validation on (`MTL_DEBUG_LAYER`): its shaders through Apple's Metal compiler, its every Metal call through Apple's checks; fails if it closes within 70 s; keeps its toy-log.txt, stderr, any crash report and screenshots (at rest, the rules card, the origins card). Can't vouch for the phone's memory limit or GPU | `bash tools/toys/sim_launch.sh <toy> [out]` (macOS; `SIM=1 toys/ios/build.sh` builds it) | `toys.yml`'s IPA job |
| Toy launch check | the launch check above with a toy's check binary, manifest and budget (`toy.json`'s `memory_mb` if set) (same CLI as before) | `bash tools/toys/launch_check.sh <toy> [--ipa f]` | `toys.yml` |
| Launch safety in the kit | `KIT_IPHONE_LIMITS=1`: the device requested as on an iPhone; `KIT_MEMORY_REPORT=<f>`: the GPU's peak live bytes (wgpu's counters) and the bytes made through the kit's helpers; GPU errors fatal headless, logged on the device, with the launch's available memory in `toy-log.txt` | `toys/crates/toy-kit/src/budget.rs` | the launch check |
| Toy check: touches | every finger event in a plan played again without it (a shadow run, same clock); one that changes nothing 0.4 s on prints `WARN: tap at t=… did nothing` (fails with `"strict_taps": true` or `KIT_STRICT_TAPS=1`; `"inert": true` exempts an event); a toy that isn't deterministic is judged near the finger against its own noise | opt-in in `<toy>-check play` (`KIT_TAP_CHECK=1` or `--tap-check`; on in `toys.yml`'s check) | every toy's plans |
| Toy check: places by name | a plan touches `"tap": "name"` (the toy's `Toy::anchors`) or `{"lonlat": [lon, lat]}` (`Toy::project`, `Globe::point`), resolved when the event starts; an unresolved place fails the plan | in any plan | toys that name their places |
| Toy check: play-all | many plans in one process, the GPU made once, on N threads; a line a plan, exit 1 if any failed; each plan's lines in `<out>/<plan>.log` | `<toy>-check play-all <dir or plans…> <out> -j N` | toys' CI, Adrift's later |
| adrift-check play-all | every Adrift plan in one process, N at a time: one device and the assets' bytes shared, each plan its own App, so each plays exactly as `play` would (`tools/adrift/play_all_same.sh` proves it); `<out>/<name>.log` and `<out>/<name>/` a plan, `ok`/`FAIL` a line | `adrift-check play-all <plan-or-dir>... <out-dir> [-j N]` | `tools/adrift/touch-play.sh`, `gpu-runner.sh` |
| Toy check: loudness, stretch | a plan with sound prints its integrated LUFS, RMS and peak; `--stretch` writes `<shot>.stretch.png`, contrast-stretched, to find faint panels and seams | `<toy>-check play … [--stretch]` | the toys' critics |
| Toy clip | a plan rendered at 1080×1920, 30 fps, with its sound, encoded H.264 high yuv420p + AAC 48 kHz, +faststart (ready to post) | `<toy>-check clip <plan> <out.mp4>` (encodes with `bash tools/kit/clip.sh <frames-dir> <out.mp4> [fps]`) | any toy |
| Same renders | every toy's plans played by two sets of check binaries and every PNG and WAV compared byte for byte: a kit change that must not change a look | `bash tools/kit/same_renders.sh render <bin-dir> <out>`, then `compare <a> <b>`; for a toy that differs from itself run to run (Mold), `python3 tools/kit/render_noise.py <a> <b>` against two runs of the base; `bash tools/kit/play_all_speed.sh <toy>` times play-all | kit changes |

## First after the reset: Paul's two asks (Nov 3, 3:45 PM ET)

Paul put every app on one page of his phone. He asked that all the globe apps share Adrift's
two-finger rotate ("that should've been part of the shared engine"), and that each app get a tiny
"info" button that explains what the app is, briefly and visually. Both are deferred: neither could be
proved on a phone for every toy before the 6 PM reset.

**1. One globe camera, Adrift's gestures, for every globe toy** (Whales, Mold, Fireflies, Bell, Swell; Battle
Ink has no globe).
- **What already exists:**
  - toy-kit's `Camera` already has `roll`, and its view and shaders draw it.
  - What's missing is the hand. `globe::Hand` turns and pinches; Adrift's
    `adrift-core/src/touch/camera.rs` (`CameraControl`, 762 lines with tests) also does:
    - a trackball drag that carries the orb over the poles (no north to stop at);
    - a two-finger twist that rolls about the fingers' midpoint and keeps what they hold;
    - a pinch anchored to the fingers;
    - a fling;
    - `fly_to`, which gives Home: the short way back, landing upright.
- **Design:**
  - Lift `CameraControl` into the kit as `globe::Orb`, unchanged apart from its imports, its tests with
    it. `Globe` gets `orb: Orb` beside `Hand`, and a `Globe::gestures(Gestures::Orb | Gestures::Hand)`
    switch, so each toy moves on its own round.
  - Home is a `Globe::home()` the toy calls from whatever it uses for home (Adrift: over-pinch or three
    fingers up). The kit doesn't choose the gesture.
  - Each toy's session moves its own toy. Its plans keep passing: a one-finger drag and a pinch do the
    same as before away from the poles. Its feel is judged on a phone.
  - Adrift moving onto the kit's `Orb`, which would leave one copy instead of two, is Adrift's
    conductor's call.
- **Proof per toy:**
  - its plans, plus a new twist plan (`pinch` with a rotation, or raw `down`/`move` pairs);
  - `same_renders.sh` on its untouched plans;
  - one IPA, and a launch on the phone.

**2. A tiny info button, a kit component.**
- **What it is:**
  - one small mark in a corner, drawn by the kit at reading size, kept clear of the app's touches;
  - a tap opens a few seconds' visual demonstration of the app's one gesture and what it does: a ghost
    finger performing it on the live scene;
  - at most one line of text, then a tap anywhere closes it.
- **How it's built:**
  - Each app supplies `Toy::demo(&self) -> Option<Demo>`, default None, which hides the mark. A `Demo`
    is a short plan of fingers (the check's own event format, so the same JSON is a tested plan too)
    and an optional line.
  - The kit plays the ghost finger and draws the line. VoiceOver reads the line; Reduce Motion shows a
    still with the line.
- **Taste:** Paul asked for this text, so keep it to the one line. Nothing else on screen changes.
- **Proof:** a plan that taps the mark and screenshots the demo (`"tap": "info"` through
  `Toy::anchors`); a phone run for its size and corner.

## Built (Nov 4, 2026): Paul's two asks

Both landed in the kit, not toy by toy (branch `toys-unify`, then main), so a new globe toy has them
by standing on the kit:

- **The globe camera** is `globe::Hand` itself, rewritten with Adrift's gestures' math (not a second
  `Orb` beside it: every globe toy already routed its fingers through `Hand`, so all five moved at
  once, with no switch to forget). Drag over the poles, twist, anchored pinch, fling on Adrift's
  decay, Home by over-pinch or three fingers up, landing upright. Each toy sets `globe.set_home()` and
  passes Reduce Motion; Mold and Fireflies no longer ease the roll away when left alone (a twist
  stays; Home rights it); Fireflies' own anchored pinch is gone (the kit's is the same). Adrift keeps
  its own `CameraControl`: the kit's is the same numbers (`globe::feel` = `touch/motion.rs`) and the
  same functions, lifted, so the two feel alike; moving Adrift onto the kit is still its conductor's
  call. Proof: the kit's tests (over the pole, twist, pinch held, fling, tap within the slop), each
  globe toy's `plans/twist.json`.
- **The info mark** is `info` in the kit, drawn by `Host` over every toy that gives `Toy::demo`
  (all seven), and the shell's one line in system text. Terra Incognita has its own (SwiftUI,
  `InfoDemo` in `PlaygroundRoot.swift`, beside the cloud button while no prototype runs). Adrift has
  none yet (its shell and look are its conductor's). Proof: every toy's `plans/info.json`.
  Since Nov 13 (Paul: "The info should pop up when you first open an experience") it opens by itself
  a second into a toy's first visit (`info::First`; Terra asks for it, `Ctx::show_info`, as each toy
  first opens since launch, and not on its Earth), its line on a small dark glass so it reads on frost
  and paper as on night. A plan sees it only with `"first_info": true`.
- **The rules mark** (Nov 4, painting-company row 214; Paul: "a second button with just a tiny bit
  more detail. Just the rules"): a "?" beside the info mark, `Toy::rules()` in each toy, the shell's
  system text on a dark glass above the marks, closed by a tap anywhere, no timer. Words: the rules
  the real thing follows and how a finger joins in, two to four short lines; the dim "Real: ...
  Drawn: ..." line went on Nov 13 (Paul: "'Real vs Drawn' is a confusing paradigm. It could just
  explain what rules the real world phenomenon follows"). Judged by two fresh critics on screenshots (`plans/rules.json`; the shell's text drawn
  on by `tools/kit/shell_text.py`; the Simulator's `out/sim/rules.png` is the real text).
  Open: the card covers the lower half (Battle's flanks, Swell's wavefront) while it shows; a light
  card on the paper scenes was suggested and not taken (one look for all seven).
- **The origins mark** (Nov 4, painting-company row 219; Paul: "one button should explain the
  rules... a separate button can explain where those rules came from"): an asterisk left of the "?",
  `Toy::origins()` (a `Rules`: the science and history, then a dim "From the science / Made up"
  note), the same card and closing tap; the "?" can then be the rules alone. `Toy::card_at_top(ctx)`,
  asked as a card opens, hangs it from the top (Mold: over whichever half has less lit land). On Mold
  only, as a trial; any toy adopts it by giving `origins()`. Judged by two fresh critics on
  `plans/rules.json` (now "?", then the asterisk); the Simulator keeps `rules.png` and `origins.png`.
- **Launch safety, one step further:** `tools/toys/sim_launch.sh` opens each IPA's toy in the iOS
  Simulator with Metal's validation on (Apple's own Metal compiler and API checks, which Linux can't
  run), in the IPA job; and `toy-log.txt` now has every launch step with the memory left.

**Next (Nov 4):**
1. On the phone: Mold opens and pinches past 2x without going black (the fixed build's
   `toy-log.txt` has no `gpu:` error line); the twist and Home feel right in each globe toy; the
   mark's corner and size (bottom right, 26 x 34 points in) clear the home indicator.
2. Adrift's info mark: not built (its shell is its own; Adrift's conductor's call). Its gestures
   are the kit's reference, so its camera already matches.
3. Battle Ink has no globe: it has the info mark, not the camera.
4. `sim_launch.sh` doesn't zoom or touch: a pipeline that fails only when first drawn would still
   pass it if it were made lazily (every toy makes its pipelines at launch today).

## Where it stands (handoff, Nov 3, 2026, ~1 PM ET)

**Landed.**
- On main: the launch check (0209eb6), the check's new options (5024250) and toy-kit 0.2.0 as a
  library (85b00e81). Every toy's CI and the kit job are green there.
- On the `adrift` branch, reaching main with Adrift's next round: `play-all` (kit-play-all) and
  `--artifact` (kit-artifact).
- No kit branch has work left that isn't merged (`kit-*` can be deleted).

**Who uses what.**
- **The seven toys** (Whales, Murmuration, Mold, Fireflies, Bell, Swell, Battle): toy-kit from the
  path, the launch check and the touch check (`KIT_TAP_CHECK=1`) in CI.
- **busquets:** toy-kit at `rev = "85b00e81298a1bd3014e29ca5d9b1cbe80675a25"`, without the globe and
  with `web` (b6f3375); its own copy of the kit is deleted.
- **Adrift:** `webgpu_shot.cjs`, `artifact.py` through `web-build.sh --artifact`, `adrift-check play-all`.
- **traveling-squadron:** not reached; it can pin the same commit.
- **Mold:** `mold-build-71` writes the GPU's limits and the memory available before and after launch
  to its `toy-log.txt`. Its next launch on a phone shows whether a closure was memory or a GPU error.

**Open, in order.**
1. **The tag.** `git tag -a toy-kit-v0.2.0 85b00e81 && git push origin toy-kit-v0.2.0`, by anyone whose
   access pushes tags (a cloud session's git proxy refuses tags). The pinned commit is the same
   version, so nothing waits on it.
2. **The launch check in `adrift.yml`.** Adrift's conductor agreed to it as a job that gates nothing at
   first; send them the budget numbers before it blocks anything. adrift-core's Gpu needs what
   toy-kit's has: a shader dump (`TOY_DUMP_SHADERS`) and `KIT_IPHONE_LIMITS`. Then
   `tools/kit/launch_check.py --check '<adrift-check play cmd>'` works as is.
3. **`tools/kit/same_renders.sh` keeps every frame of both runs.** It filled the disk once (8.6 GB).
   Give it a mode that keeps only the hashes.
4. **Mold isn't deterministic** (its cells are claimed by racing GPU atomics), so its touches and its
   renders can only be judged against its own noise. A fixed claim order would fix both. That is the
   Mold session's call, because it may change the look.
5. **`play-all` is CPU-bound on Linux's software Vulkan** (1.4–1.6× faster on 4 CPUs). Its gain on the
   homelab GPU is unmeasured, and the GPU runner's switch to it is untested until its next run.
6. **Places by name** (`Toy::anchors` / `Toy::project`): no toy implements them yet; each toy's
   session adds them.

## What the kit can't prove

A Linux check can't run Apple's Metal compiler or the phone's memory pressure. The device's
`toy-log.txt` and a run on the phone stay the last word: a round isn't done until it has opened on one.
