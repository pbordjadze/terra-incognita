# Sounding the deep

A runtime-3 pack: its scripts (`voyage.js`, `sounding.js`, `about.js`) draw nothing themselves; the
app's globe draws the stations, the illustrated passages, the ship and the hanging depth line, the date
at the foot and the caption, and `about.js` fills the panel the chip's Open shows.

## Data

- Observations: Murray, John; Renard, A F (1891). Annotated record of the detailed examination of Mn
  deposits from the HMS Challenger stations. PANGAEA, https://doi.pangaea.de/10.1594/PANGAEA.849072.
  CC BY 3.0. 94 events, dates reduced to days; exact duplicates
  collapsed (`data.js`, prepared for the first version of this study, "One ship, two rhythms", now
  removed and kept in the repository's history; its SHA-256 and transformations are recorded inside
  it). `tools/playground/sounding-the-deep/fetch-stations.cjs` reads the published export again.
- Depths and positions: the recorded fathoms, converted at 1.8288 m a fathom, from Murray, John;
  Renard, A F (1891), Report on the deep-sea deposits, as transcribed in the file AWI submitted to
  PANGAEA (https://epic.awi.de/38636/1/NGDC_MURRAY1891_V01.txt). PANGAEA 849072's event elevation,
  which this pack first used, converts the fathoms at 1.6421 m a fathom (about 10 % too shallow:
  Station 9, recorded 3,150 fathoms, 5,761 m, was shown as 5,172 m). CHA-64's depth is the printed
  table's (2700) fathoms (p. 56); the file scales it as PANGAEA does. Six positions differ from
  PANGAEA's by more than 0.01 degree: CHA-VIIB, CHA-13, CHA-189, CHA-247 and CHA-286 take the file's
  (CHA-VIIB, 32°43' N 15°52' W, matches the printed table, p. 36); CHA-176 keeps PANGAEA's longitude,
  which the printed table (p. 90, 173° 52' E) supports. `tools/playground/sounding-the-deep/recorded-depths.py`
  applies this to `data.js` and says what it changed. Found by the fact-check of October 28, 2026.
- Land reference: Natural Earth ne_110m_land (public domain), used only to keep illustrated passages
  off land. See LICENSE-NATURAL-EARTH.md.

## Rules (from the first version, `voyage.js`)

- A passage between two stations is drawn only when they are 14 days apart or less and twenty points
  along the great circle between them stay off the land reference. It is an illustration, never a
  logged course; across a longer gap there is no ship.
- On a station's date the ship is drawn at that station (a display convention; no time of day is
  asserted).
- The hanging line's length is the recorded depth at 18 points a kilometre (capped at 170 points; the
  deepest station here, 3,625 fathoms, is about 120).
- A depth is said as recorded, in fathoms, then about how far that is in metres (to 10 m): it is the
  length of line paid out, read to the nearest mark, not corrected for drift.

## Not claimed

The record is a selection of stations, not the full expedition track. Positions are historical
measurements. The ship picture is original schematic art.
