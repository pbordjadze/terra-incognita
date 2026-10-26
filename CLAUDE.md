# Terra Incognita — engineering notes

A demo app: a medieval map that plays as a film, from the whole world down to a street, each scale a
finished picture in the chosen look. This file is the map and the rules.

## Map

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
  - `AtlasScreen`: SwiftUI around it.
  - Debug builds take `-film 21.5` (the film held at that moment, for CI's screenshots).
- **The globe** (`App/TerraIncognita/Globe/`, the button beside the look menu): the whole Earth
  drawn live, in the colours of Wikipedia's location maps (WikiProject Maps), a little prettier.
  - `Globe.metal`: one fragment per pixel inverts the orthographic projection and samples the
    map's distance fields, so coasts, borders and rivers stay a constant width at any zoom.
    `tools/globe/preview.py` is the same, line for line, in numpy: change both together, and look at
    the preview before CI.
  - `GlobeView`: the camera (drag, pinch, fling, double tap; it turns slowly when left alone),
    tapping a country (the locator's red, from the country map `ids.png`), the names (CATextLayers
    placed as the camera moves, in order of claim: oceans, seas, countries, capitals; none
    overlapping, none past the globe's edge). `GlobeScreen`: its caption. Made the first time it is
    opened and kept (its maps decoded once), paused while the film shows.
  - The data, `Globe/`: built by `tools/globe/build.py` from Natural Earth and NASA's GEBCO (its
    docstring is the textures' layout, which the shader decodes). Rebuild, never hand-edit. The
    textures are uploaded as decoded, without colour management, which would bend the distances.
  - Debug builds take `-globe` (open on it), `-globe-at lon lat zoom` (held there) and
    `-globe-select <name>`.
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
- **CI** (`ci/` and `.github/workflows/ci.yml`):
  - `plates`: every look in `Atlas/` checked by `ci/check_atlas.py` (what the player would trap on,
    and no unused file in the bundle), the globe's data by `ci/check_globe.py`, and the tools' syntax.
  - `apple`: Debug and Release builds. The plates and the globe's maps are compared byte for byte
    in the bundle. Simulator screenshots of every look's film at moments, and of the globe, run only
    when the workflow is started by hand (`ci/sim_shots.sh`).
  - `ipa`: on `main`, the `build-<n>` prerelease and the SideStore source on the `sidestore`
    branch. CI owns that branch.
  - `report`: to `refs/ci-shots/<branch>`. `CI_BRANCH=<branch> ci/fetch.sh <sha> <dir>` waits for it.

## Rules

- Public repository. Plans, briefs and the backlog live in the owner's private company repository;
  a commit message says what changed and why and quotes no one.
- No network: everything loads from the bundle; no analytics, no accounts.
- The macOS runners are shared with paint-by-number and pigeon: look at their Actions runs before
  a push, one branch at a time while paint-by-number's `main` runs.
- Name, icon and listing are the owner's to decide (the icon is a placeholder).
- Times given to the owner are Eastern (ET).
