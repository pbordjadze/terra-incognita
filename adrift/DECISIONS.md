# Adrift: decisions

Engineering's rulings. Every craft reads this at the start of each round; a ruling stands until a
later entry changes it. Plans and budgets are in painting-company row 196, not here.

## 1. What Adrift is (R0)

One sea, one globe, glass floats the reader puts on it. Written from scratch in Rust on wgpu: no
engine, a small runtime grown as Adrift needs it. Its own app in Terra's SideStore source (bundle id
`com.pbordjadze.adrift`) and a web build (WebGPU) opened by link. Terra's Swift app is untouched.
Terra's ground, ocean and clouds and its data tools are to learn from and borrow (the ocean sheets
and their decoding are the first borrowing), never to copy whole.

## 2. Module boundaries (R0)

One library crate, `adrift/crates/adrift-core`, and thin shells. Each directory has one owner; a
change outside your directory is a demand (DEMANDS.md), unless Engineering made it.

| Path | Owner | Holds |
| --- | --- | --- |
| `adrift-core/src/lib.rs`, `app.rs` | Engineering | `App`: the state (floats, clock, camera, undo), `frame()`, `pointer()`, applying intents |
| `adrift-core/src/model.rs` | Engineering | shared types: `LonLat`, `Float`, `FloatState`, `Wake`, `Clock`, `Camera`, `PointerEvent`, `Intent`. Changed only by a ruling here |
| `adrift-core/src/gpu/` | Engineering | wgpu device, the `View` uniform, textures from `adrift/data`, the offscreen renderer the checks use |
| `adrift-core/src/sea/` | Truth | decoding the data sheets; `Rules`: where a float goes in a step (current, eddy wander, leeway, coast, ice), deterministic for a seed; the maybes' sampler; the drifters' tracks; `rules.wgsl`, the GPU mirror of `Rules::step` for thousands of maybes |
| `adrift-core/src/look/` | Visual | every drawing shader (`*.wgsl`) and the Rust that owns its pipeline: the globe's ground, the sea, the floats, the lamp, the wakes, the maybes, the nearest drifter, the date and credits as drawn |
| `adrift-core/src/touch/` | Motion | `Gestures`: pointer events to intents (ring, put, hold, carry, release, off the rim, undo, ride, the globe's own drag, pinch, fling, the date); `camera.rs` (orbit, inertia, riding a float); `motion.rs` (curves, springs, the beat's lengths); Reduce Motion |
| `adrift/crates/adrift-check` | Engineering | headless Linux renders and scripted sessions to PNG; every craft judges its work in it |
| `adrift/crates/adrift-web`, `adrift/ios/` | Engineering | the shells |
| `adrift/data/`, `tools/adrift/data/`, `adrift/SOURCES.md` | Truth | the built sheets the app loads, their builders, and what is measured against what is drawn, with credits |
| `.github/workflows/adrift.yml`, `tools/adrift/*.sh` | Engineering | CI, setup |

Crafts work on branches `adrift-truth`, `adrift-visual`, `adrift-motion`, each rebased on `adrift`
at a round's start; Engineering alone merges into `adrift` and, at the end, into `main`.

## 3. The seams (R0)

- **Positions** are `LonLat` in degrees, WGS84 as the data has it; the globe is a unit sphere. A
  position's unit vector is `LonLat::to_xyz()` (x toward lon 0, y north, z toward lon 90... see
  model.rs: the same frame as Terra's `Globe.metal`: `lat = asin(g.y)`, `lon = atan2(g.x, g.z)`).
- **Time** is `Clock.day`, days as f64 since 2000-01-01 00:00 UTC. The sea's month comes from the
  day of the year, interpolated between months as Terra does (the climatology is a typical year,
  not a given one; Truth says how the date is to be read, and the look says it that way).
- **The step.** `sea::Rules::step(&self, at: LonLat, day: f64, dt_days: f64, seed: u64, k: u32) -> Step`
  returns the new position and what held it (`Held::Coast`, `Held::Ice`, or free). The same
  (at, day, dt, seed, k) always gives the same answer, on the CPU and in `rules.wgsl`; a float's
  real path is `k = 0`, its maybes `k = 1..n`.
