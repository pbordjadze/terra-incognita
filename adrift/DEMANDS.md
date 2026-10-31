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

### D1 Truth → Engineering (R0)  [answered]
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
### D1 Visual → Engineering (R1)  [answered]
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

**Answer (Engineering, R1):** done as asked: `app.rs` has one `look: Look`, `look_state()` as
README › Wiring gives it (`resting` from the gestures' `Press`, `bloom` 1 while a float is held,
`drifter` and `rise` empty until Truth's tracks), `Look::prepare` and `Look::draw` in the frame; the
R0 sprites and text pass are not drawn. One change in your directory: the maybes pass reads Truth's
path buffer (`sea::GpuPath`, 24 bytes: lon, lat at location 0, held at 1), and `Look::draw` takes
the count the app last ran. `gpu/text.rs` stays the runtime's tool.
### D2 Visual → Motion (R1)  [answered]
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
**Answer (Motion, R1, recorded by Visual):** in range: `Intent::Press { at, since }` at touch-down
on the sea and `Unpress` on a lift or a move before the beat (Engineering's D1 Motion answer),
and `motion::thicken(now - since)` as the 0..1 the look draws (0 a tap's ring, 1 the float at
`BEAT` 0.40 s); the app hands it to the look as `LookState.resting`.

### D3 Visual → Engineering (R2)  [answered]
### D4 Visual → Engineering (R2)  [answered]
### D1 Motion → Engineering (R1)  [answered]
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

**Answer (Engineering, R1):** in range: `Intent::Press { at, since }`, `Unpress` and `Redo` exist
(model.rs); the app keeps the resting finger for the look's ring (`motion::thicken`) and a redo
stack mirroring undo; a float carried off the disc and let go is taken out (the app watches a
carried float the gestures no longer hold; give it its own intent when you emit one and I'll
apply it). `Carry { float, to }` stays a place: say when you want `Option<LonLat>` and both sides
change together. Your plans run in `adrift-check -- play` against the real globe: the stand-in's
island at 70° W 20° N is open sea there, so `carry-to-land` and `land-silent` need a real coast.
### D2 Motion → Visual (R1)  [answered]
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
**Answer (Visual, R2):** in range, with the give-back taken: the date is drawn under the globe's
disc (baseline cy + r + 40 reading points, clamped to 72 above the foot), centred, the date's
width plus 24 points wide and 34 tall; Motion's `date_band` follows that rule now (ba442b5) and
`Look::date_bounds()` gives the exact rect each frame for the probe to take over. Nothing else
takes touches: the note and the credits are words the globe is touched through. The ring's
thickening is drawn from `thicken(now - since)` exactly (`floats.wgsl`: a plain ring to 0.4, glass
0.4–0.8, the lamp 0.5–1).

### D2 Truth → Visual (R3)  [answered]
I need anything in [a credits panel that opens from the foot's credit line and shows the five
sentences of `sea::CREDITS`, in the words' type, scrollable if it must be] so that the datasets'
licences are met on the device (the Global Drifter Program's CC BY asks for its sentence whole;
the foot's line only names it) and a reader can see that the sea is a typical year's.
It works when [on the device, a tap on the credit line shows the five sentences and a tap
outside closes it, and `adrift-check` has a plan whose shot shows them].
I could give back [the foot line's length: it can be shorter still if the panel carries the rest].
**Answer (Visual, R3):** in range: a page of credits, words only, the five sentences of
`sea::CREDITS` whole in Libertinus 11 reading points wrapped to a readable line under a small
italic head, on a page of the paper's tone (the night's by night) centred over the globe
(`words.rs › credits_page`; `look/studies/r3-credits-page-phone-half.png`). It opens while
`LookState.credits_open` is set and `Look::credit_bounds()` says where the credit line is; the
tap that opens it and the tap that closes it are Motion's (D5) and the state Engineering's (D6),
asked for in this round. The foot line stays as it is.

### D3 Truth → Visual (R3)  [answered]
I need anything in [a float that rides a guess looking different from one that rides a
measurement: `Step.measured` (and `Flight.measured`) is 1 where a drifter measured this month's
current here and falls to 0 where the sheet's value is smoothed in from far away (under the polar
ice, Hudson Bay, some gulfs); a bead or a float with measured under 0.5 drawn hollow, or fainter,
or with a dashed ring] so that nothing on the screen claims the current where no drifter went.
It works when [a float put in Hudson Bay in July shows the mark while one in the Gulf Stream does
not, in a `render` of a scene with both; Engineering carries `measured` on the `Float` and `Bead`
by a ruling, which this demand asks for too].
I could give back [nothing in the rules; the mark's form is Visual's].
### D3 Motion → Engineering (R3)  [answered]
I need anything in [the web shell (`adrift-web/src/lib.rs`) routing its wheel through
`App::wheel(x, y, factor)` with the cursor's position, and its keys through `App::key`: ⌘Z or
Ctrl-Z undo, ⇧⌘Z, Ctrl-Shift-Z or ⌘Y/Ctrl-Y redo (a bare `z` may stay as an alias; space and the
arrows for the date may stay)] so that on the web what is under the cursor stays under it as the
wheel zooms (as a pinch does on the device), a wheel stops a fling as a touch does, and undo has
its mirror, as VOCABULARY › What the shells owe says.
It works when, in the web build, a wheel over Iceland keeps Iceland under the cursor at every
zoom, and ⌘Z then ⇧⌘Z take a put float away and bring it back.
I could give back: the modifier rule (any `z`/`y` without a text field is harmless) if the
shell's key handling must stay one match; the anchoring is the part that matters.

**Answer (Engineering, R3):** done, with thanks for the line numbers: the rewrite had been lost in the branch rebuild. The web shell sends the wheel to `App::wheel(x, y, factor)` about the cursor (through `Gestures::wheel` and the probe, so a fling stops) and keys to `App::key` (⌘/Ctrl-Z undo, ⇧⌘Z and ⌘Y redo); the device shell forwards the shake and plays the haptic on Put and Hold.
**Answer (Visual, R3):** in range: a float whose `measured` is under a half has its rim dashed
(eight dashes round the glass), and a bead laid where the sea was a guess is hollow (its edge and a
ring of amber, no heart); `LookFloat.measured` and `LookBead.measured` carry the ruling's values
(`look/studies/r3-measured-hudson-bay-and-gulf-stream-phone.png`: Hudson Bay in July dashed, the
Gulf Stream whole). The maybes keep their grey for a self aground; a guessed sea isn't marked on
them, they are maybes already.


### D5 Visual → Motion (R3)  [open]
I need a tap on the drawn credit line (the rect `Look::credit_bounds()` gives, through the app's
probe, as the date's is) to open the page of credits and any tap while it is open to close it,
as intents (`ShowCredits`, `HideCredits`, or one `ToggleCredits`), with the date's own tap and
drag unchanged, so that the datasets' licences are met on the device (Truth's D2) with nothing
added to the screen but a line that was already there. It works when a plan taps the credit
line and the shot shows the page, taps the globe and the shot shows it gone, and a drag on the
date still scrubs. I could give back the close-on-any-tap: a tap on the page itself could be
enough, if a tap on the globe must stay the globe's.

### D6 Visual → Engineering (R3)  [answered]
I need `App.credits_open` set by Motion's intents (D5) and handed to the look as
`LookState.credits_open`, the probe answering Motion from `Look::credit_bounds()` as it does from
`date_bounds()`, and a plan step that can set it for the checks, so that the page of credits the
look draws is reachable on the device and in `adrift-check`. It works when
`adrift-check play` with the step shows the page over the globe (the look's render of it is
`look/studies/r3-credits-page-phone-half.png`). I could give back the plan step if `-- play`
can send `ToggleCredits` as an intent already.

**Answer (Engineering, R3):** done: `Intent::ToggleCredits` (model.rs; the one intent D5 names, ruled), `App.credits_open` flipped by it and handed to the look as `LookState.credits_open`; `App::credit_bounds()` exposes `Look::credit_bounds()` for Motion's probe (touch/probe.rs is Motion's: it maps the tap); a plan step `{"intent": "ToggleCredits"}` sets it for the checks.

## Answered

(none yet)
