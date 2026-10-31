# Terra's tools

Every tool this repository has, as `main` has it (October 31, 2026; painting-company rows 178 and 195):
what it does, where it lives, who uses it, and where it stops. Read it before you build; add to it when you do.

Every experience was retired on October 31, 2026 (painting-company row 195). The tools stay; one whose
only users were retired says **no current users**, and what it was made for is at
`before-retirement-2026-10-31` (the packs, the camelid, the films, Wikipedia's words).

**The rules**

1. **Picture your experience first.** Write down what the reader should see, touch and feel before
   reading this page, so what exists doesn't shrink what you picture.
2. **A tool that falls short is the tool's bug.** Fix it, extend it, or build a new one beside it
   (CLAUDE.md › The runtime). Never settle for less than you pictured because a tool can't do it yet.
3. **Keep this page true in the same commit.** A commit that adds a tool adds its line; a commit
   that starts using one adds your experience to its *used by*; a commit that changes a limit
   changes it here. Two tools for one job is a bug too: look under [One job, two tools](#one-job-two-tools)
   before you write a third.

*Used by* is read from the code, not from intentions. "Packs" are the folders in `Prototypes/`
(`hello-pack`, the delivery check, parked, is the only one now); "natives" are the entries in
`App/TerraIncognita/Playground/Catalog.swift` and `Quiet/QuietGlobe.swift`.

- [What a pack calls: the runtime](#what-a-pack-calls-the-runtime)
- [What a native prototype calls: the stage](#what-a-native-prototype-calls-the-stage)
- [The stage's own layers](#the-stages-own-layers)
- [Art: pictures drawn from code](#art-pictures-drawn-from-code)
- [Data: builders from sources](#data-builders-from-sources)
- [Seeing it without a Mac: previews](#seeing-it-without-a-mac-previews)
- [Checks and tests](#checks-and-tests)
- [CI and delivery](#ci-and-delivery)
- [One job, two tools](#one-job-two-tools)

## What a pack calls: the runtime

The contract with examples is [`runtime3.md`](runtime3.md); the script side is
`Prototypes/_runtime/terra.js`, the native side `App/TerraIncognita/Playground/` (`PackScript.swift`,
`Prototype.swift`) and `Globe/` (`GeoLayers.swift`, `GlobeImages.swift`, `GlobeMarker.swift`), the
preview's mirror `tools/playground/harness/`.

| Tool | What it does | Call | Used by | Limits |
| --- | --- | --- | --- | --- |
| Geography | GeoJSON drawn by the globe, constant width at any zoom | `terra.geo.set(id, features, { style })` | hello (`ring`) | 100,000 vertices, 64 layers, 400 places a layer |
| ↳ `border` | lines; `lineWidth` in reading points | `style: 'border'` | no current users | |
| ↳ `area` | filled shapes | `style: 'area'` | no current users | |
| ↳ `dot`, `ring` | points | `style: 'dot'` / `'ring'` | hello (ring) | |
| ↳ `label` | names on a halo, earlier wins room | `style: 'label'` | no current users | |
| ↳ `symbol` | a pack's picture at a point | `style: 'symbol'` | no current users | |
| ↳ `sounding` | a line hanging from a point, with a label | `style: 'sounding'` | no current users | made for one pack |
| ↳ `ghost`, `track` | dashed outline, fine dashes | `style: 'ghost'` / `'track'` | no current users | |
| ↳ touchable | features take touches (points 22 pt, lines 14, areas inside); a named place is a VoiceOver button | `touchable: true` | no current users | |
| ↳ `flow` | light running along lines the way their coordinates go, moved by Core Animation | `flow: true` or `{ speed, dash, gap, color }` | no current users | a few thousand vertices a flowing layer (drawn every frame); none under Reduce Motion |
| Pictures on the globe | a picture in lon/lat bounds, curving with the globe, faded by zoom | `terra.images.set(id, { image, bounds, opacity, minZoom, fade })` | no current users | 8 pictures, 4096 px a side; equirectangular only |
| Sprites | things that live and move: `at`, or a timed `path` (`seconds`, `loop`) | `terra.sprites.set(id, { at \| path, looks, label, touchable })` | no current users | 64 sprites, 24 frames a look, 400 pt a side (200 before runtime 5); one file a frame (no sheets) |
| ↳ looks by zoom | one thing a trace from afar and itself close in; `every`/`for` a glint now and then; `face` mirrors heading west | `looks: [{ image \| frames, fps, size, minZoom, maxZoom, every, for, face }]` | no current users | |
| ↳ dated way | the globe's date puts the sprite on its way; it walks as the date plays, stands where the reader drags | `path: { coordinates, dates }` | no current users | runtime 5; not drawn outside its dates |
| Doorway | a pack's life on the globe while it isn't running; a touch starts it | `prototype.json` › `doorway` (`look` or `looks`, `path`, `breathes`) | no current users (hello is parked: no doorway) | the same budgets as a sprite's look |
| Glint | the one mark every doorway shows from afar | `doorway-glint.png`, from `tools/playground/glint_png.py` | no current users | the natives draw theirs in Swift (`Catalog.swift`'s `Glint`) from the same recipe |
| Touch | taps and drags on the pack's things, or every tap with `"taps": true` | `terra.on('touch' \| 'tap', fn)` | hello (`tap`) | no long press; a drag's `moved` keeps the target it began on |
| ↳ schemes | who gets a one-finger drag | `terra.touch.scheme('touchables' \| 'one-finger' \| 'hold')` | `touchables` (default): hello; `hold`, `one-finger`: no current users | |
| The date | the date at the foot: `year` or `day` (a day is the globe's season), ticks, play, `carry` | `terra.time.show({...})`, `.set`, `.play`, `.hide`, `terra.on('time')` | no current users | |
| ↳ pace, speed | an uneven pace (deep time fast); playback slower as the reader comes close | `pace: [[date, seconds], …]`, `terra.time.speed(f)` | no current users | runtime 5; 0.001–100× |
| Caption | the words at the foot | `terra.caption.set({ title, text, wikipedia })` | hello | |
| Panel | longer words and sources in a native sheet | `terra.panel.show({ title, text, sections, sources })`, `terra.on('panel')` | hello | plain text only |
| Wikipedia's words | retired (row 195): the keys are accepted and ignored, `terra.settings.wikipedia` is always false | `wikipedia: 'Title'` on caption, panel, sections | no current users | |
| The globe | hide today's borders and names; stop the idle turn; zoom deeper than 8 | `terra.globe.set({ historical, still, maxZoom })` | no current users | `maxZoom` up to 32 |
| Camera | the camera now and as it moves; screen ↔ globe; fly | `terra.camera`, `terra.on('camera')`, `terra.project`, `terra.unproject`, `terra.flyTo` | no current users | `camera` at most 20 times a second; a flight is dropped while the reader's hands are on the globe |
| Many at once | in front or behind; how the reader came in | `terra.focus`, `terra.on('focus')`, `terra.entry` | no current users | 6 experiences at once |
| Settings | Reduce Motion, text size | `terra.settings`, `terra.on('settings')` | hello | |
| Files | a pack file as text or JSON | `terra.read`, `terra.readJSON` | no current users | nothing outside the pack |
| Timers, frames | browser timers; `requestAnimationFrame` for a short eased change | `setTimeout`, `requestAnimationFrame` | hello (timers); rAF: no current users | never to animate the globe; JavaScriptCore runs without a JIT: prepare data at build time |
| Open, close | the chip's Open; the end | `terra.on('open' \| 'close')`, `terra.close()` | open: hello; close: no current users | |

**Shared script: none yet.** The retired packs repeated small helpers instead (a great-circle
`along(a, b, t)` seven times, a `frames(name, n)` file-name list six): the first candidates for a
shared script module when packs come back. See [One job, two tools](#one-job-two-tools).

## What a native prototype calls: the stage

A native prototype (`App/TerraIncognita/<Name>/`, one entry in `Catalog.swift`) asks its
`PrototypeStage` (`Playground/Prototype.swift`) for what it changes; the stage gives it all back.

| Tool | What it does | Call | Used by | Limits |
| --- | --- | --- | --- | --- |
| Style | the globe's style while it runs (crossfades) | `stage.style = .night` … | night (`.persian`, `.wikipedia`: no current users) | |
| Marker | a native picture on the globe (`GlobeMarker`: paths, frames, a look per zoom; the sprites' own drawing) | `stage.place(GlobeMarker(...))` | no current users | |
| Lens | a native experience over the one globe: its canvas over the names, its authored camera, chrome over the playground; `GlobeLease` puts back hooks, modes, camera, chosen country, VoiceOver's words, gestures | `stage.openLens { globe, close in ... }` | no current users | the other experiences' things hide while it is open |
| Screen | a full screen of its own | `stage.present(id) { ... }` | no current users | only for an experience with a map of its own |
| Quiet rules | names on demand, clouds that clear where you look, nothing at the foot until a choice | `stage.quietNames()`, `.quietClouds()`, `.quietFrame()` | quiet-names, quiet-clouds, quiet-frame | |
| Still sea | hides the ocean layer | `stage.stillSea()` | no current users | |
| Veils | whether the clouds and sea thin while it runs | `veils: false` in its entry | the four quiet rules | |
| Words, flights, stillness | the foot, a flight, no idle turn: what packs reach through `terra.*` | `stage.describe`, `.fly(to:)`, `.holdStill()` | the pack host (`Playground.swift`) | |

## The stage's own layers

Built in, owned by no experience; every experience sits on them.

| Tool | What it does | Where | Used by | Limits |
| --- | --- | --- | --- | --- |
| The globe | the whole Earth drawn per pixel from distance fields; the ground and night (the Wikipedia and Persian styles remain, chosen by nothing); names placed by the design rules | `Globe/Globe.metal`, `GlobeView.swift`, `GlobeScreen.swift`, spec `Globe/type.json` | everything | change the shader and `tools/globe/preview.py` together |
| Clouds | the month's clouds where satellites usually see them | `Clouds/` (`CloudsLayer`, `Clouds.metal`), switch `CloudsSwitch` | the stage; quiet-clouds; veiled under experiences | twin `tools/clouds/web/shader.js`, line for line |
| Ocean | the month's surface currents as streamlines, glitter and surf; a tap rings the water | `Ocean/` (`OceanLayer`, `Ocean.metal`) | the stage; hidden by `stillSea()` and the Persian page | twin `tools/ocean/web/shader.js`, line for line |
| Packs | finds bundled and remote packs, checks hashes, runs their scripts in JavaScriptCore | `PackStore.swift`, `PackScript.swift`, `PackPanel.swift`, `GlobeDate.swift` | hello | HTTPS GETs only while the workbench is open or a pack opens |
| Live counts | stages, markers, globes, films and scripts, to prove an ending leaves nothing | `LiveCount.swift`, the workbench | every UI test that ends a prototype | |

## Art: pictures drawn from code

A pack's art is code, rebuilt and never hand-edited.

| Tool | What it does | Run | Used by | Limits |
| --- | --- | --- | --- | --- |
| The one art drawer | draws a sheet in headless Chromium at 3 px a point: a canvas sheet (`.js`, 2D drawings, frames named `name-1.png …`) or an SVG sheet (`.cjs`, a Node module whose `sheet()` gives SVG markup and an index to write); `--contact` lays them all on one page | `node tools/packart/render.cjs <sheet> <pack dir> [--only a,b] [--scale 3] [--contact out.png]` | no current users (its sheets, `tools/whales/art.js`, `monsoon/art.js`, `beacons/art.js`, `leadline/art.js`, `lowsea/art.js`, `vine/art.js`, `tools/herds/draw.cjs`, `tools/water/boto.cjs`, drew retired packs) | SVG only from Node; no sprite sheets (one file a frame) |
| All the art again | every pack's art redrawn in place; a clean `git status` says a drawer change left it the same to the byte | `tools/packart/all.sh` | no current users (its packs are gone: it fails until a sheet has a pack again) | a new sheet adds its line |
| Brush | the sheets' helpers: seeded random, smooth curves, ink under fill, glow, colour mix, the halo round a tiny figure | `window.Brush` in a sheet, `tools/packart/brush.js` | no current users | |
| The herds' animals | one camelid drawn from parameters (inverse-kinematic legs, gaits) in every kind, look and pose; writes `herds/art.js`, the index | an SVG sheet: `node tools/packart/render.cjs tools/herds/draw.cjs Prototypes/herds` (or `node tools/herds/draw.cjs`) | no current users | 458 of the pack's 500 files |
| The boto | a river dolphin surfacing, 24 frames; writes `boto.json` | an SVG sheet: `node tools/packart/render.cjs tools/water/boto.cjs Prototypes/gathering-of-water-alive` | no current users | |
| Glint | the doorway's glint from afar | `python3 tools/playground/glint_png.py [png ...]` | the scaffold's template | standard library PNG |
| Marker | the template's and hello-pack's marker | `python3 tools/playground/marker_png.py` | template, hello | standard library PNG |
| Persian ornaments | band, braid, finial, rosette, headpiece, from geometry | `python3 tools/globe/ornament.py [--sheet out.png]` | the Persian style's code (no current users) | |
| Plates | a look's plates, layer by layer, from its page | `node tools/plates/export.cjs <look> "<Name>"`, contract `tools/plates/kit.js` | no current users (the films' `Atlas/` is retired) | 1638 × 3545 a plate |
| River pictures | the gatherings' doorway and glint, the Amazon's plume | `tools/water/build_pack.py`, `build_alive.py` | no current users | |

## Data: builders from sources

Each reads public sources (named in its docstring) and writes a bundled file. Rebuild, never hand-edit.

| Tool | Writes | Run | Used by |
| --- | --- | --- | --- |
| Globe | `Globe/` (distance fields, terrain, ids, `globe.json`, fonts) | `python3 tools/globe/build.py <cache>` | the globe; read by many builders below |
| Ground | `Globe/ground/` (twelve months' land, sea ice, sea floor), `regions.json` | `python3 tools/globe/ground.py <cache>` | the ground style |
| Night | `Globe/ground/night.jpg` (Black Marble) | `python3 tools/globe/night.py <cache>` | night |
| Clouds | `CloudsSky.png`, `CloudsLand.png` (ISCCP-H), for the app and its twin | `python3 tools/clouds/build.py <cache>`; noise calibration `calibrate.py` | clouds |
| Ocean | `OceanCurrents.png`, `OceanStream.png` (NOAA drifters) | `python3 tools/ocean/build.py <cache>` | ocean |
| Wikipedia | `Wikipedia.json`, each pack's `wikipedia.json` | `python3 tools/wikipedia/build.py [--app] [--pack id]`, list `articles.txt` | no current users (Wikipedia's words retired) |
| Plate data | `tools/plates/data/` GeoJSON by scale | `python3 tools/plates/build_data.py <downloads>` | no current users |
| Herds | `Prototypes/herds/data.js`: every animal's dated way, checked against the globe's land | `python3 tools/herds/build.py` (lineages in `lineages.py`) | no current users |
| Sight lines | `fires-across-the-sea/sight.js`: which peaks see which, over the globe's terrain | `python3 tools/beacons/build_sight.py` | no current users | ~10 km a pixel: thin ridges missed |
| Depths | `lead-line/depths.js`: the sea's depth on a 1024 × 512 grid | `python3 tools/leadline/build_depths.py` | no current users | a cell's depth, not a point's |
| Challenger's way | `lead-line/way.js`: stations, ports and turns, dated | `python3 tools/leadline/build_way.py` | no current users |
| Shelves | `low-sea/shelf-*.png`: shelves under 200 m as land | `python3 tools/lowsea/build_shelves.py <NE folder>` | no current users |
| Tethys | `sea-of-whales/tethys-*.png`: the Eocene sea over Egypt | `python3 tools/whales/build_sea.py <NE folder>` | no current users |
| The vine | `vine/data.js`: branches with a year at each vertex, leaves along them | `python3 tools/vine/build.py` | no current users |
| Rivers | `Water/` from RiverATLAS; the packs from it | `tools/water/build.py`, `build_pack.py`, `build_alive.py`, audit `audit_source.py` | no current users |

## Seeing it without a Mac: previews

| Tool | What it does | Run | Used by | Limits |
| --- | --- | --- | --- | --- |
| Desktop preview | every pack over a stand-in globe with the app's own `terra.js`; `/atlas` the living globe, `?with=` several at once | `node tools/playground/serve.cjs`, `harness/` | every pack session; `check.cjs`, `explore.cjs`, `moments.cjs`, `RemotePackTests` | close to the app, not pixel-identical: the globe is vectors on a canvas, not `Globe.metal`; no clouds or ocean |
| Pack check | runs a pack headless, fails on errors, screenshots doorway, anchor, sprites, zooms, a touch | `node tools/playground/check.cjs <id> \| --all [--wikipedia] [--device]` | every pack (`--wikipedia`: no current users) | fixed screenshots; not run by CI |
| Living check | two packs by touch side by side, front and back, one ended | `node tools/playground/check.cjs --living` | no current users (it needs two current packs) | not run by CI |
| Pack moments (1) | a plan of waits (ms), flights, strip, play, tap by name, Open; prints the state at each shot | `node tools/playground/explore.cjs <id> <out> '<plan>'` | no current users (written with the herds) | see [One job, two tools](#one-job-two-tools) |
| Pack moments (2) | a plan of camera, waits (s), touches and drags at places, strip, play; exit 1 on page errors | `node tools/packart/moments.cjs <id> <out> '<plan>' [--root]` | no current users (written with the rebuilt experiences) | see [One job, two tools](#one-job-two-tools) |
| River drive | the gathering alive driven as a reader would, with assertions | `node tools/water/alive_check.cjs` | no current users | one pack's |
| Retired river study | the former web study's interactions | `node tools/water/interaction_check.cjs <url>` | `Water/` only (no longer in the app) | |
| Globe preview | the globe as `Globe.metal` draws it, in numpy, with names | `python3 tools/globe/preview.py out.png [--style ground --month 7]` | every globe change | names placed roughly |
| Metal off the Mac | shaders compiled as C++ and drawn on the CPU; stacked layers; Metal beside its twin | `python3 tools/metal/metal.py shot globe+ocean+clouds out.png`, `twin clouds`; `layers.py` mirrors the layers' Swift | globe, clouds, ocean | README › What it can't vouch for: derivatives, precision, not Metal's compiler |
| Clouds look, film | the clouds' twin over the globe preview, stills or a film | `node tools/clouds/look.cjs`, `film.cjs` (via `compose.py`) | clouds | film needs ffmpeg |
| Ocean look | the ocean's twin over the ground style | `node tools/ocean/look.cjs` (via `tools/clouds/compose.py`) | ocean | |
| Plates preview | a look's film frames or video | `python3 tools/plates/preview.py <look> --times …` | no current users | |

## Checks and tests

| Tool | What it checks | Run | Where it runs |
| --- | --- | --- | --- |
| Pack validator | the contract, sizes (500 files, 50 MB a pack, 25 MB a file, 24 frames), the offline boundary | `python3 tools/playground/playground.py validate [ids]` | CI `plates`, `packs.yml` |
| Playground tool tests | `playground.py` itself | `python3 tools/playground/test_playground.py` | CI `plates`, `packs.yml` |
| Preview's own check | a fixture pack using every tool once, held to `runtime3.md` | `node tools/playground/harness_check.cjs` | by hand |
| Shader check | every `.metal` compiled and its bindings, in seconds | `python3 tools/metal/metal.py check` | CI `plates` |
| Stand-in tests | the CPU Metal's builtins; whether `twin` notices drift | `tools/metal/tests/test_metal.py`; `twin_drift.py` (slow) | CI `plates`; by hand |
| Globe data | maps, ornaments, ground, fonts, `type.json`'s design rules (sizes, contrast) | `python3 ci/check_globe.py` | CI `plates` |
| Syntax | `node --check`, `py_compile` on the tools | in `ci.yml` › `plates` | CI `plates` (not `tools/herds/*.cjs`, `tools/water/*.cjs`) |
| UI tests | the playground (plain launch, every prototype listed, a native and a pack together, the bundled pack, remote revisions), clouds, ocean, quiet | `App/TerraIncognitaUITests/`, helpers `PlaygroundSupport.swift`; `ci/encounter_tests.sh` | CI `apple` (`[test Class]`, `[playground-check]`) |
| Simulator shots | the playground, the globe at places and by night, the stage at places | `ci/sim_shots.sh` | CI `apple` (by hand, `[shots]`) |
| Exploratory QA | an agent using the app through Maestro | `qa-explore.yml`, `ci/qa/` (synced from painting-company) | on request |

## CI and delivery

| Tool | What it does | Run |
| --- | --- | --- |
| Narrowing tags | what a branch push runs (`[build-only]`, `[shots]`, `[test Class]`, `[playground-check]`, `[ipa]`) | `ci/fast.py` (first line of the commit message) |
| Report | waits for a commit's CI report and unpacks it; finds a screenshot by name | `CI_BRANCH=<b> ci/fetch.sh <sha> <dir>`, `python3 ci/shots.py <dir> <name>` |
| Simulators | the iPhone and iPad simulators wanted | `ci/simulators.sh` |
| Packs | pack, publish to the `packs` branch, retire (main's publish retires a pack deleted from `Prototypes/`), list | `python3 tools/playground/playground.py pack \| publish \| retire \| list`; `packs.yml` |
| Remote revision fixture | a hello-pack revision with other words, for `RemotePackTests` | `ci/remote_packs.sh <out>` |
| SideStore | the source JSON for `build-<n>` IPAs | `ci/sidestore_source.py`; `ci.yml` › `ipa` |
| One-time retirement | the former second study's listing | `ci/retire_study2.sh` |

There is no `swift.sh`: Swift is built only by the macOS jobs.

## One job, two tools

Found October 29, 2026 (painting-company row 178). Each says what differs and what to do. Since the
retirement (row 195) the packs named below are at `before-retirement-2026-10-31`.

- **Art drawers: `tools/packart/render.cjs`, `tools/herds/draw.cjs`, `tools/water/boto.cjs`** (merged).
  All three drove headless Chromium to turn code into PNG frames: `render.cjs` from a browser sheet of
  canvas drawings, the other two from SVG built in Node, each with its own Playwright loop and an
  index the pack reads. What differed was the drawing language and the index, not the rendering, so
  `render.cjs` now draws both kinds of sheet and the other two are SVG sheets for it; every pack's
  art came out the same to the byte.
- **Pack moments: `tools/playground/explore.cjs` and `tools/packart/moments.cjs`.** Both play a pack
  in the preview from a JSON plan and screenshot it; they were written the same night. `explore`
  has flights with a duration, a tap on a touchable thing by name, the chip's Open, and prints the
  date, speed, sprites and camera at each shot; `moments` has touches and drags at places, a tap at
  a screen point, `--root`, play set rather than toggled, and fails on page errors. Their `wait`s
  differ (milliseconds against seconds). They should be one (`moments.cjs`, taking `explore`'s
  steps, with `wait` in seconds), its users' plans converted in the same commit.
- **Things moved by the date by hand.** The lead line's ship and the monsoon's dhows are set on a
  timed `path` again on every `time` event, the seconds computed from the date's rate; runtime 5's
  dated ways (`path: { coordinates, dates }`, the herds) do exactly that in the globe, smoothly and
  under scrubbing. The lead line's way is already dated (`way.js`) and could move onto it as it
  stands; the monsoon's dhows sail a yearly round, which a dated way can carry as one year's dates.
  The vine's stems, regrown by `terra.geo.set` on every `time` event, are the same idea for lines:
  no dated geography exists yet.
- **Script helpers repeated in packs.** `along` (six packs, and the herds' `slerp`) and `frames`
  (six) are the first candidates for a shared script module (CLAUDE.md › Reuse first).
- **Three hand-written PNG encoders** (`glint_png.py`, `marker_png.py`, `harness_check.cjs`'s
  fixture pictures): small and standard-library by design; worth one shared helper only when a
  fourth appears.
- **Not duplicates:** the glint drawn in Python for packs and in Swift for natives (one recipe, two
  hosts); `check.cjs` (any pack, fixed shots) beside `alive_check.cjs` (one pack's behaviour asserted);
  `tools/clouds/compose.py`, already shared by the clouds' and the ocean's looks.
