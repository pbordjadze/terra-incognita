# Terra Incognita — engineering notes

A demo app: a medieval map that plays as a film, from the whole world down to a street, each scale a
finished picture in the chosen look. This file is the map and the rules.

## Map

- **The player** (`App/TerraIncognita/Atlas/`). A look is a ladder of *plates*, portrait pictures,
  each sitting inside the one before at its `child` rectangle.
  - `AtlasCanvas`: draws at most three plates as Core Animation layers (the camera's, its parent
    while it doesn't cover the screen, its child fading up) and only moves and scales them each
    display-link frame. That is what keeps it at 120 Hz: never draw or lay out per frame.
  - `Camera.swift`: `Ladder` converts a camera between nested plates, `normalized` keeps it in the
    deepest opaque one, and `Film` is the drifts and dives.
  - `Plates.swift`: decodes `Atlas/`.
  - `AtlasScreen`: SwiftUI around it.
  - Debug builds take `-film 21.5` (the film held at that moment, for CI's screenshots).
- **The plates** (`Atlas/`). `looks.json` and `<look>/plates.json`, with layer images, written by
  `node tools/plates/export.cjs <look> "<Name>"` from the look's page `tools/plates/<look>/index.html`
  (the page contract is in `tools/plates/kit.js`). Re-export, never hand-edit.
  - Between scales more than 4× apart, the exporter adds pass plates (the wider scale magnified about
    the closer one), so no picture is blown up far before the next takes over. The film dives
    straight through them.
  - Plates are 1638×3545: about 1.4 phone screens at 3×.
- **Map data** (`tools/plates/data/`). Built by `tools/plates/build_data.py` (sources in its
  docstring). d3 needs polygon exteriors clockwise; the build fixes winding after rounding.
  - The closest scale in the app is a public neighbourhood (Short North). A private one never comes
    into this public repository.
- **The first demo** (`Web/`, `MapView.swift`, `BundleScheme.swift`, `PageStatus.swift`): the map in
  dense characters by WebGL, no longer the app's screen.
  - `cell.glsl` must stay quick for a phone's GPU compiler, which builds it at the first frame while
    iOS watches WebKit's GPU process: a heavy function gets one call site, and a loop over a heavy
    body takes its count from a uniform (`uZero`).
- **CI** (`ci/` and `.github/workflows/ci.yml`):
  - `web`: headless Chromium check of `Web/`, plus a WebKit report.
  - `apple`: Debug and Release builds. The plates and `Web/data` are compared byte for byte in the
    bundle. Simulator screenshots of the film at moments run only when the workflow is started by
    hand (`ci/sim_shots.sh`).
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
