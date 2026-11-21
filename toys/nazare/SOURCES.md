# Nazaré: sources

What is measured, what is modelled, what is drawn, and from where. The floor is built by
`tools/toys/nazare/build.py` (its docstring is the layout); the swell follows `src/sea.rs`.

## The floor (measured)

- **The sea's depth and the land's height**: EMODnet Bathymetry's Digital Terrain Model (mean depth,
  1/16 arc-minute, about 115 m), fetched through its WCS for 39.40–39.82° N, 9.30–9.03° W; its lone
  spikes taken out (a 3 × 3 median where a cell stands 40 m off it), resampled to a 50 m grid. The
  Nazaré Canyon's head reaches to within a few hundred metres of the headland.
- **The canyon's edge at its head** (measured, then drawn in): EMODnet's cells blur it, so its northern
  edge in the last kilometres is burnt into the floor as Hope et al. (2026, section 1) give it from the
  lighthouse: 660 m at a bearing of 228°, then to 912 m at 251°, then west at 266° (held to 800 m, where
  the soundings show the canyon again); a shelf 15 to 20 m deep sloping 2.4° down toward it; the canyon
  beyond at least 100 m deep (its thalweg about 150 m at the terminus). `src/floor.rs`.
- **The places**: Nazaré's headland and lighthouse (the Forte de São Miguel Arcanjo), Praia do Norte
  north of it, the town south of it.

## The swell (the physics)

- **Its speed**: the linear dispersion relation, ω² = g k tanh(k h) (Fenton and McKee's explicit form):
  over deep water a swell's speed is set by its period alone; once the water is shallower than about half
  its wavelength it slows, the longer swell feeling the bottom deeper.
- **The canyon as a wall**: Hope, Rainville, Seldal et al., "Focusing of Swell at the Nazaré Submarine
  Canyon" (EGUsphere preprint, June 2026, doi:10.5194/egusphere-2026-2778), from buoys over the canyon and
  stereo cameras on the cliff: swell on the shelf is refracted and reflected at the canyon's edge; swell
  longer than about 7 s is mostly turned back rather than carried across; the turned-back swell crosses the
  arriving swell off Praia do Norte, and where their crests meet they stand into peaks. Here the canyon's
  incised upper part (deeper than 45 m and 1.6 times the shelf around it, within 6 km of the lighthouse, and
  the measured edge) is a wall to swell of 8.5 s or longer, crossed by swell of 5.5 s or shorter, turning
  back a share in between.
- **The arriving swell's crests**: lines of equal travel time, |∇τ| = 1/c (fast sweeping, Zhao 2005), from a
  plane swell arriving over the open sea's edges; the sets travel at the group speed.
- **Its height**: rays at right angles to the crests, turning toward slower water (Snell's law), from the
  open sea; where they crowd the waves grow (refraction, held to 2.6 times, as diffraction holds a real
  caustic), and they grow as they slow (shoaling, by the group speed).
- **The turned-back swell**: Huygens' construction: every point of the wall's face a source starting when the
  arriving crest reaches it, as tall as the arriving wave there; its crests the first arrivals from the face
  (a second eikonal), its height the nearest source's, spreading as it goes (halving over 1.2 km) and
  shoaling.
- **Breaking**: two crossing trains stand steeper before they break than one train alone (McAllister et al.
  2024, cited in Hope et al. 2026): a crossed sea breaks at its depth here, against the McCowan 0.78 of a
  single train; inside the break the broken waves are held to 1.2 times the depth.

## Drawn the way Max drew (the look)

- **Nelson Max, *Carla's Island*** (Lawrence Livermore, 1981) and his "Vectorized procedural models for
  natural terrain: waves and islands in the sunset" (SIGGRAPH '81, Computer Graphics 15(3)), read from the
  paper's abstract (ACM Digital Library, 10.1145/800224.806820) and the film's stills in the ACM SIGGRAPH
  History Archive: an analytic sea surface ray traced as a height field; the direct ray reflected at the
  sea, its reflection tested roughly against the islands; eight bits a pixel, the sky's and the reflected
  ray's elevation angle, the land's normal; a colour table that turns those bytes into colour, changed as the
  loop repeats so the same bytes go down into sunset (Shoup's colour-table animation); Stokes' sharpened
  crests. Here the same: each pixel's byte from `src/max.wgsl`, the table from `max::palette`, rebuilt as the
  sun goes down.
- **What Max didn't do**: the sea surface is this floor's two trains, not his long-crested sum of
  sinusoids (a few of those remain, fading with distance, as the wind's chop); the front face is steepened
  toward breaking (Stokes alone gives gentle humps); the white water, the sun's glitter band and the surfer
  are drawn.

## Drawn (not measured)

- **The swell**: 7 m in the open sea, opening on 17 s from 295° (a winter groundswell from the
  west-north-west, as on Nazaré's big days); a swipe toward the coast sends one from where the finger came
  (between 240° and 345°), its period between 5 and 21 s as the swipe is long. A set every nine waves; a
  swell sent times its set so its biggest wave stacks at the peak about six seconds later; a swipe out to
  sea sends nothing.
- **The peak**: where the two trains together stand nearest breaking and tallest, 10 to 40 m deep, within
  1.6 km of the lighthouse. The tallest wave surfed there was measured at 26.21 m (Sebastian Steudtner,
  October 2020, Guinness World Records); the model's peaks stand near 15 to 20 m.
- **The camera**: on the fort by the lighthouse over the shelf; when a set will stack, down to the channel
  beside the peak, low, where the tow teams wait, riding the swell and keeping ahead of the wave once it has
  passed the peak; then back up.
- **The tow surfer**: 1.8 m on a pale board, on the big wave's face while it stands, for its scale.
- **Time**: 2.2 seconds of sea a second from the fort, real time down at the wall.
- **The scene**: a winter sunset from the west-south-west, the sun going down over a few minutes.
- **The sound**: a crash each time a crest reaches the peak, as loud as it is tall (the sets loudest), over a
  low roar, louder down at the wall; the biggest tap the hand.
