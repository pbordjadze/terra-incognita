# Shimmering: sources

What the toy follows and what it draws. It loads no data: the bees, the comb, the cliff and the hornet are
made by code (src/comb.rs, src/comb.wgsl).

## The place

- **Chitwan, Nepal** (27.58° N, 84.43° E): the lowland Terai along the Rapti river, where giant honeybees
  (*Apis dorsata*) hang their single open combs under cliff overhangs and from tall trees, often many colonies
  together, and where honey hunters climb to them.

## The rules it follows

- **Shimmering.** Giant honeybees build one comb in the open, covered by a curtain of bees several layers
  deep. When a hornet (*Vespa*) hovers near, bees on the surface flip their abdomens upward in a quick
  sequence, a wave that runs across the curtain in a fraction of a second, like a stadium's Mexican wave.
  The waves start from a few bees near the hornet and spread outward. Kastberger, Schmelzer and Kranner
  (2008), "Social waves in giant honeybees repel hornets", *PLoS ONE* 3(9): e3141.
- **Each bee follows its neighbour.** A wave spreads because a bee flips when bees beside it flip, after a
  short delay; `comb.rs` lets a flipping bee set its resting neighbours off at random, sooner the more of
  them are flipping, so fronts run at roughly half a metre a second. The first switch stops this: only the
  bees under the hornet flip.
- **A bee rests after a flip.** A bee that has just flipped does not flip again at once, which keeps the
  waves crisp: they pass and the curtain goes still behind them (an excitable medium). The second switch takes
  the rest away, and the bees set each other off again and again, the curtain chattering.
- **The waves repel the hornet.** Kastberger et al. found hornets kept further from the comb where the waves
  were strongest; the toy's hornet is pushed back by flipping bees near it and, pushed hard enough, backs off
  for a few seconds. The third switch lets it hover unbothered.

## Drawn (not measured)

- One bee a centimetre of comb, the surface layer only; the flip's timing (about a third of a second), the
  rest (a second), the spread's speed and the hornet's flight are chosen to read on a phone, not measured.
- The overhang and the hornet are drawn by code.

## The look

Drawn in the style of Larry Cuba's *Calculated Movements* (1985), made on a Datamax UV-1 in the Zgrass
graphics language.

- "A choreographed sequence of graphic effects constructed from simple elements combined in a hierarchical
  structure. The simplest element is a linear figure, like a ribbon, that appears, flies through space and
  then disappears." "Produced on the Datamax UV-1 with the ZGrass graphics language." (Cuba's statement,
  ACM SIGGRAPH History Archives, history.siggraph.org/?p=19906.)
- "I had four colors: white, black, light grey and dark gray"; "ribbon-like figures follow a single
  trajectory against a middle gray background"; the raster "allowed Cuba to work for the first time with
  solid areas and "volumes" rather than just dots of light" (Gene Youngblood's interview with Cuba,
  reprinted at prehysteries.blogspot.com, 2008).
- "Calculated Movements was produced at Cuba's studio in Santa Cruz on the Datamax UV-1 personal computer"
  (the same page).
- The UV-1's picture: Bally's chips made "real-time color video at 320x201x2 bits per pixel" (the 1983 Zgrass
  paper as summarised by ballyalley.com and bitsavers' *Extended Memory Use in the ZGrass System*; other
  sources give 202 or 204 rows).
- So the comb is drawn on a raster 320 pixels across, each pixel one sample of the scene in one of the four
  colours, nothing smoothed (`comb.wgsl`). Every two rows of bees are one ribbon, a segment a bee: dark where it
  hangs, twisting to show a narrower white face where a bee has flipped, so a shimmering wave is a ribbon
  turning over.

**Drawn, not sourced:** which grey is the ground (middle grey: the light one), the overhang's stepped lip; the raster square and as tall as the screen (the UV-1's frame is 201
rows on a 4:3 picture); the hornet as three white segments with black bands, twice its size.
