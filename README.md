# Terra Incognita

A medieval map that plays as a film: from the whole world down to a single street, through every
scale in between, each scale a finished picture in the chosen look. The app opens on its globe;
the Film button opens the flight. Touch the map to wander, then resume. The look menu switches
between the looks on the same flight.

- `App/TerraIncognita/Atlas/`: the player. `AtlasCanvas` (Core Animation layers moved and scaled by
  a display link; the camera, the film, pan and pinch), `Camera.swift` (nested plates as one
  continuous zoom, the film's shots), `Plates.swift` (the bundled `Atlas/`), `AtlasScreen` (captions,
  the look menu, Resume).
- `Atlas/`: each look's plates, made by `tools/plates/export.cjs` from the look's page
  (`tools/plates/<look>/index.html`; the contract is in `tools/plates/kit.js`).
- `tools/plates/build_data.py`: the map data the pages draw from, in `tools/plates/data/`.
- `App/TerraIncognita/Globe/`: the whole globe, drawn live by Metal in the colours of Wikipedia's
  location maps; spin it, pinch it, tap a country. Its data, `Globe/`, is made by
  `tools/globe/build.py`.
- The first demo, the map drawn in characters by WebGL, is retired; it is in the history at 1f9b941.

Install from SideStore: add `https://raw.githubusercontent.com/pbordjadze/terra-incognita/sidestore/source.json`.

The atlas opens on the globe. A small seed mark and “Look closer” open one offline historical
encounter: move a magnifying glass over the Indian Ocean, draw back a layer of evidence, and
follow words or lives. Evidence and limitations are available inside the encounter; the research
ledger is [docs/encounter-research.md](docs/encounter-research.md). The film remains behind its button.

The looks: Aisling's Forest, Paper Almanac, Night Voyage, Chalk and Slate, and The Sampler.

Map data: Natural Earth and US Census TIGER/Line (public domain); © OpenStreetMap contributors
(ODbL), for the streets and buildings of the closest scale; NASA Blue Marble (public domain), for
where the forest look's woods grow; NASA's GEBCO elevation and bathymetry (public domain), for the
globe's relief and seas. Fonts (SIL Open Font License, their licences beside them in
each look's folder and the globe's): IM Fell English, EB Garamond, Uncial Antiqua, Alegreya, Libre
Caslon Text, Cormorant and Libertinus Serif. The globe follows the WikiProject Maps conventions for
location maps; it is not affiliated with Wikimedia. Nothing leaves the device.

An offline Aegean atlas leaf is available from the leaf button beside the globe controls. Its
bundled physical map, optional notes and source qualifications are under `Leaf/`; the native host
is `App/TerraIncognita/Leaf/LeafScreen.swift`. Local preview: `node tools/aegean/serve.cjs`, then open
`http://127.0.0.1:8787`. With Playwright and Chromium available, `node tools/aegean/check.cjs` exercises
the interactions and reading layouts. `CHROMIUM_PATH` can select the browser executable. Native
screenshots use `-leaf`; CI's manual `leaf_only` option photographs this route on either device.
