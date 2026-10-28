# The ocean: sources

The ocean is part of the app's stage (`App/TerraIncognita/Ocean/`): a Metal layer over the globe's
drawing and under the clouds. Its web twin (`web/`: `shader.js`, the native `Ocean.metal` line for
line, and a small page to host it) is the desktop preview, what `look.cjs` draws over the globe's
preview; the app has no web view for it. Neither makes requests or keeps anything.

## What it shows

How the sea's surface usually moves in this month, drawn as streamlines: the paths the water takes,
closer together where it runs faster, with light running along them the way it goes. Close in, the
light on the water glitters and the surf comes in to the coasts. Touch the water and it rings.

| | Measured (the record) | Drawn (our choices) |
| --- | --- | --- |
| Which way, how fast | the month's mean near-surface current in each quarter-degree cell (averaged here into half degrees) | the streamlines' spacing (a contour every so much of the stream function, finer as the reader comes closer) |
| Where it was measured | which cells the climatology has a value for, this month | nothing is drawn where no drifter went (under the polar ice, some gulfs): the lines fade out within about 3° of the last measured cell |
| Motion | the current's direction, and its speed relative to other places | the light's pace: about 26 points a second for a metre a second at any zoom, much faster than the water (a drawn pace, not to scale) |
| Light | | the sheen of the sky from the upper left, at the whole Earth; glitter close in, in drifting patches of wind |
| Coasts | where the coast is (the globe's own, `Globe/geo.png`) | the surf: swells coming in a set at a time, and a line of foam |
| Touch | | rings spreading from the finger |

- **The record.** The NOAA Global Drifter Program's climatology of near-surface currents, its
  monthly means: Laurindo, L. C., Mariano, A. J. and Lumpkin, R. (2017), *An improved near-surface
  velocity climatology for the global ocean from drifter observations*, Deep-Sea Research I 124,
  73–92, doi:10.1016/j.dsr.2017.04.009; version 3.10, using drifter data to February 2023; from
  NOAA AOML's ERDDAP (`drifter_monthlymeans`). Licence: Creative Commons Attribution 4.0. "This
  study used data collected and made freely available by the NOAA Global Drifter Program
  (https://www.aoml.noaa.gov/phod/gdp/) accessed from https://erddap.aoml.noaa.gov/gdp/erddap/index.html".
  Drogued drifters follow the water about 15 m down; the climatology combines their tracks, since
  1979, with satellite altimetry. It is a typical month, not this year's sea.
- **Streamlines.** The contours of the month's stream function, solved from the measured currents'
  vorticity (`build.py`): the rotational part of the flow, which carries most of it (it matches the
  measured currents with a correlation of about 0.95 east-west and 0.85 north-south). The rest is
  the sea rising and sinking, as at the equator, which a streamline can't show. Lines are drawn only
  where the water runs faster than a few centimetres a second, fainter where it runs slower: the
  quiet middles of the gyres (the Sargasso Sea) are left calm.
- **Month.** The globe's month, which the ground and the clouds show too: a pack's date to the day
  moves it.
- **Reduce Motion** holds everything still: the light, the glitter (which goes) and the surf; a touch
  leaves a ring that fades where it is.
- **Where it doesn't belong.** Not on the Persian page (a manuscript's water lies still), nor under
  the authored sequences (the camel's and the whale's deep pasts had other seas). While an
  experience runs it thins to about half. The parked prototype *The still sea* hides it, to compare.
- **The native one** draws at up to the screen's own pixels and follows the GPU's timings, coarser
  when a frame costs more than about 4 ms; thirty frames a second while the camera rests. What a
  frame costs is in the workbench's log.

## Credit owed

The drifter climatology is CC BY 4.0: the app should credit the NOAA Global Drifter Program
wherever it credits its sources. It has no credits page yet; one is owed before any public release.
