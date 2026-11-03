# Sand table (spike)

**The idea.** Cannae as a sand table at night, seen from straight above under one low torch that moves slowly and flickers. Your finger ploughs a groove through real sand. The sand it pushes out heaps up in ridges either side, and the torch's raking light catches every crest. The armies are heaps of coloured grain: red ochre for Rome, pale lime for Hannibal's foot, black beans for Hasdrubal's horse and the Numidians. Grain slides down into your grooves and runs along them, the way a Tsuro path carries its piece, then piles up where the groove ends and presses on toward the enemy. Where the colours meet they churn and mix, grain by grain, and leave stained, trampled ground.

**What's new about it.** There are no units, counters, arrows or interface: it's a material. The plan is literally cut into the ground, and the light is the only thing that makes it legible. It reads as a photograph of a tabletop rather than a game screen.

**What's real underneath.** The table is shaped to Cannae's actual relief: Terrarium z13 tiles, about 9.2 x 6.9 km centred on 41.30 N 16.13 E, relief exaggerated about 2.6x (`prep.py` builds `relief.js`). That gives the Ofanto valley floor and the Canne terraces on the south-east. The opening positions follow the usual reconstruction:
- the Romans deep in the centre, the river on their right;
- Hannibal's convex crescent facing them;
- the Africans held back on both wings;
- Hasdrubal's horse by the river.

The scripted plan is the battle itself: Hasdrubal round the Roman rear, then both African wings turning inward.

**What's faked.** The river isn't traced. It's laid by eye along the foot of the right-bank terrace (the plain slopes north, so the lowest line at this resolution isn't the channel) and given drawn meanders. Army sizes are proportional, not counted. Combat is a crude rule: grain jitters and slows wherever enemy grain is dense, and sometimes stops for good.

**The physics.**
- A 512x384 height field slumps to its angle of repose: talus 0.62 per cell, two relaxation passes a frame.
- About 13k grain particles slide downhill, are pushed apart by crowd pressure, follow a groove-direction field, keep off the wet river and kick little tracks.

**The look** is one fragment pass:
- bicubic heights for smooth normals;
- a facet per sand grain, with quartz glints and dark mineral grains;
- army grains drawn as little domes, each one picked by colour fraction, so a mix shows as speckles, not a blend;
- shadows marched toward the flame;
- occlusion from the blurred height;
- dust drifting at the table's edges;
- ACES tone mapping.

**What the finger does.** A drag cuts a groove (ridges, slumping, a direction that carries grain). A tap presses a fingertip crater, scatters nearby grain and sends it marching.

Run: `node render.cjs` (playwright + ffmpeg). Writes:
- `out/moment-*.jpg`, at 1x: a full-size PNG of the grain is 13 MB of noise;
- `out/detail-245-grain.png`, a 2x crop that shows the grain;
- `out/film.mp4` and `out/contact.png`.

`prep.py` rebuilds `relief.js` from the tiles.
