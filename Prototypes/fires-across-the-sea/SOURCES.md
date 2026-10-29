# Fires across the sea

A runtime-4 pack: the globe draws a pyre on every peak (symbols), the fires as sprites while they are
new and as still pictures once they are old (the runtime holds 64 sprites), their smoke, the lines of
sight along which each fire was seen, and nothing else. The art is drawn from code by
`tools/beacons/art.js` through `tools/packart/render.cjs`; rebuild, never hand-edit. The script says
nothing on the screen but the name; this file and the panel say what is true and what is drawn.

## What is true

- Aeschylus, *Agamemnon* 281–316: Clytemnestra's account of the beacon chain from Mount Ida above Troy
  by Lemnos, Athos, Euboea (Makistos), Messapion, Kithairon, Aigiplanktos in the Megarid and
  Arachnaion to the palace at Argos. Which Euboean peak Makistos is is not settled; the pack has
  Kandili and Dirfys, and the relay finds its own way.
- Whether two fires can see each other is worked out, not drawn: two points see each other over the
  sea when their distance is at most the sum of their distances to the horizon, about 3.86 km × √(height
  in metres) each with standard refraction (3.57 km × √h without it). The fire's own height and the
  watchman's eye are neglected; land in the way between two peaks is not considered, so a few lines of
  sight across a mainland are generous.
- The peaks, their names and heights are from the English Wikipedia articles of the islands and
  mountains named in `data.js`: a mountain at its summit's coordinates, an island at its article's
  coordinates (its centre, not the summit: an error of a few kilometres on an island).

## What is drawn, and not claimed

- The seconds a watchman takes to see a fire and light his own are drawn (a moment, and a little more
  for a farther fire). The fires' burning and going out, and Ida lighting itself when all is dark, are
  for the picture.
- No claim is made that beacons stood on all these peaks, only that fires there could have been seen
  from where the chain runs.
