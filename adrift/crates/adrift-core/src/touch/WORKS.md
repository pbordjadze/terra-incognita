# Motion and interaction: the works Adrift is measured against

Five things that feel right under a finger, and what each does that Adrift's touch must match.
A critic who finds Adrift short of any of these has found a bug, not a taste.

1. **iOS Maps (Apple), the globe view.** One finger turns, two fingers zoom and turn at once, and
   the point under the fingers never slides out from under them; a fling decays like a wheel, not a
   motor; a double tap zooms in about the tap; a single tap on a place drops a card without a mode.
   Match: the pinch anchored to the fingers (two passes, as Terra's), the fling's exponential decay,
   the double tap's flight, and never a visible hand-over between one finger and two.

2. **Terra Incognita's globe (`GlobeView`).** Drag, pinch, fling, double tap, a slow turn when left
   alone, and the `hold` scheme: a finger that rests 0.3 s within 8 points is the pack's, any other
   drag is the globe's, so a pan is never taken for a pack's drag nor a drag for a pan. Match: the
   same decision rule (rest, then act), the same sense that the globe is always under the hand, and
   its numbers where they are right (lat ±80, decay 3.2/s, 4°/s idle, 0.7 s flights, cuts under
   Reduce Motion).

3. **Monument Valley (ustwo), the hold-and-drag.** Nothing is explained; a thing you can move
   answers the moment you touch it, and the whole screen is the control. Match: no toolbar, no
   tutorial; the first touch on the sea answers (it rings) so the reader learns by doing; a held
   float shows what holding does (its maybes bloom) before it is dragged.

4. **Procreate, the two-finger tap undo and the three-finger swipe.** Undo is a gesture you can do
   without looking, it takes back exactly one thing, and redo is its mirror. Match: undo by the
   system's gestures (shake; three-finger swipe left; Cmd/Ctrl-Z on the web), one touch at a time,
   nothing else moving; redo its mirror.

5. **iOS's own scroll views and the Dynamics of UIKit.** Deceleration you can interrupt by
   touching, a rubber band you can feel, springs that are critically damped (they settle, never
   bounce twice), and Reduce Motion honoured: no fling, cuts for flights. Match: a touch stops any
   motion at once; the ride's catch-up and the float's return are critically damped springs; every
   curve in `motion.rs` has a Reduce Motion form.

What none of them have, and Adrift must: one beat (0.4 s) that is the only thing to learn, used the
same way everywhere (rest on the sea to put, rest on a float to hold, drag after resting to carry);
and the ride, a camera that drifts with a thing the sea moves, left by any drag.
