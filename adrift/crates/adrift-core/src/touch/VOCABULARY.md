# The gesture vocabulary

One finger does everything; there is one beat to learn (0.4 s, `motion::BEAT`), and it means the
same thing everywhere: *rest, then act*. A touch that moves before the beat is the globe's. A touch
that rests the beat is about what is under it. Nothing marks, nothing points, nothing waits.

| You do | On | It does |
| --- | --- | --- |
| tap | the sea | it rings (nothing changes) |
| rest a beat | the sea | the ring thickens into a float, held under your finger; lift and the sea has it |
| rest a beat | a float | you hold it: its maybes bloom, the nearest drifter rises; lift and they fold back |
| rest, then drag | a float (or the one just put) | you carry it: set it down on other water, or off the rim and it is taken out |
| tap | a float | you ride it: the camera drifts with it, until any drag |
| drag (before the beat) | anything | the globe turns; let go moving and it flings |
| two fingers | anything | zoom and turn, the point under the fingers staying under them |
| double tap | anything | fly in about the point (out to the whole globe from deep in) |
| drag | the date at the foot | scrub the date; the sea pauses with it |
| tap | the date | pause or play |
| tap | the credit line | the page of credits opens; any tap while it is open closes it |
| three-finger swipe left · shake · ⌘Z | | undo the last touch; nothing else moves |
| three-finger swipe right · ⇧⌘Z | | redo it |

## Inputs

`Gestures::feed` receives `model::PointerEvent { id, phase: Down|Move|Up|Cancel, x, y, t }`
(points, seconds) and `Gestures::tick(now)` runs each frame (the beat elapses without an event).
To decide, it asks a `Probe` (`touch/probe.rs` answers for `App`) what is under a point:
`Hit::Float(id, place)` (within reach, the nearest), `Hit::Date` (inside `touch::date_band`),
`Hit::Sea`, `Hit::Land`, `Hit::Off` (off the disc); where a float is; which float is ridden; the
clock; the frame (size, reading scale, disc); Reduce Motion. It reads the state no other way. It
returns `Out`: `model::Intent`s, which `App` applies, and `CameraMove`s, which `App` hands to
`CameraControl::apply`; the look draws the state and never reads pointers. Undo also arrives as
`Gestures::shaken()` and `Gestures::key(Key::Undo)`; `Gestures::wheel(x, y, factor)` zooms about a
mouse's cursor (the web build).

## What the shells owe

The device shell turns touches into `PointerEvent`s (one id a finger, times in seconds), calls
`Gestures::shaken` on the system's shake, plays a light haptic when it applies `Put` or `Hold` (the
beat is felt as well as seen, Terra's `hold` feel), and reads Reduce Motion into `App`. The web
shell forwards pointer events the same way, `Gestures::key` for ⌘Z (Ctrl on other systems) and
`Gestures::wheel` for a mouse's wheel or a trackpad's pinch. The date's touchable rect is the
look's own (`Look::date_bounds()`, the drawn glyphs with their halo's room, once Visual's look is
on `adrift`; `probe.rs` reads it through `App`); until then `touch::date_band(frame)` stands in
by the look's rule: a baseline 40 reading points under the disc's bottom edge, never lower than
72 above the foot (so past zoom 2 it lies over the globe), the middle half of the width, 44 tall.

## The intents (model.rs, Engineering's) and how the vocabulary maps onto them

