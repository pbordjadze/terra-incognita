# Terra Incognita — engineering notes

A playground for a wandering atlas: the app opens on a live globe in Wikipedia's location-map style,
and a developer workbench starts one prototype at a time on it or beside it. Earlier directions (the
Persian style, the plate films) are parked prototypes, kept whole. This file is the map and the rules.

## Documentation scope

This file describes the implemented demo and its engineering constraints. For current product
direction, read `terra-incognita/DESIGN.md` and its linked latest conversation capture in
`pbordjadze/painting-company` before design work. Existing style descriptions and label-placement
rules below document the implementation; they do not select the next product treatment.
Keep design discussions and plans in that private repository.

## Map

- **The playground** (`App/TerraIncognita/Playground/`; how to add a prototype: `docs/playground.md`).
  `PlaygroundRoot` is the app: the globe, the workbench button (top right) and, while a prototype
  is active, its chip (Open, ✕). `Playground` holds the one active `PrototypeStage`, which owns
  everything its prototype adds (markers, a style, an overlay, a screen, an invitation) and takes
  it all away when it ends: one at a time is the workbench's isolation rule. `Catalog.swift` lists
  the native prototypes, one entry each; web packs (`Prototypes/<id>/`, and remote catalogs) are
  found by `PackStore` and hosted by `WebPack.swift` (scheme `terra-pack`, bridge
  `Prototypes/_runtime/terra.js`, runtime 1). `GlobeMarker`s follow the geography on the globe.
  The workbench's Live line counts stages, markers, globes, films and web views: ending a
  prototype returns them to one globe and nothing else. `WORKBENCH` (build settings) compiles it in.
  - Debug builds take `-prototype <id>` (`-prototype-open` opens its screen), `-workbench`,
    `-packs-url <catalog>`; the older `-camelid`, `-whale`, `-leaf`, `-look`/`-film`,
    `-globeStyle persian` start the matching prototype.
  - Links: `terraincognita://open/<id>`, `terraincognita://source?url=<catalog>` (asks first).
  - Tools: `tools/playground/` (`playground.py new|validate|pack|publish|retire|list`,
    `serve.cjs` previews over a stand-in globe, `check.cjs` checks one in Chromium).
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
  drawn live, in two styles (the Persian one is the parked `persian` prototype):
  Wikipedia's location-map colours (WikiProject Maps), a little prettier, and the Persian, the same
  map in colours and a frame studied from a 1529 Persian manuscript (`Globe.metal`'s `P_` palette).
  A change of style crossfades.
  - `Globe.metal`: one fragment per pixel inverts the orthographic projection and samples the
    map's distance fields, so coasts, borders and rivers stay a constant width at any zoom; both
    styles, mixed by the style uniform. `tools/globe/preview.py` is the same, line for line, in
    numpy (`--style persian`; iPad-sized by default): change both together, and look at the preview
    before CI.
  - `GlobeView`: the camera (drag, pinch, fling, double tap; it turns slowly when left alone),
    tapping a country (lit from the country map `ids.png`), the names (pictures with their halos
    drawn in, placed as the camera moves by the design rules below). `GlobeScreen`: its caption (in
    the Persian style, a headpiece with the name in its cartouche), and an active prototype's
    invitation above it. Made at launch and kept (its maps decoded once), paused while a screen
    or the workbench covers it.
  - The data, `Globe/`: built by `tools/globe/build.py` from Natural Earth and NASA's GEBCO (its
    docstring is the textures' layout, which the shader decodes). Rebuild, never hand-edit. The
    textures are uploaded as decoded, without colour management, which would bend the distances.
    The Persian ornaments (band, finial, rosette, headpiece) are drawn from code by
    `tools/globe/ornament.py` (its docstring is their layout; no source image is used), and
    `type.json` is the type (Design rules).
  - Debug builds take `-globe-at lon lat zoom` (held there) and `-globe-select <name>`
    (`-globe` is accepted and changes nothing: the globe is always the opening view).
- **The Aegean leaf** (`Leaf/`, `App/TerraIncognita/Leaf/`): an offline HTML/canvas leaf in an ephemeral
  WKWebView, the `leaf` prototype (its own screen). The Metal globe and plate player are suspended while it
  is open. The host supplies system text size, Reduce Motion and scene activity and permits only
  bundled navigation. `Leaf/SOURCES.md` records the data and schematic qualifications.
  - `tools/aegean/build.py <cache>` prepares the committed chart from public source rasters and
    vectors; never hand-edit `Leaf/data/chart.js`. Then run `tools/aegean/paint.cjs` against the local
    server to prepare both matching engraving images. Runtime uses no fetch, external service or storage.
  - `node tools/aegean/serve.cjs` previews it locally; `node tools/aegean/check.cjs` exercises the actual
    browser gestures, camera continuity, keyboard alternatives, stillness and enlarged text.
    `ci/check_leaf.py` checks bundle completeness and the offline boundary. Native shots use `-leaf`;
    manual CI can select `leaf_only` to avoid photographing every film and globe style.
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
  off Madagascar and an invitation), the river study (`Water/`, `App/TerraIncognita/Water/`, `water`;
  `docs/water-engineering.md`).
- **Playground checks** (`App/TerraIncognitaUITests/PlaygroundTests.swift`, `RemotePackTests.swift`,
  helpers in `PlaygroundSupport.swift`): the plain launch, two dissimilar prototypes in succession
  with the live counts back to the plain globe's, parked work, the bundled pack, and a remote
  revision served by `tools/playground/serve.cjs` arriving, rolling back and opening offline.
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
  - `packs.yml` (Linux only): a push that changes only web packs skips the macOS workflow; this one
    validates them and, once R2 is configured (repository secrets), publishes changed packs as new
    revisions (`[no-publish]` holds it back). Nothing is published without those secrets.

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
  pack's page can reach nothing outside its pack. No analytics, no accounts.
- Prototypes are additive: a new one adds a folder and an entry, and changes no other. Park, don't
  delete; ending a prototype must leave the plain globe as it was (the Live counts say so).
- The macOS runners are shared with paint-by-number and pigeon: look at their Actions runs before
  a push, one branch at a time while paint-by-number's `main` runs.
- Name, icon and listing are the owner's to decide (the icon is a placeholder).
- Times given to the owner are Eastern (ET).
