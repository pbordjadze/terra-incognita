# Terra Incognita

A playground for a wandering atlas. The app opens on a live globe, drawn by Metal: the ground as
satellites saw it this month, in a light key, with the sea's floor and its ice, and the land's own
names as you come closer: spin it, pinch it, tap a country. The small grid button opens the
**workbench**, which starts one prototype at a time, on the globe or on its own screen; ✕ on its
chip returns the plain globe. How to make and publish one: [docs/playground.md](docs/playground.md).

On the workbench today: the quiet globe's four rules (**Names on demand**, **Clouds clear where you
look**, **The Earth at night**, **The quiet frame**), which run together, and the **Delivery check**
(parked), the smallest pack (`Prototypes/hello-pack/`). Every experience was retired on October 31,
2026 (painting-company row 195); they are all in the history at `before-retirement-2026-10-31`.

The code:

- `App/TerraIncognita/Playground/`: the workbench, the doorways, the running prototypes' stages, the
  catalog of native prototypes, and the runtime, which runs packs' scripts in JavaScriptCore
  (`Prototypes/`, bundled or downloaded from the `packs` branch; [docs/runtime3.md](docs/runtime3.md)).
- `App/TerraIncognita/Globe/`: the globe; its data, `Globe/`, is made by `tools/globe/build.py`.
  `Clouds/` and `Ocean/` are the sky and the sea on it; `Quiet/` the quiet globe's rules.
- `tools/playground/`: scaffold, preview, check, pack and publish packs. `Water/` is the former web
  study's data, kept as the source `tools/water/` builds from.
- The first demo, the map drawn in characters by WebGL, is retired; it is in the history at 1f9b941.

Install from SideStore: add `https://raw.githubusercontent.com/pbordjadze/terra-incognita/sidestore/source.json`.
A retirement notice for the separate Study II source is prepared in `ci/retired-study2-source.json`;
CI publishes it only from a `main` commit tagged `[retire-study2]`.

Map data: Natural Earth (public domain); NASA Earth Observatory's Blue Marble Next Generation (Reto
Stöckli; public domain) for the globe's ground, month by month, and NASA's Black Marble for its
lights at night; the NSIDC Sea Ice Index, version 4 (NOAA@NSIDC) for its sea ice; NASA's GEBCO
elevation and bathymetry (public domain), for the globe's relief and seas; NOAA's ISCCP-H for the
clouds and the NOAA Global Drifter Program's climatology (CC BY 4.0) for the ocean's currents.
Fonts (SIL Open Font License, their licences beside them in `Globe/fonts/`): Libertinus Serif and
Uncial Antiqua. The globe follows the WikiProject Maps conventions for location maps; it is not
affiliated with Wikimedia.

Nothing leaves the device, with one exception you choose: the workbench can download prototype
packs from sources you add (plain GETs, no identifiers; see docs/playground.md).
