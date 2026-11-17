# Fairy Circles: sources

What the toy follows and what it draws. It loads no data: the grass, the sand and the rain are made by code
(src/veld.rs, src/veld.wgsl).

## The place

- **The NamibRand Nature Reserve, Namibia** (about 25.0° S, 16.0° E), on the eastern edge of the Namib, where
  the grassland of *Stipagrostis* bunchgrasses on red sand is pocked by millions of bare circles, 2 to 12 m
  across, each ringed by taller grass and spaced in a near-hexagonal lattice that runs for hundreds of
  kilometres along the desert's margin. (Wikipedia, "Fairy circle (arid grass formation)".)

## The rules it follows

- **Grass competes for scarce rain, and roots draw water from nearby.** The best-supported explanation is
  self-organisation (Getzin et al. 2016, 2022; Meron's and Klausmeier's vegetation models): where rain is too
  little for a closed sward, the grass at a circle's edge draws water out of the bare disc through its roots,
  so the disc stays bare and its ring grows tall. Water moves through the ground farther than grass can
  spread (a scale-dependent feedback, a Turing-type pattern), and that difference of reach sets the lattice.
  `veld.rs` is Klausmeier's model on flat ground: rain in; evaporation and uptake out; uptake rising with
  the grass already there; soil water spreading a hundred times as far as grass. The first switch makes the
  water spread no farther than the grass: the discs fill in.
- **More rain closes the circles, drought widens them.** In wetter years circles shrink and grass closes over
  them; in a run of dry ones they widen, and in a still drier climate the grass would break into spots. The
  second switch makes the rain plentiful (the circles close); the finger's shower does the same on a patch;
  a shake is a dry year (the circles widen, some join, and the grass between narrows), easing back over
  about fourteen seconds.

## What is the toy's own

- The time: the model runs about six of its time units a second, years pressed into seconds.
- The opening lattice is seeded as a jittered hexagonal grid of discs and run a little before the first
  frame, so the toy opens on circles; the rule then holds them.
- The colours and the low sun are drawn, not measured.
