# The sea between: sources and preparation

A runtime-3 pack (`docs/runtime3.md`): the Aegean leaf (`Leaf/`, its own sources in `Leaf/SOURCES.md`)
moved onto the globe. Its script (`main.js`) runs in JavaScriptCore and draws nothing itself; the
globe draws the engraving lying on it, the marginal drawings as sprites, the names, the sounding
glass, the words at the foot and the panel. It depicts generalised present-day physical geography;
no political boundary or historic reconstruction is asserted. It is not a navigational chart.
Everything is in the pack: no network, no storage, no record of the reader's actions.

Everything here except `notes.js` and this file is prepared from the leaf's committed files by
`node tools/aegean/pack.cjs` (which runs `tools/aegean/pack_engraving.py`); rebuild, never hand-edit.
Nothing is fetched to build it.

## Geographic base (as the leaf's)

- **Coastlines:** Natural Earth, `ne_10m_land.geojson`, 1:10 million, public domain.
  <https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_land.geojson>,
  <https://www.naturalearthdata.com/about/terms-of-use/>.
- **Relief:** NASA Visible Earth, *Topography*, grayscale GEBCO 2008 imagery (25 m increments),
  <https://eoimages.gsfc.nasa.gov/images/imagerecords/73000/73934/gebco_08_rev_elev_21600x10800.png>.
- **Approximate depths:** NASA Visible Earth, *Bathymetry*, grayscale GEBCO 2008 imagery,
  `(255 - gray) × 43 m`,
  <https://eoimages.gsfc.nasa.gov/images/imagerecords/73000/73963/gebco_08_rev_bath_21600x10800.png>.
  NASA imagery use: <https://www.nasa.gov/nasa-brand-center/images-and-media/>.
  These are generalised legacy imagery derivatives, not current hydrographic survey data.

`tools/aegean/build.py` prepared them as the leaf's chart (`Leaf/data/chart.js`): a local cylindrical
chart, longitude contracted by the constant cos(37.5°), so every chart unit is a fixed step of
longitude and of latitude (a plate carrée up to scale).

## What the pack holds

- **`engraving.png`** (3072 × 2895): the leaf's day engraving, the drawing of `Leaf/data/day.png`
  (water strokes, coordinate ruling, land, relief contours and gradient hachures derived from
  elevation, not invented mountain symbols, and coast; no lettering), painted again by the leaf's own
  drawing code (`leaf.js` `makeMap`, in paint mode, as `tools/aegean/paint.cjs` does) at 3.0157
  pixels a chart unit instead of 1.65, so it stays sharp in the pack's close views. Only that scale
  is changed, and only in what Chromium is served. The whole chart is painted from its corner, so
  pixel columns and rows are equal steps of longitude and latitude; the sheet is cropped at whole
  pixels 1292, 1303 to 4364, 4198, and its bounds are exact: west 19.50013, south 33.399635, east
  30.20013, north 41.39941 (degrees). That is about what the leaf first shows on an iPad; beyond it
  the globe's own map carries on. A pixel is about a screen pixel on the 13-inch iPad at zoom 20,
  and on an iPhone at zoom 32; a neat line is ruled inside its edge, as round the leaf's page, and it
  is reduced to 256 colours without dithering. Decoded, it is about 36 MB.
- **`data.js`**: from `chart.js`, the sounding grid (350 × 315 cells over 15–35° E and 31–45° N, roughly
  5 km cells: the land mask, one bit a cell, and the depth in 25 m steps, one byte a cell, 255 meaning
  6,375 m or deeper); the 250, 500, 1,000, 1,500, 2,000, 3,000 and 4,000 m depth contours in degrees,
  simplified to about 0.003° (some 300 m); the leaf's own names and their places (`leaf.js`
  `geoLabels`, less the countries and cities the globe letters itself); and the places of the
  marginal drawings, each checked against the land mask.
