# Battle Ink: sources

What is measured, what is modelled from the accounts, what is drawn (painting-company row 206).

## Measured (data)

- **Cannae's ground**: Terrarium elevation tiles (Mapzen, now AWS Open Data: Terrain Tiles,
  s3.amazonaws.com/elevation-tiles-prod), zoom 14 (about 9.5 m a pixel at 41.3 N), built from SRTM
  (NASA/USGS, public domain) and other public sources; credit: Mapzen, AWS Terrain Tiles, SRTM.
  `tools/toys/battle/build.py` turns a window of the lower Ofanto valley between the river and the Cannae
  ridge (Canne della Battaglia, 41.2958 N 16.1517 E) onto the toy's sheet: `toys/battle/data/ground.png`
  (elevation, the river's signed distance) and `ground.json`. The relief, the 1 m contours and the hill
  shading are this data; nothing is hand-drawn.
- **The Aufidus (Ofanto)**: traced from the same elevation, not drawn: the cheapest path along the valley
  floor (cost growing with the square of height above the local floor), smoothed along its length. It is
  the modern floodplain's lowest line. The river of 216 BC has moved since, and where it ran on the day is
  argued over (the battle has been put on either bank); this toy takes the common reconstruction with the
  river on the Roman right and the valley's present lowest line as its course, and says so here rather
  than claiming more. (OpenStreetMap's mapped channel could not be fetched from the build container.)

## Modelled (from the accounts)

- **The orders of battle** (Polybius 3.113–117, Livy 22.45–49): the Roman infantry in maniples, three
  lines, depth over breadth, the Roman horse by the river and the allied horse on the open flank;
  Hannibal's convex crescent of Gauls and Spaniards, the Africans set back behind both its horns,
  Hasdrubal's heavy horse by the river, the Numidians on the right. About 3,050 units of a few dozen men,
  roughly in the armies' proportions (some 86,000 Romans and allies against some 50,000).
- **The rules** (`src/sim.rs`, from the ink-orders spike): each unit only looks round itself. It rides the
  ink if it is in a stroke's reach; else it seeks the nearest enemy; bodies take room, so depth really
  shoves; at contact, Lanchester-like attrition, harder from the flank and rear and far harder on men
  running; morale broken by enemies behind or on both sides, by blows, by being outnumbered, by friends
  running; men cut off and running are cut down. Hannibal's men give less than they take (a factor chosen
  so a good plan ends near the recorded outcome, some 50,000 Romans fallen against some 6,000 of his). The
  Romans advance and press the centre. Nothing is scripted: the double envelopment comes out of three
  strokes; a bad plan breaks the crescent.
- **The ghost of the day**: the faint dashed graphite under the armies is the movements as the accounts
  give them (Hasdrubal's ride round the Roman rear, the Africans turning in, the Roman press, the crescent
  as it stood and as it was bent back), drawn from the same reconstruction.

## Drawn (not measured)

- The engraving: the paper, the burin cuts (one a unit, oriented along its army's smoothed current, their
  swell running the way it goes), the inks (iron-gall blue-black for Rome; Tyrian purple, the Phoenician
  dye, for Carthage), the marbled chevrons where the inks meet, the stains the currents leave, the fallen
  laid into hatching. The frame is turned so the Romans face down the screen: north-east is up.
- The ending: the page freezes where the day was decided (the moment Hannibal's men stand all round
  Rome's standing mass, or an army is badly broken, or the day has run long): every cut keeps the way it
  last lay, men who got away and men standing alone fade, the pen's strokes dry to faint channels and the
  real day's ghost rises, then settles into a fine engraved line beside them, and the camera draws back to
  the whole field, river to ridge. Nothing is rearranged: each plan's page is its own fight.
- The camera: it opens on the deployment and, once the pen rests, eases in on the fight (the men moving,
  fallen or ringed in), never further out than the opening.
- The speeds, attack and morale numbers were tuned by eye (the spike's, and this round's).
