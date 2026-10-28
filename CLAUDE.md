# Terra Incognita — engineering notes

A playground for a wandering atlas: the app opens on a live globe, the ground as satellites saw it
this month in a light key (the Wikipedia style it opened on before is parked), with each prototype's
doorway on it, and a developer workbench starts one prototype at a time. Earlier directions (the
Persian style, the plate films) are parked prototypes, kept whole. This file is the map and the rules.

## Documentation scope

This file describes the implemented demo and its engineering constraints. For current product
direction, read `terra-incognita/DESIGN.md` and its linked latest conversation capture in
`pbordjadze/painting-company` before design work. Existing style descriptions and label-placement
rules below document the implementation; they do not select the next product treatment.
Keep design discussions and plans in that private repository.

## Map

- **The playground** (`App/TerraIncognita/Playground/`; the pack contract: `docs/runtime3.md`; the
  workflow: `docs/playground.md`). `PlaygroundRoot` is the app: the globe with every current
  prototype's doorway on it (hidden at the whole-Earth view; touching one starts it), the workbench
  button (top right) and, while a prototype is active, its chip (Open, ✕) and its panel. `Playground`
  holds the one active `PrototypeStage`, which owns everything its prototype adds (markers and
  sprites, a pack's script, geography and pictures drawn by the globe, a date, words at the foot, a
  panel, a style, a screen) and takes it all away when it ends: one at a time is the workbench's
  isolation rule. `Catalog.swift` lists the native prototypes, one entry each; packs
  (`Prototypes/<id>/`, and remote catalogs) are found by `PackStore`, and their scripts run in
  JavaScriptCore in `PackScript.swift` (runtime 3, `Prototypes/_runtime/terra.js`; no web view). The
  globe draws for them: `GeoLayers.swift` (geography, touchable), `GlobeImages.swift` (pictures lying
  on the globe), `GlobeMarker` sprites (paths, frames, a look per zoom), `PackPanel.swift`. The
  workbench's Live line counts stages, markers, globes, films and scripts: ending a prototype returns
  them to one globe and nothing else. `WORKBENCH` (build settings) compiles it in.
  - Debug builds take `-prototype <id>` (`-prototype-open` opens its screen), `-workbench`,
    `-packs-url <catalog>`; the older `-camelid`, `-whale`, `-leaf` (the sea between), `-look`/`-film`,
    `-globeStyle persian` start the matching prototype.
  - Links: `terraincognita://open/<id>`, `terraincognita://source?url=<catalog>` (asks first).
  - Tools: `tools/playground/` (`playground.py new|validate|pack|publish|retire|list`,
    `serve.cjs` runs a pack over a stand-in globe in a browser, `check.cjs` checks one headless and
    screenshots it).