- **Sprites** (`whale-*.png`, `blow-*.png`, `skiff-*.png`, `skiff-folded-*.png`, `wind.png`,
  `stream-*.png`, `palm.png`): drawn in Chromium by the leaf's canvas routines (`whale`, `drawSkiff`,
  `drawWind`, `palm` and the drifting strokes of `life` in `leaf.js`), in the leaf's day inks, at 4
  pixels a point (enough for their close looks, 1.4 times larger, as the leaf's marks grow when it
  is zoomed). The whale faces east (as a sprite that turns along its way must) with its flukes
  lifted a little frame by frame; the skiff's bob and sail are frames of one loop; the small blow
  that stands for the whale from afar is original to the pack, in the same ink.
- **`notes.js`**: the leaf's notes, unchanged: summaries, sources, qualifications.

## Choices on the globe

- The sounding glass reads the grid cell under its centre, rounded to 100 m (`~2,400 m · generalised`;
  `under 50 m` when it rounds to nothing; `or more` at the grid's deepest step), and shows the
  contours inside its rim, cut at the sheet's edge. The hanging line is a display convention: 14
  points for each kilometre. The words at the foot qualify it every time.
- The whale moves on a path broadly south of Crete, over generalised depths of 1,500 m or more
  along the Hellenic Trench, and surfaces now and then. It is not a sighting, a track or an
  abundance map, and its size says nothing about an animal's size beside an island.
- The leaf draws its wind strokes at the wind note's place (25.75° E, 38.25° N); on the globe they
  sit a little south-west of it, in open water north of the Cyclades, clear of Chios's name. The
  leaf's skiff starts off Kea; here it drifts in the open north Aegean. Both are imaginative marks.
- The leaf's Santorini note qualifies an inset drawing that the pack does not have; the pack's
  panel says instead that the note has no drawing here and that the general coastline leaves out
  the small central islands, keeping the note's other qualifications (no reconstruction, no exact
  eruption date). Every other note's summary, sources and limits are shown as written.
- The pack lets the globe zoom to 32 while it runs (`terra.globe.set({ maxZoom })`; the leaf's own
  first view is about zoom 14 on an iPad and 25 on a phone). It opens on the whole sheet, about zoom
  12 on the 13-inch iPad and 13.5 on an iPhone: the script flies there itself, once it may zoom past
  8, so the manifest has no anchor. The engraving, names and drawings come in when a degree is so
  many points across the screen, not at one zoom for every screen, so the chart reads the same at
  the same size on a phone, which zooms further for it; the whole-Earth view keeps only the
  doorway's skiff and, from zoom 1.6, the whale's occasional blow.

## The notes' sources

Checked against the institutional accounts on October 26, 2026 ET, for the leaf (see
`Leaf/SOURCES.md`): not independent scholarly peer review; the broader phrasing is editorial
interpretation, and none of the notes is a complete regional account.

- **Etesians / island wakes:** European Union, Copernicus, *Aegean "Etesian winds" seen from
  Copernicus Sentinel-3*, 1 August 2024.
  <https://eu-space.europa.eu/components/earth-observation-copernicus/image-of-the-day/aegean-etesian-winds-seen-copernicus-sentinel-3>
- **Santorini:** Smithsonian Global Volcanism Program, volcano 212040,
  <https://volcano.si.edu/volcano.cfm?vn=212040>; IUGS, *The Quaternary Santorini Caldera*,
  <https://iugs-geoheritage.org/geoheritage_sites/quaternary-santorini-caldera/>; NASA Earth
  Observatory, *Santorini Volcano, Greece*, 30 June 2008 (ISS017-E-5037; no photograph reproduced),
  <https://science.nasa.gov/earth/earth-observatory/santorini-volcano-greece-8872/>.
- **Vai / native palm range:** EUFORGEN, *Phoenix theophrasti*,
  <https://www.euforgen.org/species/phoenix-theophrasti/>; Greek National Tourism Organisation,
  *Vai Palm Forest and Ancient Itanos*,
  <https://www.visitgreece.gr/en/experiences/nature/forests/vai-palm-forest-and-ancient-itanos>.
- **Deep habitat:** IUCN Marine Mammal Protected Areas Task Force, *Hellenic Trench IMMA*,
  <https://www.marinemammalhabitat.org/factsheets/hellenic-trench/>.

## Type

None is shipped: the globe letters the names, the words at the foot and the panel in the app's own
bundled type (Libertinus, SIL Open Font License, as the leaf's).
