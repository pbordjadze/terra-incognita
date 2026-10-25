# Terra Incognita

An atlas drawn in letters: the world as a medieval map, rendered entirely in characters on the
GPU. Open the app and a six-minute tour plays; touch the map to wander, then resume.

- `Web/`: the map and its film (a WebGL2 page). `render.js` (the two passes), `cell.glsl` (every
  character cell decides what it shows), `scene.js` (projections, lettering, creatures), `tour.js`
  (the film, the reader's fingers, captions and the style menu).
- `App/`: the iPhone app, a SwiftUI window holding the page; `BundleScheme` serves `Web/` at
  `terra://app/`.
- `tools/build_textures.py`: builds `Web/data/` from Natural Earth and NASA (public domain).

Install from SideStore: add `https://raw.githubusercontent.com/pbordjadze/terra-incognita/sidestore/source.json`.

Map data: Natural Earth (public domain); NASA Blue Marble, Black Marble and GEBCO elevation
via NASA Visible Earth (public domain). Fonts: JetBrains Mono, IM Fell English, EB Garamond,
Noto Sans Symbols 2 (SIL Open Font License). Nothing leaves the device.
