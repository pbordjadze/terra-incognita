# Mold: sources

What is measured, what is modelled, what is drawn.

## Measured (data)

- **The Earth's lights at night** (the mold's food): NASA Earth Observatory / Black Marble (Suomi NPP
  VIIRS), as Terra Incognita's `Globe/ground/night.jpg` (built by `tools/globe/night.py`). NASA imagery is
  in the public domain; credit: NASA Earth Observatory, Black Marble. `tools/toys/mold/build.py` keeps the
  lights on land only, at the mold's grid (`toys/mold/data/ground.png`, R).
- **The coast** (where the mold may go): Natural Earth (public domain), as `Globe/geo.png`'s signed
  distance (tools/globe/build.py). The mold may cross land and about 40 km of sea (narrow straits);
  `ground.png`'s G.
- **The globe under it**: the kit's night globe (NASA Blue Marble, GEBCO, Natural Earth, NSIDC's ice;
  `toys/README.md`, Terra's sources).

## Modelled (real behaviour)

- **The mold's agents**: J. Jones (2010), "Characteristics of pattern formation and evolution in
  approximations of Physarum transport networks", *Artificial Life* 16(2). Each agent senses the
  chemoattractant at three sensors ahead, turns toward the strongest, steps, and deposits; one agent a
  cell (occupancy); the field diffuses and decays. Food sources add attractant to the same field.
- **The experiment it echoes**: A. Tero et al. (2010), "Rules for biologically inspired adaptive network
  design", *Science* 327, 439–442: Physarum on oat flakes laid as the towns round Tokyo grew a network
  like the region's railways. Here the towns are the measured lights, every city on Earth.

## Drawn (not measured)

- The parameters (sensor angle and offset, decay, food's strength, a crowded agent letting go, a few
  agents re-budding at lit places) are chosen for the look at the globe's scale, not fitted.
- The colour (a foxfire green-white), the glow, and the slow pulse along the tubes (Physarum's shuttle
  streaming is real; its period and wave here are drawn). The fed flakes' heap and smell.
- The first seconds' easing (the mold "waking") is a pace, not a measurement.
