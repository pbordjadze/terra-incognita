# Bundled river instrument

An isolated, offline surface in `Water/`, hosted by `WaterScreen`. Launch with `-water`.
Ordinary app builds retain the existing launch screen. The separately installable study app
uses `com.pbordjadze.terraincognita.study2`, selected only by the opt-in branch build.

The drawing is the same HTML/Canvas code in the native host and local browser checks. The host
uses a nonpersistent WKWebView, restricts file navigation to its bundled folder, and opens only
explicit citation taps in the browser. CSP blocks external subresources and all connections.
Dynamic Type and Reduce Motion are forwarded from the native environment. No exploration is
persisted. Existing `Atlas/` and `Globe/` code and data are unchanged.

## Data and reproduction

- RiverATLAS v1.0, South American north and south components of the shapefile download:
  https://doi.org/10.6084/m9.figshare.9890531 . CC BY 4.0. Attribution: Linke et al. (2019),
  https://doi.org/10.1038/s41597-019-0300-6 and Lehner & Grill (2013),
  https://doi.org/10.1002/hyp.9740 . All topology here comes from `NEXT_DOWN`; none is inferred
  from the apparent proximity of lines.
- Natural Earth 1:50m land and rivers, 1:50m populated places, public domain. Land is decoded
  from the existing repository topology; labels were checked against the named river lines
  and the places dataset. Menu anchors snap to the local main stem in RiverATLAS; coarse
  Natural Earth river lines alone are insufficient for exact hit testing.
- Existing GEBCO-derived `Globe/terrain.png` and `geo.png`, for a generalized relief rendering.
- Libertinus Serif, SIL OFL, with its license in `Water/fonts/`.

`Water/manifest.json` records source component SHA-256 hashes and transformation settings.
From external, unmodified RiverATLAS South American `.shp`, `.shx`, and `.dbf` files:

```sh
# Dependencies: pyshp, shapely, numpy, pillow
python3 tools/water/build.py /path/to/riveratlas
python3 tools/water/audit_source.py /path/to/riveratlas
node ci/check_water.cjs
```

The input contains 1,620,963 reaches. Selection keeps upstream area >=500 km² and the exact
downstream closure (218,146 reaches). Chains with no retained branching junction are merged
into 18,159 edges. Geometry is simplified to 0.006° and rounded to 0.001°. Original reach lengths
are summed; only the touched partial reach is estimated along the simplified line. The graph
contains 974 termini, including 322 identified as inland. These are data counts, not a census
of real-world rivers or independent geographical findings.

The source audit verifies every constituent link, output terminus, basin ID, length sum and
outer endpoint against the original download. The ordinary check independently verifies
continuity, acyclicity, basin consistency, selection clipping, and ocean hit rejection.

## Interaction and performance

One pointer traces; two pointers pan and pinch. Move mode supplies one-pointer panning.
Keyboard cursor, zoom buttons, a whole-map control, and a named river selector provide
alternatives. The selector deliberately frames a selected route or upstream tree; direct
brushing never moves the camera. Direction controls, sources and pause are ordinary HTML
controls exposed by WebKit to assistive technology.

Base geography and the selected upstream tree are cached in separate canvases. A spatial grid
limits nearest-line queries; closer views cull offscreen geometry; the frame draws at most 36 transient routes. Drawing resolution is
capped at 2.5 device pixels per point. Idle ink runs at a lower frame rate. Reduced Motion shows
connections immediately without animated ink. The source dialog and hidden document suspend
ongoing animation; the pause control freezes it on demand.

For the bundled browser interaction check, serve `Water/` locally, install Playwright, and run:

```sh
node tools/water/interaction_check.cjs http://127.0.0.1:8765 /tmp/water-check
```

This exercises actual drags, two-touch pinch, controls, dialog scrolling and focus return at
phone, small phone, iPad, large text and reduced motion settings. Browser performance is not
a measurement of iPhone hardware. Native tests use XCTest on both iPhone and iPad simulators.

## Model limits

This is a simplified single-outlet network, not a hydrological simulation. It omits smaller
streams, and does not reproduce distributaries, floodplain exchange, changing flow, dams,
flood risk or navigation conditions. The ink's speed is invented. The documented Casiquiare
connection between the Orinoco and Amazon illustrates an important exception that this tree
cannot express; the bundled notes cite Laraque et al. (2019),
https://doi.org/10.3390/w11102068 . No historical route, cultural connection or causal claim
about a community is encoded by the ink.

## Independent testing release

The existing CI workflow has a branch-restricted opt-in `study2` job, enabled by `[study2]`
in a push message or `study2=true` on manual dispatch. It runs the Linux data gate, Debug and
Release builds, byte-for-byte bundle checks, native UI tests on both simulators, and a device
archive. Only after passing does it create an immutable `study2-<run-id>` prerelease with an IPA
and its own SideStore source. It does not update the `sidestore` branch, prune releases, merge
code or replace the ordinary app. The job never runs on `main`.
