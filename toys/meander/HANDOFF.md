# Meander: handoff (Nov 4, 2026; row 243's changes Nov 6, at the end)

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

## Row 243 (Nov 6, 2026): the push

Shipped as `meander-build-169` (2:10 AM ET, Nov 6), from main 502e1be; the IPA opened in the Simulator with Metal's validation on.

- The gesture is now a push (`River::push`, sim.rs): the channel within a fingertip (38 pt) of the finger is carried
  with it (0.93 in the core, so it slips off after ~250 pt); every push leaves a `kick` (the ground there erodes up to
  1 + 2.5 times as fast, fading over 45 years), and for ~10 s after a push `River::calm` holds the rest of the river back
  (0.85), so the next cut is the pushed bend's, ~1.5 s after the lift. Rising pentatonic ticks + haptic every 450 m the
  river gains. A tap nudges the bank and kicks it. Drawing a new course is gone (`extend`/`seed_wobble` still lay the
  opening river).
- The water is drawn by the backdrop shader from a mask the Painter paints at half the screen's pixels (Rgba8: r the
  river, g shoals and the cut's light pulse, b a lake's darkness), so it takes the photograph's grain and has no seams.
  Rings are still the Painter's on the target.
- The deposit grid uploads only the rows the river changed since the last frame (a persistent half-float copy).
- Not measured: frame rate and touch feel on a phone (the Linux check's update is ~1.4 ms on CPU).

## Row 253 (Nov 7, 2026): multitouch

Paul on build 169: "Meander is incredible—just needs multitouch." Four rounds (ROUNDS.md), each a fresh critic on films of
several fingers.

- Every finger is a `Hand` of its own (toy.rs): its push, its kick, its rising ticks (panned to it; one note and one
  haptic at most every 70 ms across fingers); a tap by one finger while others push is still a tap; Cancel is not a tap.
- A finger holds one reach (`River::push`'s `hook`, sim.rs): the node it first meets, carried with HOOK_SPAN nodes each way
  and the channel either side coming along less (HOOK_ARC), so a sweep pushes that reach over the others (cutting where
  it meets them) instead of gathering every strand. With other fingers down it lets go after LET_GO_PT (90 pt), or
  LET_GO_IN_PT (35 pt) when pushing a bend in toward its middle, leaving a kick, and catches again a fingertip clear. One
  finger alone is as it was (its pushed bend still cuts ~0.3 s after the lift in `push`).
- `River::kick` merges a kick into a nearby, just-made one, so five fingers cost what one does.
- The deposit grid is uploaded by 40-cell tiles whose half floats changed (0.39 MB a drawn frame on average with five
  fingers; it had been the whole 3 MB every frame, as the river spans every row). Pixel-identical to before.
- Plans: `two`, `share`, `rub`, `hand` (tools/toys/meander/plans.py); films show the fingers (the kit's `film.fingers`).
  Worst CPU frame with five fingers down ~2.5-3 ms on Linux (now counting the fingers' events; spikes to ~6 ms at cuts).
- Open: two loops swelling into each other can overlap a while before cutting (also in the opening frame); not
  measured: feel and frame rate under real fingers on a phone.
