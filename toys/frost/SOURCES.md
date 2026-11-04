# Frost: sources

What is real, what is drawn, and from where (painting-company row 218).

## Real (the rule)

- **How a snow crystal grows from vapour**: C. A. Reiter, "A local cellular model for snow crystal
  growth", Chaos, Solitons & Fractals 23 (2005) 1111–1119. A hexagonal grid of cells, each holding
  water; a cell that is ice or touches ice is receptive and gains a constant γ; the rest's water
  diffuses to its six neighbours; the far air holds a background level β. Ferns (dendrites) at high
  β and low γ, compact plates at low β and high γ: his phase diagram, which the toy follows
  (`toys/frost/src/frost.wgsl`; the numpy reference `tools/toys/frost/reiter.py`). It is a model of
  diffusion-limited growth from vapour, which is what frost on cold glass is (not freezing from
  liquid).
- **Six arms**: ice Ih's hexagonal lattice; every crystal's arms are 60° apart, and each crystal on
  the glass nucleates at its own random lattice angle.
- **Humidity lower on a window**: the warm room's moist air sinks along cold glass, so the bottom of
  a frosted pane is wetter; the toy's β rises and γ falls toward the bottom of the glass.
- **Seams**: two crystals both draw on the same air; where one has frozen the other finds none
  (another crystal's ice is a sink in its grid), so they meet along a boundary, as grains do.
- **The place**: Oymyakon, Sakha (Yakutia), 63.46°N 142.79°E, among the coldest inhabited places;
  winter nights near −50 °C are usual (−67.7 °C, February 1933, is its record).

## Drawn (not measured)

- The numbers: β from 0.40 at the top of the glass to 0.74 at the bottom, γ from 0.02 to 0.0008, the
  pace (300 automaton steps a second), a cell's size (one point on a phone), each crystal's reach
  (about 112 cells, the air round it then spent: a stop chosen for the eye), the glass's flaws (a
  smooth ±40% speckle in γ that breaks the model's perfect symmetry, as dust and scratches do).
- A fingertip's warmth melting a disc of ice where it touches, before the new seed freezes there.
- The night outside (sky, stars, snow, larches), the lamp behind the reader, the ice's colour and
  light: drawn in code, no photographs.
- The sound: a crackle as many clicks a second as cells freezing (frost growing is all but silent;
  this is its growth made audible), and a faint cold air. Synthesised, no recordings. The haptic
  ticks follow the same count.
