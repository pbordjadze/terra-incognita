# Sounding the deep

A runtime-2 globe pack: the page (`overlay.html`, `sounding.js`) draws nothing itself; the app's
globe draws the stations, the illustrated passages, the ship and the hanging depth line, the date at
the foot and the caption (`terra.geo`, `terra.time`, `terra.caption`, `terra.globe`). `index.html` is
its own page (about and sources), opened from the chip.

## Data

- Observations: Murray, John; Renard, A F (1891). Annotated record of the detailed examination of Mn
  deposits from the HMS Challenger stations. PANGAEA, https://doi.pangaea.de/10.1594/PANGAEA.849072.
  CC BY 3.0. 94 events, dates reduced to days, coordinates and depths as published; exact duplicates
  collapsed (`data.js`, prepared for the earlier study `one-ship-two-rhythms`; its SHA-256 and
  transformations are recorded inside it).
- Land reference: Natural Earth ne_110m_land (public domain), used only to keep illustrated passages
  off land. See LICENSE-NATURAL-EARTH.md.

## Rules (from the first version, `voyage.js`)

- A passage between two stations is drawn only when they are 14 days apart or less and twenty points
  along the great circle between them stay off the land reference. It is an illustration, never a
  logged course; across a longer gap there is no ship.
- On a station's date the ship is drawn at that station (a display convention; no time of day is
  asserted).
- The hanging line's length is the recorded depth at 26 points a kilometre (capped at 170 points).

## Not claimed

The record is a selection of stations, not the full expedition track. Positions are historical
measurements. The ship picture is original schematic art.