| Intent | Motion emits it when |
| --- | --- |
| `Ring { at }` | a finger lands on the sea (the ring starts at touch-down, not at the lift: the sea answers at once) |
| `Press { at, since }` | the same moment: the finger rests, and the look thickens the ring by `motion::thicken(now - since)` |
| `Unpress` | the finger lifts or moves before the beat, a second finger lands, or the system takes the touch: the ring lets go |
| `Put { at }` | the beat passes on the sea (the app clears the press itself); the app answers `Gestures::float_put(id)` and the finger holds it |
| `Hold { float }` | the beat passes on a float |
| `Carry { float, to }` | a held finger moves past the slop: `to` is the place under it, water or land |
| `Release { float, at: Some(p) }` | the hand opens over water (`p` there), or without having carried (`p` where it was lifted: the app sees nothing moved) |
| `Carry { to: from }` then `Release { at: Some(from) }` | the hand opens over land, or the system cancels the touch: back where it was lifted |
| `Release { float, at: None }` | the hand opens off the disc: taken out |
| `Ride { float }` | a tap on a float the camera is not already riding (the app's `Ride` toggles; Motion never sends it for the ridden one) |
| `Scrub { days }` | a drag on the date, by its horizontal move since the last event (`DAYS_PER_POINT`) |
| `SetRunning { false }` | the first move of a scrub; `SetRunning { !running }` a tap on the date |
| `Undo` | a three-finger swipe left, a shake, ⌘Z |
| `Redo` | a three-finger swipe right, ⇧⌘Z |
| `ToggleCredits` | a tap on the credit line (`Hit::Credits`, the look's `credit_bounds`); while the page is open, a tap anywhere but the date |

Still a place, not a hand: `Carry { to }` is a `LonLat`, so while the hand is off the disc the
float stays where it was last over water (the app watches the carried float and takes it out when
the hand opens there); "taken out in plain sight" waits on a `Carry` with no place, to be asked for
when the look can draw it. The camera is read, not sent (`CameraControl` is the truth).

## The state machine (one finger)

```
Idle
 ├─ Down on Sea ──────────────▶ Pressing{at,t0}    emits Ring, Press
 ├─ Down on Float(id) ────────▶ Resting{id,t0}
 ├─ Down on Date ─────────────▶ DateDown{x0}
 ├─ Down on Land/Off ─────────▶ Down{t0}            (silent)
 └─ (Riding is a camera mode, not a state: see camera.rs)

Pressing / Resting / Down
 ├─ Move beyond SLOP before BEAT ──▶ Orbit          emits Unpress (if Pressing); the ride is left
 ├─ Up before BEAT, within SLOP ───▶ Idle           Pressing: Unpress (a tap: the ring alone)
 │                                                  Resting: Ride{id}
 │                                                  Down: nothing
 │     a second Up within DOUBLE_TAP of the first ─▶ a flight in about the point (camera), still Idle
 ├─ tick past BEAT (still within SLOP)
 │     Pressing ───────────────────▶ Holding{id,from,carried:false}   emits Put{at}; haptic
 │     Resting ────────────────────▶ Holding{id,from,carried:false}   emits Hold{id}; haptic
 │     Down ───────────────────────▶ Down                             (nothing, ever)
 ├─ second finger Down ────────────▶ Pinch           Pressing: Unpress; no float is put
 └─ Cancel ────────────────────────▶ Idle            Pressing: Unpress

Holding{id}
 ├─ Move ──────────────────────────▶ Holding (carried once past SLOP)  emits Carry{id, to: Hand of the point}
 ├─ Up ────────────────────────────▶ Idle
 │     not carried ────────────────── Release{id, at: None}            (nothing changed; maybes fold)
 │     carried, over Sea ──────────── Release{id, at: Some(point)}     (undoable)
 │     carried, over Land ─────────── Release{id, at: None}            (back where it was lifted)
 │     carried, Off the disc ──────── TakeOut{id}                      (undoable)
 ├─ Cancel ────────────────────────▶ Idle            Release{id, at: None}
 └─ second finger Down ────────────▶ Holding         (ignored: the float is in hand)

Orbit{last, samples}
 ├─ Move ──────────────────────────▶ Orbit           camera.turn(by the finger's move); Camera
 ├─ Up ────────────────────────────▶ Idle            fling from the last 80 ms (none under Reduce Motion)
 ├─ second finger Down ────────────▶ Pinch
 └─ Cancel ────────────────────────▶ Idle            no fling

Pinch{a, b}
 ├─ Move ──────────────────────────▶ Pinch           zoom by the distance's ratio, clamped; the point under
 │                                                  the fingers' midpoint stays under it (two passes)
 ├─ one finger Up ─────────────────▶ Orbit           with the remaining finger, anchored where it is now
 ├─ third finger Down ─────────────▶ Three{x0}       the pinch ends where it is
 └─ Cancel ────────────────────────▶ Idle

Three{x0}
 ├─ the fingers' mean x moves ≥ SWIPE left ─▶ emits Undo once; right ─▶ Redo once
 └─ all Up / Cancel ───────────────▶ Idle

CreditsDown{x0,y0}          (Down on the credit line; or Down anywhere but the date while the page is open)
 ├─ Move beyond SLOP ──────────────▶ Orbit           a drag is always the globe's
 ├─ Up ────────────────────────────▶ Idle            emits ToggleCredits
 └─ second finger Down ────────────▶ Pinch

DateDown{x0}
 ├─ Move beyond SLOP ──────────────▶ Scrubbing       emits Play(false) once, then Scrub{day} by dx
 ├─ Up within SLOP ────────────────▶ Idle            emits Play(!playing)   (the probe says if playing)
 ├─ second finger Down ────────────▶ DateDown        (ignored)
 └─ Cancel ────────────────────────▶ Idle
```

A ride (`CameraControl::riding`): `Ride{float}` makes the camera's centre spring toward the float
every frame (critically damped, `RIDE_TAU`); zoom is untouched. Any orbit or pinch leaves it (the
camera clears `riding` itself); a tap on the ridden float changes nothing; a double tap flies in and
the ride continues at the new zoom.

## Plans

`plans/*.json` are the played sessions, one a gesture and an ambiguous case, written by
`tools/adrift/touch-plans.py` with the expectations that prove each (floats, places, the hand's
state, the intents seen, the camera). `adrift-check -- play` runs them on the real globe and fails
on a failed expectation; `tools/adrift/touch-play.sh` plays them all. Their land is real land.

## Every ambiguous case, resolved

1. **A drag that begins on a float.** Before the beat it is the globe's: the float stays, the globe
   turns under the finger (Terra's `hold` rule). After the beat it is a carry. There is no slop
   race: the beat decides, and a rest is never mistaken for a drag nor a drag for a rest. Proved by
   `plans/orbit-from-float.json` and `plans/carry.json`.
2. **A rest that comes after moving.** Once Orbit, always Orbit: a finger that drags and then rests
   puts nothing. (`plans/orbit-then-rest.json`)
3. **A lift just short of the beat** (say 0.35 s) is a tap: the ring lets go, no float. The
   thickening curve (`motion::thicken`) is eased in so the first 150 ms look like a plain ring and
   no half-float is ever seen. (`plans/almost.json`)
4. **Where is the float put**: at the Down point (the ring's centre), not the finger's point at the
   beat; the drift within SLOP is tremor, not intent.
5. **The sea under ice** is sea: a float may be put there; Truth's rules hold it till the ice goes.
6. **Land and off the disc** are silent to every one-finger touch except the drag, which is the
   globe's. No ring on land.
7. **Tap near two floats**: the nearest within reach (`motion::REACH`, 22 reading points, a
   fingertip, for a thing drawn smaller than one; the app's `hit` reads it). Beyond reach it is
   the sea.
8. **A tap during a fling or flight** stops the motion first (Terra's `touched()`), then is a tap
   (the sea rings). A touch always stops the camera's own motion at once.
9. **Double tap**: zoom ×2 about the point (toward it below zoom 4, about the centre above, as
   Terra), out to 1 from past half the deepest; 0.7 s eased, a cut under Reduce Motion. Its first
   tap rang the sea, which changes nothing, so a double tap costs nothing.
10. **A second finger** before the beat ends the press (no float) and begins a pinch. During a hold
    or carry it is ignored: the float is in hand and the globe does not move under it. A third
    finger ends a pinch and waits for a swipe (undo/redo).
11. **One finger lifts from a pinch**: the drag goes on with the other, anchored where it is now, so
    nothing jumps.
12. **A carry released over land** goes back where it was lifted; over sea it is set down; off the
    disc it is taken out; "in plain sight" waits on a `Carry` that can say "over the rim" (D1): until
    then the float stays where it was last over water while the hand is off the disc. Back over the
    disc before lifting and it is in hand again.
13. **Cancel** (the system takes the touch) undoes the touch in progress and nothing else: a press
    lets go (the ring already rang), a hold or carry puts the float back, an orbit stops without a
    fling.
14. **Riding and holding another float**: the hold shows its maybes while the camera goes on
    following; a carry (a drag) leaves the ride first, so the globe is still under the hand.
15. **The date**: the drawn date under the globe is the control (Visual draws it; the probe
    reports its bounds). Any drag from it scrubs by its horizontal component (`DAYS_PER_POINT`: a year across
    the screen's width, never less than a day a point); the first move pauses the clock, and it stays
    paused after: the reader stopped time. A tap toggles pause and play. The sea pauses with the
    date because the clock does (Engineering's `Clock`), not because the look is told.
16. **Undo granularity**: what changed the sea: a put, a carried release, a float taken out
    (`App`'s undo stack). Ring, Ride, a hold without a carry, the camera and the date change nothing
    and are not undone.
17. **Reduce Motion**: no fling (the globe stops where it is left), flights and the ride's catch-up
    are cuts, the idle turn is off. The beat and the ring are unchanged (they are the gesture, not
    decoration).
18. **The idle turn**: after 8 s untouched, with no float on the sea and no ride, the globe turns as
    the Earth does, 4°/s. With a float put, it holds still: a float you are watching must not be
    carried out of view.

19. **The credits**: the credit line along the foot is the one other thing drawn that takes a touch
    (D5): a tap on it opens the page; while the page is open, a tap anywhere but the date closes it
    and nothing else happens (no ring, no float: the beat waits for the page to go), a drag is the
    globe's and a drag on the date scrubs, as ever. `plans/credits.json`.

## The numbers (`motion.rs`)

| | | why |
| --- | --- | --- |
| `BEAT` | 0.40 s | longer than a slow tap (~0.25 s), shorter than iOS's long press (0.5), so a rest reads as deliberate and a tap never puts |
| `SLOP` | 8 reading pt | Terra's hold scheme; a resting fingertip strays this far |
| `REACH` | 22 reading pt | a fingertip, for a thing drawn smaller than one (the app's `hit` reads it) |
| `DOUBLE_TAP` | 0.30 s, 40 pt | iOS |
| `FLING_DECAY` | 3.2 /s | Terra's; stops under 2 pt/s |
| `FLIGHT` | 0.70 s ease-in-out cubic | Terra's |
| `RIDE_TAU` | 0.6 s critically damped | catches up in about a second without overshoot |
| `IDLE_AFTER`, `IDLE_RATE` | 8 s, 4 °/s | Terra's |
| `ZOOM` | 0.8…8 | Terra's; the look may ask deeper |
| `LAT_LIMIT` | ±80° | Terra's: the pole never flips |
| `SWIPE` | 40 pt | three-finger undo |
| `DAYS_PER_POINT` | 365 / width, ≥ 1 | a year across the screen |
| `DATE_BELOW_DISC`, `DATE_FOOT`, `DATE_BAND` | 40, 72, 44 pt | the stand-in date band: under the disc, above the foot, its height |
