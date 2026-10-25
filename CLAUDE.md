# Terra Incognita — engineering notes

A demo app: a medieval world map drawn in characters (WebGL2 in a WKWebView), with a six-minute
tour that plays on launch. This file is the map and the rules.

## Map

- `Web/`: everything you see. `index.html` (overlay: captions, chapters, Resume, style menu,
  cards), `render.js` (WebGL2: the cell pass writes each cell's character, colour and ground into
  two small textures; the glyph pass paints them from a font atlas; a mix pass blends two frames
  for transitions; the inks), `cell.glsl` (one fragment per character cell: inverts the
  projection, samples the textures, picks strokes from a line's direction, engraves the
  creatures, the Kells treatments), `scene.js` (projections forward and inverse, the camera as a
  look of lon/lat/zoom, lettering, sprites, hidden things), `tour.js` (the script, its clock, the
  reader's fingers, the UI), `main.js` (start-up and the loop). `#t213` in the URL plays from
  213 s, `#t213p` waits there, `#t213s` waits and stops drawing after two seconds; the page keeps
  `#r<T>` in its address so a reload resumes the film.
- `Web/data/` is built by `tools/build_textures.py` (its docstring is the texture layout, which
  `cell.glsl`'s decoders mirror): rebuild, never hand-edit. `Web/fonts/` are OFL fonts, bundled.
- `App/`: `TerraIncognita.xcodeproj` (synchronized folder `TerraIncognita/`; `Web` is a folder
  reference copied into the bundle). Debug builds take `-tour t213s` (handed to the page as
  `window.TI_TOUR`).
- `ci/` and `.github/workflows/ci.yml`: `web` (headless Chromium check and screenshots, and a
  WebKit pass that reports without gating), `apple` (Debug and Release builds, the bundled data
  compared byte for byte, simulator screenshots as a report: hosted runners have no GPU, so the
  simulator draws WebGL on the CPU and is too slow to gate on), `ipa` (on `main`: the
  `build-<n>` prerelease and the SideStore source on the `sidestore` branch; CI owns that branch),
  `report` (to `refs/ci-shots/<branch>`; `CI_BRANCH=<branch> ci/fetch.sh <sha> <dir>` waits for it).

## Rules

- Public repository. Plans, briefs and the backlog live in the owner's private company repository;
  a commit message says what changed and why and quotes no one.
- No network: the page loads only from the bundle; no analytics, no accounts.
- The macOS runners are shared with paint-by-number and pigeon: look at their Actions runs before
  a push, one branch at a time while paint-by-number's `main` runs.
- Name, icon and listing are the owner's to decide (the icon is a placeholder).
- Times given to the owner are Eastern (ET).
