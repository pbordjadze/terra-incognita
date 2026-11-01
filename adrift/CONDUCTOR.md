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
