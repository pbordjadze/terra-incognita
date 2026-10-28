# Borders on old maps

A runtime-2 globe pack: the page (`overlay.html`, `borders.js`) draws nothing itself; the globe draws
the dated map's borders and names, the chosen country (filled) and its outline on the closest earlier
map (dashed), the date at the foot and the caption. `index.html` is its own page (about and sources).

## Data

- Dated maps: André Ourednik and contributors, historical-basemaps,
  https://github.com/aourednik/historical-basemaps/tree/da7a4b735ecef70aebdc9c73e409d8a2500d50f3,
  files `geojson/world_<year>.geojson` for 1530, 1600, 1815, 1878, 1920, 1938, 1945 and 1994. GPL-3.0;
  see LICENSE-HISTORY.txt. `data.js` was prepared for the earlier study `time-atlas` (its selection,
  exclusions, simplification and limitations are recorded inside it, in `TIME_DATA.source`).
- Land reference: Natural Earth ne_110m_land (public domain), only to tell uncovered land from sea.

## Rules

- The year picks the nearest dated map; borders never morph or move between maps, and the caption
  names the map shown whenever the year has none of its own.
- A country is matched across maps by its name (Poland also by its older forms, the Ottoman Empire by
  any Ottoman name). The dashed outline is the same country on the closest earlier map that has it.
- The 1945 map's "Germany (UK)", "(USA)", "(France)" and "(Soviet)" are labelled as the British,
  American, French and Soviet zones of occupation.

## Not claimed

These are generalised maps, not treaty boundaries on a given day. A country missing from a map is
missing from this extract, not a statement about the people there.
