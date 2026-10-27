# Time Atlas — data and implementation

An offline, runtime-1 screen pack for the prototype playground. Open **A place through time**
in the workbench. The compact date expands into the timeline; drag to scrub and pinch to
change its scale. Its numeric time camera is independent of the geographic camera.

## Dated maps

André Ourednik and contributors, Historical Basemaps:
https://github.com/aourednik/historical-basemaps/tree/da7a4b735ecef70aebdc9c73e409d8a2500d50f3

Source files are `geojson/world_<year>.geojson` for 1530, 1600, 1815, 1878, 1920,
1938, 1945 and 1994. Imported historical geometry is GPL-3.0; see LICENSE-HISTORY.txt.
The code and physical land are separate from this licensed data collection.

The early two maps contain only Ottoman features. Later maps retain explicitly selected European
polities and components intersecting [-12,35,35,62]. Blank physical ground has no political coverage.
Coordinates are simplified at 0.025 degrees and rounded to four decimal places. Source names and
BORDERPRECISION remain upstream metadata, not independently verified accuracy. Exact allowlists,
exclusions, revision and transformation are recorded in `TIME_DATA.source` inside data.js.
The rebuild tool is `tools/playground/time-atlas/build-data.cjs`.

These are generalized, exploratory maps, not precise event records. The selected year and the
nearest source snapshot are separately labeled. The renderer never morphs border geometry or
asserts that a snapshot was valid throughout the interval to its neighbor. A missing selected
state means no matching feature exists in this extract. It does not describe a people's existence
or imply uninterrupted national identity.

Poland's 1920/1938/1945/1994 shapes provide the principal interaction example. All source years have
uncertainty within the year. In particular, 1945 combines postwar political arrangements rather
than representing a single dated day. This prototype is not suitable for historical instruction
without further verification and richer event-level sourcing.

## Physical land

Natural Earth, public domain, ne_110m_land.geojson:
https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_110m_land.geojson

The physical land is present-day orientation geography, not historical coastline reconstruction.
See LICENSE-NATURAL-EARTH.md. No modern political boundaries are used as a historical reference.

## Host and lifecycle

The playground injects `terra` (runtime 1). This pack subscribes to settings and close, follows
textScale, Reduce Motion and active state, and releases event handlers, observers and animation
frames on close. It makes no requests and stores no reader state. The host owns Back; this page
keeps that button's space clear. There are no shared Swift or Metal edits.

The timeline and map use separate numeric cameras. Selecting a country proposes a span based on
the available dated maps; manually pinching can widen or narrow it. Clearing selection retains the
date and time scale. “This place” holds a point rather than following a state's changing shape.
Playback follows the same numeric year as manual scrubbing, pauses on interaction and does not
resume automatically after backgrounding. Geological data and ship movement are not included.

## Verification

`node tools/playground/time-atlas/core.test.cjs` checks anchor preservation, temporal bounds,
deep-time arithmetic, map inversion, holes and snapshot/subject semantics.

`node tools/playground/time-atlas/check.cjs` exercises Chromium and WebKit browser interactions,
captures phone and iPad layouts, and checks settings and disposal through the injected runtime.

The pack can be previewed with the shared playground server and checked by its ordinary validator.
Browser WebKit is not an iOS simulator or a physical-device performance measurement.
