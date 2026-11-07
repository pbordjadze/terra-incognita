# Shortcut: sources

What is real and what is drawn. Everything on screen and in the ear is drawn from code; nothing is a
photograph, a map of a real place or a recording. (The crate is still `jam`: the toy began as Ghost Jam,
the Nagoya ring, row 218, at 3032c9a in `main`'s history; row 244 made it Salt Flat, cars on the salt,
whole in `main`'s history before row 255 replaced it with this.)

## The paradox (real)

- D. Braess, "Über ein Paradoxon aus der Verkehrsplanung", *Unternehmensforschung* 12 (1968) 258–268;
  in English as D. Braess, A. Nagurney and T. Wakolbinger, "On a paradox of traffic planning",
  *Transportation Science* 39 (2005) 446–450. In a network where each driver picks their own quickest
  route, adding a road can make every driver's trip slower. His example is the town's shape here: two
  ways round, each a road that slows with its load and one that does not, and a link joining their
  middles.
- J. G. Wardrop, "Some theoretical aspects of road traffic research", *Proceedings of the Institution of
  Civil Engineers* 1 (1952) 325–378: drivers choosing for themselves settle where no one can do better
  by switching (the user equilibrium), which is not where they do best together (the system optimum).
- T. Roughgarden and É. Tardos, "How bad is selfish routing?", *Journal of the ACM* 49 (2002) 236–259:
  how much slower the selfish equilibrium can be; with steep, capacity-limited roads, as here, it can be
  much slower than with gently rising ones.
- The story often told with it: Stuttgart, 1969, where closing a street was reported to ease traffic
  (Terra's light for this toy is there). Told, not measured; the toy doesn't depend on it.

## The roads' slowing (a real kind of rule, drawn numbers)

- A road's speed falls as its cars crowd it: `v = vf / (1 + k vf / C)`, with `k` the cars a lane a
  point, `vf` the road's free speed and `C` a lane's capacity. So a road's time is its free time plus a
  fixed delay for every car on it, growing steeply as its flow nears its capacity: the shape of the
  traffic engineers' volume-delay functions (Bureau of Public Roads, *Traffic Assignment Manual*, 1964;
  Davidson's function, 1966) and of Braess's own increasing link costs.
- Every lane carries at most the same `C` cars a second whatever its speed limit, as highway capacity
  manuals put a lane's capacity at roughly 1,800–2,200 cars an hour fairly independent of its free speed
  (Transportation Research Board, *Highway Capacity Manual*). Here `C` is 1.6 cars a second of the town's clock, which runs 1.7 times the screen's (`PACE` in
  `lib.rs`): the toy's time runs fast, and a town of four roads stands for a city.
- Cars keep a gap of 7 points a lane nose to nose and queue at a road's end when there is no room on
  the next.
- **Drawn:** the numbers. The narrow streets: two lanes, 200 points a second free. The long roads round:
  out wide round the lake, 25 points a second on a phone (on a wider screen they swing further out and run faster in proportion, taking the same time), so wide they never fill. A road the reader draws: four
  lanes, 220 points a second. Cars come in at 3.6 a second. They are tuned so the obvious bridge makes
  the mean trip about a quarter slower (the test `a_bridge_slows_everyone` holds it at over a fifth) and
  closing it brings it back.

## The drivers (the rule real, the people drawn)

- At every junction each driver takes the next road whose time as it looks right now (its length at the
  speed its crowd gives it), plus the quickest way on from its end, is least, each road seen with an
  error of their own of up to 12 per cent either way, and the roads beyond the next weighed half by
  what is seen now and half by what the drivers remember of the last 12 s (they can't see that far). That is selfish, myopic routing, the assumption
  under Wardrop's equilibrium and the way a navigation app routes: nothing in it mentions bridges, and
  the paradox, the crowding of the streets and the emptying of the long roads are not in the code.
- After a change the town sloshes a little (everyone avoids the road that looked slow, then crowds it
  once it empties) before it settles; the drivers' memory damps it (the test
  `the_town_settles_after_a_cut` holds it within a tenth of the old after 16 s).

## The colour (drawn from the rule)

- A car's light is how long its whole way takes right now: the roads it came by, the one it is on and
  the quickest way on from there, each at the speed its crowd gives it now (what a driver setting out on
  the same way now would expect), against the town's usual: the mean of that over the half minute before
  the toy opens, with the town's own roads. White when quicker, warm white at the usual, orange, then
  red when about a quarter slower; eased, slow to redden (about a second and a half) and quick to cool, so
  a moment's worsening as a closed road's cars go round doesn't flash. A road the reader drew
  wears the mean light of the cars on it.

## The town (drawn)

- The lake, the fields and woods, the lit windows, the street lamps, the night: drawn from code. No
  real place.
- The sound: Ghost Jam's small engines (`src/sound.rs`), the cars nearest the middle heard, and a soft
  thump when a road is built or closed. Drawn, not recorded.
