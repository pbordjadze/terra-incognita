# A gathering of water, alive

A runtime-4 globe pack (`docs/runtime3.md`): A gathering of water (`Prototypes/gathering-of-water`)
with its water running. The same rivers, links, lengths and areas; what changes is what a touch
does and what is drawn with it. `core.js` is that pack's graph, with the stretches' lengths
exported; `main.js` decides what the globe draws and says. No web view, no network.

## What it does

- **Touch a river** and light runs from the touch down its way to the sea (or an inland ending)
  while what gathers above the touch lights up. **Rest a finger** (0.3 s) **and draw across the
  rivers** and every stretch crossed lights at once; their lights run together where the rivers
  meet; lifting the finger lights what gathers above all of them. A drag without the rest turns the
  globe (`terra.touch.scheme('hold')`), so a pan is never taken for a touch. A touch away from every
  river lets it all go.
- **The light** runs along what is lit (`flow` on the globe's lines: beads moved by Core Animation),
  and the way down grows heavier where more of what was lit runs together (one, two or three, four
  or more). Both are drawn, not measured: the light's speed is invented (about 70 points a second on
  screen, no way down longer than about eight seconds) and the weight counts touches, not water.
  Under Reduce Motion the light arrives at once and does not run.
- **The words at the foot** name where the lit water goes ("Toward the Amazon"). The distances and
  areas A gathering of water said at the foot are in the panel's **What is lit** (Open), with the
  same rules as before.

## Data

- **The network, its links, names**: exactly A gathering of water's (`network.json` and
  `places.json`, copied by `tools/water/build_alive.py`): RiverATLAS v1.0 (Linke et al. 2019,
  https://doi.org/10.1038/s41597-019-0300-6; data https://doi.org/10.6084/m9.figshare.9890531,
  CC BY 4.0, https://creativecommons.org/licenses/by/4.0/; links as derived in Lehner & Grill 2013,
  https://doi.org/10.1002/hyp.9740). See `Prototypes/gathering-of-water/SOURCES.md` for what was
  modified and the rules kept from the study (snapping, distances, areas, names). The towns are
  left out here, for fewer words on the map.
- **Into the sea** (`sea.json`): for the Amazon's, the Tocantins's and the Orinoco's mouths, a way
  out of the estuary placed by hand (drawn, not measured), then streamlines of the month's mean
  surface current from seeds across the mouth and shelf, traced for at most 50 days. The currents
  are the ocean layer's own (`App/TerraIncognita/Ocean/OceanCurrents.png`, from the NOAA Global
  Drifter Program's monthly climatology: Laurindo, Mariano & Lumpkin 2017, Deep-Sea Research I 124,
  73–92, https://doi.org/10.1016/j.dsr.2017.04.009, CC BY 4.0). They are a month's mean, not a
  day's: the ways a drop would take if the month's average held. The month is the device's (the
  globe's own month unless another experience sets a date).
- **The Amazon's plume** (`plume-01.png` … `plume-12.png`): drawn from the same currents: drops
  released across the mouth, carried 60 days with a little invented spreading, counted, smoothed and
  tinted from a silt brown near the mouth to green offshore. Not observed: the real plume's reach,
  shape and colour change from year to year. What it follows is documented: the plume carried
  northwest along the Guianas by the North Brazil Current, and turned east for part of the year
  (Muller-Karger, McClain & Richardson 1988, *The dispersal of the Amazon's water*, Nature 333,
  56–59, https://doi.org/10.1038/333056a0; Coles et al. 2013, *The pathways and properties of the
  Amazon River Plume in the tropical North Atlantic Ocean*, JGR Oceans 118, 6894–6913,
  https://doi.org/10.1002/2013JC008981). The traced streamlines show the same seasons on their own:
  in March every seed goes northwest toward the Caribbean; in August and October the offshore ones
  turn east north of the equator.
- **The boto**: drawn by `tools/water/boto.cjs` from code (SVG, no photograph traced): a pink river
  dolphin's head with its round melon and long beak, its back with the low ridge it has instead of a
  fin, its breath, the ripples. A drawn mark, not a sighting: it surfaces close in (about 10 points
  a degree and nearer), a moment after the light has passed, in a lit stretch of Strahler order 6 or
  more in the Amazon, Orinoco or Tocantins system east of 77.5°W, swims a short way up and down it,
  and dives when touched. Where botos live, how they look and behave, and their status are from
  *Amazon river dolphin*, Wikipedia, https://en.wikipedia.org/wiki/Amazon_river_dolphin (linked in
  the panel, for a reader who wants to read on).
- **Pictures**: `doorway.png` is drawn by `build_alive.py` from the network's own Amazon lines (the
  main stem lit, a boto's back at the mouth); `glint.png` and `doorway-glint.png` are A gathering of
  water's.

## What it does not claim

Nothing here simulates rainfall, travel time, present flow, flood risk or navigability; the light
and its weight are invented, the sea's paths are a month's mean currents, the plume's tint is drawn
from them, and the boto is drawn. The single-outlet tree cannot show the Casiquiare (see A gathering
of water's panel).