- **Intents** (`model::Intent`) are the only way touch reaches the state: Motion produces them from
  pointer events, Engineering applies them, Visual draws the result and never reads pointers.
- **The `View` uniform** (`gpu/view.rs`) is the one every drawing shader takes first: frame
  (width, height, pixels a point, reading scale), disc (centre x, y, radius, zoom), rotation
  columns, time (seconds, month a, month b, mix): Terra's `OceanUniforms` layout, so its shaders
  port line for line.
- **Draw order**: ground, sea, wakes, maybes, drifter, floats, lamp, chrome. Each look module is a
  `Pass` with `prepare(&View, &State)` and `draw(&mut RenderPass)`; Engineering calls them in order.
- **The checks** (`adrift-check`): `cargo run -p adrift-check -- render scene.json out.png` draws a
  scene (camera, date, floats, held one) at a size; `-- play plan.json out/` runs a plan of pointer
  events and dates over time and screenshots its moments. A craft's claim is a PNG from these or a
  device capture, not a description.

## 4. The cascade (R0)

A craft never lowers its own bar for a trade-off; it files a demand in DEMANDS.md on the craft that
could remove it, in the form there. A received demand is the receiver's next goal, answered in range
or with a counter-range, never a bare no. At most two new demands a craft a round. A demand that
comes round unchanged twice is ruled on here. Every round's build is tagged `adrift-r<n>` and kept;
the best ships.

## 5. Rounds (R0)

- **R0** (Engineering): the skeleton that builds on Linux, the web and the device: the ground, Terra's
  sea ported, one float drifting by the current alone, the checks, CI, the shells. Crafts: their
  works and critics, round-1 plans, and (Truth) the data that doesn't wait on the skeleton.
- **R1**: each craft lands its module on the skeleton; critics judge the running build; demands.
- **R2**: demands answered; the device build judged; polish.
- **R3** if the budget allows: the best round shipped.

## 6. R0 landed; what Engineering carries into R1 (R0, Oct 31)

The skeleton is on `adrift`: the ground and sea ported, a float on the mean current with a bead a
day, plain gestures, a text pass (`gpu/text.rs`: a glyph atlas from Libertinus at three sizes; a
look lays out lines with `Text::line` and `TextPass` draws them; the atlas is Engineering's, what is
written and where is Visual's), the headless checks, the web shell, the device shell built by script,
and `adrift.yml`. Known gaps that are Engineering's, in R1:

- **The frame's budget.** The sea is drawn at the screen's every pixel every frame; Terra draws it at
  a fraction and ten to thirty times a second at rest. R1: the ground and sea drawn to a texture at a
  scale the frame time sets (full when it affords, down to half), composited under the sprites, and
  redrawn only when the camera, the date or the sea's clock moved. The target: 120 Hz on the
  device while a finger moves, and a frame under 4 ms at rest.
- **The maybes on the GPU.** Truth's `rules.wgsl` runs as a compute pass over N seeds from a held
  float's put day to now, writing positions the sprite pass reads; N by the budget (thousands).
- **Nothing is lost.** The floats, the date and the camera are kept across launches (the device's
  documents, the web's local storage), written when the app goes to the background; no account, no
  network.
- **Undo** is an inverse per touch (put, move, take out) so nothing else moves; it keeps a short
  history. How it is reached is Motion's.
- **The web link** is published by Engineering from each round's build (a private claude.ai artifact;
  the owner shares it); CI keeps the same build as the `adrift-web` artifact.
- **Text on the globe** needs a ground under it (CLAUDE.md: every name on a halo); the foot's words
  are drawn plain in R0 and lie over the sea at high zoom. Visual's, with the text pass as it is or
  extended (ask).

## 7. R1 merged; R2 opens (Oct 31)

All three heads are in `adrift`: Truth's sheets and rules (the step is a *flight*, the eddy velocity
carried on the `Float`), Visual's `Look` (the app fills a `LookState`; the R0 ground and sea stay
until Visual takes them), Motion's vocabulary over a `Probe` (touch/probe.rs answers it from the
app; `CameraControl` owns the camera's motion). Rulings from the merge:

- **Integrate on `adrift` only.** A craft's branch starts from the `adrift` Engineering names when a
  round opens and never re-bases on its own; `app.rs`, `model.rs` and `assets.rs` are changed by
  demand, not by hand, so three parallel seams don't have to be reconciled again (this round's cost).
- **The integrator's small changes** in a craft's directory are allowed where a seam needs them (a
  buffer's usage, a decode, a layout) and are named in DEMANDS.md's answer; the craft keeps or redoes.
- **Plans.** `adrift-check -- play` takes both forms (a scene with screen points; Motion's camera,
  absolute `t`, fingers by id at a place or `xy`, `expect`s) and fails on a failed expectation.
  Motion's plans are the touch's tests on the real globe: their land must be real land.
- **The maybes** are Truth's `sea::Maybes`, seeded from the float's put place and stepped a quarter
  day at a time with the flight, drawn from the same buffer by Visual's pass.
- **R2 for Engineering:** the silver (nearest drifter) wired when Truth hands the track; CI's IPA
  green and the first SideStore build; the web link republished; the frame's budget measured on the
  device; `tools/adrift/lookbench` and `touch-standalone` retired as `adrift-check` covers them.

## 8. R2 closed; the first build on main; R3 (Oct 31, 09:05 UTC)

R2 landed from all three crafts (Truth: the put velocity, the drifter's window with its dates, no
maybe on land; Visual: the bloom from the float, the eased rest, the night with the Black Marble
lights, the silver with its label, the lamp on Put; Motion: Press/Unpress/Redo, the date band on the
drawn date, 24 plans passing on the real globe) and `adrift` was merged into `main` at 480529d: the
first Adrift build to the SideStore source is main's adrift.yml run 14. Rulings:

- **The device builds on every push of `adrift`** (two minutes of a Mac each), so the integration
  branch never needs a tag commit; and crafts' merges are batched between runs, since each push
  restarts the branch's one run.
- **R3** is judged against the device: the owner's captures (a fling toward the pole, the beat under
  a finger) decide Motion's remaining knobs; Visual's remaining knobs wait for the same captures; Truth
  sets the maybes' count and step the honesty needs, for the device budget. Each craft sends its
  exit-interview answers when R3 is done.

## 9. R3 closed and shipped (Oct 31, 09:25 UTC)

Main 5823c0d is the shipped round: every demand in DEMANDS.md answered (Truth's eddy memory and the
put velocity; Visual's Look wiring, eases, night and lights; Motion's press, redo and the credits'
tap; Truth's credits page and guess mark on Visual), 24 tests and 28 played plans green, the device
IPA published to the SideStore source beside Terra, the web link republished. What is parked on the
owner's device captures (a fling toward the pole; the beat under a real finger; the frame stats the
shells now log every five seconds): Motion's last knobs on the fling and the spring, Visual's on the
night's near-black and the specks' three pixels, and the frame budget's thresholds. Nothing further
is asked of the crafts; each has sent its exit interview.

Engineering's own account, for the record: the spiral's cost was one round of parallel integration
(three crafts each built against a seam that then moved; the branch had to be rebuilt once); its
gain was that every ask between crafts arrived as a testable sentence with a range and an
observable, and all twelve were answered in range without a bare no. The one process change that
would have saved the most: the shared types, the plan form and the "add a field" procedure named on
day one, before the crafts began.

## 10. Closed (Oct 31, 09:45 UTC)

Main's run 23 published adrift-build-23 and the two-app source at 09:29; the close was reported.
Nothing is open on the crafts. What a next session does first, from all four accounts: write
`model.rs`, the check's plan form and the "add a field" procedure (a `LookState` field is Visual's
to add and `app.rs` fills it; a `Probe` question is Motion's; a `Rules` method is Truth's) before
any craft starts; let the check take a look's state (`--set look.field=value`) so a craft photographs
a state the app can't yet reach without editing `app.rs`; let the check relink assets itself when
the manifest changes; and either give the integrator force-with-lease on the integration branch or
rule that the integrator merges and never rebuilds. The device's verdict is the one thing this
session could not give.

