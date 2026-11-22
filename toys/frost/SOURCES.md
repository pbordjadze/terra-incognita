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
  glass), a seed every 0.7 s at first, the glass frosted over in about 20 s: the timing is drawn.
- A fingertip's warmth melting the ice along its way (wider where it rests); the meltwater freezing
  again as smooth clear ice that frost finds hard to catch on, so it creeps back only from the
  edge (real in kind: hoarfrost nucleates on flaws and on ice, and a wiped patch of a frosted window
  stays clearer; the fringe's depth, ~14 points, and the healing, about two minutes, are drawn).
- A shake as the stove's warmth sweeping the window clear: drawn.
- The window (a wooden frame and crossbar), the night outside (sky, stars, snow, larches), the lamp
  behind the reader, the meltwater's film, the ice's colour and
  light: drawn in code, no photographs.
- The sound: a crackle as many clicks a second as cells freezing (frost growing is all but silent;
  this is its growth made audible), and a faint cold air. Synthesised, no recordings. The haptic
  ticks follow the same count.

## The look

Drawn in the style of Peter Foldès' *Metadata* (1971), made on the key-frame animation system of the
National Research Council of Canada, the system he went on to use for *Hunger* (1974). Sources, quoted
only from pages fetched on November 22, 2026:

- Wikipedia, *Hunger (1974 film)* (https://en.wikipedia.org/wiki/Hunger_(1974_film)): "Foldes developed a
  computer animation application in collaboration with the National Research Council Canada (NRC) in
  1969"; Nestor Burtnyk "and physicist Marceli Wein programmed a "key frame animation" package to create
  animated sequences from key frames"; "Foldes' first film using Burtnyk's program was a 1971 computer key
  frame animated short experimental film involving freehand drawings called *Metadata*"; the technique
  "allowed object shape modification and line interpolation techniques"; "Stylistically, the short film's
  line drawings dissolve and reshape to form various images."
- SIGGRAPH's history archive, *Metadata* (https://history.siggraph.org/animation-video-pod/metadata-by-foldes/):
  "Images were drawn by means of a mouse, an early precursor of a graphic tablet"; 16 mm film.

So, as the system worked: an artist's key drawings, and the computer's drawings between them. Here the
automaton is the artist: every 0.2 s the ice it has grown is taken as a key drawing, and every frame the
window shows the drawing between the last two (`keys.rs`), its lines the frost's edge and the contours of
its age, one pen weight, white on the night (`pane.wgsl`). The night outside and the window's frame stay
as they were.

**Drawn, not sourced:** how often a key is taken; that the in-between is made by interpolating the two
keys' fields, so a line slides between its places, rather than point to point along each line as the
NRC's program did; the pen's width and white ink on a dark ground (the sources don't give the films'
colours); the frosted glass a little paler inside the ice.
