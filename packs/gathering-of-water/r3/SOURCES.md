# A gathering of water

A runtime-3 globe pack (`docs/runtime3.md`): the river study that was a full-screen web page in
`Water/` (docs/water-engineering.md), now drawn by the globe itself. `core.js` is the study's graph
(Water/core.js: the nearest line, the way downstream, the tree upstream) on longitude and latitude;
`main.js` decides what the globe draws and says. No web view, no HTML, no network.

## Data

- **The network and its links**: RiverATLAS v1.0, South America (north and south components),
  https://doi.org/10.6084/m9.figshare.9890531, CC BY 4.0
  (https://creativecommons.org/licenses/by/4.0/). Linke, S. et al. (2019), *Global
  hydro-environmental sub-basin and river reach characteristics at high spatial resolution*,
  Scientific Data 6, 283, https://doi.org/10.1038/s41597-019-0300-6. The connections are each
  reach's `NEXT_DOWN`, as derived in Lehner, B. & Grill, G. (2013), *Global river hydrography and
  network routing*, Hydrological Processes 27, 2171–2186, https://doi.org/10.1002/hyp.9740. None
  is inferred from lines lying near each other.
- **Modified**: `tools/water/build.py` kept the reaches with at least 500 km² upstream plus their
  exact downstream closure (218,146 of 1,620,963), merged chains with no retained tributary into
  18,159 stretches, simplified the lines to 0.006° and rounded them to 0.001°, and summed the
  original reach lengths; its source checksums and settings are in `network.json`'s `build` (copied
  from `Water/manifest.json`). `tools/water/build_pack.py` then simplified the lines again for the
  globe (Douglas-Peucker to 0.012°, each vertex marked with the coarsest of 0.1°, 0.05°, 0.025° and
  0.012° it survives at; the ends of every stretch unmoved, so each still meets the next) and kept
  every link, Strahler order, length and upstream area unchanged.
- **River names and towns** (`places.json`, from `Water/places.js` and `tools/water/labels.json`):
  orientation aids chosen for the study. River anchors are snapped to the nearest stretch of order 3
  or more, as the study snaps them; town positions are from Natural Earth 1:50m populated places
  (public domain, https://www.naturalearthdata.com/), less the capitals the globe names itself.
- **The Casiquiare exception** (cited in the panel): Laraque, A. et al. (2019), *Water and Sediment
  Budget of Casiquiare Channel Linking Orinoco and Amazon Catchments, Venezuela*, Water 11, 2068,
  https://doi.org/10.3390/w11102068.
- **Pictures**: `doorway.png` is drawn by `build_pack.py` from the Amazon system's own lines (order
  7 and up); `glint.png` is drawn there from code. Coasts, relief, borders and the other names are
  the globe's own.

## Rules kept from the study

- A touch snaps to the nearest line drawn (24 points, as the study's), only among the orders shown
  at that zoom. Its way down follows `NEXT_DOWN` to an outlet: the coast, or an inland ending
  (`ENDORHEIC` at the outlet).
- A distance is the sum of the mapped reach lengths below the touch, rounded to 10 km; the first,
  partial stretch is estimated along its simplified line. An area is RiverATLAS's upstream area at
  the lower end of the touched stretch, and at the outlet for a whole system.
- The words name a river the water passes only when its way down runs through that river's named
  stretch below the touch; a system's name (from the study's anchors) is given to every stretch
  draining to the same outlet.
- The glint that runs down a traced river, and the three drifts of ink while nothing is touched,
  show direction only: their speed is invented (the study's: a whole way in at most 9 seconds, the
  drifts at most 34). Under Reduce Motion neither is drawn; nothing depends on them.
- Earlier ways down (from earlier touches, or a brush across many) stay 24 seconds; the last stays
  until another touch, or a touch away from the rivers clears it.

## The relief

The study's `Water/terrain.jpg` is georeferenced cleanly (equirectangular, 84°W–32°W, 58°S–15°N,
`tools/water/build.py`'s `terrain()`), but it is a re-colouring of the globe's own GEBCO relief
(`Globe/terrain.png`, the same texels upsampled), with an opaque sea. Laid on the globe it would add
no relief the globe does not already shade, and would cover the globe's coasts and borders inside a
rectangle, so this pack leaves it out.

## Not claimed

This is a simplified single-outlet network, not a hydrological simulation. It omits smaller streams
and does not reproduce distributaries, floodplain exchange, changing flow, dams, flood risk or
navigation conditions. The tributaries lit above a touch are those retained here, not every stream
or a surveyed catchment boundary. An inland ending does not mean an empty landscape. No historical
route, cultural connection or causal claim about a community is encoded by the ink.
