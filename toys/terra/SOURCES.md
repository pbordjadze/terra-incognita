# Terra Incognita: sources

What is measured, what is drawn, and from where. Each toy Terra holds has its own (`toys/<toy>/SOURCES.md`);
inside Terra they are the same toys, unchanged.

## Measured

- **The Earth**: the kit's globe (toy-kit `globe`), the month's ground from NASA's Blue Marble in Terra's
  light key, the sea by depth (GEBCO), the month's sea ice; by night the Black Marble's lights. The month
  is the reader's own (the device's clock).
- **The rivers** (painting-company row 258): HydroSHEDS' HydroRIVERS v1.0, every river reach with at least
  300 km2 upstream, its lines smoothed a little (they trace a 15-second grid) and simplified to about 220 m
  (`tools/toys/terra/rivers.py`, `toys/terra/data/rivers.bin`). Which show is drawn: a river comes in once its
  catchment is large for the scale on screen, so the closer the Earth, the more of them. HydroRIVERS is free for
  non-commercial and commercial use under the HydroSHEDS licence; cite: Lehner, B., Grill, G. (2013): Global river
  hydrography and network routing: baseline data and new approaches to study the world's large river systems.
  Hydrological Processes, 27(15): 2171-2186. Data available at www.hydrosheds.org.
- **The ground round the scenes** (painting-company row 258): Copernicus Sentinel-2 L2A true colour (TCI) from the
  AWS open-data bucket `sentinel-cogs` (found with Element 84's earth-search), 200 km squares at about 100 m a pixel
  round the Rio Beni (Meander) and Ham Wall (Murmuration), and 20 km at 10 m round the
  last (`tools/toys/terra/region.py`, `toys/terra/data/*.jpg`); the scenes' dates from mid-July to mid-September
  2024 (Ham Wall's from April 2023 to September 2024, Britain being seldom clear). Drawn: a later scene matched in
  colour to the one under it; toward a square's edge its broad colour eased into the month's ground; by a dusk, its light put into the night's tones. Licence: Copernicus open data; credit: contains modified Copernicus
  Sentinel data 2023-2024. Meander's own sheet is its own (`toys/meander/SOURCES.md`).
- **The places of the lights**: where each toy opens. Whales: the middle of the two whales on their real
  routes (`toys/whales/data/routes.json`), moving along them as the light shows them. Murmuration: Ham
  Wall, Somerset (the toy's `PLACE`). Fireflies:
  Elkmont, Great Smoky Mountains (the toy's `PLACE`, `toys/elkmont/SOURCES.md`). Bell: Valdivia, Chile, 1960 (the first of the toy's
  great quakes). Swell: the mid-Pacific where the toy opens. Battle: Cannae, Apulia. Throats: near Los Banos,
  California (the side-blotched lizards' study population, `toys/throats/SOURCES.md`). Meander: the Rio Beni's
  floodplain, Bolivia (the toy's Sentinel-2 scene). Frost: Oymyakon, Yakutia.

## Drawn

- Every light's likeness of its toy: the whales' song-rings, a small flock,
  a scatter of glows bursting together every few seconds (the fireflies' beat, quickened), a ring crossing the Earth at no
  real speed, a train of crests, an engraving's two inks closing. They are pictures of the rules, not the
  toys' simulations.
- The flights, the night falling, the crossfades, the Earth mark, the bells; a scene's window onto the toy in its
  light, growing with the zoom, and the Earth falling into that scene's dusk or night as it grows; the rivers' ink
  and widths.
