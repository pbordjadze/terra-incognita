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
| The eddy wander | the drifters' velocity variance about that mean, cell by cell, and their Lagrangian integral time scale, measured here from their tracks in 10° boxes (`eddy.png`, `eddy-time.png`, `eddy-time.json`) | a random flight: the float carries an eddy velocity that decays over the measured time scale and is kicked so its variance stays the measured K / T, so a float keeps its way for a few days as a drifter does and the cloud of its maybes spreads as 2 K t beyond that (Taylor 1921); a float is put with a first velocity drawn from that same measured variance (the water is already moving), and each maybe with its own. The kicks are not each path's own dice but a world's: world k is one random eddy field, shared by every float's k-th path (below: The worlds); deterministic, so a path can be replayed. (`Rules::step` is the plainer random walk, the same spread without the memory.) |
| The wind | the month's mean 10 m wind in each degree cell (`wind.png`) | a float's leeway: 3% of that wind, downwind (below); the wind preset's things ride all of it, with measured gusts (below: The wind) |
| The coast | the globe's own signed distance to the shore (Terra's `geo.png`) | a float that steps onto land is put back at the water's edge, one of the coast's own pixels (about 10 km) off, and held there |
| Where it may be put | how far the nearest drifter-measured cell is, by month (the currents sheet's B; `Rules::record`) | anywhere the mask shows water; where no drifter went the float rides the smoothed-in current and wander, plausibly (below: Where a float may be put) |
| Between steps | | the drawn float glides along its velocity (`Rules::glide`) between its quarter-day steps, so it runs rather than ticks |
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

## Where a float may be put, and what unrecorded water does

A float may be put on any water the coast mask shows (truth underneath, not on top: DECISIONS.md
§ 11). Where no drifter of the programme ever came within three degrees (the Caspian, the Great
Lakes, Baikal, Victoria, Hudson Bay, the Red Sea, the Persian Gulf, the seas under the ice: 7% of
the sea by area) the sheets hold a current and a wander smoothed in from the nearest measured
water, hundreds of kilometres away, which is no record of that water. There the float rides no
current at all: it goes on the wind's leeway and the wander alone (`rules::reach`: the current
weighted by the record's reach, whole at 0.5 and over, none at 0, in proportion between), the way
a lake's surface drifts with the wind; the smoothed-in current would have run a Red Sea float at
68 cm/s on the Gulf of Aden's monsoon and piled a lake's floats on one shore. With the wander
(the Caspian's about 32 km a day, Lake Michigan's 48, Hudson Bay's 27, all smoothed in too and
kept, so the water looks alive) nothing sits dead and nothing jitters, and the coast holds what
reaches it.
What is true is written here, not on the screen: `Rules::record(at)` says how far the nearest
measured cell is (1 there, 0 three degrees away) and `Rules::may_put(at)` sorts water into
measured, a reach, and no record, for anyone who wants the honest answer; nothing in the app
calls them. The ice is different: a float put on the month's ice is held (`Held::Ice`) until the
ice retreats, which is the record speaking.

## The coast against the phone

The coast is the globe's distance field at 4096 pixels round, about 9.8 km a pixel at the
equator, narrower east-west toward the poles. A held float sits one pixel off the shore. At the
phone's closest zoom (8) the globe's radius is about 1,440 points, so a point is about 4.4 km: the
mask's pixel is 2.2 points, the hold-off 2.2 points, and the float's own symbol (7 points round)
31 km across. At the whole Earth (zoom 1) a point is 35 km and the symbol 250 km across: a float
put a safe 20 km off Panama draws over the isthmus. So a float "on the coast" in the owner's
screenshot is the symbol's size on the sea, not the mask's: the mask is finer than the symbol at
every zoom, and finer than the point from zoom 4.5 in. What would fix the look at the whole Earth
is the symbol's (Visual's) to decide; the mask would need 8192 pixels only for a zoom beyond 8.

## Time is reversible

