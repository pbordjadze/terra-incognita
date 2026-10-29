# Terra's tools

Every tool this repository has, as `main` has it (October 29, 2026; painting-company row 178): what
it does, where it lives, who uses it, and where it stops. Read it before you build; add to it when you do.

**The rules**

1. **Picture your experience first.** Write down what the reader should see, touch and feel before
   reading this page, so what exists doesn't shrink what you picture.
2. **A tool that falls short is the tool's bug.** Fix it, extend it, or build a new one beside it
   (CLAUDE.md › The runtime). Never settle for less than you pictured because a tool can't do it yet.
3. **Keep this page true in the same commit.** A commit that adds a tool adds its line; a commit
   that starts using one adds your experience to its *used by*; a commit that changes a limit
   changes it here. Two tools for one job is a bug too: look under [One job, two tools](#one-job-two-tools)
   before you write a third.

*Used by* is read from the code, not from intentions: in the tables of what packs and natives call it is
written by `python3 ci/check_tools.py --write` (CI fails a catalog the code has moved past). "Packs" are the folders in `Prototypes/`
(`hello-pack` is the smallest example, parked); "natives" are the entries in
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
| Geography | GeoJSON drawn by the globe, constant width at any zoom | `terra.geo.set(id, features, { style })` | every current pack but herds, and hello | 100,000 vertices, 64 layers, 400 places a layer |
| ↳ `border` | lines; `lineWidth` in reading points | `style: 'border'` | fires, gathering alive, gathering of water, lead line, low sea, vine | |
| ↳ `area` | filled shapes | `style: 'area'` | lead line, low sea | |
| ↳ `dot`, `ring` | points | `style: 'dot'` / `'ring'` | gathering alive, gathering of water, lead line (dot); gathering alive, gathering of water, hello, lead line, monsoon (ring) | |
| ↳ `label` | names on a halo, earlier wins room | `style: 'label'` | gathering alive, gathering of water | |
| ↳ `symbol` | a pack's picture at a point | `style: 'symbol'` | fires, vine, whales | |
| ↳ `sounding` | a line hanging from a point, with a label | `style: 'sounding'` | lead line | made for one pack |
| ↳ `ghost`, `track` | dashed outline, fine dashes | `style: 'ghost'` / `'track'` | none yet | |
| ↳ touchable | features take touches (points 22 pt, lines 14, areas inside); a named place is a VoiceOver button | `touchable: true` | fires, gathering alive, gathering of water, lead line, monsoon, vine | |
| ↳ `flow` | light running along lines the way their coordinates go, moved by Core Animation | `flow: true` or `{ speed, dash, gap, color }` | gathering alive | a few thousand vertices a flowing layer (drawn every frame); none under Reduce Motion |
| Pictures on the globe | a picture in lon/lat bounds, curving with the globe, faded by zoom | `terra.images.set(id, { image, bounds, opacity, minZoom, fade })` | gathering alive, herds, low sea, whales | 8 pictures, 4096 px a side; equirectangular only |
| Sprites | things that live and move: `at`, or a timed `path` (`seconds`, `loop`) | `terra.sprites.set(id, { at \| path, looks, label, touchable })` | every current pack | 64 sprites, 24 frames a look, 400 pt a side (200 before runtime 5) |
| ↳ sprite sheets | many frames in one picture: a frame named `'sheet.png#x,y,w,h'` (pixels), in a sprite's or a doorway's look; decoded once, cut without copying (`Globe/SpriteSheet.swift`, which a native prototype may use too) | `frames: ['art/early.png#0,0,376,335', …]`, written by the art drawer | herds | runtime 6; not for `terra.images` or a `symbol` (one file each); 4096 px a side |
| ↳ looks by zoom | one thing a trace from afar and itself close in; `every`/`for` a glint now and then; `face` mirrors heading west | `looks: [{ image \| frames, fps, size, minZoom, maxZoom, every, for, face }]` | every current pack (`every`: monsoon, whales; `face`: gathering alive, herds, lead line, low sea, monsoon, whales) | |
| ↳ dated way | the globe's date puts the sprite on its way; it walks as the date plays, stands where the reader drags | `path: { coordinates, dates }` | herds, lead line, monsoon, vine | runtime 5; not drawn outside its dates |
| Doorway | a pack's life on the globe while it isn't running; a touch starts it | `prototype.json` › `doorway` (`look` or `looks`, `path`, `breathes`) | every current pack (`path`: herds, lead line, low sea, monsoon; `breathes`: fires, vine) | the same budgets as a sprite's look |
| Glint | the one mark every doorway shows from afar | `doorway-glint.png`, from `tools/playground/glint_png.py` | every current pack | the natives draw theirs in Swift (`Catalog.swift`'s `Glint`) from the same recipe |
| Touch | taps and drags on the pack's things, or every tap with `"taps": true` | `terra.on('touch' \| 'tap', fn)` | every pack (`tap`: hello) | no long press; a drag's `moved` keeps the target it began on |
| ↳ schemes | who gets a one-finger drag | `terra.touch.scheme('touchables' \| 'one-finger' \| 'hold')` | `touchables` (default): all; `hold`: gathering alive; `one-finger`: none yet | |
| The date | the date at the foot: `year` or `day` (a day is the globe's season), ticks, play, `carry` | `terra.time.show({...})`, `.set`, `.play`, `.hide`, `terra.on('time')` | herds, lead line, monsoon, vine (`day`: lead line, monsoon) | |
| ↳ pace, speed | an uneven pace (deep time fast); playback slower as the reader comes close | `pace: [[date, seconds], …]`, `terra.time.speed(f)` | pace: herds, lead line; speed: herds | runtime 5; 0.001–100× |
| Caption | the words at the foot | `terra.caption.set({ title, text, wikipedia })` | every pack | |
| Panel | longer words and sources in a native sheet | `terra.panel.show({ title, text, sections, sources })`, `terra.on('panel')` | every pack | plain text only |
| Wikipedia's words | names a subject's article, shown when the reader's switch is on | `wikipedia: 'Title'` on caption, panel, sections; `tools/wikipedia/build.py --pack <id>` | every current pack but gathering alive and herds | bundled, never fetched; nothing reads `terra.settings.wikipedia` yet |
| The globe | hide today's borders and names; stop the idle turn; zoom deeper than 8 | `terra.globe.set({ historical, still, maxZoom })` | every current pack (`historical`: herds, lead line, whales) | `maxZoom` up to 32 |
| Camera | the camera now and as it moves; screen ↔ globe; fly | `terra.camera`, `terra.on('camera')`, `terra.project`, `terra.unproject`, `terra.flyTo` | every current pack (`project`: gathering alive, herds, lead line; `unproject`: gathering alive, gathering of water; `flyTo`: fires, gathering alive, gathering of water, herds, lead line, monsoon) | `camera` at most 20 times a second; a flight is dropped while the reader's hands are on the globe |
| Many at once | in front or behind; how the reader came in | `terra.focus`, `terra.on('focus')`, `terra.entry` | focus: herds, lead line, monsoon, vine; entry: fires | 6 experiences at once |
| Settings | Reduce Motion, text size, Wikipedia's switch | `terra.settings`, `terra.on('settings')` | gathering alive, gathering of water, hello, herds, lead line | |
| Files | a pack file as text or JSON | `terra.read`, `terra.readJSON` | gathering alive, gathering of water | nothing outside the pack |
| Timers, frames | browser timers; `requestAnimationFrame` for a short eased change | `setTimeout`, `requestAnimationFrame` | every pack (rAF: lead line) | never to animate the globe; JavaScriptCore runs without a JIT: prepare data at build time |
| Open, close | the chip's Open; the end | `terra.on('open' \| 'close')`, `terra.close()` | open: every pack; close: fires, gathering alive, gathering of water, lead line, low sea, whales | |

**Shared script** (`Prototypes/_shared/shared.js`, the global `Shared`; [`runtime3.md`](runtime3.md) › Shared script):
helpers a pack would otherwise write again. A pack carries a copy, `shared.js`, first in its `scripts`.

| Tool | What it does | Call | Used by | Limits |
| --- | --- | --- | --- | --- |
| Great circles | the place `t` of the way between two places; the angle between them; a place's unit vector | `Shared.along(a, b, t)`, `Shared.arc(a, b)`, `Shared.unit(p)` | herds, lead line, monsoon, whales (along: herds, lead line, monsoon, whales; arc: herds, monsoon, whales; unit: lead line) | |
| Frame names | `name-1.png … name-n.png`, the art drawer's canvas frames | `Shared.frames(name, n)` | fires, lead line, low sea, monsoon, whales | |
| Copies | writes each pack's copy; `validate` fails one that differs; held to the helpers it replaced | `playground.py share`, `node tools/playground/shared_check.cjs` | every pack listing `shared.js` | change it compatibly |

## What a native prototype calls: the stage

A native prototype (`App/TerraIncognita/<Name>/`, one entry in `Catalog.swift`) asks its
`PrototypeStage` (`Playground/Prototype.swift`) for what it changes; the stage gives it all back.

| Tool | What it does | Call | Used by | Limits |
| --- | --- | --- | --- | --- |
| Style | the globe's style while it runs (crossfades) | `stage.style = .persian` … | persian, wikipedia, night | |
| Marker | a native picture on the globe (`GlobeMarker`: paths, frames, a look per zoom; the sprites' own drawing) | `stage.place(GlobeMarker(...))` | camelid | |
| Lens | a native experience over the one globe: its canvas over the names, its authored camera, chrome over the playground; `GlobeLease` puts back hooks, modes, camera, chosen country, VoiceOver's words, gestures | `stage.openLens { globe, close in ... }` | camelid | the other experiences' things hide while it is open |
| Screen | a full screen of its own | `stage.present(id) { ... }` | film | only for an experience with a map of its own |
| Quiet rules | names on demand, clouds that clear where you look, nothing at the foot until a choice | `stage.quietNames()`, `.quietClouds()`, `.quietFrame()` | quiet-names, quiet-clouds, quiet-frame | |
| Still sea | hides the ocean layer | `stage.stillSea()` | still-sea | |
| Veils | whether the clouds and sea thin while it runs | `veils: false` in its entry | the four quiet rules | |
| Words, flights, stillness | the foot, a flight, no idle turn: what packs reach through `terra.*` | `stage.describe`, `.fly(to:)`, `.holdStill()` | the pack host (`Playground.swift`) | |

## The stage's own layers

Built in, owned by no experience; every experience sits on them.

| Tool | What it does | Where | Used by | Limits |
| --- | --- | --- | --- | --- |
| The globe | the whole Earth drawn per pixel from distance fields; three styles (ground, Wikipedia, Persian) and night; names placed by the design rules | `Globe/Globe.metal`, `GlobeView.swift`, `GlobeScreen.swift`, spec `Globe/type.json` | everything | change the shader and `tools/globe/preview.py` together |
| Clouds | the month's clouds where satellites usually see them | `Clouds/` (`CloudsLayer`, `Clouds.metal`), switch `CloudsSwitch` | the stage; quiet-clouds; veiled under experiences | twin `tools/clouds/web/shader.js` (and its `tools/clouds/web/sky.js`), line for line |
| Ocean | the month's surface currents as streamlines, glitter and surf; a tap rings the water | `Ocean/` (`OceanLayer`, `Ocean.metal`) | the stage; hidden by still-sea and the Persian page | twin `tools/ocean/web/shader.js` (and its `tools/ocean/web/sea.js`), line for line |
| Wikipedia's words | the switch, the cards, the one view every prose site goes through | `Wikipedia/` (`Words`, `WikipediaCard`, `WordsRule.isTip`), `Wikipedia.json` | every caption and panel | articles bundled by `tools/wikipedia/build.py` |
| Packs | finds bundled and remote packs, checks hashes, runs their scripts in JavaScriptCore | `PackStore.swift`, `PackScript.swift`, `PackPanel.swift`, `GlobeDate.swift` | every pack | HTTPS GETs only while the workbench is open or a pack opens |
| Live counts | stages, markers, globes, films and scripts, to prove an ending leaves nothing | `LiveCount.swift`, the workbench | every UI test that ends a prototype | |
| The player | plates in a ladder, drifts and dives at 120 Hz | `Atlas/` | film (parked) | never draw or lay out per frame |

## Art: pictures drawn from code

A pack's art is code, rebuilt and never hand-edited.

| Tool | What it does | Run | Used by | Limits |
| --- | --- | --- | --- | --- |
| The one art drawer | draws a sheet in headless Chromium at 3 px a point: a canvas sheet (`.js`, 2D drawings, frames named `name-1.png …`) or an SVG sheet (`.cjs`, a Node module whose `sheet()` gives SVG markup and an index to write); `--contact` lays them all on one page | `node tools/packart/render.cjs <sheet> <pack dir> [--only a,b] [--scale 3] [--contact out.png]` | canvas sheets `tools/whales/art.js`, `tools/monsoon/art.js`, `tools/beacons/art.js`, `tools/leadline/art.js`, `tools/lowsea/art.js`, `tools/vine/art.js`; SVG sheets `tools/herds/draw.cjs`, `tools/water/boto.cjs` | SVG only from Node; sprite sheets only from SVG sheets (a picture's `sheet`; `done(names)` hears each frame's name), at most 4096 px a side |
| PNG in Node | reads and writes PNGs with zlib alone, so frames go into a sprite sheet pixel for pixel (never through a canvas's premultiplied alpha) | `require('tools/packart/png.cjs')`: `read(buffer)`, `write({ width, height, data })` | the art drawer's sprite sheets | 8-bit, not interlaced |
| All the art again | every pack's art redrawn in place; a clean `git status` says a drawer change left it the same to the byte | `tools/packart/all.sh` | changes to the drawer, Brush or a sheet | a new sheet adds its line |
| Brush | the sheets' helpers: seeded random, smooth curves, ink under fill, glow, colour mix, the halo round a tiny figure | `window.Brush` in a sheet, `tools/packart/brush.js` | the six sheets above | |
| The herds' animals | one camelid drawn from parameters (inverse-kinematic legs, gaits) in every kind, look and pose, one sprite sheet a kind (`art/<kind>.png`); writes `herds/art.js`, the index, and the doorway's frames in `prototype.json` | an SVG sheet: `node tools/packart/render.cjs tools/herds/draw.cjs Prototypes/herds` (or `node tools/herds/draw.cjs`) | herds | 15 sheets, 7.2 MB (450 files and 11 MB as one file a frame) |
| The boto | a river dolphin surfacing, 24 frames; writes `boto.json` | an SVG sheet: `node tools/packart/render.cjs tools/water/boto.cjs Prototypes/gathering-of-water-alive` | gathering of water, alive | |
| Glint | the doorway's glint from afar | `python3 tools/playground/glint_png.py [png ...]` | every current pack, the scaffold's template | standard library PNG |
| Marker | the template's and hello-pack's marker | `python3 tools/playground/marker_png.py` | template, hello | standard library PNG |
| Persian ornaments | band, braid, finial, rosette, headpiece, from geometry | `python3 tools/globe/ornament.py [--sheet out.png]` | persian | |
| Plates | a look's plates, layer by layer, from its page (`tools/plates/<look>/`, its scripts beside it; d3 and topojson in `tools/plates/vendor/`) | `node tools/plates/export.cjs <look> "<Name>"`, contract `tools/plates/kit.js` | film (`Atlas/`) | 1638 × 3545 a plate |
| River pictures | the gatherings' doorway and glint, the Amazon's plume | `tools/water/build_pack.py`, `build_alive.py` | both gatherings | |
| Camel art | generated illustrations, cropped once at launch | `CamelidAnimals` asset catalog, `CamelidArt.swift` | camelid | not drawn from code (the one exception) |

## Data: builders from sources

Each reads public sources (named in its docstring) and writes a bundled file. Rebuild, never hand-edit.

| Tool | Writes | Run | Used by |
| --- | --- | --- | --- |
| Globe | `Globe/` (distance fields, terrain, ids, `globe.json`, fonts) | `python3 tools/globe/build.py <cache>` | the globe; read by many builders below |
| Ground | `Globe/ground/` (twelve months' land, sea ice, sea floor), `regions.json` | `python3 tools/globe/ground.py <cache>` | the ground style |
| Night | `Globe/ground/night.jpg` (Black Marble) | `python3 tools/globe/night.py <cache>` | night |
| Clouds | `CloudsSky.png`, `CloudsLand.png` (ISCCP-H), for the app and its twin | `python3 tools/clouds/build.py <cache>`; noise calibration `calibrate.py` | clouds |
| Ocean | `OceanCurrents.png`, `OceanStream.png` (NOAA drifters) | `python3 tools/ocean/build.py <cache>` | ocean; gathering alive (its sea ways) |
| Wikipedia | `Wikipedia.json`, each pack's `wikipedia.json` | `python3 tools/wikipedia/build.py [--app] [--pack id]`, list `articles.txt` | the switch; seven packs |
| Plate data | `tools/plates/data/` GeoJSON by scale | `python3 tools/plates/build_data.py <downloads>` | the plates |
| Herds | `Prototypes/herds/data.js`: every animal's dated way, checked against the globe's land | `python3 tools/herds/build.py` (lineages in `lineages.py`) | herds |
| Sight lines | `fires-across-the-sea/sight.js`: which peaks see which, over the globe's terrain | `python3 tools/beacons/build_sight.py` | fires | ~10 km a pixel: thin ridges missed |
| Depths | `lead-line/depths.js`: the sea's depth on a 1024 × 512 grid | `python3 tools/leadline/build_depths.py` | lead line | a cell's depth, not a point's |
| Challenger's way | `lead-line/way.js`: stations, ports and turns, dated | `python3 tools/leadline/build_way.py` | lead line |
| Shelves | `low-sea/shelf-*.png`: shelves under 200 m as land | `python3 tools/lowsea/build_shelves.py <NE folder>` | low sea |
| Tethys | `sea-of-whales/tethys-*.png`: the Eocene sea over Egypt | `python3 tools/whales/build_sea.py <NE folder>` | whales |
| The vine | `vine/data.js`: branches with a year at each vertex, leaves along them | `python3 tools/vine/build.py` | vine |
| Rivers | `Water/` from RiverATLAS; the packs from it | `tools/water/build.py`, `build_pack.py`, `build_alive.py`, audit `audit_source.py` | both gatherings |

## Seeing it without a Mac: previews

| Tool | What it does | Run | Used by | Limits |
| --- | --- | --- | --- | --- |
| Desktop preview | every pack over a stand-in globe with the app's own `terra.js`; `/atlas` the living globe, `?with=` several at once | `node tools/playground/serve.cjs`, `harness/` | every pack session; `check.cjs`, `moments.cjs`, `RemotePackTests` | close to the app, not pixel-identical: the globe is vectors on a canvas, not `Globe.metal`; no clouds or ocean |
| Pack check | runs a pack headless, fails on errors, screenshots doorway, anchor, sprites, zooms, a touch; waits for the pack to stop re-drawing after the camera rests | `node tools/playground/check.cjs <id> \| --all [--wikipedia] [--device]` | every pack; CI `packs.yml` › preview (`--all`) | fixed screenshots; a touch on a line aims where no other line is within reach |
| Living check | two packs by touch side by side, front and back, one ended; nothing either had is taken away but by its own script | `node tools/playground/check.cjs --living` | runtime 4; CI `packs.yml` › preview | the first two packs with doorways, or the two named |
| Pack moments | the one pack photographer: a plan of the camera put or flown, waits (seconds), touches and drags at places, a tap on a thing by name, the date's strip, play, Open; screenshots with the date, speed, sprites, camera and caption printed; exit 1 on a page error or an error logged | `node tools/packart/moments.cjs <id> <out> '<plan>' [--device] [--root] [--no-idle]` | the rebuilt experiences, the herds | the preview's time is the wall clock: two runs differ by what moved meanwhile |
| River drive | the gathering alive driven as a reader would, with assertions | `node tools/water/alive_check.cjs` | gathering alive | one pack's |
| Retired river study | the former web study's interactions | `node tools/water/interaction_check.cjs <url>` | `Water/` only (no longer in the app) | |
| Globe preview | the globe as `Globe.metal` draws it, in numpy, with names | `python3 tools/globe/preview.py out.png [--style ground --month 7]` | every globe change | names placed roughly |
| Metal off the Mac | shaders compiled as C++ and drawn on the CPU; stacked layers; Metal beside its twin | `python3 tools/metal/metal.py shot globe+ocean+clouds out.png`, `twin clouds` (`tools/metal/twin.cjs` draws the twin); `layers.py` mirrors the layers' Swift | globe, clouds, ocean | README › What it can't vouch for: derivatives, precision, not Metal's compiler |
| Clouds look, film | the clouds' twin over the globe preview, stills or a film | `node tools/clouds/look.cjs`, `film.cjs` (via `compose.py`) | clouds | film needs ffmpeg |
| Ocean look | the ocean's twin over the ground style | `node tools/ocean/look.cjs` (via `tools/clouds/compose.py`) | ocean | |
| Plates preview | a look's film frames or video | `python3 tools/plates/preview.py <look> --times …` | film | |

## Checks and tests

| Tool | What it checks | Run | Where it runs |
| --- | --- | --- | --- |
| Pack validator | the contract, sizes (500 files, 50 MB a pack, 25 MB a file, 24 frames), a doorway's sheet frames inside their sheet, the offline boundary | `python3 tools/playground/playground.py validate [ids]` | CI `plates`, `packs.yml` |
| Playground tool tests | `playground.py` itself | `python3 tools/playground/test_playground.py` | CI `plates`, `packs.yml` |
| Shared script | `Shared` against the helpers it replaced, bit for bit | `node tools/playground/shared_check.cjs` | `packs.yml` |
| Dated ways | a pack's sprites on dated ways against where an older revision's script placed them, headless, at every step of its strip | `node tools/playground/dated_check.cjs <id> [rev] [--step d] [--offset d]` | by hand (lead line, monsoon) |
| Preview's own check | a fixture pack using every tool once, held to `runtime3.md` | `node tools/playground/harness_check.cjs` | by hand (one camera-timing check is sensitive to a loaded machine, so CI doesn't run it) |
| Preview in CI | every pack's check and the living check on Linux, Playwright pinned (1.63.0), screenshots as the `preview` artifact; publishing waits for it | `packs.yml` › preview | every push that touches packs or `tools/playground/` |
| Shader check | every `.metal` compiled and its bindings, in seconds | `python3 tools/metal/metal.py check` | CI `plates` |
| Stand-in tests | the CPU Metal's builtins; whether `twin` notices drift | `tools/metal/tests/test_metal.py`; `twin_drift.py` (slow) | CI `plates`; by hand |
| Looks | every look's plates as the player reads them, no unused file | `python3 ci/check_atlas.py` | CI `plates` |
| Globe data | maps, ornaments, ground, fonts, `type.json`'s design rules (sizes, contrast) | `python3 ci/check_globe.py` | CI `plates` |
| This catalog | the *Used by* of what packs and natives call, generated from their code; every path named exists; every script under `tools/` and `ci/` named | `python3 ci/check_tools.py [--write]` | CI `plates`, `packs.yml` |
| Wikipedia | every key the code names has a sound article | `python3 ci/check_wikipedia.py` | CI `plates` |
| Syntax | `node --check`, `py_compile` on the tools | in `ci.yml` › `plates` | CI `plates` (not `tools/herds/*.cjs`, `tools/water/*.cjs`) |
| UI tests | the playground (plain launch, succession, by touch, lens, parked, remote revisions, globe-drawn packs, herds, `hold`), clouds, ocean, quiet, Wikipedia, camelid | `App/TerraIncognitaUITests/`, helpers `PlaygroundSupport.swift`; `ci/encounter_tests.sh` | CI `apple` (`[test Class]`, `[playground-check]`) |
| Simulator shots | films, the globe in each style, the stage at places | `ci/sim_shots.sh` | CI `apple` (by hand, `[shots]`) |
| Camelid capture | the sequence as the simulator draws it | `ci/camelid_capture.sh` | CI `apple` (`[camelid-capture]`) |
| Exploratory QA | an agent using the app through Maestro | `qa-explore.yml`, `ci/qa/` (synced from painting-company) | on request |

## CI and delivery

| Tool | What it does | Run |
| --- | --- | --- |
| Narrowing tags | what a branch push runs (`[build-only]`, `[shots]`, `[test Class]`, `[playground-check]`, `[ipa]`) | `ci/fast.py` (first line of the commit message) |
| Report | waits for a commit's CI report and unpacks it; finds a screenshot by name | `CI_BRANCH=<b> ci/fetch.sh <sha> <dir>`, `python3 ci/shots.py <dir> <name>` |
| Simulators | the iPhone and iPad simulators wanted | `ci/simulators.sh` |
| Packs | pack, publish to the `packs` branch, retire, list | `python3 tools/playground/playground.py pack \| publish \| retire \| list`; `packs.yml` |
| Remote revision fixture | a hello-pack revision with other words, for `RemotePackTests` | `ci/remote_packs.sh <out>` |
| SideStore | the source JSON for `build-<n>` IPAs | `ci/sidestore_source.py`; `ci.yml` › `ipa` |
| One-time retirement | the former second study's listing | `ci/retire_study2.sh` |

There is no `swift.sh`: Swift is built only by the macOS jobs.

## One job, two tools

Found October 29, 2026 (painting-company row 178). Each says what differs and what to do.

- **Art drawers: `tools/packart/render.cjs`, `tools/herds/draw.cjs`, `tools/water/boto.cjs`** (merged).
  All three drove headless Chromium to turn code into PNG frames: `render.cjs` from a browser sheet of
  canvas drawings, the other two from SVG built in Node, each with its own Playwright loop and an
  index the pack reads. What differed was the drawing language and the index, not the rendering, so
  `render.cjs` now draws both kinds of sheet and the other two are SVG sheets for it; every pack's
  art came out the same to the byte.
- **Pack moments: `explore.cjs` and `tools/packart/moments.cjs`** (merged). Both played a pack
  in the preview from a JSON plan and screenshotted it, written the same night. `moments.cjs` is the one
  now: it took `explore`'s timed flight (`fly`), tap on a thing by name (`tap`), Open and the state printed
  at each shot, its `wait` in seconds; `explore.cjs` is gone. The same plans give the same states and
  pictures (but for what moved with the wall clock).
- **Things moved by the date by hand** (merged). The lead line's ship and the monsoon's dhows were set on a
  timed `path` again at every `time` event, the seconds worked out from the date's rate; they are on
  runtime 5's dated ways now (the ship's whole voyage; each dhow's year, its berths held and its voyages
  dated along their routes, the reader's sendings included), which the globe moves smoothly and under
  scrubbing; `tools/playground/dated_check.cjs` found them where they were to a metre. The vine's stems,
  regrown by `terra.geo.set` on every `time` event, are the same idea for lines: dated geography, built on
  claude/georgia with the vine's remaking.
- **Script helpers repeated in packs** (merged). The great-circle `along` (the lead line, the monsoon,
  the whales; the herds' `slerp`) and `frames` (five packs, and the vine's own until it moves onto it with
  its remaking) are `Shared.along` and `Shared.frames` in the packs' shared script, bit for bit what they
  replaced. (The gatherings' `along` is
  another thing: how far along a river a touch lies.)
- **Three hand-written PNG encoders** (`glint_png.py`, `marker_png.py`, `harness_check.cjs`'s
  fixture pictures): small and standard-library by design; worth one shared helper only when a
  fourth appears.
- **Not duplicates:** the glint drawn in Python for packs and in Swift for natives (one recipe, two
  hosts); `check.cjs` (any pack, fixed shots) beside `alive_check.cjs` (one pack's behaviour asserted);
  `tools/clouds/compose.py`, already shared by the clouds' and the ocean's looks.
