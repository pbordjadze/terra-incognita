# Clouds, in a web view

The web twin of the native **Clouds** prototype (`App/TerraIncognita/Clouds/`): a runtime-1 globe
pack, an overlay over the globe taking no touches, parked in the workbench. Its shader
(`shader.js`) is the native one's (`Clouds.metal`) line for line, and it is what the desktop tools
(`tools/playground/clouds/look.cjs`, `film.cjs`) draw over the globe's preview. It makes no
requests and keeps nothing. What follows describes both.

## What it shows

The clouds of this month, over the globe, where satellites usually see them. Each place is under
drawn cloud for the share of the time it was seen cloudy in this month, on average from 2000 to 2016,
and its sky is made of the kinds of cloud seen there. The particular clouds are not observed: they
are drawn so that their cover matches the record, and they are not today's weather.

| | Measured (the record) | Drawn (this pack's choices) |
| --- | --- | --- |
| Where | how often each 1° cell is cloudy, this month | the clouds' shapes |
| What kind | the shares of stratocumulus and stratus, altostratus and nimbostratus, cirrostratus, deep convection, cumulus, and thin clouds aloft | how each kind looks: decks, sheets, towers, puffs, filaments |
| When | the month (the day between two months is a blend of them) | the drift, a simulated day a minute |
| Weather | | the cyclones in the westerlies: five a hemisphere, each six days, at invented places between 38° and 60° |

- **Thick or thin.** Clouds whose optical thickness is above 3.55 (ISCCP's medium and thick; they
  mostly hide the ground) are drawn as clouds; the thin ones above the low clouds (altocumulus,
  cirrus) as faint filaments. Cumulus, low and thin, appear only from about twice the whole
  globe's size, as small puffs: at the whole Earth they would be finer than a pixel.
- **Winds.** Each kind drifts the way the winds usually blow at its height: west in the tropics (the
  trade winds), east beyond about 30° (the westerlies), faster aloft. The speeds are round
  approximations to typical zonal means, not a reanalysis; the polar caps turn with the westerlies.
- **Cyclones.** Anticlockwise in the north, clockwise in the south, as cyclones turn. Where they
  show depends on the measured cloud.
- **Light and shadow.** Lit from the north-west, as the globe's relief is. Shadows fall to the
  south-east, offset by a few points (more for higher clouds): a convention for height, not to
  scale. On the sea they are a cool blue-grey; on land, the globe's own shadow colour (land.png:
  the globe's coast, from Globe/geo.png).
- **Coming closer.** Pinching in, the reader comes down through the clouds: the high ones first,
  then the middle and the low, each growing about the screen's centre (nearer the eye than the
  ground) and parting as it passes, its shadow going with it. Below them, cumulus float over the
  ground. The heights at which each passes are a drawing convention, not altitudes.
- **Reduce Motion** holds the clouds still (no drift). The descent follows the reader's own pinch.
- **The native one** draws at a fraction of the screen's pixels (about 1.25 a point to start) and
  follows the GPU's own timings, finer while a frame is cheap, coarser when it isn't; the
  workbench's log says what a frame costs. Its look (`CloudsLook`) holds the shadows' colours and the
  light, so a globe drawn another way can give its own, a sun among them.

## Data

**ISCCP-H**, the International Satellite Cloud Climatology Project's H-series, its gridded
monthly means (HGM, "ISCCP Basic", product version v01r00), 1° equal-angle cells, combining
geostationary and polar-orbiting satellites' views every three hours. Licence, as stated in each
file: "No constraints on data access or use."

- Cite as (NCEI's form): Rossow, W.B.; Walker, A.; Golea, V.; Knapp, K. R.; Young, A.; Inamdar A.;
  Hankins, B.; and NOAA's Climate Data Record Program (2016): International Satellite Cloud
  Climatology Project Climate Data Record, H-Series (HGM Basic, monthly means, January 2000 –
  December 2016). NOAA National Centers for Environmental Information. Accessed October 28, 2026.
  doi:10.7289/V5QZ281S
- And, as the files ask: Rossow, W.B. and R.A. Schiffer, 1999: Advances in understanding clouds from
  ISCCP. Bulletin of the American Meteorological Society, 80, 2261–2287.
  doi:10.1175/1520-0477(1999)080<2261:AIUCFI>2.0.CO;2

`sky.png` (and the native `CloudsSky.png`, the same file) is made by `tools/playground/clouds/build.py` (its docstring is the encoding): the 204
months averaged into twelve; ISCCP's eighteen cloud types grouped by height and thickness; each field
smoothed by about a cell (more towards the poles, where cloud over snow and ice is least certain;
some blocks and seams in the record would otherwise look like weather). Types are known only by day,
so the kinds are shares of the daytime clouds, applied to the day-and-night amount.

## Limits

- A month's average is not a day: the record says how often, not when. A place cloudy half the time
  is drawn half covered, all the time.
- 1° cells, smoothed: a coast's sharp edge of cloud (fog against a desert shore) is softened.
- The polar regions, and anywhere over snow and ice, are the least certain part of the record.
- The drift's speeds, the cyclones, the heights and the light are drawn, as above.
