# Adrift: sources

What the floats ride is measured; what is drawn from it is ours. This file tells the two apart for
every sheet the rules read (Adrift's own in `adrift/data/`, built by `tools/adrift/data/`, never
hand-edited; Terra's borrowed as they are; `adrift/data/manifest.json` says where each is and how
it is decoded) and names the credit each dataset is owed. Nothing is fetched at run
time; everything is bundled.

## What a float does

| | Measured (the record) | Drawn (our choices) |
| --- | --- | --- |
| The current | the month's mean near-surface current in each half-degree cell, from the drifters' climatology (`currents.png`); the stream function solved from it (`stream.png`) | a float moves with the cell's mean current, interpolated between cells and between months by the day of the year |
| The eddy wander | the drifters' velocity variance about that mean, cell by cell, and their Lagrangian integral time scale, measured here from their tracks in 10° boxes (`eddy.png`, `eddy-time.png`, `eddy-time.json`) | a random flight: the float carries an eddy velocity that decays over the measured time scale and is kicked so its variance stays the measured K / T, so a float keeps its way for a few days as a drifter does and the cloud of its maybes spreads as 2 K t beyond that (Taylor 1921); a float is put with a first velocity drawn from that same measured variance (the water is already moving), and each maybe with its own; deterministic for a seed, so a path can be replayed. (`Rules::step` is the plainer random walk, the same spread without the memory.) |
| The wind | the month's mean 10 m wind in each degree cell (`wind.png`) | a float's leeway: 3% of that wind, downwind (below) |
| The coast | the globe's own signed distance to the shore (`coast.png`) | a float that steps onto land is put back at the water's edge and held there |
| The ice | each month's usual sea-ice concentration (`ice.png`) | a float is held where the concentration is 15% or more (the Sea Ice Index's own edge) and let go when the month's ice retreats past it |
| The real drifters | a subsample of the Global Drifter Program's tracks at one point a day (`drifters.bin`) | which run is "the nearest that passed there": the one whose point lies closest to the float's, in any year |
| The date | | the climatology is a typical year, not a given one: the date on the screen is the day of a typical year, and a float's path is what a typical year's sea would do, never what the sea did on that day |

## The datasets

- **The current and its variance.** The NOAA Global Drifter Program's climatology of near-surface
  currents: Laurindo, L. C., Mariano, A. J. and Lumpkin, R. (2017), *An improved near-surface
  velocity climatology for the global ocean from drifter observations*, Deep-Sea Research I 124,
  73–92, doi:10.1016/j.dsr.2017.04.009; version 3.10, drifter data to February 2023; its monthly
  means (`drifter_monthlymeans`) and its current variance (`drifter_currentvariance`: the residual
  about the time-mean, seasonal, spatial-gradient and SOI-related currents, with the drifter-days
  that measured each cell), from NOAA AOML's ERDDAP. Licence: Creative Commons Attribution 4.0.
  Drogued drifters follow the water about 15 m down; the climatology combines their tracks since
  1979 with satellite altimetry. The currents sheet is Terra's own, read as it is
  (`App/TerraIncognita/Ocean/OceanCurrents.png`, `tools/ocean/build.py`): where no drifter went (under the
  polar ice, some gulfs) the current is smoothed in from the measured cells and the sheet says so
  in its B channel; the rules treat a float there as riding a guess.
- **The drifters' tracks.** The same programme's 6-hourly interpolated trajectories
  (`drifter_6hour_qc`; Lumpkin, R. and Centurioni, L., *Global Drifter Program quality-controlled
  6-hour interpolated data from ocean surface drifting buoys*, NOAA NCEI, doi:10.25921/7ntx-z961;
  CC BY 4.0), the years 2016–2023, used twice: whole, to measure the eddy time scales; and
  subsampled, to bundle (below).
