# Fireflies: sources

What is measured, what is drawn, and from where (painting-company row 204).

## Measured

- **The Earth's lights at night**: NASA's Black Marble (NASA Earth Observatory / NASA Goddard, VIIRS
  Day/Night Band), public domain (NASA imagery is not copyrighted; credit: NASA Earth Observatory).
  Used as Terra's night sheet `Globe/ground/night.jpg` (built by `tools/globe/night.py`). The fireflies
  are placed on it by `tools/toys/fireflies/build.py` (`data/fireflies.bin`): one at each patch of lit
  pixels, brightest first, none closer than 75 km to another; a firefly's light is the sheet's own.
- **The ground by night** (land, sea floor, ice, coasts): the kit's globe, Terra's data (NASA Blue
  Marble, Natural Earth, GEBCO), credited in Terra's sources.

## Behaviour (real science, simplified)

- **Pulse-coupled oscillators**: R. E. Mirollo and S. H. Strogatz, "Synchronization of pulse-coupled
  biological oscillators", SIAM J. Appl. Math. 50 (1990). Each firefly charges along a concave curve,
  flashes at the top, and a seen flash advances its neighbours by a fixed charge; their proof that such a
  population falls into step is the toy's rule.
- **Synchronous fireflies**: *Photinus carolinus* in the Great Smoky Mountains and *Pteroptyx* in
  Southeast Asia's mangroves flash in step, found locally and spreading, with waves travelling through
  a swarm (R. Sarfati, J. C. Hayes, O. Peleg, "Self-organization in natural swarms of Photinus
  carolinus synchronous fireflies", Science Advances 7, 2021; J. Buck, "Synchronous rhythmic flashing
  of fireflies II", Q. Rev. Biol. 63, 1988). Flash colour near 560 nm (yellow-green) as theirs.

## Drawn (not measured)

- Every firefly's own period (1.6 s, spread 7%), the coupling's strength, a seen flash's delay (15 ms
  plus 0.18 ms a km, so a flash is seen across a continent in about a second, as waves), the 260 km a
  firefly sees, the refractory spell after a flash: chosen for the eye, the shape of the real thing at the
  scale of the planet, not its numbers. Cities are not fireflies; their lights stand in for them.
- The flash's rise and fall, its yellow-green tint and bloom; the lights' resting level between flashes; the night's colours (near-black ground, the lights' ember-to-white ramp and their bloom, read from the measured sheet at coarser scales); a touch's ring of light, which carries a flash from the finger to the lights it reaches (a finger's flash seen, not a measured thing); the chimes (fireflies are silent: a patch of the screen whose lights flash together rings, a pentatonic glass bell synthesised in code, no recordings).