- **The player** (`App/TerraIncognita/Atlas/`). A look is a ladder of *plates*, portrait pictures,
  each sitting inside the one before at its `child` rectangle.
  - `AtlasCanvas`: draws at most three plates as Core Animation layers (the camera's, its parent
    while it doesn't cover the screen, its child fading up) and only moves and scales them each
    display-link frame. That is what keeps it at 120 Hz: never draw or lay out per frame.
  - `Camera.swift`: `Ladder` converts a camera between nested plates, `normalized` keeps the film's
    camera in the deepest opaque one, and `Film` is the drifts and dives (`cut`: the same film under
    Reduce Motion, each dive a dip through the background).
  - Free mode keeps the reader's camera in the deepest plate the view lies wholly inside
    (`contained`; only the first plate clamps), and pinching in past a plate's rest slides the view
    towards the closer plate so it arrives inside it at the hand-over (`steered`): no jumps.
  - `Plates.swift`: decodes `Atlas/`.
  - `FilmScreen`: SwiftUI around it, the parked `film` prototype: made when opened (no plate is
    decoded for the plain globe), with its look menu and Back.
  - Debug builds take `-film 21.5` (the film held at that moment, for CI's screenshots).
- **The globe** (`App/TerraIncognita/Globe/`, the opening view): the whole Earth
  drawn live, in three styles. The ground (`GlobeStyle.base`, the plain globe's since October 28,
  2026; `Globe.metal`'s `G_` palette): the month's land as NASA's Blue Marble saw it, in a light
  key, the sea by depth with its floor's relief and the month's usual sea ice, a haze at the limb,
  no names at the whole Earth and the land's own names (`Globe/regions.json`) before the
  countries'. Wikipedia's location-map colours (WikiProject Maps), a little prettier: the parked
  `wikipedia` prototype. The Persian, the same map in colours and a frame studied from a 1529
  Persian manuscript (`P_`): the parked `persian` prototype. A change of style crossfades.
  - `Globe.metal`: one fragment per pixel inverts the orthographic projection and samples the
    map's distance fields, so coasts, borders and rivers stay a constant width at any zoom; the
    styles mixed by their uniforms. `tools/globe/preview.py` is the same, line for line, in
    numpy (`--style persian`, `--style ground --month 7`; iPad-sized by default): change both
    together, and look at the preview before CI.
  - `GlobeView`: the camera (drag, pinch, fling, double tap; it turns slowly when left alone),
    tapping a country (lit from the country map `ids.png`), the names (pictures with their halos
    drawn in, placed as the camera moves by the design rules below). `GlobeScreen`: its caption (in
    the Persian style, a headpiece with the name in its cartouche), and an active prototype's
    invitation above it. Made at launch and kept (its maps decoded once), paused while a screen
    or the workbench covers it.
  - The data, `Globe/`: built by `tools/globe/build.py` from Natural Earth and GEBCO, as NASA Visible Earth's imagery (its
    docstring is the textures' layout, which the shader decodes). Rebuild, never hand-edit. The
    textures are uploaded as decoded, without colour management, which would bend the distances.
    The Persian ornaments (band, finial, rosette, headpiece) are drawn from code by
    `tools/globe/ornament.py` (its docstring is their layout; no source image is used), and
    `type.json` is the type (Design rules). The ground's maps (`Globe/ground/`: twelve months'
    land, their sea ice, the sea floor) and `regions.json` are built by `tools/globe/ground.py`
    (its docstring is their layout and sources); GlobeView reads them only once the style is chosen.
  - Debug builds take `-globe-at lon lat zoom` (held there), `-globe-select <name>`, `-globe-month n`
    (the ground's month)
    (`-globe` is accepted and changes nothing: the globe is always the opening view).
- **The sea between** (`Prototypes/sea-between/`, a runtime-3 pack): the engraved Aegean leaf lying
  on the globe, its drawn life as sprites, its checked notes in the panel, and a sounding glass
  (`SOURCES.md` in the pack). `-leaf` starts it. It is built from the former web leaf in `Leaf/`, which
  is no longer in the app and stays as its source: `tools/aegean/build.py <cache>` prepares
  `Leaf/data/chart.js` (never hand-edit), `tools/aegean/paint.cjs` its engravings, and
  `tools/aegean/pack.cjs` with `pack_engraving.py` the pack from them.
- **The camelid sequence** (`App/TerraIncognita/Camelid/`, the `camelid` prototype): the illustrated camel in Arabia opens
  an authored camera sequence on the same Metal globe. `CamelidStory` holds playback state;
  `CamelidCanvas` composes cached animal images and projected dispersal bands as native layers.
  Pan, pinch or double-tap pauses the clock and hides the authored composition until resumed.
  Prehistoric scenes hide modern politics and explicitly identify modern geography as a reference.
  `CamelidAnimals` in the asset catalog is original generated art, cropped once by `CamelidArt`.
  Optional evidence and the textual equivalent are bundled in `CamelidEvidence`.
  Debug: `-camelid <seconds>` holds a complete composition, `-camelid-play` runs playback, and
  `-camelid-reduce-motion` exercises held camera compositions. `ci/camelid_capture.sh` captures
  actual simulator output after the interaction suite; `[camelid-capture]` requests this report-only
  step in the ordinary macOS job. Its outcome and command diagnostics are preserved separately. Branch
  `[ipa]` builds have a separate downloadable IPA artifact and never update the shared source.
- **The clouds** (`App/TerraIncognita/Clouds/`), part of the stage: the clouds of the globe's month
  (`GlobeView.month`, the ground's; a pack's date to the day moves it), where satellites usually see
  them (NOAA's ISCCP-H, 2000–2016), drawn as weather that could be; zooming in passes down through
  them. `CloudsLayer` is the globe's `sky`, a Metal layer of its own over the globe's drawing and
  under everything else on it (names, markers, what experiences draw), drawn as the globe ticks (ten
  times a second while the camera rests), so `Globe.metal` is untouched. It draws at a fraction of
  the screen's pixels and follows the GPU's timings (the workbench's log says what a frame costs).
  `CloudsLook.on(style)` says how they sit on each globe style: near-black shadows that darken any
  ground, and none on the Persian page. The reader's switch is the cloud button beside the
  workbench's (`CloudsSwitch`, remembered); while an experience runs they thin to a veil (`veiled`),
  and they fade out for the authored films. `Clouds.metal` and its web twin
  `tools/clouds/web/shader.js` are the same line for line: change both together (the twin is the
  desktop preview: `tools/clouds/look.cjs` and `film.cjs` draw it over the globe's preview).
  `tools/clouds/build.py` makes `CloudsSky.png` and `CloudsLand.png` for both; never hand-edit them;
  `tools/clouds/SOURCES.md` separates what is measured from what is drawn. Debug: `-clouds-days <d>`
  holds the drift, `-clouds-month <1-12>` the month.
- **The whale sequence** (`App/TerraIncognita/Whale/`, the `whale` prototype): a regional fossil illustration at Wadi
  Al-Hitan opens an authored sequence on the same globe. The entry appears from 2× scale; the
  map's accessibility action “Go to Whale fossil, Wadi Al-Hitan” moves there without starting playback.
  Debug: `-whale <seconds>` holds a composition and `-whale-play` runs the native clock.
  `ci/whale_capture.sh` captures selected simulator compositions and an optional full phone video
  using unique first-frame and authored-clock-completion acknowledgements, rather than guessed
  launch or ending delays. These are not frame-rate measurements. The quiet geographic view
  during the large time jump is intentional.
  Manual CI can select `interaction_suite=whale` on a branch for focused `WhaleEncounterTests`,
  and `whale_capture=true` requests report-only capture after interactions. Pushes with
  `[whale-capture]` request the same capture. Ordinary pushes and main always run all interaction
  suites; focused branch checks omit duplicate film/globe/leaf screenshots. Branch `[ipa]` builds
  are available in the `prototype-device-ipa` artifact without changing the shared SideStore source.
- **The other studies**: the Indian Ocean encounter (`App/TerraIncognita/Experience/`, `ocean`: a seed
  off Madagascar and an invitation), the river study **A gathering of
  water** (`Prototypes/gathering-of-water/`, a runtime-3 pack: South America's rivers, touchable, each
  traced to the sea), built by `tools/water/build_pack.py` from the former web study's data in
  `Water/` (no longer in the app; `docs/water-engineering.md`).
- **Playground checks** (`App/TerraIncognitaUITests/PlaygroundTests.swift`, `RemotePackTests.swift`,
  `GlobePackTests.swift`, helpers in `PlaygroundSupport.swift`): the plain launch, two dissimilar
  prototypes in succession with the live counts back to the plain globe's, parked work, the bundled
  pack, a remote revision served by `tools/playground/serve.cjs` arriving, rolling back and opening
  offline, and the runtime-2 packs drawn by the globe (their date moved, nothing left after).
  `[playground-check]` (or `interaction_suite=playground`) runs only these on a branch; main always
  runs every interaction suite before publishing the ordinary SideStore update.
- **The plates** (`Atlas/`). `looks.json` and `<look>/plates.json`, with layer images, written by
  `node tools/plates/export.cjs <look> "<Name>"` from the look's page `tools/plates/<look>/index.html`
  (the page contract is in `tools/plates/kit.js`). Re-export, never hand-edit.
  - Between scales more than 4× apart, the exporter adds pass plates (the wider scale magnified about
    the closer one), so no picture is blown up far before the next takes over. The film dives
    straight through them.
  - Plates are 1638×3545: about 1.4 phone screens at 3×.
  - A page that draws on a canvas paints pass plates itself at their magnification (the exporter
    passes `magnify`, `cx`, `cy`; the page sets `window.platesMagnifies`), so they stay sharp.
  - Preview a look's film without the phone: `python3 tools/plates/preview.py <look> --times …`
    (frames) or `--video` (no gold sheen).
- **Map data** (`tools/plates/data/`). Built by `tools/plates/build_data.py` (sources in its
  docstring). d3 needs polygon exteriors clockwise; the build fixes winding after rounding.
  - The closest scale in the app is a public neighbourhood (Short North). A private one never comes
    into this public repository.
- **The first demo**, the map in dense characters by WebGL in a web view, was retired on October 26,
  2026. Its last state is in `main`'s history at 1f9b941 (`git show 1f9b941:Web/index.html`; also
  `MapView.swift`, `BundleScheme.swift`, `PageStatus.swift`, `tools/build_textures.py`,
  `tools/sprites.py`, `ci/web_check.cjs`).
- **CI** (`ci/` and `.github/workflows/ci.yml`; tags: `[build-only]`, `[playground-check]`, `[ipa]`):
  - `plates`: every look in `Atlas/` checked by `ci/check_atlas.py` (what the player would trap on,
    and no unused file in the bundle), the globe's data by `ci/check_globe.py`, and the tools' syntax.
  - `apple`: Debug and Release builds. The plates and the globe's maps are compared byte for byte
    in the bundle. Simulator screenshots run only when the workflow is started by hand
    (`ci/sim_shots.sh`): on the 13" iPad Pro (the primary device; `device` also takes `iphone` or
    `both`), every look's film at moments (unless `film` is off) and the globe in each style.
  - `ipa`: on `main`, the `build-<n>` prerelease and the SideStore source on the `sidestore`
    branch. CI owns that branch.
  - `report`: to `refs/ci-shots/<branch>`. `CI_BRANCH=<branch> ci/fetch.sh <sha> <dir>` waits for it.
  - `qa-explore.yml`, exploratory QA on request: an agent uses the app in a simulator through
    Maestro's MCP server and reports (`refs/qa-runs/<run id>`); `ci/qa/README.md` says how. It
    and `ci/qa/` are generated from the copy all the apps share (painting-company row 152):
    change them there and sync, never here.
  - `packs.yml` (Linux only): a push that changes only packs skips the macOS workflow; this one
    validates them and, on `main`, publishes changed packs as new revisions to the `packs` branch,
    which the app reads through raw.githubusercontent.com (`[no-publish]` holds it back).

## The runtime: one library that grows

Every new experience, and every new element on the map, is built on the playground's runtime: a
pack using the tools the runtime offers (the plan is painting-company row 154). Built into the app
instead: the stage itself (the globe, camera, date, caption, touch routing, workbench, downloads)
and what every experience relies on and none owns. When in doubt, it's runtime.

- **Reuse first.** Read what the runtime offers (`docs/playground.md` › The contract, and the
  shared script modules once there are any) before building.
- **Missing? Build it into the runtime,** shaped so the next experience can use it. What exists
  never limits an experience: extend a tool or add one beside it. No tool is built ahead of an
  experience that needs it.
- **No web views.** Nothing new runs or draws in a web view. If a tool is missing, build it
  (script, or Swift with the next release) and let the experience wait for it. Until runtime 3
  (pack scripts in JavaScriptCore) exists, the next new experience starts by building it.
- **Script before native.** A tool goes in shared script when it can (it ships with packs, no
  IPA); in Swift (a new `terra.*` capability, raising the runtime version) only when it must draw
  or animate every frame, take touches, be fast, or reach the system. The script decides; the
  globe draws.
- **Whoever changes a tool updates everything that uses it, in the same change.** A tool with one
  user may change freely; with more, only compatibly. Packs on older runtime versions keep working
  until they have moved.
- **A tool lands complete:** in the contract with an example, tested, mirrored in the preview
  (`serve.cjs`) so an agent can see it without a Mac, and accessible inside the tool (VoiceOver,
  text size, Reduce Motion).
- **Many at once.** A tool assumes other experiences may share the stage: it asks for the camera,
  date and touches rather than taking them, and leaves nothing behind when its experience ends.
- **Runtime 3 is here** (`docs/runtime3.md`): packs run in JavaScriptCore and the globe draws for
  them. There are no web views in the app: the Aegean leaf and the river study are packs now. The
  native prototypes move onto the runtime's tools when they are next reworked.

## Design rules

The globe is for reading, on the iPad first, then the iPhone. The numbers live in one spec,
`Globe/type.json`, which `GlobeView` and `GlobeScreen` read, `tools/globe/preview.py` mirrors, and
`ci/check_globe.py` holds to these rules (fonts bundled, sizes, contrast): change the spec, not the
code, and let the check tell you when a choice breaks a rule.

- **Scale for the reader's eye, not the pixels.** A larger screen is held further away and its
  points are physically larger, so lines and names are drawn at a reading scale that grows with
  the screen's shorter side (1.0 on a phone, 1.2 on the 13-inch iPad; `reading` in the spec). It
  multiplies the names, their halos and gaps, the touch reach, and every line's width (the shader's
  pixels per point are pixels per reading point). The reader's text size enlarges the names on
  top, up to 1.5×. Captions follow the reader's text size only (as system text does) and run at
  most 560 pt wide, a readable line.
- **Hierarchy by importance.** Oceans, then seas, then countries in three sizes by label rank (the
  most prominent largest), then capitals; claimed in that order, so when room runs short the less
  important names give way first, never the other way round.
- **Every name on a halo of its ground.** The sea's or the land's tone, round-joined, 1.5 reading
  points each side, so no coast, border or graticule line crosses a letter.
- **Contrast.** Every ink at least 4.5:1 against what is round it (WCAG AA): a name's halo, a
  caption's ground, the cartouche's gold; large text (18 pt, or 14 pt bold, iOS points taken as
  WCAG's) at least 3:1, which lets the Persian rubric keep the manuscript's vermilion by being set
  large. The check computes it, and fails a spec the app couldn't read (a missing field, a style, a
  font not in `Globe/fonts/`, which the app registers whole).
- **Readable or not shown.** A name is fully opaque or absent; it fades only in time, never as a
  steady half-tone because of where it is. Its cut-off near the limb has slack (shown above 0.4 of
  facing, kept down to 0.3), so it doesn't flicker there. Nothing is under 10 pt; a title shrinking
  to fit stops at 13 pt (the cartouche letters the shorter name, so it rarely needs to).
- **On its ground, with room.** A name lies wholly on the globe and on screen, never on the frame,
  the paper, the caption or the buttons (`GlobeView.keepClear`), and keeps 0.4 of its size clear
  of any other.
- **The globe fills its room.** As large as the space between the buttons and the caption allows,
  frame included; that space is measured when the screen's size or the style changes, never when a
  caption's text does, so a tap never moves the globe.
- **Touch.** A tap on the sea within 10 reading points of a country chooses it: small islands and
  states are smaller than a fingertip.
- **Language.** Real names in English, in Libertinus (Uncial only for Wikipedia's seas); capitals
  letter-spaced; no invented or decorative script.

## The Persian style's sources

Every choice in the Persian style traces to one of two references; nothing is a default.

- **The manuscript**: LACMA's Khamsa of Nizami (M.85.237.16, Tabriz, 1529), its illuminated
  opening and its landscapes, inspected in LACMA's scans (no pixel of them is in the app). Its
  colours are sampled from the scans and white-balanced by one gain that makes its lead-white
  pigment neutral (the varnish's cast is not the painter's): paper F2E4C8, lapis 5F6CA7, gold
  A08C60, the cartouche's gold 9D8A57 (both golds then lifted, +0.10 OKLab lightness and chroma
  x1.3, to C5AA6E and C2A862: a scan shows shell gold unlit and olive), salmon C47C66, pink
  D1A38F, green 79A18B, light blue A2B7D0 (lapis let down with white: the hue and chroma of every
  water tint), black 251F14, white F5F5F5, the margin's blue ink 6D6793, the text's ink 3B2C16, the
  rubrics' vermilion DD6C51, the landscapes' meadow CCD6C8 and rocks CCBDCB (shadows A298A7).
- **Wikipedia's map conventions** (WikiProject Maps, location and topographic maps): water C6ECFF
  and its depth ramp from D8F2FE, coasts, rivers and water names 0978AB, borders and country
  names 646464, land F6E1B9, the highest tint F5F4F2; country names in capitals, water names in
  italic.

How they combine: the map's tints and lines take their lightness from the Wikipedia entry for
their kind and their hue (and chroma) from the manuscript colour for it, so the map keeps
Wikipedia's tonal design and reads like it; a name's ink may then be darkened, never hue-shifted,
only as far as the contrast rule needs. The frame's rulings follow the order and proportions
measured across the opening spread's frame, from its text panel out (Globe.metal's `P_` frame).
On the map itself, what is drawn on the paper uses the margin's vocabulary (blue ink: coasts,
rivers, finials, the capitals' rosettes); illumination (gold, lapis, salmon) stays in the frame,
the caption's headpiece, and the one chosen country, gilded as the cartouche. The gold is matte
(lifted, as above, but never shining): no sheen, no shadows, no vignette (the manuscript has none).

## Rules

- Public repository. Plans, briefs and the backlog live in the owner's private company repository;
  a commit message says what changed and why and quotes no one.
- No network, with one deliberate exception: the workbench fetches prototype packs from its
  configured sources while it is open, and when a pack is opened (HTTPS, plain GETs, an ephemeral
  session, hashes checked). The globe and every bundled experience load from the bundle, and a
  pack's script can reach nothing outside its pack. No analytics, no accounts.
- Prototypes are additive: a new one adds a folder and an entry, and changes no other. Park, don't
  delete; ending a prototype must leave the plain globe as it was (the Live counts say so).
- The macOS runners are shared with paint-by-number and pigeon: look at their Actions runs before
  a push, one branch at a time while paint-by-number's `main` runs.
- Name, icon and listing are the owner's to decide (the icon is a placeholder).
- Times given to the owner are Eastern (ET).
