# Offline atlas leaf: sources and preparation

The leaf depicts generalised present-day physical geography. No political boundary or historic
reconstruction is asserted. It is not a navigational chart. All runtime resources are bundled;
there are no network calls or persistent records of the visitor's actions.

## Geographic base

- **Coastlines:** Natural Earth, `ne_10m_land.geojson`, 1:10 million.
  <https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_land.geojson>
  Public domain: <https://www.naturalearthdata.com/about/terms-of-use/>.
- **Relief:** NASA Visible Earth, *Topography*, grayscale GEBCO 2008 imagery,
  <https://eoimages.gsfc.nasa.gov/images/imagerecords/73000/73934/gebco_08_rev_elev_21600x10800.png>.
  The grayscale increment is 25 m. Preparation follows the existing globe's source decoding.
- **Approximate depths:** NASA Visible Earth, *Bathymetry*, grayscale GEBCO 2008 imagery,
  <https://eoimages.gsfc.nasa.gov/images/imagerecords/73000/73963/gebco_08_rev_bath_21600x10800.png>.
  Depth is `(255 - gray) × 43 m`, as in the existing globe pipeline.
  NASA imagery use: <https://www.nasa.gov/nasa-brand-center/images-and-media/>.
  These are generalised legacy imagery derivatives, not current hydrographic survey data.

`tools/aegean/build.py` crops and projects these sources into a local cylindrical chart, with
longitude contracted by cos(37.5°). Contours and short gradient hachures are derived from elevation,
not invented mountain symbols. They are an illustration of terrain, not survey contours.

The sounding glass shows generalised 250, 500, 1,000, 1,500, 2,000, 3,000 and 4,000 m depth contours.
Its readout is rounded to 100 m and explicitly qualified. The hit-test/readout grid is 350×315 over
20° longitude and 14° latitude (roughly 5 km cells here); it must not be used to infer local water
depths or island-scale coast accuracy. The vector coast retains the source's greater detail.
The scale bar is approximate at the chart's central latitude.

Source files were retrieved October 26, 2026 ET. Rebuild with a cache containing `land.geojson`,
`elevation.png` and `bathymetry.png`: `python3 tools/aegean/build.py <cache>`. No download occurs at
runtime or in that script. The committed derivative is `data/chart.js`.

## Optional place notes

The original short summaries, source titles, URLs and qualifications are bundled together in
`notes.js`. The claims were checked against the following institutional accounts October 26, 2026 ET:

- **Etesians / island wakes:** European Union, Copernicus, *Aegean “Etesian winds” seen from
  Copernicus Sentinel-3*, 1 August 2024. The text specifically relates the observed wind streaks
  and vortices to the Cyclades islands in the 31 July image.
  <https://eu-space.europa.eu/components/earth-observation-copernicus/image-of-the-day/aegean-etesian-winds-seen-copernicus-sentinel-3>
- **Santorini:** Smithsonian Global Volcanism Program, volcano 212040; the description and image
  captions identify the submerged caldera and the younger central islands. Cross-checked with
  IUGS's *The Quaternary Santorini Caldera*, especially its onshore/offshore morphology caption.
  <https://volcano.si.edu/volcano.cfm?vn=212040>
  <https://iugs-geoheritage.org/geoheritage_sites/quaternary-santorini-caldera/>
  No causal claim about the collapse of a civilisation or a precise Bronze Age eruption date is made.
- **Vai / native palm range:** EUFORGEN, *Phoenix theophrasti*, opening species description,
  identifies Crete and southwestern Türkiye as the native range. Greek National Tourism
  Organisation's *Vai Palm Forest and Ancient Itanos*, section “Vai Palm Forest,” locates the grove.
  <https://www.euforgen.org/species/phoenix-theophrasti/>
  <https://www.visitgreece.gr/en/experiences/nature/forests/vai-palm-forest-and-ancient-itanos>
  No legend about sailors planting the palms is presented as history.
- **Deep habitat:** IUCN Marine Mammal Protected Areas Task Force, *Hellenic Trench IMMA*, summary
  and criterion B2 relate the steep bathymetric feature to sperm whale habitat.
  <https://www.marinemammalhabitat.org/factsheets/hellenic-trench/>
  The moving animal is not a sighting or a mapped track.

Checked by the implementing agent against the institutional pages; this is not independent
scholarly peer review. No source photograph is reproduced. Brief source passages support the
relationship itself, rather than separate facts at its endpoints. The broader poetic phrasing is
editorial interpretation. None of these notes is a complete regional account.

## Artwork, type and sound

The skiff, botanical emblem, marginal animal, wind strokes and sound synthesis are original
illustration. Their movement is imaginative, not measured weather, traffic or animal behavior.
The adjustable paper light is an illustration choice, not a real solar model, date or hour.

Libertinus Serif, Philipp H. Poll and contributors, SIL Open Font License 1.1. The three font files
and their complete license are under `fonts/`. No additional font service is contacted.

The Santorini inset deliberately supplements the general coastline's Thera outline with two
schematic inner islets (not surveyed shapes). Their spatial relationship was also checked by
inspecting NASA astronaut photograph ISS017-E-5037, Expedition 17 crew, NASA/JSC, 19 April 2008,
NASA Earth Observatory, *Santorini Volcano, Greece*, 30 June 2008:
<https://science.nasa.gov/earth/earth-observatory/santorini-volcano-greece-8872/>.
No photograph is reproduced. The inset's schematic status is printed on it and explained offline.
