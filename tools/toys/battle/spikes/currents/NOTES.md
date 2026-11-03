# currents: armies as two inks running on an engraved plate

- **The idea.** No units, no icons. Each army is a current: a crowd of parcels (one about 33 men) whose
  mean motion, smoothed on a coarse grid, is that army's flow field. Every parcel is drawn as one burin cut
  traced along its army's streamline through it, so the cuts' density is the army's strength, their direction
  its intent, and a swell running along each cut shows which way the current goes.
- **What's novel.** It reads as a copperplate engraving that is alive: the hatching *is* the streamlines,
  in iron-gall blue-black (Rome) and murex purple (Carthage: the Phoenician dye), multiplied into warm paper.
  Where the inks touch they slide past each other in opposite senses and the shear curls into eddies; the
  plate keeps a faint stain of where each current ran and where parcels fell, so by the end the paper holds
  the battle's own record, drawn by nobody.
- **The finger.** A stroke lays a channel (a burnished groove that fades in about 8 s) for the army whose ink
  is under its start; that army's parcels nearby are carried along it, like a Tsuro path, then seek the
  nearest enemy (Hasdrubal's horse, once the Roman horse breaks, goes for the allied horse and then the
  Roman rear by itself). A tap is an impulse, a ring that pushes both currents out. The film's plan: Hasdrubal's ride
  round the Roman rear, then the two African wings turned inward, then a tap.
- **Real underneath.** The ground is Terrarium elevation (SRTM-derived, z13, about 18 m a cell) of the plain
  between the Ofanto and the Cannae ridge, turned so the Roman front faces south-west down the screen with
  the river on its right flank; contours every metre (the ridge's crowd into its hatching). The river is the
  thalweg found by pit-filling and D8 flow accumulation (`ground.py`), not drawn. Orders of battle and sizes
  (Roman deep centre, convex Gallic-Spanish crescent, Africans behind its horns, heavy horse by the river,
  Numidians on the open flank; 2,750 vs 2,120 parcels) follow the usual account.
- **Tools.** `ground.py <tile dir>` bakes `ground.js`; `render.cjs` (`QUICK=1` for moments only) makes
  `out/`; `profile.cjs` times a step and a draw. The cuts are WebGL triangles multiplied into a 2D plate
  (Canvas 2D paths were 1 s a frame in headless Chromium).
- **Faked.** The physics is crude (pressure, contact push, a chosen rotation sense at contact, attrition that
  rises when surrounded); the river's exact 216 BC course is unknown and the DEM's channel is a guess; the
  envelopment needs the finger's strokes (without them the cavalry fight and the centre grind, no more).
