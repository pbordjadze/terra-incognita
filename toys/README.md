# The toys

Small toys, each its own app beside Terra Incognita and Adrift in the same SideStore source (painting-company
rows 201–204). Each does one core thing, beautifully, on real data. Working titles (the names and icons are
the owner's to decide):

| toy | directory | bundle id | release tag |
| --- | --- | --- | --- |
| Whales: two whales, their songs and their travels in the real oceans | `whales/` | `com.pbordjadze.toy.whales` | `whales-build-<n>` |
| Murmuration: a flock on simple rules over the real sky, and you one of them | `murmuration/` | `com.pbordjadze.toy.murmuration` | `murmuration-build-<n>` |
| Mold: a slime mold on the night Earth finding the routes between cities' lights | `mold/` | `com.pbordjadze.toy.mold` | `mold-build-<n>` |
| Fireflies: every night city a firefly, falling into step | `fireflies/` | `com.pbordjadze.toy.fireflies` | `fireflies-build-<n>` |
| Bell: strike the Earth and it rings in its own measured free oscillations | `bell/` | `com.pbordjadze.toy.bell` | `bell-build-<n>` |
| Swell: waves on the real sea, at the speed its depth gives them | `swell/` | `com.pbordjadze.toy.swell` | `swell-build-<n>` |
| Carol: a toy chess trainer; tap a piece, then where it goes (row 215) | `carol/` | `com.pbordjadze.toy.carol` | `carol-build-<n>` |
| Ghost Jam: 22 cars on the real 230 m ring at Nagoya; tap one and a jam runs backward (row 218) | `jam/` | `com.pbordjadze.toy.jam` | `jam-build-<n>` |
| Meander: a river on the Rio Beni's floodplain that wanders and cuts off oxbow lakes; a finger lays its course (row 218) | `meander/` | `com.pbordjadze.toy.meander` | `meander-build-<n>` |
| Terra Incognita: the toy that holds the others, the whole Earth with a living light for each (row 217) | `terra/` | `com.pbordjadze.terraincognita` | `terra-build-<n>` |

Rust on wgpu, as Adrift is. Nothing is a web view; nothing is fetched at run time; no accounts, no analytics.

## What a toy should be

Something happening that a finger can touch, beautiful at first sight. No words on screen it wasn't asked
for, no chrome, nothing pinned to the glass. Truth underneath (real data, real behaviour, credited in the
toy's `SOURCES.md`), never printed on top. Emergent rather than over-designed: simple rules, a little
randomness, and let it happen. Small and finished-feeling; safe to poke anywhere. One core thing, done well.

## The map

- `crates/toy-kit`: what every toy stands on.
  - `Toy` (lib.rs): `new`, `pointer`, `update` (all simulation, GPU compute too), `draw` (every pixel of
    the target), `synth` (sound: a `Synth` that runs on the audio thread), `save`/`restore`, `shaken`,
    `settings` (dark appearance, Reduce Motion). `Ctx` carries the GPU, the data, the screen and
    `ctx.haptic()`.
  - `Globe` (globe/): the real ground of a month (NASA's Blue Marble in Terra's light key, the sea by
    depth, the month's ice) or by night (`globe.night = 1.0`: the Black Marble's lights), a hand that
    turns and pinches it, `project`/`unproject` between places and points, and the `View` uniform
    (`globe::wgsl(own)` prepends it and the shared helpers to a pass's WGSL; `globe.view_bind()` is
    its group 0). Copied from adrift-core on Nov 2, 2026 so neither breaks the other: change the kit
    here, for every toy, compatibly (every toy builds in CI when the kit changes).
  - The globe's hand is Adrift's gestures (drag over the poles, two-finger twist and anchored pinch,
    fling, Home by over-pinch or three fingers up): a globe toy calls `globe.set_home()` once its
    opening camera is set, and lets its own camera wait while `globe.hand.busy()`.
  - `info`: the info mark. A toy's `demo()` gives its one gesture (strokes, played as real touches
    under a ghost fingertip) and one line; the kit draws the mark bottom right, the shell the line.
    Beside it a "?": a toy's `rules()` gives the few rules that make what is on screen and one line
    of what is real and what is drawn (its SOURCES.md, in a sentence); the shell sets them on a dark
    glass, closed by a tap anywhere. Plain words, no title, nothing to do: a person can glance and go.
    `plans/rules.json` photographs it; `python3 tools/kit/shell_text.py <out>` draws the shell's text
    on the check's shots (the Simulator's `rules.png` is the real one).
  - `paint` (soft 2D discs, rings, strokes, glows), `backdrop` (a full-screen fragment of your own),
    `layer` (a texture that keeps what is painted into it), `sound::Bells` (struck notes) and `web`
    (a WebGPU shell, the `web` feature, wasm32): from busquets' copy of the kit (Nov 3, 2026). The
    globe is the `globe` feature, on by default; other repositories take the kit as a git dependency
    (`CHANGES.md` says how).
  - `check` (check.rs): the headless check. A plan of fingers (taps, drags, pinches, holds, raw
    down/move/up, a shake), its moments as PNG, a film of frames, the sound as WAV. Its header is the
    plan format.
  - `ios_shell!(MyToy)`: the device's C ABI (`ios/Sources/toy.h`), one line in the toy's `lib.rs`.
  - A host that links several toys (Terra) turns on the kit's `hosted` feature, which leaves each toy's own
    C ABI out, and exports its own with `ios_shell!(host MyHost)`; it drives the toys it holds through
    `Toy` as a shell does (`terra/src/rooms.rs`), on its own device and data.
  - `Toy::buttons` / `press`: things on screen VoiceOver should offer as buttons (Terra's lights, its Earth
    mark); the shell reads them through `toy_buttons`. None by default.
