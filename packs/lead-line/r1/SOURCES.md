# The lead line

A runtime-4 pack: the globe draws the ship (a sprite, her frames drawn from code by
`tools/leadline/art.js` through `tools/packart/render.cjs`), her stations (dots and rings, touchable),
the leads let down where the sea is touched (the globe's sounding lines), and the sounding glass with
the sea floor's contours under it. The script says nothing on the screen but the name and the depth at
the foot of each line; this file and the panel say what is true and what is drawn.

## What is true

- The stations: 94 dated stations of HMS Challenger, 2 February 1873 to 6 May 1876, with their
  positions and recorded depths, from Murray and Renard (1891), *Annotated record of the detailed
  examination of Mn deposits from the HMS Challenger stations*, PANGAEA, https://doi.org/10.1594/PANGAEA.849072
  (CC BY 3.0; the licence: https://creativecommons.org/licenses/by/3.0/ ), as prepared and corrected for the first version of this study (`stations.js`, which
  records its provenance and transformations; the depths are the recorded fathoms at 1.8288 m). A
  selection, not the expedition's full log; the positions are nineteenth-century measurements.
- The ports and the months she reached them: the expedition's itinerary as the Narrative of the cruise
  gives it (Thomson and Murray 1885; summarised in Wikipedia, "Challenger expedition").
- The depths everywhere else: the globe's own terrain map (GEBCO, through `tools/globe/build.py`),
  averaged to cells of about a third of a degree (`tools/leadline/build_depths.py`). A lead finds the
  depth of a cell, not of a point, and the shelf's edge is drawn at the grid's coarseness.
- Natural Earth is not used here.

## What is drawn, and not claimed

- Between dated points the ship is drawn on the great circle at a steady pace; the ports' days within
  their months are drawn, and so are the turns of her course between them
  (`tools/leadline/build_way.py`). It keeps her at sea and in the right ocean in the right month; it is
  not her logged course.
- A line runs out at about two seconds a mile of depth and the lines are drawn to one scale (14 points a
  kilometre, at most 180); a real sounding to 3,000 fathoms took about three-quarters of an hour.
- The ship is an original drawing after her rig in outline.
