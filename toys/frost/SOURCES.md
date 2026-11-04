# Frost: sources

What is real, what is drawn, and from where (painting-company row 218).

## Real (the rule)

- **How a snow crystal grows from vapour**: C. A. Reiter, "A local cellular model for snow crystal
  growth", Chaos, Solitons & Fractals 23 (2005) 1111–1119. A hexagonal grid of cells, each holding
  water; a cell that is ice or touches ice is receptive and gains a constant γ; the rest's water
  diffuses to its six neighbours; the far air holds a background level β. Long fast ferns
  (dendrites) at high β, stubbier slower stars at lower β, compact plates at high γ: his phase
  diagram, of which the toy uses the dendritic side
  (`toys/frost/src/frost.wgsl`; the numpy reference `tools/toys/frost/reiter.py`). It is a model of
  diffusion-limited growth from vapour, which is what frost on cold glass is (not freezing from
  liquid).
- **Six arms**: ice Ih's hexagonal lattice; every crystal's arms are 60° apart, and each crystal on
  the glass nucleates at its own random lattice angle.
- **Humidity lower on a window**: the warm room's moist air sinks along cold glass, so the bottom of
  a frosted pane is wetter; the toy's β rises toward the bottom of the glass.
- **Seams**: two crystals both draw on the same air; where one has frozen the other finds none
  (another crystal's ice is a sink in its grid), so they meet along a boundary, as grains do.
- **The place**: Oymyakon, Sakha (Yakutia), 63.46°N 142.79°E, among the coldest inhabited places;
  winter nights near −50 °C are usual (−67.7 °C, February 1933, is its record).

## Drawn (not measured)

- The numbers: β from 0.46 at the top of the glass to 0.74 at the bottom, γ from 0.0016 to 0.0005, the
  pace (300 automaton steps a second), a cell's size (one point on a phone), the glass's flaws (a
  smooth ±30% speckle in γ that breaks the model's perfect symmetry, as dust and scratches do).
- Each crystal grows in its own grid of 512 x 512 cells, with the air held at β a little beyond its
  tips (not across the whole pane): a bound for the phone's GPU.
- Frost starting by itself where the glass meets the frame (real: the frame's edge is the coldest
  glass), a seed every few seconds, more often low: the timing is drawn.
- A fingertip's warmth melting a disc of ice where it touches, before the new seed freezes there.
- The window (a wooden frame and crossbar), the night outside (sky, stars, snow, larches), the lamp
  behind the reader, the meltwater's film, the ice's colour and
  light: drawn in code, no photographs.
- The sound: a crackle as many clicks a second as cells freezing (frost growing is all but silent;
  this is its growth made audible), and a faint cold air. Synthesised, no recordings. The haptic
  ticks follow the same count.
