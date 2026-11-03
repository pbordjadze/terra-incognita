# Murmuration: sources

What is measured, what is drawn, and where each comes from. No data file is bundled: the sky and the
flock are worked out from the constants and rules below (src/sun.rs, src/sky.rs, src/flock.rs).

## The place and the hour (measured)

- **Ham Wall, Somerset Levels** (51.153° N, 2.787° W), an RSPB reserve whose reed beds hold a winter
  starling roost. The toy shows the dusk of the device's own date there (UTC day), from 24 minutes
  before sunset to 28 after, at one and a half times the real pace, then rests.
- **The sun's position**: NOAA Global Monitoring Laboratory's Solar Calculator formulas (public
  domain, after J. Meeus, *Astronomical Algorithms*, 1991). Sunset is the moment the upper limb meets
  the horizon (−0.833°, refraction included).

## The sky (measured constants, drawn method)

- Single scattering on a spherical Earth (so its shadow rises in the east), with Rayleigh, Mie and
  ozone coefficients as in E. Bruneton, *Precomputed Atmospheric Scattering: a New Implementation*
  (2017, BSD licence; constants after Bucholtz 1995, Penndorf 1957 and the US Standard Atmosphere's
  ozone profile). Code written here; nothing copied.
- Drawn: the further bounces taken as one more scattering of the sky's mean light; the sun's light
  white above the air; the eye's adaptation (an exposure that follows the sky only partly, and a
  white balance a little toward the dusk's warm light); the skyline of hedges, trees and low hills.

## The flock (measured rules, drawn weights)

- Each bird heeds its seven nearest neighbours by rank, not distance: M. Ballerini et al., "Interaction
  ruling animal collective behavior depends on topological rather than metric distance", *PNAS* 105
  (2008) 1232–1237.
- Cruise speed about 10 m/s, turning by banking, stronger cohesion at the flock's edge, and attraction
  to the roost: the StarDisplay model, H. Hildenbrandt, C. Carere & C. K. Hemelrijk, "Self-organized
  aerial displays of thousands of starlings: a model", *Behavioral Ecology* 21 (2010) 1349–1359.
  Simplified here: the weights, the noise and the flock's size (4,000 birds) are drawn.
- The wings lie in the plane the bird banks in, so birds turning together show more wing and darken,
  which is how the dark bands that cross a murmuration are seen (A. Procaccini et al., "Propagating
  waves in starling murmurations", *Animal Behaviour* 82 (2011) 759–765).
- Drawn: the agitation waves' starts (every 3 to 8 s a patch of birds 9 m across startles into a sharp
  banked turn; the wave that crosses the flock is alignment's own), each bird's roll and glide pose,
  the reed bed the watcher stands in (stems and plumes in three layers, swaying), far birds
  drawn a little fuller so a distant flock reads dark as the eye sees it, the starling's silhouette (wingspan 0.38 m, drawn a third larger so a flock reads on a phone),
  its wingbeat (about 10 Hz, in bouts with glides), the roost's slow wander, and that your bird counts
  four times in its neighbours' reckoning so a lean is felt.

## The sound (drawn)

Filtered noise: the far hush of the whole flock, the rush of wings passing near the eye, and your own
bird's wingbeats while you fly it. No recording is used.
