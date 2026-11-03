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
  - `check` (check.rs): the headless check. A plan of fingers (taps, drags, pinches, holds, raw
    down/move/up, a shake), its moments as PNG, a film of frames, the sound as WAV. Its header is the
    plan format.
  - `ios_shell!(MyToy)`: the device's C ABI (`ios/Sources/toy.h`), one line in the toy's `lib.rs`.
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
- The device keeps the toy's log: a panic's message and place, every GPU error (logged, not fatal: a
  toy drawn wrong beats a toy gone), a toy that could not start, the GPU's limits, and the memory
  available as the toy starts and once it has started (the second line missing: it died starting). It is `toy-log.txt` in the Files app,
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
