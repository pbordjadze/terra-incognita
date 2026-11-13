# The ocean: sources

The month's currents (`Globe/ocean/`, built by `build.py`), read by Adrift. The Xcode-built Terra's
ocean layer that first drew them was retired on November 13, 2026 (tag
`before-xcode-retirement-2026-11-13`).

## The data

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
