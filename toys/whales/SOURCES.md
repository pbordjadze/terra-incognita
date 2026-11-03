# Whales: sources

Every recording and dataset the toy carries, what is measured and what is drawn, and its licence.
Nothing is fetched at run time; the built files in `data/` are made by the tools named here (never
hand-edited).

## The songs (`data/humpback.song`, `data/blue.song`; `tools/toys/whales/recordings.py`)

- **Humpback whale song.** MBARI (Monterey Bay Aquarium Research Institute), *Pacific Sound*: the
  MARS cabled observatory's hydrophone in Monterey Bay, 891 m deep, the 16 kHz decimated archive
  (registry.opendata.aws/pacific-sound), recording of 2020-11-28, 00:01:40–00:04:40 UTC: a male
  humpback singing near the hydrophone. Licence: **CC BY 4.0**. Credit: "Humpback whale song:
  MBARI Pacific Sound, MARS hydrophone, Monterey Bay (CC BY 4.0)". Found with
  `tools/toys/whales/scout.py`.
- **Blue whale call.** NOAA Pacific Marine Environmental Laboratory (PMEL) Acoustics Program,
  "Northeast Pacific blue whale call" (`nepblue24s10x.wav`), as NOAA publishes it: played ten times
  faster than it was sung, which brings its ~16 Hz moan into hearing. A work of the US government:
  **public domain**. Credit: "Blue whale call: NOAA PMEL Acoustics Program".

What is done to them, and only this: the band each animal sings in is kept (the rest is the sea's
and ships' noise, and the blue file's clipping), the steady noise floor is lowered gently, the level
evened (a gentle 2:1 compressor and a soft ceiling, so the song is not buried under the
clicks' peaks), the end crossfaded into the start so each loops, resampled to 12 kHz and stored as 4-bit
IMA ADPCM. Nothing is added or synthesised.

## The year (`data/routes.json`; `tools/toys/whales/routes.py`)

Measured (from the literature; the dated places in the tool):

- Humpback, the California–Oregon feeding group that breeds off mainland Mexico: Calambokidis et al.
  2008, *SPLASH: Structure of Populations, Levels of Abundance and Status of Humpback Whales in the
  North Pacific* (final report, Cascadia Research); Urbán et al. 2000, *J. Cetacean Res. Manage.*
  2:101–110.
- Blue whale, the Eastern North Pacific population, satellite tracks: Bailey et al. 2009,
  *Endangered Species Research* 10:93–106; Mate et al. 1999, *Marine Mammal Science* 15:1246–1257.
- When they sing, month by month: Ryan et al. 2019, *PLoS ONE* 14:e0222456 (humpback song in
  Monterey Bay, MBARI's MARS hydrophone); Oestreich et al. 2020, *eLife* 9:e53740 and Stafford et
  al. 2001, *J. Acoust. Soc. Am.* 109:3087 (blue whale song).

Drawn (the toy's, not measured): the way between dated places (great circles, eased) and a slow
wander along it, kept 25 km from the globe's coast; a floor under each whale's singing season so
neither is ever quite silent; the year's pace (a day a second); the rings (how a song looks here:
their timing and colour come from the recording's own loudness and pitch, their spread a drawn
speed, not sound's 1.5 km/s); the hydrophone a touch lowers (a song reaches it when its ring does,
quieter and muffled the farther it came, hardly at all from behind the land: the coast is the globe's
own, the attenuation and muffling are drawn); the land's silence; the whales' shapes and swimming;
the wake.

## The ground

The globe is the kit's (Terra Incognita's data: NASA's Blue Marble and Black Marble, Natural Earth,
GEBCO), credited in Terra's sources.
