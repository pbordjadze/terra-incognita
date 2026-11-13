# Terra Incognita

The whole Earth as satellites saw it this month, and on it a living light for each of the toys: touch
one and fall into it. Terra and the toys it holds are written in Rust on wgpu (`toys/`; Terra itself is
`toys/terra/`), beside Adrift, glass floats on the real sea (`adrift/`). Each is its own app in the same
SideStore source. Nothing is a web view, nothing is fetched at run time; no accounts, no analytics.

- `toys/`: the toy kit (`toys/crates/toy-kit`), every toy and Terra; how they fit: `toys/README.md`.
- `adrift/`: Adrift; `adrift/README.md`.
- `Globe/`: the Earth's data the apps borrow (the coast's distance fields, terrain, twelve months' ground
  and sea ice, the night's lights, the month's ocean currents, fonts), built by `tools/globe/` and
  `tools/ocean/`; never hand-edited.
- `tools/`: the builders and checks (`docs/tools.md`, and the kit's in `docs/kit.md`).

Install from SideStore: add `https://raw.githubusercontent.com/pbordjadze/terra-incognita/sidestore/source.json`.

The Xcode-built Terra that came first (a Metal globe with a workbench of prototypes and packs) was
retired on November 13, 2026; it is whole on the branch `before-xcode-retirement-2026-11-13`.

Map data: Natural Earth (public domain); NASA Earth Observatory's Blue Marble Next Generation (Reto
Stöckli; public domain) for the ground, month by month, and NASA's Black Marble for its lights at
night; the NSIDC Sea Ice Index, version 4 (NOAA@NSIDC) for the sea ice; NASA's GEBCO elevation and
bathymetry (public domain) for the relief and seas; the NOAA Global Drifter Program's climatology (CC BY
4.0) for the ocean's currents. Each toy credits its own data in its `SOURCES.md`. Fonts (SIL Open Font
License, the licence beside them in `Globe/fonts/`): Libertinus Serif.