- **The eddy time scale** (`tools/adrift/data/eddy.py`). Each drogued drifter's 6-hourly velocity
  about the climatology's monthly mean at its place; its autocorrelation over lags of 6 hours in
  10° × 10° boxes, integrated to the first zero crossing (Taylor 1921; Davis 1991; the method of
  Zhurbas, V. and Oh, I. S. (2004), *Drifter-derived maps of lateral diffusivity in the Pacific and
  Atlantic Oceans in relation to surface circulation patterns*, J. Geophys. Res. 109, C05015,
  doi:10.1029/2003JC002241), capped at 10 days. Measured (2016–2023, 1.6 million drogued drifter-days): zonal time scales of 1.6 to 4.5
  days (the 10th to 90th percentile of the measured boxes; median 2.8), meridional 1.3 to 3.2
  (median 2.1), as the literature has them (Lumpkin,
  Treguier and Speer 2002; Zhurbas and Oh 2004). Then K = ⟨u′²⟩ T cell by cell, zonal and
  meridional: a median of about 4,800 m²/s zonal and 3,200 meridional in the measured sea, about
  1,000 in the quiet gyres, tens of thousands in the Gulf Stream, Kuroshio, Agulhas and the equatorial bands. Honest limits:
  the box time scale is smoothed over about 15°; in the jets the variance holds the current's own
  meandering as well as its eddies (Zhurbas and Oh's shear correction is not made) and K is
  capped at 40,000 m²/s in the sheet. Checked against the record: of the bundled drifters that
  passed Cape Hatteras, half were east of 65° W thirty days on (median 64.8° W); of 500 flights
  put there in a typical July, 46% are (median 65.9° W), with 6 on a coast.
- **The wind.** The NCEP/NCAR Reanalysis 1 (Kalnay, E. et al. (1996), *The NCEP/NCAR 40-Year
  Reanalysis Project*, Bull. Amer. Meteor. Soc. 77, 437–471), its long-term monthly means of the
  10 m wind over 1991–2020, from NOAA PSL (public domain; "NCEP/NCAR Reanalysis 1 data provided by
  the NOAA PSL, Boulder, Colorado, USA, from their website at https://psl.noaa.gov"). A monthly
  mean is the mean of a wind that veers and gusts: a float's leeway on it is a typical month's,
  smoother than any real month's.
- **The leeway coefficient.** A floating object's leeway is a few percent of the 10 m wind,
  measured object by object (Allen, A. A. and Plourde, J. V. (1999), *Review of Leeway: Field
  Experiments and Implementation*, U.S. Coast Guard R&D Center; Breivik, Ø., Allen, A. A.,
  Maisondieu, C. and Roth, J. C. (2011), *Wind-induced drift of objects at sea: The leeway field
  method*, Applied Ocean Research 33, 100–109). No glass float has been measured. The nearest
  measured object is a sphere: Breivik et al.'s WWII L-MK2 mine, 105 cm across, three quarters
  under, drifting downwind at 2.0% of the wind with negligible crosswind leeway. The scaling of
  Daniel, P. et al. (2002), leeway/wind = √(ρ_air A_air / (ρ_water A_water)) with A the profile
  areas above and below the waterline, gives that sphere 1.7%; Adrift's float, a glass float with
  its lamp and cell in the bottom, floats three fifths under, for which the same scaling gives
  2.7%, or 3.1% calibrated on the mine. **Adrift uses 3.0%, downwind, no crosswind component.**
  A drifter's own slip is smaller (about 1% once its drogue is lost: Poulain, Gerin, Mauri and
  Pennel 2009) and is already in the climatology.
- **The ice.** The NSIDC Sea Ice Index, version 4 (Fetterer, F., Knowles, K., Meier, W. N.,
  Savoie, M. and Windnagel, A. K., *Sea Ice Index, Version 4*, NSIDC, doi:10.7265/N5K072F8): each
  month's mean concentration over 2015–2024 on the satellites' 25 km grid, through Terra's own
  `Globe/ground/ice-MM.png` (`tools/globe/ground.py`); the pole the satellites never see counted
  as ice.
- **The coast.** Natural Earth's 1:50m land and lakes (public domain, "Made with Natural Earth"),
  through Terra's `Globe/geo.png` (`tools/globe/build.py`): the signed distance to the shore at
  4096 × 2048, about 10 km a pixel at the equator. A float is held at that shore: an island smaller
  than a pixel is not there, and the shelf's shallows are open sea.
- **The bundled drifters** (`tools/adrift/data/drifters.py`). Runs of whole days at 00:00 UTC
  from the 6-hourly tracks of 2016–2023, drogued or not (a drifter that lost its drogue still went
  where the sea took it, a little more with the wind: the run's flag says), chosen greedily by the
  2° cells they add until each cell the programme's drifters passed has three runs through it or
  the budget of 4 MB is spent, positions to 1/180° (about 600 m). What is on the screen in silver
  is a real track, a real year, a real buoy with its GDP identifier; it is not the float's own
  forecast, and no day on it is the day on the screen.

## What the screen says, and where each thing comes from

| On the screen | Its source |
| --- | --- |
| The date ("19 July") and its note ("a typical year, by the drifters' record") | `Clock` read against the climatology's typical year (`sea::sheets::months`); never a year |
| A float's place and wake | `Rules::flight` on the sheets above, one bead a day |
| A held float's maybes | the same rules, k = 1..n, on the GPU (`rules.wgsl`), from the put place and day |
| The silver drifter and its label ("Drifter 145813, 2 Jun to 18 Jul 2016") | `Drifters::track`: a real GDP buoy, its real calendar dates, the month about where it passed nearest |
| The credit line | `sea::CREDIT_LINE`; the full sentences, `sea::CREDITS` |

## How many maybes, and how often

Measured (`sea::tests::maybes_budget`, a tool run by hand): thirty days after a put off Cape
Hatteras (a jet) and in the Sargasso Sea (quiet), clouds of 256 to 2048 paths at steps of a
quarter, a half and a whole day against 8192 paths at a quarter day. **512 paths at a half-day
step tell the same shape** (Kolmogorov's difference of the clouds' cumulative shares under 0.05
along both axes, the share east of 65° W and the spread within the reference's noise), at a
sixteenth of 2048 paths at a quarter day. A whole-day step is not honest in a jet: the paths jump
the shelf and the coast catches nearly twice as many (73 of 2048 against 41), and the spread
across the stream narrows. The cloud need not be re-run more often than the day moves half a
day.

## Credit owed

The app credits, wherever it credits its sources: the NOAA Global Drifter Program (CC BY 4.0: "This
study used data collected and made freely available by the NOAA Global Drifter Program
(https://www.aoml.noaa.gov/phod/gdp/) accessed from https://erddap.aoml.noaa.gov/gdp/erddap/index.html"),
with Laurindo, Mariano and Lumpkin (2017) for the climatology and Lumpkin and Centurioni for the
trajectories; the NCEP/NCAR Reanalysis 1 from NOAA PSL; the NSIDC Sea Ice Index, version 4; and
Natural Earth. The credits are drawn by Visual from `manifest.json`'s `credit` fields, so a sheet
rebuilt from a new source carries its own credit.
