# The Delta: sources

What the toy follows and what it draws. It loads no data: the land and its water are made by code
(src/land.rs, src/land.wgsl), seen through src/ip.rs. The tilt is the phone's.

## The place

- **The Okavango Delta, Botswana** (about 19.3° S, 22.9° E): an inland delta where the Okavango River,
  fed by the rains on the Angolan highlands, spreads over a fan some 250 km long at the edge of the Kalahari.
  The flood reaches the Panhandle around March and the far ends of the delta in June to August, in Botswana's
  dry season. (T. S. McCarthy and W. N. Ellery, "The Okavango Delta", *Trans. Roy. Soc. S. Afr.* 53, 1998;
  Wikipedia, "Okavango Delta".)

## The rules it follows

- **The land is very nearly flat.** The fan falls about 1 m in 3,400, some 70 m over its length, so relief of
  a metre or two decides where the water goes: the channels are a little lower, the islands a little higher.
  `land.rs` lays a fan falling away from the Panhandle's mouth at that rate, gentle unevenness over it, a
  river that wanders and splits into thinner, shallower branches, and islands a metre or two high.
- **Islands begin as termite mounds.** Most of the delta's tens of thousands of islands started as a mound
  of *Macrotermes* termites on the floodplain; trees took root on it, and as they drew water the salts were
  left in the island's middle, bare and pale, with a ring of trees round it (McCarthy, Ellery and
  Dangerfield, 1990s). The second switch lays the islands down: the flood covers where they stood.
- **Water runs from higher to lower, and sinks.** The flow moves water between cells from the higher surface
  to the lower, faster the deeper it is (as Manning's law has it, about depth to the five-thirds), so a
  thin sheet barely moves and a channel runs. Nearly all that enters never leaves: about 96 % is drunk and
  breathed off by the plants or evaporates, and 2 % sinks into the sand (McCarthy and Ellery). The first
  switch leaves only the evaporation: the flood spreads on and on.
- **Where water lies, grass greens**, and stays green a while after the water goes: the old courses show.

## What is the toy's own

- **The tilt is scaled down.** The real fan leans a hundredth of a degree; a phone tipped 15° here leans the
  land about three times its own fall (the tilt is divided by some 300), and past 30° more tilt changes
  nothing. A finger pressed leans the land toward its side, as a hand pressing the edge of a tray.
- **The time is pressed.** The flood that takes months to cross the delta crosses the screen in seconds,
  and the screen shows about 14 by 30 km of the fan.
- The colours, the trees and the low sun are drawn, not measured.

## The look

Drawn in the style of the Sandin Image Processor (Dan Sandin, University of Illinois at Chicago Circle,
first played in public in 1973), an analog computer for video that Sandin gave away for others to build.

- **The machine:** "In 1971 to 1974 he designed and built the Image Processor, an analog computer for video
  image processing", modelled on the Moog synthesizer's modules ("I just went through all the Moog
  modules…"); "It took me about a full year to build it before it was running even in black and white"
  (*Eigenwelt der Apparatewelt*, Ars Electronica 1992 catalogue). Its first performance was on January 23,
  1973 (Video History Project, *Sandin Image Processor: excerpts*).
- **What is followed** (the same two sources): the camera processor "takes a black and white video signal";
  the Amplitude Classifier gives "8 discrete 'digital' channels… corresponding to 8 contiguous grey levels
  evenly spaced from black to white"; "wiring from the Amplitude Classifier through the adder/mixers to the
  color encoder results in a 'threshold based colorizer'"; the Differentiator's "shorter time constants
  respond to sharp horizontal edges". So the land, drawn as before, is turned to a grey signal, inverted,
  cut into eight levels, and each level keyed to its own colour; rises along a line light white
  (`ip.wgsl`).

**Drawn, not sourced:** the eight colours chosen (the player chose them; here the dry sand is dark, the wet
grass violet, the water's fringe magenta (a second keyer: only next to water), the water cyan and white,
the trees dark green); the inverter before the classifier; the
threshold's slow drift, the lines' time-base wobble, the chroma's lag, the raster's dark lines and the
tape's snow (none with Reduce Motion, except the raster); 260 picture elements across and 30 frames a
second.
