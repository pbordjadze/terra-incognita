# pressure-front (simulation spike)

**The idea.** No units, no counters: each army is a density of men laid over the ground like ink, drawn as
its own topography (contour lines, deeper = darker). Each has a will (a potential field toward the enemy),
cohesion (a surface tension), and a shove that grows with depth. Where the two touch, a black wet front
forms and eats both in proportion (Lanchester). Density pressed from two sides at once loses its nerve,
goes pale and dotted, flees for its camp, and dies faster when it can't get there. The ground keeps a
brown stain where men fell, so the end of a battle is a map of where it was decided.

**What's novel.** The front line is the protagonist: a single ink line that bows, buckles, folds into a
U, or tears open. Nobody animates it; it is the boundary between two pressures.

**What's real underneath.** The ground: Terrarium elevation around 41.30 N 16.13 E (rotated so the Ofanto
valley runs up the screen; slopes slow the ink, the scarp of Cannae on the right). The deployment of 216 BC
as the sources give it: Rome deep and massed, its horse on the wings; a thin convex crescent of Gauls and
Spaniards, Africans deep at its horns, Hasdrubal's heavy horse by the river, the Numidians on the right.

**What's faked.** The river's course is drawn (the ancient Aufidus has wandered and the DEM can't place
it). The rates (speed, kill, nerve: Rome's levies drain twice as fast as Hannibal's veterans) are tuned by
eye. Cavalry is not a separate kind: it is ink carried faster along a stroke.

**What the finger does.** Press on an army and draw: the stroke becomes a channel of its will, and the
density lying on it is carried along it, Tsuro-like, faster than it would walk, then turns on the nearest
enemy where the ink ends. Press and hold still: a pin, a hold-the-line order (no advance, the enemy's
shove mostly refused, cohesion doubled); it lingers a few seconds after the finger lifts.

**Plans in the films.** good: Hasdrubal's horse round behind Rome, both African horns folded inward, the
centre pinned; Rome is cupped in a bowl and ground away (Rome 38% left, Carthage 77%). bad: everything
straight at Rome, the thin centre too; the deep column tears through the middle, splits Carthage in two
and pours out south (Rome 57%, Carthage 49%).

Run: `node render.cjs both && python3 shrink.py` (films, moments, contact sheets); `node probe.cjs 600 good` tunes without
drawing; `python3 build_terrain.py <scratch>` rebuilds the ground.

**Continuum or discrete, for drawing?** The continuum feels more alive under a finger: a stroke doesn't
order "these 12 units", it bends a current, and you watch the ink decide how much of the army goes with
it. Its weakness is identity: there is nobody to follow, so no single story can be told, and thin
streams (the horse behind Rome) read as faint smears rather than a charge.
