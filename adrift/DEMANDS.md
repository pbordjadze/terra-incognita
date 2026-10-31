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

### D1 Visual → Engineering (R1)  [open]
I need `app.rs` to draw the marks through `look::Look` in place of `SpritePass` and `instances()`:
one `look: Look` field (`Look::new(&gpu, &view_layout)`), a `look_state()` that fills
`look::LookState` from the state (each float's `at`, `lit` as 1 − lit_for / 3 clamped, `held` as
`hand == Held`, its wake's places oldest first; `resting` from the gestures, D2; `maybes` and
`drifter` empty until Truth's sampler and tracks exist, with `bloom` and `rise` 0; `night` false;
the date as "14 October" from `clock.calendar()`; the credits line), then in `render`
`self.look.prepare(&self.gpu, &state, width_pt, height_pt, reading, [cx, cy, r])` and in the pass
`self.look.draw(&mut rp, &self.view_bind)` after the sea; `look/README.md › Wiring` has the lines.
So that the float, its lamp, the wake, the words and (when they come) the maybes and the silver
drifter are what the app draws, at every zoom and in `adrift-check`'s pictures, instead of R0's
plain forms.
It works when `adrift-check render scenes/gulf-stream-close.json` shows the glass floats with their
amber lamps and beaded wakes, and the date and credits in Libertinus under the disc
(`look/studies/check-gulf-stream-close-phone.png` and `check-foot-phone.png` are what it gives
with the wiring applied).
I could give back `sprites.rs` and `sprites.wgsl` (the look keeps them until the wiring lands,
then parks them).

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

## Answered

(none yet)
