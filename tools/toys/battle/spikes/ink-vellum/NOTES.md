# ink-vellum: living ink on vellum (a visual spike)

- **The idea.** The glass is a calfskin page. Cannae's two armies lie on it as wet pools of two inks: Rome in
  iron gall (blue-black wet, browning as it dries), Carthage in vermilion. The finger draws the plan as a thin
  line of the ink it was dipped in (a stroke takes the ink of the pool it starts in), and the line is a wet
  channel: the pool seeps along it, wicks into the skin's grain, piles where the stroke ends. Where the two
  inks meet the water blooms and lifts the vermilion into feathered tide-lines; a tap is a drop of clear
  water that blooms whatever it falls on. Left alone, it dries into a blotted manuscript.
- **What's novel.** The battle is not drawn, it is *wetted*: no units, no arrows, no UI, only liquid behaving
  in a skin. The look is physical (absorption for iron gall, cover for vermilion, coffee-ring edges,
  granulation in pores and follicles, brown halos where gall ink bit the skin, wet sheen) rather than graphic.
- **What's real underneath.** The page's relief is Cannae's ground (Terrarium tiles, ~11 km, rotated 55° so the
  Ofanto runs up the left third): higher land paler, slopes shaded, and the Ofanto (D8 flow accumulation with
  its upstream catchment injected) is a darker, thinner, wetter line in the skin that drinks more ink. Water
  runs down the cockles' and the ground's slopes. The armies are laid where the accounts put them (Roman deep
  centre facing down with the river on its right, the crescent bowed toward it, Africans on its wings,
  Hasdrubal by the river, Numidians opposite), and the scripted plan is the battle's.
- **What's faked.** The flow is crude: semi-Lagrangian advection along stroke directions, an 8-neighbour
  anisotropic diffusion with a wet-front threshold, evaporation faster at thin edges, deposition as the water
  goes. Not conservative, not Navier-Stokes. The grid is 448x336; the full-size skin (follicles, fibres,
  pores) carries the detail. Scale is exaggerated (the armies span ~45% of a page that is ~11 km wide).
- **Files.** `index.html`, `ink.js` (all of it), `ground.js` (built by `build_ground.py`), `render.cjs`
  (header: how to run), `out/`.
