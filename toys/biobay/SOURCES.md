# Mosquito Bay: sources

What the toy follows and what it draws. It loads no data: the water, its light, the roots and the frogs are
made by code (src/bay.rs, src/bay.wgsl, src/sound.rs).

## The place

- **Puerto Mosquito, Vieques, Puerto Rico** (18.098° N, 65.443° W), a shallow mangrove bay with a narrow mouth
  on the island's south coast, counted among the brightest bioluminescent bays in the world. Its light is
  mostly the dinoflagellate *Pyrodinium bahamense*; the bay's narrow mouth and the red mangroves
  (*Rhizophora mangle*) round it keep the water calm and rich enough for them to stay thick in it.
  (Wikipedia, "Mosquito Bay" and "Pyrodinium bahamense".)

## The rules it follows

- **Only stirring makes light.** A dinoflagellate flashes when the water around it is deformed: a paddle,
  a swimmer, a fish or a raindrop bends the water, and the cells in that bent water flash for a fraction
  of a second. Still water is dark. `bay.rs`: each cell's light comes from its stirring, which settles in
  about a second and spreads a little into the water around it.
- **A flash tires them.** Flashing uses up the chemical that makes the light (luciferin), which the cells
  make again over hours (in the day, by photosynthesis). Stirred again and again, the same water gives a
  weaker light. `bay.rs` spends a cell's charge as it lights and gives it back slowly (a time constant of
  22 s, much quicker than the real hours, so the toy can be played with); the second switch turns this off.
- **The moon drowns it.** The light is faint; the bay's tours run on dark nights, because under a bright
  moon the glow is hard to see. The first switch puts the moon up: the water takes its glints, and the
  blue light all but disappears.
- **Fish light their own way.** Fish in the bay show as streaks of light, and they dart from a hand put in the
  water; the third switch takes them (and the fry) out.

## Drawn (not measured)

- Every number the rules above don't give: how bright, how wide a finger's stirring is (2.2 cm), how fast the
  stirring settles and the light dies (1.3 s, 0.45 s), the fish's sizes, speeds and darts, the fry's sparks,
  the rain's shower (a shake), the haze and the single cells' sparks.
- **The look**, in homage to Paint by Moonlight (painting-company row 359): the water is the app's night
  paper, a deep plum, and the moon's glints on it are its three moonlight paints (a marshmallow, a candle
  wax, a pale lilac); the stars' reflections, the vignette, and the mangroves' prop roots along the top are
  made up after the place.
- **The sound**: the water a finger stirs, small splashes for raindrops and darting fish, and the coquí
  frogs (*Eleutherodactylus coqui*, Puerto Rico's own) calling "co-quí" from the mangroves, drawn as two
  short tones rather than recorded.
