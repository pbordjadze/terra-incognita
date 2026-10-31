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

## R1 (October 31, 2026): the running build on Linux

What was judged: `adrift-check -- play` over `plans/check/*.json` on the R0 skeleton with this
module landed (branch `adrift-motion`), 22 plans checked against their expectations by
`check_play.py` (`cancel` and `reduce-motion` wait on two steps the check has yet to take), and the
strips in `critics/`: `r1-put.jpg` (0.2 s, 0.39 s, 0.6 s, 1.5 s of a rest on the sea),
`r1-carry.jpg` (held, carried, set down, undone), `r1-take-out.jpg` (held, the hand off the disc,
gone, undone), `r1-ride.jpg` (0.1 s, 1 s, 3 s of a ride, then left by a drag).

**The hater** (wants it to fail; judged the PNGs and the logs, not the text):
- *The beat is invisible until it is over.* In `r1-put.jpg` the ring at 0.2 s and at 0.39 s is the
  same ring a tap makes; the float pops in at 0.4 s with nothing having promised it. A reader has no
  way to learn that resting does anything but wait. The rule says "the ring thickens"; the build
  does not. (D1, D2.) The one mercy: a lift at 0.35 s shows no half-float, because nothing is drawn.
- *The float carried off the rim does not go anywhere.* `r1-take-out.jpg`, second frame: the hand is
  off the disc and the float sits off Nova Scotia as if set down. "Taken out in plain sight" is a
  sentence, not a picture. (D1.)
- *The date is a control nobody can see.* The band at the foot scrubs and pauses (the log proves it)
  and nothing is drawn there. Worse than a toolbar: an invisible one. (D2.)
- *The ride's first tenth of a second snapped to the float in the first run.* A zero-length frame
  made the spring jump to its target; fixed in the same round (`motion::spring` on `dt = 0`), and
  the strip now shows the camera easing from 40° W to 31.4° W in a second without overshoot. The
  bug would never have been seen on a 120 Hz device, only in the check: keep the check.
- *The fling toward the pole runs away in longitude.* `ride` after the leaving drag: 89° W at
  3.4 s, 168° W at 6 s. Correct geometry (the surface under the finger keeps its screen speed)
  but worth a device test: does it feel like a wheel, or like the globe escaping?
- *The hand's reach is 14 points*, a toddler's fingertip. Motion wants 22. The app's `hit` decides.
- Not failed: the globe never moved under a carry (`carry`, `carry-fresh`: camera unchanged in
  every line); a drag that began on a float left the float where it was; a drag never put a float;
  land never rang; the double tap arrived at zoom 2 and 4 at the tapped place; the idle turn waited
  8 s and held still once a float was on the sea; undo took back exactly one touch each time
  (`undo-gestures`: 6 checks).

**Holt** (can you wander, stop, undo and leave without penalty; does something unscripted happen):
- *Wander*: yes. Every one-finger touch does something or nothing, never the wrong thing; a drag
  is always the globe's; two fingers always zoom. No mode was entered in 24 sessions.
- *Stop*: the date can be stopped by a tap, and the sea stops with it (`date-tap`, `date-scrub`),
  but only if you already know the band is there (the hater is right).
- *Undo*: one touch at a time, nothing else moving: `carry` undone put the float back and the
  camera stayed; `take-out` undone gave the float back where it was lifted, after a fix this round
  (before it, undo returned a float to a place the reader never left it: a penalty for leaving).
- *Leave*: nothing marks, nothing points; the globe turns by itself only on an empty sea.
- *Unscripted*: the floats moved between every shot by the real current (every `float` check needed
  a degree of slack for it): the sea did something nobody chose. Not yet felt: no maybes bloom on
  a hold (Truth's sampler), so the one gesture that shows the rules have other answers shows
  nothing but a halo.
- Verdict: the vocabulary holds; the build does not yet show it. Nothing in Motion's module is
  waiting on Motion; three things are waiting on D1 and D2.