- `terra/`: Terra Incognita as a toy (painting-company row 217): the whole Earth with a light for each toy it
  holds (`doors.rs`), the toy opened inside it on a touch (`rooms.rs`). Its data is the held toys' merged
  (`tools/toys/terra/manifest.py`); CI checks it whenever a toy it holds changes. A toy added to Terra is a
  dependency in its `Cargo.toml`, a `Door` and a name in `doors.rs`/`lib.rs`, and its name in the manifest
  tool.
- `<toy>/`: one crate a toy (`src/lib.rs` the toy, `src/bin/check.rs` its check binary), `toy.json` (its
  app name and bundle id), `manifest.json` (the data it loads, by name, from where in this repository;
  its own built data under `<toy>/data/`, never hand-edited, built by a tool in `tools/toys/<toy>/`),
  `icon.png` (1024 px, drawn from code: `tools/toys/icon.py`), `plans/` (played in CI), and the
  toy's own `ROUNDS.md` (one line a round) and `SOURCES.md` (every dataset and recording: what is
  measured, what is drawn, the licence and the credit).
- `ios/`: the one Swift shell for every toy (a CAMetalLayer, touches, haptics, the sound through
  AVAudioEngine in the ambient category, the state kept in the background) and `build.sh <toy>`.
- `tools/toys/`: `assets.py` (a toy's data directory), `icon.py`, `film.sh` (a played film to MP4 and a
  contact sheet), `ci_plan.py` (what CI does for a push), `starter.rs.in` (the first toy file).
- `.github/workflows/toys.yml`: CI (its header). `ci/sidestore_apps.json` lists every app the
  SideStore source carries; the source is regenerated from the releases after every build.

## When it crashes on the phone

A toy isn't done until it opens on a phone; the Linux renders can't prove that. Two things help:

- `tools/toys/launch_check.sh <toy>` (in CI's check; `tools/kit/launch_check.py` does the work): every
  shader the toy builds at launch, translated to Metal as wgpu does on iOS; the launch played at the
  phone's size under an iPhone's GPU limits, any GPU error fatal; peak memory against 1400 MB (or
  `toy.json`'s `memory_mb`, when a toy needs its own); with `--ipa <file>` (in CI's IPA job), every manifest file bundled.
- `tools/toys/sim_launch.sh <toy>` (in CI's IPA job, macOS): the toy opened in the iOS Simulator with
  Metal's validation on, so every pipeline goes through Apple's own Metal compiler; it fails if the toy
  closes or logs a GPU error. It found Mold's launch crash (a varying named `half`, a Metal type).
- The device keeps the toy's log: a panic's message and place, every GPU error (logged, not fatal: a
  toy drawn wrong beats a toy gone), a toy that could not start, the GPU's limits, every launch step
  with the memory still available (the last line is the step it died in), and the first frames. It is `toy-log.txt` in the Files app,
  On My iPhone › the toy's name; share it from there. iOS's own report is in Settings › Privacy &
  Security › Analytics & Improvements › Analytics Data (an entry named after the toy; a `JetsamEvent`
  entry means it was killed for memory).

## Build and check on Linux

    sudo apt-get install -y mesa-vulkan-drivers ffmpeg     # once: Vulkan in software, for wgpu
    python3 tools/toys/assets.py <toy> --link
    cd toys && cargo run --release -p <toy> --bin <toy>-check -- play <toy>/plans/start.json build/<toy>/out
    bash ../tools/toys/film.sh build/<toy>/out              # when the plan has a film

A full-size phone frame (1179×2556) takes about a second on the software driver; add `--size 2064x2752
--ppp 2` for the 13-inch iPad. Look at every render before CI: CI renders the same plans.

## The round

1. Build on your branch (`toy-<toy>`), from `main`. Render and play on Linux until it is right.
2. Prove the device build when the Swift or the GPU's limits matter: a push whose first line carries
   `[toys-ipa <toy>]` builds the IPA on a branch (an artifact, not released).
3. Ship: merge into `main` with a merge commit (no `[toys-ipa]` needed there). The merge builds your
   toy's IPA once, releases it as `<toy>-build-<n>` and regenerates the SideStore source. One IPA a toy
   a round: the five macOS runners are shared with every app, so check on Linux first and don't merge
   twice for one round. Push to `main` only what touches your toy's directory (and, compatibly, the kit:
   a kit change builds every toy's check, and the other toys' sessions must be told).
4. A critic judges the round from full-size renders and played plans against the taste above; the next
   round fixes the weakest point it names.

The repository is public: commit messages say what changed and why, cite "painting-company row 20x" for
the ask, and quote no one. Don't touch `adrift/` (read it freely: `adrift/crates/adrift-core` is where
the kit came from, and its `look/` and `touch/` are worth reading).
