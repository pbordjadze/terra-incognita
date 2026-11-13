# Nazaré: sources

What is measured, what is modelled, what is drawn, and from where. The floor is built by
`tools/toys/nazare/build.py` (its docstring is the layout); the swell follows `src/sea.rs`.

## The floor (measured)

- **The sea's depth and the land's height**: EMODnet Bathymetry's Digital Terrain Model (mean depth,
  1/16 arc-minute, about 115 m), fetched through its WCS for 39.40–39.82° N, 9.30–9.03° W; its lone
  spikes taken out (a 3 × 3 median where a cell stands 40 m off it), resampled to a 50 m grid. The
  Nazaré Canyon's head reaches to within a few hundred metres of the headland.
- **The places**: Nazaré's headland and lighthouse (the Forte de São Miguel Arcanjo), Praia do Norte
  north of it, the town south of it.

## The swell (the physics)

- **Its speed**: the linear dispersion relation, ω² = g k tanh(k h) (Fenton and McKee's explicit form):
  over deep water a swell's speed is set by its period alone; once the water is shallower than about half
  its wavelength it slows, the longer swell feeling the bottom deeper.
- **Its crests**: lines of equal travel time, |∇τ| = 1/c (fast sweeping, Zhao 2005), from a plane swell
  arriving over the open sea's edges; the sets travel at the group speed.
- **Its height**: rays at right angles to the crests, turning toward slower water (Snell's law), from
  the open sea; where they crowd the waves grow (refraction, held to 2.5 times, as diffraction holds a real
  caustic), and they grow as they slow (shoaling, by the group speed). A wave breaks at 0.78 of the depth
  (McCowan). Over the canyon the swell runs on; on the shelf beside it, it bends toward the canyon's rims
  and the rays crowd at Praia do Norte; the bay south of the headland lies in the canyon's lee.

## Drawn (not measured)

- **The swell**: 6 m in the open sea, opening on 17 s from 290° (a winter groundswell from the
  west-north-west, as on Nazaré's big days); a swipe toward the coast sends one from where the finger came
  (between 225° and 345°), its period between 8 and 22 s as the swipe is long. A set every nine waves, two or
  three big ones in it; a swell sent times its sets so one breaks at Praia do Norte five seconds later.
- **The crests**: the first-arrival times smoothed over a few cells (where a faster path through the canyon
  overtakes, the real sea holds both arrivals and diffraction rounds the corner).
- **Time**: nine seconds of sea a second.
- **The scene**: dusk; crests lit by the afterglow from the west, as bright as the waves' energy; white
  water as white as the swell is focused where it breaks; the land shaded from the west; the town's
  lights and the lighthouse's flash drawn, not placed from data.
- **The sound and the spray**: a crash each time a crest reaches the tallest wave off Praia do Norte, as loud as it is
  tall (the sets loudest), over a low roar; a set's big wave there breaks as a white line rolling in, throws spray
  that drifts over the beach, and taps the hand.
