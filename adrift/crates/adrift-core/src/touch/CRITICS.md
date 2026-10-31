# Critics: motion and interaction

Two critics each round, judging the running build and nothing else: a fresh *hater* who wants
Adrift to fail, and a *Holt* who asks whether you can wander, stop, undo and leave without penalty,
and whether something unscripted happens. Their verdicts here, with the PNGs they judged.

## R0 (October 31, 2026): no build to judge

There is no skeleton yet, so there is no PNG and no verdict. What exists is the state machine
(VOCABULARY.md), 10 unit tests and 24 played plans that pass in `touch-replay` over a stand-in sea
(`tools/adrift/touch-standalone`). A plan passing there proves the intents come out in the right
order with the right places; it proves nothing about how it feels. The critics' briefs for R1, so
they are not written by the builder after the fact:

**The hater (R1 brief).** Play `plans/*.json` through `adrift-check -- play` and look for:
- a half-float: lift at 0.35 s (`almost`) and find any frame where the ring has visibly started to
  become glass. If `thicken` is readable before 150 ms, the beat is a trap.
- the globe moving under a carry (`carry`, `carry-fresh`): any camera change is a fail.
- the point under the finger sliding (`orbit-from-float`, `pinch`): compare the taken point's
  screen position frame to frame against the finger's.
- a fling that feels like a motor (constant speed) or a brake (stops dead): the decay curve read
  off successive frames must be exponential.
- a ride that overshoots or lags visibly behind a float on a fast current (`ride`).
- anything that waits: a tutorial, a hint, a toolbar, a cursor. One pixel of chrome that is not
  the date and the credits is a fail.
- the sea ringing on land, a float put on land, a touch off the disc doing anything but turning.

**Holt (R1 brief).** In a played session of two minutes with no plan (random taps, rests, drags, a
pinch, a shake): did a float ever go somewhere nobody chose (the rules, not a script)? Could every
touch be taken back by one undo, and did undo move nothing else? Was there a moment the reader was
made to wait, or told what to do? Did leaving (no touch for a minute) mark, point or wait?

The PNGs for both go in `adrift/crates/adrift-check/out/motion/` (Engineering's directory; a demand
if it needs one) and are named here.
