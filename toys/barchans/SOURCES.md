# Barchans: sources

What is real and what is drawn. The sources were found by web search on Nov 13, 2026 and are cited as
those results gave them. Barchans loads one file of its own, `data/opening.bin`: the rule's own field
after a long run from scattered heaps, built by `cargo run --release -p barchans --example opening`.

## Real

- **The rule.** Werner, B. T. (1995). Eolian dunes: computer simulations and attractor interpretation.
  *Geology* 23(12), 1107–1110 (https://planetarygeoscience.uchicago.edu/doc/Werner_1995.pdf): sand as
  slabs, picked up anywhere out of the lee, carried a fixed hop downwind and dropped with a higher chance
  on sand than on bare ground, a 15° shadow behind crests, avalanching at the angle of repose; forms like
  barchans grow from it. `dunes.rs` is that model (the chance on bare ground lower than Werner's, the
  reg being hard; the shadow kept as a map remade four times a sweep).
- **The place.** The barchan field between Tarfaya and Laayoune in the Atlantic Sahara, Morocco, driven
  by the trade winds (Elbelrhiti, Andreotti & Claudin 2008, https://arxiv.org/abs/cond-mat/0609120 and
  https://www.phys.ens.psl.eu/~foldingslidingstretchinglab/papers/A30_2007JF000767.pdf). A dune there
  was measured moving about 21 m a year (https://mdpi.com/2072-4292/11/8/965).
- **The song.** Andreotti, B. (2004). The song of dunes as a wave-particle mode locking. *Physical Review
  Letters* 93, 238001. doi:10.1103/PhysRevLett.93.238001: dunes in the Atlantic Sahara sing at 100 ± 5 Hz
  when their slip faces avalanche. The toy's drone is at 100 Hz, as loud as its big dunes' avalanches.

## Drawn

The pace (years in seconds), the scene's turn (the wind runs down the screen), the sun, the colours, the
grains blowing. The wind's direction once you drag it is yours.
