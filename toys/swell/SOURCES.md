# Swell: sources

What is measured, what is drawn, and from where.

## Measured

- **The sea's depth**: GEBCO's bathymetry, through Terra's globe (`Globe/terrain.png`'s G, built by
  `tools/globe/build.py`), and its coasts from Natural Earth (`Globe/geo.png`), averaged to half a
  degree by `tools/toys/swell/build.py` into `data/depth.bin`.
- **How a long wave moves on it**: the linear long-wave (shallow-water) equation, the physics of
  tsunami propagation: a wave runs at sqrt(g h) (about 200 m/s over the abyss, tens over a shelf), so it
  slows over ridges and shelves, refracts round seamounts and islands, grows as it shoals (Green's law)
  and reflects from coasts. Solved on the sphere on the globe's own grid (`src/sea.rs`).
- **The great tsunamis' sources** the idle sea rises from: offshore of Tōhoku 2011, Valdivia 1960,
  Sumatra–Andaman 2004, Alaska 1964 and Maule 2010 (USGS epicentres; NOAA NCEI's tsunami database).
- **The ground**: the kit's globe, Terra's data (NASA Blue Marble, Natural Earth, GEBCO), credited in
  Terra's sources.

## Drawn (not measured)

- **Time**: 4,800 seconds of sea a second (a wave crosses the Pacific in about 18 s).
- **The drop**: a hump of sea 170 km wide raised at rest (a real tsunami's source is the sea floor's
  uplift over a fault, hundreds of km long); a held finger raises one every 1.1 s.
- **The loss**: a little everywhere and more at the coasts (the real sea loses wave energy to friction
  and breaking at the shore), and a wall past 78° of latitude (the grid's limit).
- **The waves' height and light** (drawn as sunlight on the slopes, exaggerated), the chimes (a coast's
  note by its latitude on a pentatonic scale, sounded when a wave arrives above the coast's recent
  swell), the plunk.