## 11. R4 opens: the first device verdict (Oct 31, 14:35 UTC)

The owner ran Adrift on a phone (painting-company row 196 has the words). In short: it opens fast and
runs well, the night reads, and five things stand between the screen and the hand. The water's
glitter at night draws the eye away from the floats, where living lights on the land would not. The
date control isn't obvious the first time, and nothing else on the screen says it can be changed.
No way was found to clear the sea of floats or to bring the globe home. Holding a float to see its
maybes doesn't yet come to the hand. And floats were put on the Black and Caspian seas and the
Baltic, where the record may not speak. These are the round's goals, not a spec: each becomes a
demand on the craft that owns it, in the cascade's form, and the Holt critic judges the answer
(undoable, no penalty, nothing marks or points or waits).

**Who owns which goal.**
- *Motion*: a way to clear the sea and to bring the globe home that a hand finds without being told
  (`Intent::ClearAll` and `Intent::Home` exist as of this head: one touch, undo gives every float
  back, the home flight uses the camera's own ease; what gesture reaches them is Motion's); the date
  control obvious the first time (what a hand expects of the band, with Visual for its look); picking
  a float up to see its maybes made natural (what does a hand expect: a tap that shows them a while,
  a rest that lifts?). `App::hit`'s reach is still Motion's number.
- *Visual*: the water's glitter quieted where it competes with the floats, and the land's lights
  alive by night instead (the ground pass has the view's `time`, and the stage redraws every tick
  while the sea moves, so this is a shader's change); the date band's look saying it can be moved and
  whether the date runs or rests; whatever affordance Motion's new gestures need drawn.
- *Truth*: where a float may be put. The Black and Caspian seas, the Baltic and the ice's edge: does
  the record speak there, and if not, does the sea hold the float, refuse it, or carry it as a marked
  guess? A float on land must be impossible (`Hit::Land`); check the coast mask against the sheets'
  extent and say in SOURCES.md what the answer rests on.
- *Engineering*: the seams below, the check's asset relinking (it now links what the manifest names
  and the directory lacks), the scene's `credits_open`, integration and the build.

**The "add a field" procedure (from every exit interview).**
- A `LookState` field is Visual's: add it in `look/mod.rs` and fill it in `App::look_state()`, which
  is Visual's to edit; nothing else in `app.rs` is.
- A `Probe` question is Motion's: add it to the trait in `touch/gestures.rs` and answer it in
  `touch/probe.rs` from the app's public state; a private field it needs is a demand.
- A `Rules` method or a sheet is Truth's, in `sea/`; its call in `app.rs` is a demand or an
  invitation.
- A new `Intent`, a `Scene` or plan field is Engineering's: ask by demand; it is answered within the
  round's first hour.
- Branch from the head named in the round's message; the integrator merges and never rebuilds.

**The round's build** is whatever stands on `adrift` when the three heads are merged and the plans
pass; it ships to the SideStore source as the next `adrift-build-<n>` and the web link's next
version.

**Truth underneath, not on top (ruled 14:50 UTC, from the owner's second word on the round;
painting-company row 196).** The sea's rules stay honest; the screen stops saying so. This round: the
credits line leaves the foot (the credits page stays one tap away: Motion and Visual say where the
tap is; failing a better answer, a rest on the date opens it); the note under the date leaves; the
guess mark goes, or is all but invisible (Visual's call, off by default); a float may be put wherever
the coast mask says water, and the Black, the Caspian and the Baltic need only behave plausibly, not
be ruled on. Truth's round is the sea feeling alive and right to the hand and eye (the wake, the
wander, the speeds as a finger reads them), not its annotation; SOURCES.md keeps the whole account.
Whales, if they come, by rules that feel true before strict telemetry. Delight first.

