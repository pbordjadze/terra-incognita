# Shortcut (was Salt Flat, was Ghost Jam): handoff (Nov 7, 2026)

painting-company row 255 (Salt Flat was row 244, Ghost Jam row 218). Braess's paradox to play with: a small
town at night from above, cars in at the bottom and out at the top round a lake. On the left a narrow
street and then a long slow road; on the right the long road first and then a narrow street. Each driver
takes whatever way looks quickest right now; a road slows as it fills. Draw (or tap, then tap) from road to
road to build one; slash across a road to close it (a barrier); tap a closed road to open it; tap the
water and it ripples; shake for the town's own roads. The obvious bridge across the lake makes every trip
about a quarter slower and the town goes from white to red; closing it cools it within about 15 s. Nothing
in the code mentions bridges. `EXPERIENCE.md` is the plan as first written; ROUNDS.md a line a round;
SOURCES.md what is real and drawn. Salt Flat is whole in `main`'s history before row 255's merge; the
Nagoya ring at 3032c9a.

The crate, directory, bundle id (`com.pbordjadze.toy.jam`), release tag (`jam-build-<n>`) and Terra's link
(`jam::Jam`) are unchanged. The display name (`toy.json`, a working title), the SideStore listing and
Terra's door (now "Shortcut" over Stuttgart, its little window a town round a lake) changed.

## How it works

- `src/town.rs`: the town and the drivers (header: the rules). A road is two directed links; a link's
  speed is `vf / (1 + k vf / CAP)` from its crowd (every lane carries at most `CAP` cars a second); every
  0.25 s a Dijkstra back from the way out gives each node's quickest time on (`ahead`) and its eased
  memory (`usual`, 12 s); at a junction a driver takes the link minimising its time now plus, beyond it,
  half the seen and half the remembered time on, each with a personal error of ±12%. Building splits the
  roads it lands on (`node_on`); cutting closes what a stroke crosses. Tests: `a_bridge_slows_everyone`
  (over a fifth slower with the bridge, back when it is cut) and `the_town_settles_after_a_cut`.
- `src/lib.rs`: the gestures and the drawing. A car's light is `Town::way` (how long its whole way takes
  at the roads' speeds now: the roads it came by, the one it is on, the quickest way on) against the
  untouched town's mean, eased; white, warm white, orange, red at about a quarter slower. Roads, cars,
  windows, lamps, barriers, the pin and the invitation (a road of light drawing itself across the lake
  until the first change) with `paint`; the ground and the lake (with a tap's ripple) a `backdrop`. The
  reader's edits are kept as a list and replayed on restore or a change of screen proportions. A road the
  info mark's ghost (ids from 2^48) closes opens again 3 s later.
- A wider screen (the iPad) swings the long roads further out, their speed scaled to keep their time.
- `src/sound.rs`: Ghost Jam's engines, the 24 cars nearest the middle; a thump on a build or a cut.

## Where it stands

See ROUNDS.md for the critic's verdict each round and the final report for the log.
`cargo test -p jam`, every plan with the tap check, and `tools/toys/launch_check.sh jam` (2 shaders to
MSL, ~140 MB peak) pass on Linux.

## What I'd do next

1. Feel it on a phone: whether the drag and the slash are forgiving under a finger, the frame rate with
   ~110 cars and their glows, the sound (untuned for a town).
2. The paradox's size is a quarter, set by the numbers in `town.rs`; steeper streets or slower long roads
   make it larger but the town sloshes longer after a change.
3. A second puzzle: a town where a closed road helps without a bridge (Braess in reverse, Seoul's
   Cheonggyecheon or Stuttgart's street), or two kinds of traveller.
