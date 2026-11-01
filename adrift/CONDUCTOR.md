# Conducting the sea (a spike, painting-company row 199)

Not a round and not on `main`: branch `adrift-conductor-spike`. The web build of this branch opens
conducting (`?free` opens the ordinary sea).

**What it is.** A take runs at 6 days a second off Cape Hatteras in a typical June. In **Lamps** (the
default) three lamps travel as lines; in **Cloud** one lamp's 2048 possible seas bloom as a cloud.
A gesture looks back over the seas that already ran (`conduct.rs`) and picks the one that did what
the hand did:

- **tap**: the sea drawn nearest the finger;
- **sweep**: the sea that, over the last stretch as long as the stroke, veered most the stroke's
  way from the others' drift (a longer stroke picks a more extreme one);
- **circle** (three quarters of a turn or more): the sea whose path turned most the same way;
- **rest** a finger: the date holds until it lifts.

The lamp's journey since its last cue is rewritten to that sea's path, and a new 2048 bloom from
where it is, carrying its eddy velocity. A take is its cues (day, sea) and **Again** replays it
exactly (`conduct::tests::again_replays_the_take`). In Lamps a gesture goes to the lamp it starts
nearest; a gesture started away from all of them (more than 90 pt) goes to every lamp at once.

**How it is honest.** The 2048 seas run on the CPU on a fixed half-day grid from each cue, so what is
drawn, what a gesture is matched against and what a replay computes are the same numbers (the
GPU's maybes are not used: their step follows the frame's clock). A world's kicks are fresh each
quarter day, so continuing a chosen path in all 2048 worlds after a cue is the same law as one
unbroken flight; the eddy velocity is carried across the cue, not drawn again.

**Measured** (`tools/adrift/conduct-plan.py` writes the plans; `adrift-check play` prints a line per
cue): a gesture matches 100 to 490 of the 2048 seas (5–24%; a tap counts the seas within 24 pt, a
sweep those within 30° of its direction, a circle those that turned a quarter or more its way); the
look back costs 0.1–0.6 ms (tap, sweep) and 1.6–8 ms (circle) natively; the cue is drawn on the frame
after the finger lifts. The web build logs the lift-to-frame time of every cue to the console.

**Not done.** The phone as a baton: the claude.ai artifact's frame refuses motion events, so the
swing-to-sweep code (`App::conduct_swing`) has no caller in the published page.

## In the app (R11)

Conducting is now the studio's step 2: while a take records (or replays for its export), a gesture
cues a lamp (`App::cue`, `conduct::Worlds`). What Truth checked and set this round:

**A conducted lamp is its world, bit for bit.** A cue at a moment between quarter-day marks makes the
lamp's leg world k up to the next mark, and begins the next leg on that mark with world k's exact
place and eddy, still in world k (`Worlds::cut`). The take's start cuts the same way from the lamp's
own sample. So the lamp's path, from its leg's start until the next cue, is world k's unbroken flight
step for step: nothing interpolated ever becomes a leg's start, and the law after a cue is the law
before it. Tested by driving the App (`conduct::tests`): a take begun between marks leaves every
frame's lamps where they would have been, to the bit, for 7 days; after a sweep the lamp is world
k's unbroken flight (gap 0.0 degrees over 6 days). The seam's first version began a leg at the
interpolated place with the last mark's eddy and stepped a partial quarter day. That met different
kicks: a lamp 7 days on was 24 km (mean) and 66 km (worst) from where it would have been, which is
visible at a take's zoom.

**The chooser does what it says** (`conduct::tests`, a lamp off Hatteras in a typical June, ~30 pt a
degree). A tap picks a world at least as near as the one under the finger, and none matches when no
world is within 24 pt. A sweep's pick moved the stroke's way against the worlds' mean, beyond 70% of
them, and 14–19% of the worlds matched (within 30° of the stroke). A circle's pick turned the
circle's sense (±8–9 rad over its look-back), and 13–16% of the worlds turned a quarter or more that
way. These are inside the spike's 5–24%. A tap's matches range from a handful to a third of the
worlds, depending on how dense the cloud is under the finger.

**Cost** (`conduct::tests::conduct_cost`, release, a 2.1 GHz Xeon core; the phone assumed 2–3x
slower):

| | native | phone (est.) |
| --- | --- | --- |
| a flight step | 0.46 µs (0.58 before the ice's fast path) | |
| a quarter-day mark of 1024 worlds | 0.47 ms | 1.0–1.4 ms |
| six lamps' mark, spread over 5 frames (`Worlds::prepare`) | 0.59 ms a frame | 1.2–1.8 ms a frame |
| choose: tap / sweep | 0.02 / 0.05 ms | ≤ 0.15 ms |
| choose: circle, 1 s / 3 s of look-back | 0.48 / 1.24 ms | 1–4 ms, once, on the cue's frame |

The seam stepped 2048 worlds a whole mark at a time. Every lamp crosses the same grid mark on the
same frame, so six lamps cost about 9 ms in one frame of every 7.5 natively (some 20+ ms on the
phone): a hitch four times a second. Three changes:

- the next mark is prepared a share each frame (`prepare`: the same pure steps, earlier, so replays
  are unchanged);
- sea ice is not read nearer the equator than 37° (no month's sheet has any there:
  `sea::tests::no_ice_nearer_the_equator`);
- `WORLDS` is 1024. A gesture matches the same share of 1024 as of 2048, and a sweep's furthest pick
  is the top one instead of the top two.

`HISTORY` stays 65 marks (16 days: 5 s of look-back at 3 days a second; 1.6 MB a lamp).

**The tempo: 3 days a second while conducting.** What the worlds do in a stroke (`the_tempo`,
off Hatteras at ~30 pt a degree; a stroke's look-back is its seconds × the rate):

| rate | stroke | look-back | lamp's drift | worlds' spread | a sweep's 0.95 pick | turned ≥ ¼ in 1 s |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 1 s | 1 day | 5 pt | 12 pt | 13 pt | 6% |
| 2 | 0.5 s / 1 s | 1 / 2 days | 5 / 10 pt | 12 / 23 pt | 13 / 24 pt | 14% |
| 3 | 0.5 s / 1 s | 1.5 / 3 days | 8 / 15 pt | 17 / 32 pt | 18 / 35 pt | 18% |
| 4 | 0.5 s / 1 s | 2 / 4 days | 10 / 20 pt | 23 / 40 pt | 24 / 44 pt | 23% |
| 6 | 0.5 s / 1 s | 3 / 6 days | 15 / 31 pt | 32 / 55 pt | 35 / 59 pt | 27% |

A cue reads when the chosen world sits visibly apart from the lamp's other possibilities: about a
fingertip (30 pt) for a one-second stroke. At 2 days a second a half-second sweep chooses among
worlds 12 pt apart, half a fingertip: the hand's choice barely shows. At 6 (the spike's) everything
reads, but a second of clip is six days and the eddies blur into drift. The sweep's 0.95 pick was
already 59 pt out, and the mark's cost triples past the phone's budget. **3** gives a one-second
stroke a 35 pt answer, and a circle finds turners among 18% of the worlds. It keeps the wander's
own wiggle legible (a quarter-day mark every 5 frames), and fits the phone at 1024 worlds. This is
the date's rate while the hand conducts, an `App` setting (`Clock.rate` while recording); the
ordinary sea can keep 2. Zoom scales every pt figure: at a closer camera 2 days a second would do.