## 12. R5 opens: the second device verdict (Oct 31, 17:00 UTC)

The owner on build 30 (painting-company row 196 has the words): the maybes are the heart of it, and
the round is built round them. His asks, in his order: the date scroller buttery smooth; rewinding
time brings the lamps back to where they were; a scrub leaves the date running if it was running,
with a gentle sign of running or resting; the trails went wrong (wakes that zigzag and double back,
their heads apart from any lamp); each lamp its own colour, its maybes in the same, so each cloud
reads as its lamp's; and once a lamp is chosen and its maybes shown, zooming out must not let it go.
His screenshot also showed heavy white bands across the sea close in by day and faint grey rings on
the water.

**Who owns which goal.**
- *Engineering*: the scrub's cost (measured in the check: CPU per frame while scrubbing with thirty
  floats; the maybes stepped on from where they are with a budget of steps a frame instead of
  re-simulated from the put, the floats' catch-up budgeted); time reversible, which is the trail bug
  too: a float's place is a function of its put (place, day, seed) and the date, on a fixed
  quarter-day grid from the put, so the rewind restores it and its wake is read off the same path;
  a carry starts a new leg; a float before its put is not there; the choice (`Intent::Choose`,
  `Unchoose`, `App.chosen`, set by `Ride`, cleared by a tap on the sea or the float's going).
- *Motion*: the scrub leaves the clock as it found it (running stays running); zooming or panning
  away keeps the choice and ends only the ride; whatever else the hand expects of a choice.
- *Visual*: a hue per lamp (`LookFloat.id`) carried by its maybes, wake and silver so each cloud is
  its lamp's; the gentle sign of running or resting at the date; the screenshot's white bands and grey
  rings found and quieted (the glitter or the streamlines' light close in; old rings or the cloud's
  ghosts); whatever the chosen state needs drawn (`LookFloat.chosen`).
- *Truth*: the flight a pure function of (state, day, dt, seed, k) on the fixed grid, stated and
  tested, so a replayed path is the same path; the sampler's cost per call (the scrub's budget is
  spent there); `Maybes` able to keep and restore a checkpoint of its paths so a rewind of the cloud
  is a restore and a few steps, not a re-simulation from the put.

Seams at the head named in the round's message: `Intent::Choose`/`Unchoose`, `App.chosen`,
`LookFloat.id`/`chosen`, the check's `chosen` expectation. The path grid and the maybes' budget
follow on `adrift` within the hour; the crafts need not wait for them.

## 13. R6 opens: the third device verdict (Oct 31, 19:15 UTC)

The owner on build 40 (painting-company row 196 has the words): the best thing yet, and five ideas.
In short: the night's lights don't distract, which is right, but could be lovely enough to be gazed
at for themselves, without distracting; a lamp's maybes, small warm dots, are hard to tell from the
cities' lights at night (one problem with the last: they share a colour and a size, so one hand
on both); dropping a lamp and scrubbing fast through time is the joy, so the scrub must be quick
enough that going back to look is free, and a speed limit fits, since every maybe is a seeded
flight and a scrub back replays the same cloud; two lamps dropped near each other: can even their
clouds diverge, several chosen at once and scrubbed together, the differences seen in a way as
natural as the rest (and the clouds diverge because the currents make them, not because the drawing
says so); and a lamp at the globe's edge is clumsy beside the rest: a very small panel off the globe
of every lamp out there, and on the globe only the possibilities, each equal (the lamp is maybe
k = 0 underneath, the same rules with its own seed, so it may be drawn as one possibility among the
rest, and the panel holds where it was dropped). These are goals, not specs: creative answers are
welcome, trimming freely is welcome, truth stays underneath.

**Who owns which goal.**
- *Engineering*: several clouds out at once and the scrub's cost with them (the seams below);
  integration, the build, the measure.
- *Visual*: the lights and the maybes as one problem (the night's lights beautiful enough to be the
  centrepiece without distracting; a maybe never read as a city); the lamp as one possibility among
  its maybes, each equal; the panel's look; several clouds at once and their differences seen
  naturally; whatever the edge needs.
