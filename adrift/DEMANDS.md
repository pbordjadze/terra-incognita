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

### D1 Truth → Engineering (R0)  [open]
I need anything in [a float's state carrying its eddy velocity (two f32, zonal and meridional,
m/s) through the step: `Rules::step` taking and returning that velocity beside the position (a
`Drift { at, eddy }` or a field on `Float`), and `rules.wgsl`'s path buffer holding it, 24 bytes a
path instead of 16] so that the wander has the memory the drifters measured: a random flight
(an Ornstein–Uhlenbeck velocity with the Lagrangian time scale, 2–4 days, which `eddy.py` already
measures and can ship in the sheet) instead of a memoryless walk, so a float put in the Gulf
Stream rides it as real drifters do, and a wake bunches in an eddy and spreads in a jet instead of
scattering everywhere at once.
It works when [of 500 maybes put off Cape Hatteras in a typical July, thirty days on, the median
is within 2° of the bundled GDP runs' 64.8° W, the share east of 65° W is within ten points of
their half, and none is on a coast north of the cape; today's walk gives 68.2° W, 38%, and 3%
ashore (the test `the_flight_off_hatteras_keeps_to_the_record` prints both), and the Sargasso spread test still holds].
I could give back [the step depending only on (at, day, dt, seed, k): with memory it depends on
the velocity carried too, still deterministic for a seed; and 8 bytes a path on the GPU].
The flight itself is ready in `sea/rules.rs` (`Rules::flight`, `Drift`, `flights`) and
`rules.wgsl` (`maybes_flight`, `Path.eddy_u/v`), tested; it gives 65.9° W, 46%, 6 of 500 ashore.
### D1 Visual → Engineering (R1)  [open]
I need `app.rs` to draw the marks and the words through `look::Look` in place of `SpritePass`,
`instances()`, its own `MaybesPass` field and the `chrome()` text: one `look: Look` field
(`Look::new(&gpu, &view_layout)`; `Look` holds the maybes pass, same `draw(rp, view_bind,
&Maybes)` as before), a `look_state()` that fills `look::LookState` from the state (each float's
`at`, `lit` as 1 − lit_for / 3 clamped, `held` as `hand == Held`, its wake's places oldest first;
`resting` from the gestures, D2; `bloom` 1 and `rise` 0 with `drifter` empty until Truth's
tracks exist; `night` false; the date as "14 October" from `clock.calendar()`, the note and the
credits line as `chrome()` has them), then in `render`
`self.look.prepare(&self.gpu, &state, width_pt, height_pt, reading, [cx, cy, r])` and in the pass
`self.look.draw(&mut rp, &self.view_bind, &self.maybes)` after the blit, with `text_pass` not drawn
(the look's words carry their halos, Libertinus Regular and Italic from one signed-distance atlas
at any size; `gpu/text.rs` can stay the runtime's tool for anything else). `look/README.md ›
Wiring` has the lines. So that the float, its lamp, the wake, the maybes, the words and (when it
comes) the silver drifter are what the app draws, at every zoom and in `adrift-check`'s pictures,
instead of R0's plain forms and un-haloed text.
It works when `adrift-check render scenes/gulf-stream-close.json` shows the glass floats with their
amber lamps and beaded wakes, the date and the note on a halo over the globe, and the credits along
the foot (`look/studies/check-gulf-stream-close-phone.png`, `check-foot-over-globe-phone.png`,
`check-foot-phone.png` are what it gives with the wiring applied).
I could give back the words pass and its atlas, if `gpu/text.rs` gains a halo of what is under
each glyph and sizes that don't need a face per pixel size.

### D2 Visual → Motion (R1)  [open]
I need `Gestures` to tell the look where a finger is resting on the sea before the beat ends and
how far through the beat it is: anything in the range of a `pub fn resting(&self, now: f64) ->
Option<(LonLat, f32)>` (the place under the finger and 0..1 of `BEAT`), or the same two values
on `Out`, so that the ring the finger draws can thicken into the float as one continuous motion
(`floats.wgsl`: the white ring draws in, the glass forms over the middle of the beat, the lamp
lights at its end) instead of a float appearing from nothing at the beat.
It works when `adrift-check play plans/put-and-drift.json` shot at 0.2 s and 0.35 s after the
finger goes down shows the ring at two stages of thickening, and at the beat the float with its
lamp where the ring was.
I could give back the ring's starting radius (16 reading points) if the gesture's rest slop needs
it smaller.
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
