# Night Walk: sources

What is real and what is drawn. Everything on screen and every sound is drawn from code; nothing is a
photograph, a dataset or a recording.

## The walk (real: the rule itself)

- The walker follows the Metropolis rule exactly: N. Metropolis, A. W. Rosenbluth, M. N. Rosenbluth,
  A. H. Teller and E. Teller, "Equation of State Calculations by Fast Computing Machines", *Journal of
  Chemical Physics* 21 (1953) 1087. Each try picks a spot uniformly at random in a disc whose radius
  is the stride (so a step and its way back are equally likely); the walker goes there if the land is
  higher, and if lower with the chance `exp(K · Δh)` (K = 7, `src/land.rs`); off the screen there is
  no land and it never goes. Every try, taken or not, leaves one footprint where the walker then
  stands, so a refused try deepens the footprint it stands in.
- The footprints therefore pile up in proportion to `exp(K · h)` once the walk has wandered long
  enough, and the map drawn in the dark is the log of the pile (each footprint a soft spot two cells
  wide, the map 144 cells across), less a constant: the land's own height, measured. `cargo test -p
  walk` checks it: after 1.5 million tries the map's log differs from the land's by 0.08 on average
  where the footprints are thick (`the_map_is_the_land`), and that a tiny stride takes nearly every try
  and stays on one hill, a stride of a third of the screen crosses between hills many times a night,
  and a stride longer than the screen refuses about 98 tries in 100 (`strides_feel_different`).
- The map's contours are 0.7 apart, a factor of about two in footprints; the land's contours at dawn
  are drawn at the same heights, so where the walk has covered a hill well its contours lie on the
  land's.
- 120 tries a second; a night is 42 seconds (about 5 000 tries).

## Drawn

- The land: made up fresh each night from a seed (two to four rounded hills far apart, each with a
  few smaller rises and hollows, gentle swells on the plain between); not a real place.
- The night, the lantern, the ring (the stride), the fresh footprints (warm, fading in two seconds),
  the faint glance at a spot the walker tried and didn't go to, the dawn's colours (green low ground
  to pale summits) and its hill shading.
- The sound: a footfall for each step taken, louder the longer the step (a refused try is silent);
  wind; far crickets by night; a few birds at dawn.

## Known

- The map's height is relative: it is placed so that its top is the land's top only when the walk has
  covered the land; when it has seen one hill, that hill is drawn too high against the others it
  never found (it has no way to know they are there). That is the truth of a short walk, kept.
