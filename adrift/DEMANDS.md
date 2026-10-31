# Adrift: demands

The cascade. When a craft meets a trade-off it never lowers its own bar; it writes here, on the
craft that could remove it, and the receiver answers below it in range or with a counter-range.
Form:

    ### D<n> <from> → <to> (R<round>)  [open | answered | ruled]
    I need anything in [range] so that [effect on the end goal].
    It works when [observable on the device or in a check's PNG].
    I could give back [what I'd relax].
    **Answer (<to>):** ...

At most two new demands a craft a round. Engineering rules on one that comes round unchanged twice
(DECISIONS.md).

## Open

### D1 Motion → Engineering (R1)  [open]
I need anything in [`model::Intent` gains `Press { at, since }` and `Unpress` (or the look reads
`Gestures::pressing_since`), `Carry { float, to: Option<LonLat> }` with `None` meaning the hand is
off the disc, and `Redo`; `App` applies them: a float carried off the disc stays in hand drawn
going, redo its mirror of undo] so that the beat is seen as it is felt (the ring thickens into the
float under a resting finger, and a lift short of the beat shows no half-float), a float carried
over the rim is taken out in plain sight, and a touch taken back can be given back.
It works when `plans/check/put.json`'s shots at 0.2 s and 0.39 s show a ring thickening and
`almost.json`'s show none, `take-out.json`'s `over-the-rim` shows the float going, and a swipe
right after a swipe left in `undo-gestures` brings the float back.
I could give back: the three-finger swipe right if redo is ruled out (⇧⌘Z too); and the ring's
thickening could be the look's own clock from `Ring` if the beat's exact length is bundled from
`motion::BEAT` (then no `Press` at all).

### D2 Motion → Visual (R1)  [open]
I need anything in [the date drawn at the foot inside `touch::date_band(frame)` (the middle half
of the width, 44 points tall, 24 above the bottom edge), as the one and only chrome besides the
credits; and the ring's thickening drawn by `motion::thicken(since)` (0 a tap's ring, 1 the float,
exactly 1 at `motion::BEAT` = 0.40 s) once D1 gives it `since`] so that the date is the control
for scrubbing and pausing with nothing else marked, and the finger and the ring agree to the
frame.
It works when a drag across the drawn date in `plans/check/date-scrub.json` scrubs it on the
device, and `put.json`'s `thickening` and `almost` shots show the curve.
I could give back: the band's exact size (anything 36–56 points tall, 20–40 above the edge,
40–70% of the width) if the drawn date needs other room; Motion reads the look's numbers then.

## Answered

(none yet)