- *Motion*: the panel's touch (choosing one, two or ten lamps; letting one go; where the hand finds
  it), the scrub's speed limit and its feel, the edge's behaviour; the gestures for `ChooseAlso`.
- *Truth*: whether two nearby lamps' clouds can diverge as the sea would make them (a shared eddy
  field a maybe's index k lives in, a world per k, would let lamp A's k-th and lamp B's k-th meet
  the same sea; or any better idea); the step's cost as several clouds run at once.

**The seams (Engineering, at the head named in the round's message).**
- `App.chosen` is a `Vec<u32>`, the front one last. `Intent::Choose { float }` makes it the only
  one; `Intent::ChooseAlso { float }` adds it in front, or lets it go if it is chosen; `Unchoose`
  lets all go; `Ride` adds its float in front (others stay); a float's going removes it.
- `App::shown_ids()`: every float whose cloud is out (the chosen, the ridden, the pressed, the held;
  the front last; at most `MAX_CLOUDS` = 8). Each has its own range of Truth's one buffer
  (`MAYBES` = 2048 a cloud, `base = slot * MAYBES`), its own keeps and bloom, stepped
  `MAYBES_BUDGET` = 24 half-days a frame each. `Params.base` and `Maybes::seed_at/keep_at` carry
  the range (k is still 1..=count within it).
- `LookState.clouds: Vec<LookCloud { id, at, origin, base, count, bloom, front }>`;
  `LookState.behind` (the days the slowest cloud trails the date: a speed limit can read it);
  `LookFloat.shown`, `origin` (its first leg's place: where it started) and `put_day`. The maybes
  pass draws each cloud in its hue from its own uniform (`maybes::CloudDraw`); the veil and the
  silver stay the front one's.
- The panel: `Look::panel_bounds()` (Visual's, empty until drawn; `Words.panel_bounds` holds it) and
  `App::panel_bounds()` for Motion's probe (a `Hit` for it is Motion's to add in `touch/`).
- The check: a scene's `choose: [index, ...]`; a plan's `expect.chosen` takes an id (the front), a
  bool, null, or the whole set as an array.

**Added to the round (19:35 UTC; painting-company row 196).** Three more of the owner's notes, given
just after the round opened: the touch's ripples are a little too intense to the eye (Visual); the
poles are odd: near one, small gestures make big jumps, and nothing here needs a north and a south,
so why keep the constraint unless it serves (Motion: the camera free to turn over a pole like any
orb; Engineering's seam below); and second thoughts on taking the lamps off the globe: watching a
few lamps' own journeys (two dropped close together and watched) is part of the pleasure too, so
both are wanted, a lamp's journey to follow and a lamp equal among its possibilities, and the build
is to show him (Visual and Motion).

- *Seam (Engineering):* `Camera.roll` (degrees, serde default 0) joins lon, lat and zoom, and
  `rotation()` is Ry(lon) · Rx(-lat) · Rz(roll), so any orientation is a camera and every shader
  follows it unchanged (they read the rotation's columns). `Camera::from_rotation(m, zoom)` reads a
  rotation back (exact at a pole), `Camera::turned(axis, angle)` turns the globe about a view-space
  axis (a trackball: a drag of dx, dy over a disc of radius r is the axis (-dy, dx, 0) and the
  angle hypot / r), `Camera::new(lon, lat, zoom)`. Tested: a drag carries the centre over the pole
  with no jump, every step the same arc. `CameraControl`'s orbit, fling, fly and ride stay
  lon/lat and are Motion's to move onto it.

## 14. R6 merged (Oct 31, 20:50 UTC)

All three crafts are on `adrift` (Visual 35f1868, Motion a75e160, Truth b9328f5), with every test
and plan green. What the round built, in short:

- *Truth*: **the worlds.** A float's k-th self lives in world k: one random eddy field on a 1°
  lattice, fresh each quarter day, scaled by the measured variance and time scale, shared by every
  float's k-th path; the lamp is k = 0, world 0. Two lamps put near each other meet the same sea, so
  their k-th selves stay together in quiet water and part by hundreds of km in a jet; as whole
  clouds they differ little (SOURCES.md › The worlds has the numbers). The lamp ranks uniformly
  within its own cloud. Same cost a step. The seed no longer enters the flight: a lamp put twice at
  one place and day takes one path.
- *Visual*: by night the cities glow by their brightness from the sheet's coarser mips and the
  maybes are cool glass rings with a spark of their lamp's hue, never warm points; a lamp is its
  glass float with its wake until its cloud is out, then one speck among its maybes; the panel (top
  left, a lamp in its hue a row, its put day and place); several clouds interleaved, and a pair of
  same-world selves from two lamps drawn as one ringed speck until the currents part them; the rim
  fades by facing; gentler rings at a tap.
- *Motion*: the panel's touch (a tap adds or lets go, a rest rides; it never rings or puts); a
  scrub's speed limit (600 days a second, braked while a cloud trails by more than six days, landing
  exactly); a lamp chosen from the panel turned in from the rim; the orb free over the poles (a drag
  keeps the grabbed point under the finger; flights turn the whole orientation; Home upright).
- *Engineering*: several clouds out at once (up to 8 of 2048, each its own range, keeps and bloom);
  the chosen set; the camera's roll; the check's `choose`, `panel`, `frames` and its "behind" line;
  the panel's rows 36 pt tall at the merge (a fingertip; Visual's 24 was under a thumb).

For the next round, from the exit interviews: name each craft's peers in its brief; let a plan's
expectation be "the same as at shot X" so a rules change doesn't break a rewind test; make `frames`
the check's default and warn on out-of-order step times or a tap that does nothing; give the panel
a minimum touch height in the seam; a density outline per cloud (an offscreen pass) and a seam for
each cloud's extent (to frame them all) are the next tools if differences must read stronger.

## 15. R7 and R8 (Oct 31, 22:20 UTC)

**R7** (shipped alone): a pinch rings the sea under both fingers; the second finger that turns a
touch into a pinch rings the water where it lands, as the first did (painting-company row 196).

**R8 opens: each lamp its own way.** The owner on build 41 (painting-company row 196 has the words):
several ways to be curious (two lamps watched as themselves, four lamps, three clouds), so each
lamp may be chosen as itself (its journey) or as its cloud (its maybes), mixed freely, at any zoom,
picked intuitively without adding much interface; and, offered the idea, the globe turning and
zooming to hold everything chosen at once (a single lamp is already followed). Goals, not specs.

- *Motion*: how a hand picks lamp or cloud for each lamp, on the globe and on the panel, and lets
  it go, with little or no new interface; the camera framing everything chosen, without fighting a
  hand that is moving it.
- *Visual*: a lamp chosen as itself made visibly chosen (its journey worth watching) without a
  cloud; the panel saying which way each is chosen; whatever Motion's gestures need drawn.

**The seams (Engineering, at the head named in the round's message).**
- `model::Shown { Lamp, Cloud }`; `Intent::Show { float, shown: Option<Shown> }` sets how a float is
  chosen (as itself, as its cloud, or not) and brings it to the front. `App.followed: Vec<u32>`
  (chosen as themselves) beside `App.chosen` (as clouds); a float is in one or neither.
  `App::shown_as(id)`. `ChooseAlso` and `Choose` still toggle and set clouds; `Unchoose` lets every
  choice go; a float's going removes it from both.
- `App::frame_points()`: the places to hold in view, each followed lamp's and each chosen cloud's
  lamp and `FRAME_SAMPLES` (16) of its maybes, stepped on the CPU (same rules, same worlds) so the
  camera can frame the clouds without reading the GPU back.
- `LookFloat.followed`. The check: a scene's `follow: [i, ...]`; `expect.followed` (an array);
  `expect.framed: n` (all of `frame_points` on the screen's face, at least n of them);
  `plans/mixed-choice.json`.
