# Terra Incognita

A medieval map that plays as a film: from the whole world down to a single street, through every
scale in between, each scale a finished picture in the chosen look. Open the app and the film plays;
touch the map to wander, then resume. The look menu switches between the looks on the same flight.

- `App/TerraIncognita/Atlas/`: the player. `AtlasCanvas` (Core Animation layers moved and scaled by
  a display link; the camera, the film, pan and pinch), `Camera.swift` (nested plates as one
  continuous zoom, the film's shots), `Plates.swift` (the bundled `Atlas/`), `AtlasScreen` (captions,
  the look menu, Resume).
- `Atlas/`: each look's plates, made by `tools/plates/export.cjs` from the look's page
  (`tools/plates/<look>/index.html`; the contract is in `tools/plates/kit.js`).
- `tools/plates/build_data.py`: the map data the pages draw from, in `tools/plates/data/`.
- `Web/`, `tools/build_textures.py`: the first demo, the map drawn in characters by WebGL (no longer
  the app's screen).

Install from SideStore: add `https://raw.githubusercontent.com/pbordjadze/terra-incognita/sidestore/source.json`.

The looks: Aisling's Forest, Paper Almanac, Night Voyage, Chalk and Slate, and The Sampler.

Map data: Natural Earth and US Census TIGER/Line (public domain); © OpenStreetMap contributors
(ODbL), for the streets and buildings of the closest scale; NASA Blue Marble (public domain), for
where the forest look's woods grow. Fonts (SIL Open Font License, their licences beside them in
each look's folder): IM Fell English, EB Garamond, Uncial Antiqua, Alegreya, Libre Caslon Text and
Cormorant. The first demo also used NASA Black Marble and GEBCO elevation (public domain) and the
fonts JetBrains Mono and Noto Sans Symbols 2. Nothing leaves the device.
