# Meander: handoff (Nov 4, 2026)

painting-company row 218. A river on the Rio Beni's real floodplain (Sentinel-2, 2024-08-15, today's river
taken out) wandering by the Howard–Knutson curvature model; when a loop's neck closes it cuts through and
leaves an oxbow lake. One gesture: draw a course. ROUNDS.md has a line a round; SOURCES.md the data and model.

## Shipped

`meander-build-139` (6:16 PM ET, Nov 4), from main 53e4baf; the IPA opened in the iOS Simulator with Metal's
validation on. Five rounds, each judged by a fresh critic (ROUNDS.md). Not yet opened on a phone.

## What Paul sees

Opens on a meandering tan river down the green forest belt, a neck already glowing and cutting within a
second or two. Draw with a finger: the line becomes water, the old course silts away in under three seconds,
the new one grows bends, and about ten seconds later a neck glows, flashes, and the loop is left as a dark
crescent like the real ones in the photograph; behind every bend, a few thin pale scroll-bar arcs (one every twenty-odd
years of its movement). A tap near the river nudges its bank toward the finger.

## How it works

- `src/sim.rs`: the centreline (nodes half a width apart), R1 = omega R0 + gamma (upstream-weighted R0),
  resampled and lightly smoothed every step; necks within a width cut; lakes erased where the river crosses
  them; `necks()` for the warning glow; `nudge()` for a tap; banks outside the belt erode a quarter as fast.
- `src/toy.rs`: the view (13 km on the long side), a 25 m deposit grid (the year the river last covered each
  cell, Rg16Float: year and presence, blurred in the shader) drawn as scroll-bar contours; the river, lakes,
  flashes and rings with the kit's Painter; the clock 20 years a second easing to 6 as a neck on screen closes.
- `src/lib.rs`: the opening course, `seed_wobble` (six random waves, so a drawn course starts bending),
  `extend` (a drawn course carried straight off both edges).

## Checks

- `cargo run -p meander --bin meander-probe --release` (no GPU): time to first oxbow for four courses, and
  whether the line crosses itself or kinks; `PROBE_DUMP=<dir>` + `tools/toys/meander/probe_plot.py` draws it.
- `MEANDER_CUTS=1 meander-check play meander/plans/draw.json out`: the seconds of each cut on screen.
- Plans: start (untouched), draw (the gesture, a film), info, rules, ipad. `tools/toys/meander/plans.py`
  writes draw and ipad (raw down/move/up strokes).
- `bash tools/toys/launch_check.sh meander`: passes (196 MB of 1400; shaders to MSL).

## Open

- Cuts often happen near the screen's edges (the river is long and the view a window on it).
- A drawn course's own shape is soon replaced by the river's own bend size (that's the physics; a larger
  drawn wiggle survives longer).
- The deposit grid (3 MB) is uploaded every third frame; its cost on the phone is unmeasured.
- No ambient sound (struck notes at a cut and a new course only).