A step is a pure function of its arguments: `Rules::flight(state, day, dt, seed, k)` reads the
sheets and hashes the path's world (k) and the quarter days the step spans (from its start day
and length as f32; the walk, `Rules::step`, still hashes the seed, the path and the start day's
bits) and nothing else; no call remembers the last. A float's place at any day is therefore the same whichever way
the date was scrubbed to reach it, if the app steps it on a fixed grid from its put: a quarter
day is exact in f32 for any day under 2^21 (the year 7745), so a grid mark always gives the same
bits, and a day off the grid (the put's first, short step) is rounded to f32 once, the same way
every time (`sea::tests::a_step_is_a_pure_function_of_its_arguments`). The maybes on the GPU are
the same rules, so a kept cloud restored and stepped on is bit for bit the cloud stepped straight
through (`a_kept_cloud_restores_and_steps_on_the_same`). A step costs about a microsecond on a
Linux box at the dev profile (`step_cost`, a tool): thirty floats scrubbed a hundred days is some
ten milliseconds, and the maybes' steps ride one command buffer a scrub (`Maybes::step_many`).

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

## The worlds (R6)

What is measured: at each place, how strongly the drifters' velocity strays from the mean (K / T)
and for how long it remembers (T). What is drawn: which eddies are where. Before R6 every path
drew its own kicks, so two floats put 50 km apart met two unrelated seas from the first step,
which no real pair of drifters does: water a few kilometres apart moves alike. Now path k of every
float lives in world k, one random eddy field (`rules::world_field`): two standard draws on a
lattice a degree square (about 110 km north-south, the size of the mesoscale eddies that part
drifter pairs; Chelton et al. 2011 measure their radii at 50 to 150 km in mid-latitudes), a fresh
lattice each quarter day, mixed bilinearly between nodes and renormalised so a kick is a unit
draw everywhere. The lattice's width is the one number here not measured per place. A step's kick
is the quarter days whose middles it spans, summed and renormalised, so the lamp's quarter-day
steps and the maybes' half-day steps read the same field. One path alone sees exactly what it saw
before (a unit kick a step: the same K / T and T; every single-cloud test passes unchanged and
the Hatteras record holds), so a cloud as a whole is the same in law; what is new is that the
k-th selves of two floats can be compared, because they met the same sea.

Two lamps put 50 km apart in a typical July, 1024 worlds, half-day steps
(`sea::tests::two_lamps_fifty_km_apart`, a tool; with ADRIFT_OUT it draws them):

| place | days | centroids apart | spread (rms) | k-th selves apart, same world: p10 / p50 / p90 | different worlds (as before R6) |
| --- | --- | --- | --- | --- | --- |
| jet: Gulf Stream off Hatteras | 30 | 83 km | 637 km | 135 / 584 / 1249 km | 265 / 716 / 1359 km |
| | 90 | 101 km | 1034 km | 298 / 1038 / 2244 km | 416 / 1188 / 2344 km |
| gyre's middle: Sargasso | 30 | 43 km | 165 km | 10 / 79 / 258 km | 77 / 197 / 357 km |
| | 90 | 50 km | 342 km | 9 / 201 / 581 km | 152 / 388 / 743 km |
| front: Brazil-Malvinas confluence | 30 | 47 km | 496 km | 88 / 464 / 939 km | 226 / 608 / 1074 km |
| front: Kuroshio Extension | 30 | 35 km | 550 km | 130 / 513 / 1068 km | 238 / 653 / 1159 km |

The honest reading. Two clouds 50 km apart stay nearly one cloud: their centroids stay 35 to 100
km apart in every place tried, because the record's mean current is a half-degree climatology and
smooth at that distance, and the measured wander is the same for both. What diverges is the
pair in each world: in the gyre's middle half the worlds keep the two lamps' selves within 80 km
for a month and a tenth keep them within 10 km for three; in a jet or a front most worlds part
them by hundreds of kilometres within a month and a tenth by more than a thousand. That is what
real pairs do (pairs that start close separate slowly in quiet water and fast where the flow
shears; LaCasce 2008 reviews it); the numbers here are not held to measured pair statistics yet,
which the bundled tracks could do where two drifters were launched together.

The lamp is k = 0, world 0: two lamps meet the same sea too, and a lamp put twice at the same
place and day takes the same path. It is one of its own cloud: ranked within its cloud along
each axis over 160 seeds, its rank is uniform at Hatteras, the Sargasso and the Agulhas (means
0.48 to 0.53; `sea::tests::the_lamp_is_one_of_its_cloud`, a tool). The cost: the kick reads four
lattice nodes a quarter day, about the hashing the twelve-uniform draws took; 8 clouds of 2048
paths at 24 half-day steps cost the same on the software GPU as before (95 ms against 95 to 104
ms, about 240 ns a path-step on llvmpipe; `sea::tests::gpu_step_cost`, a tool).

## The wind (R10, the second medium: things on the wind)

The wind preset's things (a pollen grain, a down seed) are carried by `sea::wind::AirRules`
(mirrored by `wind.wgsl`; the machine is the sea's flight, `sea/MEDIA.md`).

| | Measured (the record) | Drawn (our choices) |
| --- | --- | --- |
| The mean wind | the month's mean 10 m wind, NCEP/NCAR Reanalysis 1, 1991–2020 (`wind.png`, as the sea's leeway reads it) | a thing rides all of it (carry 1); one lifted a kilometre up would ride a faster wind than the 10 m one |
| The gusts | the 10 m wind's standard deviation about each year-month's own mean, east and north, over five years of six-hourly reanalysis (2016–2020), by month and degree cell; and their integral time scale (the anomaly's autocorrelation summed to its first zero) (`gusts.png` R, G, B; `tools/adrift/data/weather.py`) | an Ornstein-Uhlenbeck gust velocity with that variance and time scale; the time scale is the fixed point's (Eulerian), used for the moving thing's (Lagrangian). The 10-day spread that results is held to real paths (below), and agrees |
| The weather's width | the distance at which the anomaly's correlation falls to 1/e: 600–1100 km east, 500–800 km north, by latitude band (`weather.json` › scale) | world k's gust field on a lattice 15° east by 8° north (`wind::AIR_CELL`), bilinear between nodes, fresh each quarter day: roughly that width at low and middle latitudes, narrower in kilometres toward the poles (a lattice in degrees) |
| Rain | the month's mean precipitation rate, reanalysis 1991–2020 (`gusts.png` A) | rain-out at 0.36 per day per mm/day: a below-cloud scavenging coefficient of 1e-4 /s at 1 mm/h for coarse particles, linear in the rate (heavy tropical showers wash out less per millimetre than this says, so it is harsh there); one shower falls on every thing within 2.5° (`RAIN_CELL`), in world k |
| Settling | a birch pollen grain falls at about 2 cm/s (1.2–3 cm/s measured) | it comes down at 0.86 a day: that speed through a kilometre-deep mixed layer by day, nothing by night (the air above the night's shallow layer is cut off from the ground). The thing's own dice, not the world's |
| The coast, mountains | the coast sheet says where a thing came down (land or sea) | nothing stops a thing in flight; mountains are only in the 10 m wind itself (the reanalysis's own smoothed terrain) |
| Near the poles | | a step moves along the great circle beyond 75° of latitude (it crosses the pole), and in degrees nearer the equator, as the sea's step does |

**Held to the record** (`cargo test -p adrift-core wind_tests`): from 336 starts over the globe (1344 start-months) in
January, April, July and October, the maybes' 10-day spread (80 worlds each, no settling) against the
spread of ten real 10-day paths through the reanalysis's six-hourly wind from the same starts
(each year's 1st and 15th, 2016–2020; `weather.json` › paths): median ratio 1.04, 10th–90th
percentile 0.73–1.54. The cloud's middle lands within 0.42 of the real spread of the real paths'
middle (median). Two things lifted 40 km apart are 58 km apart after three days in one world and
1530 km in two. A typical July's flight: 1.06 days from the Sahara, 0.1 day from the western
Pacific's warm pool (rain).

**What it cannot say.** A thing's real path on a real day: the weather is drawn, a world's, with
the measured statistics. The time scale is a fixed point's. Lofting (convection carrying a seed a
kilometre up, where the wind is faster and the settling slower) is not modelled; the lifetime
dial stands in for it. Pollen is measured; a balloon or ash would ride other winds.

## The wind's preset (R21): a seed's life, the pace, the tempo

**What a seed is.** A seed lifted higher than the measured pollen grain: a light down seed (a
thistle's, a dandelion's) carried up a kilometre or two by the day's convection. It is not a balloon:
a balloon does not settle, it leaks and bursts, and no number here is a balloon's. Draw it as a seed.

**Its life** (`AirRules::LIFE`, the app's rules `AirRules::sharing`; `a_seeds_life`, from 216
places over land and sea, 8 worlds each):

| | fall (per day) | washout (per day per mm/day) | life, median (quartiles; 90%) | seconds at 0.3 days a second |
| --- | --- | --- | --- | --- |
| Measured: birch pollen near the ground | 0.86 | 0.36 | 0.25–0.5 days (0.25–1.0; 1.25–2.0) | 1–2 s |
| **The preset** (chosen: 4x) | 0.215 | 0.09 | 2.0–2.25 days (0.75–3.25; 4.5) | 7 s median, 15 s for the longest tenth |

Measured: the settling speed, the rain, the scavenging coefficient. Chosen: the factor 4 on both,
one dial (`sea/MEDIA.md` › lifetime). It stands in for lofting, which is not modelled: a seed
carried into a layer two kilometres deep, mixed day and night, would settle about 4x slower than a
grain in the day's one-kilometre layer that is cut off by night; rain would wash it out about as
fast as below the cloud, so the dial is kind to the rain. A seed's quarter-day marks still decide
when it comes down (the CPU's and the GPU's alike), at most once a mark.

**The pace and the tempo** (`the_winds_tempo`, as R11 measured the sea's, scale-free: the cloud
framed 100 pt in RMS radius, the sea's lamp off Hatteras 15.5 days out, seeds over Indiana and off
Virginia in May 1.5 days out):

| | spread of the worlds in a 1 s stroke | in a 0.5 s stroke | turned a quarter (1 s circle) | sweep matched |
| --- | --- | --- | --- | --- |
| Sea at 3 (its tempo) | 34 pt | 19 pt | 6% | 12–13% |
| Wind at 0.45 (`AIR_TEMPO`) | 40–41 pt | 21–22 pt | 7–8% | 10–12% |
| Wind at 0.8 | 57 pt | 40–41 pt | 7–8% | 11–13% |
| Wind at 1.2 | 86–87 pt | 40–41 pt | 9% | 12–14% |

So the first guesses hold: at 0.45 a stroke spreads the seeds' worlds as the sea's tempo spreads
the lamps', and the free pace 0.3 (`AIR_RATE`, the same 2:3 as the sea's 2 to 3) moves a seed about
25 pt a second across a 100 pt cloud, as the sea's 2 does a lamp. Faster blurs (1.2: a second's
stroke spans 87 pt, most of the cloud). The wind's tempo is coarse where the sea's is fine: a
quarter-day mark is 0.55 s at 0.45 and spreads the worlds 22 pt (the sea's: 3 pt), because a seed's
cloud is young (a few marks old) and a mark is a large share of its life.

**What the chooser does on the wind** (`conducting_on_the_wind`). A tap and a sweep do what they
do on the sea, among the seeds still up. A circle has no spinning law to find in the air: what it
picks is a world whose gust happened to turn over the stroke's day, and since the gust's memory is
about a day (the sea's eddy: three), the turn rarely goes on after the cue: it reads as a swerve,
not a loop.

**Later, for the sea** (Visual's zigzag question, R10): a Gulf Stream lamp's wander stalls and
doubles back (the random flight's velocity wanders through zero), where real drifters there more
often loop in coherent rings. A spinning random flight (Veneziani, Griffa, Reynolds and Mariano
2004: the OU velocity rotated at a measured rate) would turn the stalls into loops. Worth
measuring the spin from the drifters' tracks the way `eddy.py` measures T.

## The spinning eddies (R12)

| | Measured (the record) | Drawn (our choices) |
| --- | --- | --- |
| That drifters loop | the bundled drifters' daily tracks (1979 runs), their velocity less a 15-day running mean, in 10-day windows: 41% turn consistently (lag-a-day rotation coherence over 0.7, more than 0.25 rad a day) at a median 0.58 rad a day (a loop in 11 days), two in three anticyclonic | |
| How much of that is chance | the same statistic on a plain random flight (T = 3 days, no spin, sampled daily) gives 39%, half each sense: most of the measured "looping" is the wander's chance turning; the anticyclonic excess (65% against 50%) is the signal | a share of the sea, `SPIN_SHARE` 15% of world k's 1° cells (each drawn afresh every 30 days at its own phase), is an eddy spinning at 0.58 rad a day, 80% of them anticyclonic: rough, from the excess, and said so |
| The flight in one | | Veneziani et al. (2004)'s spinning random flight: the eddy velocity turns at the spin as it decays, at the measured variance (loopers in the record run only a fifth faster than the rest), so a float there curls, and its spread is less (K / (1 + (Omega T)^2)). The other 85% of the sea is unchanged, and every spread and record test still passes |

**What it shows, and what it can't.** With the measured memory (T about 3 days) against the spin
(Omega T about 1.6) the velocity forgets before it closes a circle. A spinning world curls (a hook
or a half loop over a week) rather than tracing rings, and in a jet (the Gulf Stream at 1 m/s) the
curl is a wiggle on the drift. A circle cue picks such a world, of the circle's sense
(`conduct::tests::a_circle_picks_a_world_that_loops_that_way`: in the open Atlantic both senses'
picks are spinning worlds, curling on beyond 97% of the worlds for six days after the cue;
`tools/adrift/data/critics/r12/circle-cue.png`). The curl is 20 to 40 km across, a few points to a
dozen at a take's zoom. Big visible loops are Gulf Stream rings and the like (100 km, orbits near
1 m/s): coherent eddies the regional statistics average away. They would need a census of them
(e.g. Chelton's eddy atlas, which tracks each eddy's centre, radius and speed), not more dice.

## Whales (not in the app; what it would take)

Whales could inhabit Adrift honestly only as the silver drifter does: real tagged animals, drawn
as data, each with its tag, its species, its real dates and its dataset's credit; never as rules.
A float drifts, so the sea's rules are its truth; a whale swims, and any rule for where it goes
(prey, season, the sea) would be a story Adrift invented. What it would take, for a next round:

- **Data.** Satellite-tag tracks published for reuse: the Movebank Data Repository publishes
  studies under CC0 (blue whales of the eastern North Pacific, Oregon State's Marine Mammal
  Institute, from Bailey, Mate and others' Argos tags; humpback and other baleen-whale tags from
  the same group), each with a DOI to cite; NOAA's Northwest Fisheries Science Center's satellite
  tags on Southern Resident killer whales (Hanson and others, 2012–2016), a federal work, by
  request; OBIS-SEAMAP's telemetry sets, whose licences are the contributors' own and often
  non-commercial, to be read one by one. Nothing scraped, nothing whose terms were not read:
  a dataset enters `adrift/data` only with its licence and credit in this file and in
  `manifest.json`.
- **The sheet.** `drifters.bin`'s layout fits as it is (a run per tag at a point a day, with a
  species in the flags); a few hundred tracks of a few hundred days are under a megabyte. The
  builder would be `tools/adrift/data/whales.py`, reading each dataset's own file, with its
  docstring naming every source and term.
- **The rules.** None: the tracks are shown, not simulated. The nearest whale that passed a
  float's water could rise as the drifter does, with its name and dates (`Drifters::track` works
  unchanged on a second file); or whales could glint on their own waters by the month of the year,
  as the tracks' own dates place them. Either needs a mark from Visual and a seam from
  Engineering (a second `Drifters` on the `App`, a `LookState` field), asked as demands then.
- **The honesty.** A tag's track has gaps and ends when the tag falls off; a whale shown at a
  date is one that was there in that year, not a typical year's whale; the label must say so, as
  the drifter's does.

## Credit owed

The app credits, wherever it credits its sources: the NOAA Global Drifter Program (CC BY 4.0: "This
study used data collected and made freely available by the NOAA Global Drifter Program
(https://www.aoml.noaa.gov/phod/gdp/) accessed from https://erddap.aoml.noaa.gov/gdp/erddap/index.html"),
with Laurindo, Mariano and Lumpkin (2017) for the climatology and Lumpkin and Centurioni for the
trajectories; the NCEP/NCAR Reanalysis 1 from NOAA PSL; the NSIDC Sea Ice Index, version 4; and
Natural Earth. The credits are drawn by Visual from `manifest.json`'s `credit` fields, so a sheet
rebuilt from a new source carries its own credit.
