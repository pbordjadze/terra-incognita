# Terra Incognita

A playground for a wandering atlas. The app opens on a live globe, drawn by Metal: the ground as
satellites saw it this month, in a light key, with the sea's floor and its ice, and the land's own
names as you come closer: spin it, pinch it, tap a country. The small grid button opens the
**workbench**, which starts one prototype at a time, on the globe or on its own screen; ✕ on its
chip returns the plain globe. How to make and publish one: [docs/playground.md](docs/playground.md).

On the workbench today:

- **Camel and Llama** and **Whale and Desert**: illustrated natural histories that start from an
  animal in Arabia and a fossil in Egypt. Following the lineage or visiting the past is voluntary;
  their original artwork, evidence and textual equivalents are bundled.
- **A small thing, far from home**: an offline historical encounter in the Indian Ocean; its
  research ledger is [docs/encounter-research.md](docs/encounter-research.md).
- **A gathering of water** (`Prototypes/gathering-of-water/`): South America's rivers on the globe;
  touch one to follow where its water goes. **The sea between** (`Prototypes/sea-between/`): an
  engraved Aegean leaf lying on the globe, with notes and a sounding glass.
- **Sounding the deep** (`Prototypes/sounding-the-deep/`): HMS Challenger exploring the deep
  ocean, 1873–76, drawn by the globe itself; touch a station to lower the line to its recorded depth.
- **Borders on old maps** (`Prototypes/borders-on-old-maps/`): Europe on eight dated maps drawn on
  the globe; a chosen country filled, its outline on the map before dashed.
- **Delivery check** (parked): the smallest pack (`Prototypes/hello-pack/`).
- Parked: **The Wikipedia globe** (the political map in Wikipedia's location-map colours, the
  globe's style until October 28, 2026), **The Persian globe** (the globe in colours from a 1529
  manuscript) and **The film** (a
  flight from the whole world down to a street in five looks: Aisling's Forest, Paper Almanac,
  Night Voyage, Chalk and Slate, The Sampler).

The code:

- `App/TerraIncognita/Playground/`: the workbench, the doorways, the one active prototype's stage, the
  catalog of native prototypes, and runtime 3, which runs packs' scripts in JavaScriptCore
  (`Prototypes/`, bundled or downloaded from the `packs` branch; [docs/runtime3.md](docs/runtime3.md)).
- `App/TerraIncognita/Globe/`: the globe; its data, `Globe/`, is made by `tools/globe/build.py`.
- `App/TerraIncognita/Atlas/`: the film player; `Atlas/` holds each look's plates, made by
  `tools/plates/export.cjs` from the look's page (`tools/plates/<look>/index.html`).
- `App/TerraIncognita/{Camelid,Whale,Experience}/`: the native experiences. `Leaf/` and `Water/` are the
  former web studies, kept as the sources their packs are built from.
- `tools/playground/`: scaffold, preview, check, pack and publish packs.
- The first demo, the map drawn in characters by WebGL, is retired; it is in the history at 1f9b941.

Install from SideStore: add `https://raw.githubusercontent.com/pbordjadze/terra-incognita/sidestore/source.json`.
A retirement notice for the separate Study II source is prepared in `ci/retired-study2-source.json`;
CI publishes it only from a `main` commit tagged `[retire-study2]`.

Map data: Natural Earth and US Census TIGER/Line (public domain); © OpenStreetMap contributors
(ODbL), for the streets and buildings of the closest scale; NASA Blue Marble (public domain), for
where the forest look's woods grow, and NASA Earth Observatory's Blue Marble Next Generation (Reto
Stöckli; public domain) for the globe's ground, month by month; the NSIDC Sea Ice Index, version 4
(NOAA@NSIDC) for its sea ice; NASA's GEBCO elevation and bathymetry (public domain), for the
globe's relief and seas; RiverATLAS (CC BY 4.0) for the river study; historical boundaries from
aourednik/historical-basemaps (GPL-3.0, its licence in `Prototypes/borders-on-old-maps/LICENSE-HISTORY.txt`)
for Borders on old maps; HMS Challenger station metadata from PANGAEA (CC BY 3.0) for Sounding the
deep. Each pack's `SOURCES.md` has the details. Fonts (SIL Open Font License,
their licences beside them in each look's folder and the globe's): IM Fell English, EB Garamond,
Uncial Antiqua, Alegreya, Libre Caslon Text, Cormorant and Libertinus Serif. The globe follows the
WikiProject Maps conventions for location maps; it is not affiliated with Wikimedia.

Nothing leaves the device, with one exception you choose: the workbench can download prototype
packs from sources you add (plain GETs, no identifiers; see docs/playground.md).
