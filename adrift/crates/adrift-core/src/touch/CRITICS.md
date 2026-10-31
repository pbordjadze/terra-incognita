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
- *The hand's reach was 14 points* on the R1 skeleton, a toddler's fingertip; Motion wants 22. (The
  merged head reads `motion::REACH`: 22.)
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

## R2 (October 31, 2026): the merged build (`adrift` at d2bd376)

What was judged: all 24 plans played by `adrift-check` on the merged tree (Engineering's run on its
check with the app's ids: 24 of 24 pass; this branch's run drew the PNGs), and the strips in
`critics/`: `r2-put.jpg` (0.2 s, 0.39 s, 0.6 s, 1.5 s of a rest on the sea), `r2-hold.jpg` (a float
held, then carried, then set down), `r2-date.jpg` (the date under the disc as the probe now reads
it), `r2-take-out.jpg` and `r2-ride.jpg`.

**The hater** (a new one, who read R1's verdict and came to see if anything changed):
- *The beat is seen now, and it over-promises.* `r2-put.jpg`: at 0.2 s a tap's ring; at 0.39 s a
  formed glass float with its lamp lit, a hundredth of a second before the beat. Lift there and a
  lit float vanishes. The ring thickening is right; the lamp lighting before the state has a float
  is a lie of ten milliseconds, and the kind a reader feels. Asked of Visual, and agreed the same
  hour: the lamp lights on `Put`, the press only forms glass; the curve stays.
- *The date is a real control now* (`r2-date.jpg`: "3 January, a typical year, by the drifters'
  record", under the disc; `date-scrub` and `date-tap` pass against `Look::date_bounds()`). Still
  nothing says it can be touched. That is the design, and the hater notes it will be the first thing
  a tester fails to find. A device test decides; no change asked.
- *The maybes do not read at phone size.* `r2-hold.jpg`, first frame: the silver drifter's track
  rises (good) and a halo; the maybes are specks under the halo at zoom 1. Hold a float zoomed out
  and the one gesture that shows the rules' other answers shows a smudge. Visual's and Truth's to
  weigh; Motion's part (the hold fires at the beat, carry only after) is proved.
- *A take-out still does not leave in plain sight*: with `Carry { to }` a place, the float waits at
  its last water until the hand opens off the disc (`r2-take-out.jpg`, first frame). Both sides
  agreed to change `to` to a hand when the look can draw the leaving; not this round. And on the
  tree these PNGs come from (d2bd376) the hand opening off the disc put the float *back* (second
  frame: afloat again at 30° W): the app's reading of `Release { at: None }` changed under the
  merge; Engineering's next push takes it out while carried (its run passes `take-out`). The
  gesture emits the same intent either way; the PNG is the proof that a seam's meaning must be
  written where both sides read it (VOCABULARY's table is that place).
- *The ride* (`r2-ride.jpg`): the camera at 31.4° W after one second, at the float after three,
  no overshoot; the stand-in's zero-step snap of R1 is gone on the real build.
- Not failed: 24 of 24 on the real globe with real land (Kansas is land; the Atlantic is sea);
  cancel and Reduce Motion now checked on the build, not the stand-in; the pinch ends a press with
  `Unpress`; the spring and the undo bugs of R1 stay fixed.

**Holt:**
- *Wander, stop, undo, leave*: as R1, and now the date can be found by a reader who looks under the
  globe; undo and redo are mirrors (`undo-gestures`, 7 of 7).
- *Unscripted*: the silver drifter that rises on a hold is a real buoy's real track, and it was not
  where the plan's author expected (east, along the Canary current). The sea answered.
- Verdict: the vocabulary is complete on the build; what remains is the look's timing of the lamp
  and the device's own verdict on the fling toward the pole and the invisible date.

## R3 (October 31, 2026): the merged build after R2; the device to come

What was judged: the 27 plans (three new: `date-ipad`, `ride-fast`, `reduce-motion-ride`) on
`adrift` at 480529d with this round's camera, and `critics/r3-date-ipad.jpg` (the 13-inch iPad's
date, paused and scrubbed), `critics/r3-ride-fast.jpg` (a Gulf Stream float ridden at zoom 4 at
1 s, 4 s and 6 s). The device captures the owner was asked for (a fling toward the pole, the beat
under a real finger) are not in yet; this entry is reopened when they are.

**The hater** (a third, who reads logs):
- *The ride trailed a fast float and nobody had measured it.* A critically damped spring trails a
  steady target by twice its time constant times the speed: at the clock's two days a second a
  Gulf Stream float moves about a degree a second, so the camera would have sat half a degree
  behind, six points at zoom 4, forever. Measured on `ride-fast` after the fix (the spring aims
  ahead by the float's smoothed speed): the camera within 0.2° of the float at 4 s and 6 s, on a
  float that curves north-east; the unit test holds it under 0.15° on a straight one.
- *The date on the iPad is where the look draws it* (`r3-date-ipad.jpg`): a tap pauses ("held"
  under the date, a word, as the rule asks), a drag of 120 points scrubs 120 days. The probe reads
  `Look::date_bounds()`, not Motion's band, so the two cannot drift apart. Still unmarked, still a
  device question.
- *Reduce Motion* (`reduce-motion-ride`, `reduce-motion`): no fling, a double tap cuts, the ride
  follows without a glide, nothing turns by itself. What still animates under it is the ring's
  thickening (the gesture itself, kept on purpose) and the look's own motion (the sea, the wakes,
  the ripples: Visual's call, not judged here).
- *The web shell is not the vocabulary.* Its wheel zooms about the centre and never stops a fling;
  its undo is a bare `z` and there is no redo, while `App::wheel` and `App::key` sit unused. D3.
- *The maybes' smudge* at phone zoom 1 stands from R2 (Visual's).

- *The credits* (`r3-credits.jpg`, D5): a tap on the credit line opens the page over the globe, a
  tap on the sea closes it and puts nothing (a rest there while the page is open puts nothing
  either), the date still scrubs beneath, a drag still turns. One line that was already there is
  the only control added, and it is the licences' due.

**Holt:** nothing new to penalise; the ride on a fast float is now a thing you can watch rather
than chase. The device will say whether the beat is felt and whether a fling toward the pole feels
like a wheel. Verdict withheld until it does.

## R4 (October 31, 2026): the owner's first words, answered on the Linux build

The first device verdict came as words, not captures: "really cool and my favorite thing so far";
"no UI to change anything and the date controls aren't super intuitive at first"; "can't reset the
globe or clear all the lamps no matter how hard I try"; "picking up a lamp to see the maybes just
isn't natural yet". Judged here: the 31 plans on `adrift` at 5cd8398 with this round's gestures,
`critics/r4-home.jpg` (the camera a third of the way home after an over-pinch) and `r4-flick.jpg`
(the date running on after a flick, "held" beneath it, no finger down). The fling toward the pole
and the beat under a finger are still unanswered by the device.

**The hater** (a fourth, who read the owner's words first):
- *He tried hard and found nothing, and two of the four new routes are still things nobody finds.*
  Three-finger swipes up and down are for a hand that already knows iOS's three-finger family;
  the hater grants them as the mirror of undo and redo but not as discovery. What a hand does on
  its own: pinch the globe all the way out and keep squeezing (home: `home.json`, the camera flies
  to the opening view, 0.7 s eased), and shake again when one shake was not enough (clear:
  `clear.json`, every float out as one touch, undo brings all back). Those two are the answer; the
  three-finger ones are the family. The device will say whether "keep squeezing" is what he does.
- *A second shake clearing the sea is one shake away from a disaster* if the system ever delivers
  one shake twice. The window (0.4–2 s after the first) is the guard; the undo is the net. Watch
  it on the device: if a single vigorous shake clears, the window moves to 0.6 s.
- *The date's flick is the first thing on the date a hand will like* (`date-flick.json`: the date
  runs on and slows, a touch stops it, the word "held" stays); but the band still looks like a
  caption, which is the owner's whole complaint. Motion cannot draw; D8 asks Visual for the band
  to answer the finger within a frame.
- *Picking up a float still shows nothing for 0.4 s.* The hater's R2 point, now the owner's. D7
  asks for `PressFloat` so the maybes rise from the finger's landing; until it lands, a hold is a
  wait, and this round's build has not fixed it. Reach is 22 points; not the problem.
- Not failed: the three new gestures leave the 28 earlier plans passing; home ends a ride; clear
  is one undoable touch; a touch stops the flicked date as it stops the globe.

**Holt:**
- *Undo without penalty*: clearing the sea is one touch and one undo gives it all back; home moves
  the camera only, never a float. The owner's "no matter how hard I try" is the real penalty: he
  was never told, and nothing marks; this round answers with what a hand does by itself (squeeze
  past the end; shake again), which Holt accepts as the right kind of answer and the device must
  confirm.
- *Wander, stop, leave*: unchanged; the flick makes the date a thing to play with rather than a
  setting.
- Verdict: the vocabulary now has a way out of everything (home, clear) and a way back from both;
  what it lacks is still the look's answer under the finger (D7, D8). Reopened for the captures.
