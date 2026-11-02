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

**R8 merged (Nov 1, 00:05 UTC).** Visual 2dbad3b, Motion c4f42fe.
- *Motion*: one touch, three ways, on the globe and on the panel alike: a tap on a lamp brings its
  cloud out (and follows it), a second tap folds the cloud and chooses the lamp as itself, a third
  lets it go; a tap on the open sea lets all go; a double tap only flies in. While following, the
  camera turns and zooms out to hold everything chosen (`frame_points`), never deeper than where the
  follow began, and a drag or pinch takes the camera back with the choices kept. A two-finger twist
  rolls the orb about the place under the fingers' midpoint while the spread zooms.
- *Visual*: a lamp chosen as itself is its lit glass float ringed in its hue with its whole kept
  journey drawn back to its put; the panel marks lamp (a ring), cloud (a glow) or neither (a dot);
  the panel sits on a quiet card over the globe.
- For next time, from the exit interviews: send a round's later goals as a numbered list in one
  message (a craft read a late goal an hour late); a `play-all` that runs every plan in one process
  with one summary line (the suite now takes over ten minutes); the framing's centre by a minimum
  enclosing circle rather than the mean.

## 16. R9 opens: the studio, step 1 (Nov 1, 07:30 UTC)

A new direction (painting-company row 199): Adrift becomes a studio for making short edits of these
lines (stochasticity as where the beauty is, a little structure dropped in), and the ocean of lamps
is its first preset. Step 1 is the scaffolding: a light recording mode, the first tools any preset
will share, a timing or two. Later steps add presets, and what differs between two presets becomes
configurable. The ocean may change where step 1 needs it to.

Every session that has worked on Adrift independently proposed the same foundation, and it is the
round's seam: **a take is data, not pixels.**

- *Engineering* (`studio.rs`, at the head named in the round's message): a `Take` is the state when
  it began (`TakeStart`: floats, clock, camera, chosen and followed, the ride, the drawn seconds,
  night) and every input the shell fed the app while it ran (each frame's time, touch, wheel, key,
  shake, and intents applied from outside through `App::command`), with the screen it was recorded
  on and an optional `trim`. `Intent::Record { on }` begins and ends one (beginning resets the live
  camera's inertia, the gestures and the ripples, so live and replay agree from the first frame);
  `Intent::Export` asks the shell to export the newest. `App::export_begin/frame/end` replay a take
  offline at a fixed rate with the budget locked at full quality, each frame rendered into an
  offscreen target and returned as BGRA, and put the live app back after; `export_drift` says how
  far the replay ended from the take (0 km in the check). `LookState.recording` (seconds),
  `.exporting` (this frame goes into the clip), `.takes`. The device: `adrift_take_export`,
  `adrift_export_begin/frame/end`; the shell writes H.264 with AVAssetWriter and offers the file
  through the share sheet (Save Video among its actions). The check: a plan's `record: true|false`
  and `export: "<name>"` (ffmpeg to `<name>.mp4`, its first and last frames, a drift check);
  `plans/take-and-export.json` (a rest, a tap, a scrub, a pinch: 172 frames, drift 0.000 km, the
  last frame identical to the live one).
- *Motion*: how a hand starts and stops a take and exports it, with nothing that gets in the way of
  playing; the first timing tool (a trim, a speed); whatever "minor selection tools" the hand needs.
- *Visual*: the exported frame clean (what of the chrome stays out of a clip), a recording's mark on
  the live screen only, whatever the export's control needs drawn.
- *Added (07:55 UTC):* the clip is 9:16 at 1080×1920 (`studio::CLIP`): the largest 9:16 window of
  the take's screen, centred on the globe's disc vertically, scaled by bilinear sampling. The owner
  has handed the run over (painting-company row 199): rounds continue as before, each build judged
  by a fresh critic session standing in for his notes, until it could be shown without a caveat;
  presets (wind, a flat plane, more), the dials that differ between them, characters, a conductor
  mode and the casting of fates are the road.

**R9 merged (Nov 1, 08:30 UTC).** Motion 981667a, Visual 8dd1493. One control at the top right
records: a tap begins a take; while recording, a touch ends it and exports at once (the touch that
stops is not in the clip, which ends on the last thing the hand meant); a rest on it re-exports the
newest take. The clip is clean (no panel, no date band, no credits, no control), the date a quiet
caption inside the 9:16 window. The live screen shows the control as a ring round a small lamp,
lit and breathing while recording, beads filling toward the take's minute. For next time, from the
exit interviews: one pure `clip_window` both the app and the look call; the clip's window in
`LookState`; a render of a scene "as exported" without replaying a take; a slow-motion export (the
replay's times scaled); the progress ring as an arc, not glyphs.

## 17. R10 opens: the clip made, not captured (Nov 1, 08:55 UTC)

A fresh critic, standing in for the owner, judged round 9's clip "just a screen recording": the
phone's view cropped (the globe cut, paper in the corners), a lurching pinch, a tap's ring on the
water in the last frame, an abrupt end, one journey a ribbon and another scattered beads; and the
live panel a column of dates and coordinates read before the ocean. Round 10 makes the clip a made
thing; the next preset's truth (the wind) starts beside it, so next round's preset seam is shaped by
something real.

- *Engineering* (b513cae): the clip is drawn at its own size (`studio::CLIP`, 1080×1920 at
  `CLIP_PPP` 3, i.e. 360×640 points) through a view of its own (`App::clip_view`, `render_view`),
  not cropped from the take's screen. Its camera follows the take's (`studio::follow`, a half-life,
  frame-rate independent, the zoom in its logarithm). Every take carries `Framing { hold_in, hold_out,
  follow }`: a still beat before the take's first moment and an eased one after its last, the date
  held and the sea's drawn seconds running (added to the drawn state for the frame only, so the
  replay ends exactly where the take did). `Take::clip_seconds`, `Take::clip_at`; the look's
  `opening`, `ending`, `clip`; the sea's rings fade with `ending`. `Take.speed` and
  `Intent::TakeSpeed` (a slow-motion clip: the replay's times scaled). `studio::clip_window` is one
  pure function. The check: `render --clip` (a scene as a clip's frame, no replay), plan expects
  `recording` and `takes`.
- *Visual*: every journey one continuous line; the panel cut to tight coloured marks tied to their
  lamps (its words to VoiceOver); the clip's look (touch feedback out by its end, the caption by
  `clip`, the last beat eased); the progress ring an arc.
- *Motion*: the clip's framing (this round it owns `Framing` and `follow`); the live pinch
  frame-rate independent; the panel marks' touch; whether a gesture earns the speed.
- *Truth*: the wind's rules (a step on the month's wind with measured variability, CPU and GPU), what
  differs between the two media written down as the first dials (`sea/MEDIA.md`), a picture of wind
  paths without the app.

**R10 merged (Nov 1, 09:20 UTC).** Visual 3944269, Motion df06f56, Truth 83db8af, and Engineering's
seams after them (a float's `trail` at every quarter-day step, the look's `panel_press`, the panel's
words for VoiceOver through `adrift_panel_count`/`adrift_panel_item`, the export's camera trace
`<name>-camera.csv`, `gusts.png` among the loaded files).

- *The clip*: drawn at 9:16 through `studio::ClipCamera` (Motion): a critically damped follow in
  rotation and log zoom, an opening beat of 0.8 s easing in from a tenth wider, an end beat of 1.2 s
  carrying the take's last drift on and settling (`Framing.open`, `.settle`). The press ring is never
  in a clip; a held lamp's ring and glow settle through the end beat; the caption sits by `clip`,
  quieter at the end. A live fling glides the same decayed distance at any frame rate; the plans
  `take-fling-pinch-20/-60` export the same clip from takes recorded at 20 and 60 fps.
- *Journeys*: every wake one tapering stroke through the quarter-day steps (the eddy's real stalls
  and loops, which Truth confirmed), round caps, no beads. The stroke tool (`look/wake.rs`: `line`,
  `smooth`) draws screen marks too.
- *The panel*: a row of marks top left, no words: each lamp's journey drawn small in its hue, ending
  in its lamp (a glow if its cloud is out, a ring if followed); 44-point cells. A tap rings the sea at
  the lamp; a rest swells the lamp. VoiceOver reads each mark's words.
- *The record control*: its progress an arc.
- *Speed*: no gesture yet (Motion, `touch/VOCABULARY.md` § 33): every candidate gives the record
  control or the date a hidden second meaning; it waits for an edit surface with the trim.
- *The wind* (Truth, not yet in the app): `sea/wind.rs` `AirRules` and `wind.wgsl` (`AirMaybes`, the
  GPU within 6 m of the CPU): the month's 10 m wind plus a gust remembered for about a day, its
  variance and time scale measured from five years of six-hourly reanalysis (`gusts.png`,
  `tools/adrift/data/weather.py`); the weather shared by a world on a 15°×8° lattice; things come
  down by rain-out or settling. Ten-day spreads against real paths: median ratio 1.04.
  `sea/MEDIA.md` sets out the two media as one machine (a mean field, a remembered wander shared by a
  world, stops) and proposes the studio's first dials: pace, wander, memory, kinship (cell and
  share), lifetime, holds, trail. `tools/adrift/data/wind_paths.py` draws wind paths without the app.

For next round, from the exit interviews: the preset seam from `MEDIA.md` (the wind preset wired,
its lifetime dial defaulting to about four times the measured life, which is short for a clip); a
tap's ring drawn above a lamp's cloud; `render` overrides (`--zoom`, `--day`) and committed baseline
renders for before/after; a small, fast export size for judging motion; an `adrift-check rules`
subcommand in place of ignored tests used as tools; a scene that shows an AirRules cloud in the real
look before wiring; `AirMaybes` stepping many at once with keeps; contact sheets of a clip in the
check's README (`ffmpeg -i clip.mp4 -vf "select=not(mod(n\,6)),scale=120:-1,tile=13x2" sheet.png`).

## 18. R11 opens: conducting, the studio's step 2 (Nov 1, 15:00 UTC)

Round 10's critic: the live app could be shown without a caveat; the clip could not ("it just
records what I did; there's no music, and the lamp wanders off"): most of a clip is the whole globe
with specks, a fling carries the lamp out of frame, the caption sits on the land. The Interviewer
chose conductor mode as step 2, from the spike (`adrift-conductor-spike`, `adrift/CONDUCTOR.md`):
individual lamps as lines, the possible seas flashed only at the cue, tap / sweep / circle / hold,
open water conducts all, a take is its cues, the sweep fired at its peak speed. The cues are the
clip's beat: the reader conducts to the music they hear, and the clip carries their timing.

- *Engineering* (bf3a53a): **the hand conducts while a take is recorded** (and while it is replayed
  for export): `App::conducting()`. Before and after a take the sea is the sea it was. A `Leg` names
  its `world` (0 the real one, `k` a maybe) and may carry its `eddy`, so a cue rewrites a lamp's
  journey since its last cue into the chosen world's path and continues from there under the same
  law. A take's start cuts every lamp's leg where it is (`cut_legs`), so the worlds a hand chooses
  among are short. `conduct.rs`: each lamp's 2048 worlds stepped on the CPU on its own path's grid
  (unbudgeted, so a replay steps alike), and the spike's chooser. `Intent::Conduct { float, gesture }`
  (`ConductGesture::{Tap, Sweep, Circle}` in screen points and seconds; `float: None` every lamp),
  undoable. The look's `conducting` and `cues`. `plans/conduct-take.json`.
- *Motion*: the conductor's hand (one finger conducts while conducting; two fingers move the
  camera; the fermata by a rest); the clip keeps the lamps in frame.
- *Visual*: the cue drawn (the seas flashed and folded into the chosen, the lamp gliding); the
  clip's date off the land; overlapping clouds kept apart by hue; the marks by night.
- *Truth*: `conduct.rs`'s worlds this round: honesty tested (the cue, the take's cut, the
  chooser), the cost on the phone, the tempo.

**R11 merged (Nov 1, 15:20 UTC).** Visual 4672625, Truth f750b14, Motion c58a55b, and Engineering's
tempo after them.

- *The hand* (Motion): only while `conducting()` (the probe asks the app, so an export's replay
  conducts alike). One finger: a tap is a Tap at its lift; a stroke is a Sweep fired mid-stroke when
  its speed (a 40 ms window) falls to 0.8 of its peak, at least 160 points a second; a loop of three
  quarters of a turn is a Circle; a rest of a beat is a fermata (the date held until the lift). The
  target is the lamp within 90 points of the touch's start, else every lamp. One finger no longer
  rings, puts, chooses or turns the globe while a take records; two fingers move the camera. The
  record control, the date and the marks are unchanged.
- *The clip keeps its lamps* (Motion): `studio::lean` before `ClipCamera`'s spring leans the clip's
  centre toward the lamps (the conducted first) as far as holds them within 0.72 of the half-width,
  and in toward specks; `take-fling-away.json` and `tools/adrift/lamp_in_clip.py`: the take loses its
  lamp in 115 of 160 frames, the clip in none.
- *The cue drawn* (Visual): all the worlds flash in the lamp's hue (the ones that did what the hand
  did in full colour), fold into the chosen over 0.15–0.8 s; the lamp glides from where it was; a ring
  where it lands; its rewritten line comes in as the worlds fold. The clip's date on the paper under
  the globe, or none when the globe fills the clip. Overlapping clouds keep their hues.
- *Honest and cheap* (Truth): cues and the take's cut land on the next quarter-day mark with the
  exact sample state, so a take's start changes no frame and a conducted lamp is its world to the
  bit (tests). 1024 worlds, stepped a share a frame (`Worlds::prepare`): six lamps ~0.6 ms a frame
  natively, an estimated 1.2–1.8 ms on the phone. Matches: sweep 14–19%, circle 13–16%.
  `adrift/CONDUCTOR.md` › In the app.
- *The tempo* (Engineering, on Truth's measure): `conduct::TEMPO`, 3 days a second while a take
  records; the free rate (2) comes back at its end.

For next round, from the exit interviews: the marks by night (the lit one reads as a city light); a
plan option for shots every N seconds with a crop, and a short gif of a cue at the speed it is felt;
the lamps' screen positions in the export's camera trace; the tempo as a dial in the clock or the
studio, not a constant; a GPU pass for the cue's worlds as short streaks folding in; circles pick a
world's stalls more than real loops (a spinning random flight would give real ones); one finger can
no longer pan during a take, by design (two fingers do).

## 19. R12 opens: conducting seen, and a song (Nov 1, 16:10 UTC)

Round 11's critic: the sweep reads as the hand's doing; the rest is too small to see ("a whole
take's work comes down to a hook a few mm long on a continent-sized map"), tap and circle read as
the app's response, a circle makes no loop, a fermata can't be seen, the marks are chrome more
legible than their lamps, and the clip is silent ("you have to zoom way in to see what your hand did,
and add the music yourself").

- *Engineering* (23104e6): **a song.** An audio file opened into Adrift (from Files or another app's
  share sheet; the app takes audio documents) is kept as the song: it plays from its start whenever a
  take begins and stops when it ends, so the reader conducts to it; the export lays it under the clip
  from the take's first moment (after the still opening beat), fading over the last second
  (`Take.song`, `Take::song_in_clip`, `App::set_song/recording/export_song`, `adrift_set_song`,
  `adrift_recording`, `adrift_export_song`; the shell's `withSong` composes it with
  AVMutableComposition and falls back to the silent clip on any failure). A clip at another speed
  stays silent. The check: a plan's `song` (a file, or `"tone"`), muxed by ffmpeg the same way
  (`plans/conduct-song.json`). The look's `song`. Songs from streaming services can't be opened as
  files; a file from Files, Voice Memos or GarageBand can.
- *Visual*: lines heavier in the clip; the cue as a fan of a few maybes collapsing into the chosen;
  the marks hidden while recording, and quieter by night; the fermata visible; the song's quiet sign.
- *Motion*: the clip frames the lamps (their span ~60% of its width); the live camera comes in to the
  lamps when a take begins; the gestures' misreads tested.
- *Truth*: a circle that makes a loop (a flight that can spin, honestly); a tap the lamp visibly
  obeys; the fan's runners-up; the wind preset's seam sketched for round 13.

**R12 merged (Nov 1, 16:40 UTC).** Visual 8bd99b3, Truth 675e95c, Motion b97de96, and Engineering's
seams after them (the cue's `fan` from the chooser's runners-up, the look's `fermata`, each lamp's
place in the export's camera trace). The song's Swift built in CI's unsigned IPA (run 50).

- *The clip frames the lamps* (Motion, `studio::frame`): the smallest circle round every lamp out and
  its last six days of line, 25 km of margin, filling 60% of the clip's width; a lone lamp no wider
  than 100 km each side; a cue on one lamp leans in toward it for four seconds, keeping the others
  within 0.75 of the half-width. The take's zoom no longer multiplies the clip's. hold_out 1.5 s.
  Live, while conducting, the camera comes in to the same frame (`CameraControl::hold_frame`, a
  0.6 s spring, zoom up to 40 only then, flown back after); a two-finger move wins for three seconds.
  A two-finger move begun by a flicking finger no longer cues a stray sweep (`SWEEP_AGE` 80 ms);
  misreads tested at 20 and 120 Hz.
- *The cue* (Visual, Truth): no spray: up to six runners-up (Truth's `Choice.fan`) drawn as faint
  lines folding into the chosen while the lamp glides on its own brighter stroke; a tap no world
  came near shows no fan. A tap now picks the world that comes nearest the finger within 1.5 days
  (`TAP_AHEAD`), so the lamp heads toward it. A circle can pick a world that curls: 15% of each
  world's one-degree cells spin (0.58 rad a day, 80% anticyclonic, measured from the drifters
  against a no-spin null; `rules::spin`, CPU and GPU; `tools/adrift/data/spin.py`); with the
  measured memory a spinning world curls a hook or half loop, 20–40 km, not a ring (rings would need
  an eddy census).
- *The look* (Visual): lines twice as heavy in the clip; the marks gone while recording, and a glass
  ring with a spark by night; a fermata a slowly breathing ring at each lamp; a song a finer second
  ring round the record control.

For next round, from the exit interviews: the clip's line weight and framing fill as dials; a tiny
export size for judging motion; `lamp_in_clip.py` reading the trace's lamps; a plan step that dumps
lamp lines; a contact-sheet tool; real-device timing (six lamps estimated 1.6–2.4 ms a frame on the
phone, borderline; a tap's or circle's look-ahead ~4–8 ms once at the cue); the lamps moving under
the finger while the live camera comes in (to feel on a device); an eddy census for real rings.

## 20. R13: two lamps he loved, kept and made into a clip (Nov 1, 17:00 UTC)

The owner had two lamps on his phone, blue and purple, whose journeys danced together, and wanted a
four-minute edit of them to a song; recording just those two by hand was hard, and he asked whether
the panel's rows identified them (they don't: a rounded date and degree is a hundred kilometres) and
whether he could keep a single lamp for good. Round 12's build was held back from `main` until this
was in, because its spin had changed every lamp's own path: his two would have come back on other
journeys.

- *A lamp keeps its law* (Truth, 7b00639): `Leg.law` (0, the default and every leg kept before
  this, is the flight as shipped up to build 49, without the spin; `sea::rules::LAW` 1 is today's);
  a fresh put takes today's law; a set-down, a take's cut and a cue's cut continue their leg's. The
  lamp's path, the worlds and the GPU maybes honour it. Law 0 is pinned to the old flight bit for bit
  by a digest of 28 paths of 80 steps; a build-46 kept state loads as law 0. Between build 46 and
  then only an exact fast path changed in the sea.
- *The studio's menu* (Engineering): a rest on the record control (it used to export the last take
  again) opens a native menu: a clip of the chosen lamps' journey (30 s to 5 min), the last take
  again, keep these lamps / let them go, copy these lamps, paste lamps (when the clipboard holds
  some). No new control; the menu appears only when asked for.
- *The journey clip* (`App::journey_take`): a take with no hand in it: only the picked lamps (the
  chosen; else the kept; else every lamp out), each drawn as itself with its whole line, from the day
  the last of them was put to today, the date at a pace that fits the length (0.25–4 days a second;
  past today when the journey is short), framed by the clip's camera, rendered offline at full
  quality with a progress bar, silent (a song is added in the editor). `adrift-check journey
  <lamps.txt> <seconds> <out.mp4>` makes the same clip from copied lamps or a phone's kept state.
- *Kept lamps* (`Float.kept`): a clear-all, its undo and its redo leave them; the kept state stores
  them; the look marks them.
- *Lamps as text* (`App::lamps_text`, `paste_lamps`; `LAMPS_HEADER`): every leg exactly, so pasted
  on any phone (or into the check) they go on bit for bit; their ids, and so their hues, kept when
  free; the date goes to the day they were copied.

## 21. R14 opens: the journey clip shows the dance (Nov 1, 17:50 UTC)

Round 13's critic on the journey clip: a still wide shot, two small dots that never come near, a
pace that doesn't adapt, a wake that fades to fragments ("the journey clip doesn't really show
them together yet"); conducted clips better, but framed too wide and off-centre.

- *Engineering* (4708322): `Intent::Rate` sets the date's pace; a journey take carries a pace
  before each frame from `journey::pace`, so a clip's date can slow and hurry and still replay
  exactly.
- *Truth*: `journey::pace` from what is worth seeing (the pair's closeness and its change, turning,
  loops); a real dancing pair for the demo and the checks (`crates/adrift-check/lamps/dance.txt`).
- *Motion*: the clip's framing tight and centred (the span ~60% of the width, the lower third kept
  clear), breathing as the lamps near and part; `lamp_in_clip.py` reading the trace's lamps.
- *Visual*: a journey's whole line held in the clip; crossings that read; the marks quieter at rest.

**R14 merged (Nov 1, 18:40 UTC).** Visual 121ebeb, Truth 8aad72a, Motion 1f671df. A journey clip's
pace (`journey::pace`) weighs the pair's closeness and its change, their winding and turning, smoothed
over two days and capped at six to one; the dancing pair (`crates/adrift-check/lamps/dance.txt`, East
Australian Current, 3.5 turns round each other in 60 days) is the demo and the checks' pair. The clip
frames the lamps alone (not their lines), their span and 10 km filling 60% of the width, their centre
8% above the middle, the zoom breathing on a slower spring that leads a moving frame; Engineering
then held a pair no closer than 60 km a side and zoom 40 (`FRAME_PAIR_MIN_KM`, `FRAME_ZOOM_MAX`), as
at 20 km and zoom 100 the clip held the lamps and lost the dance's lines. A journey's whole line is
held in the clip, crossings keep their hues; the marks at rest are 62% size and 55% alpha until a
lamp is chosen. `adrift-check journey` writes the camera trace; `tools/adrift/lamp_in_clip.py`
reads the trace's lamps (fill, centre, the lower third, out of frame).

## 22. R15 opens: the whole clip a dance (Nov 1, 19:00 UTC)

Round 14's critic: the dance clip's last third is the edit ("two colours braiding and looping,
almost romantic"), the first two thirds a camera chasing two lamps apart along a coast, their lines
off the frame, sharp elbows where a lamp meets the coast, a still opening and an ending that stops
mid-stroke ("skip to the end, that's where they dance"). Colours: a clip keeps each lamp's id, and
its hue is its id's, so the reader's blue and purple stay blue and purple.

- *Motion*: framing that holds the pair and their recent lines, pulling back rather than panning;
  a wider opening easing in; an ending that pulls out to the dance's shape; the hand take framed as
  the journey is.
- *Truth*: a bolder pace, so the clip's time goes where the dance is; the coast hold's motion smooth
  where it is the path's own; a demo pair whose dance starts early.
- *Visual*: no hard elbows in the drawn line; a lively first beat; the marks no longer chrome.

**R15 merged (Nov 1, 19:20 UTC).** Truth 478fb70, Visual 9a0d7bb, Motion e80e4d5. The coast lets a
lamp slide (law 2, `Rules::slide`: the step's and the eddy's along-shore part kept; laws 0 and 1
pinned by digests, so every lamp kept before goes on as it did). The pace bolder (`PACE_RANGE` 20,
winding and turning weighted most, eased over a second of clip time). `lamps/dance-early.txt`, an
Agulhas pair that braids from its first days, the demo. Lines resampled and eased so a tight turn
draws as a bend, ending at the lamp. A clip's first beat breathes (the lamps' glow, the date fading
in). The marks draw nothing at rest (their cells, words and press-to-reveal kept). The clip frames
the pair and their last 15 days of line, its centre on a slow spring and its zoom held so every lamp
and line stays clear of the edges and the lower third: it pulls back rather than pans; it opens three
times wider and eases in over 2.4 s; a journey's end beat (2.5 s) pulls out to the whole lines.

## 23. R16: the edges taken off (Nov 1, 20:40 UTC)

Round 15's critic: close ("'Dance' he might post as is"), with corners and spikes in the lines, a
blocky coast close in, tails off the top in the last third, a still middle in the early pair's clip.
Truth 3a202b3: the pace by visible change (the lamps' motion relative to each other and each line's
bend at the frame's scale), so a hover passes quickly. Visual cc0a31d: every join and cap round, turns
rounded over ~20 reading points in a clip; magnified land eases to its broad colour (the shelf's
shading still steps past zoom ~20). Motion a65fde2: a journey's last third eases to hold the whole
braid since the clip began, opening rather than panning; the zoom capped at 20 within 250 km of land,
easing to 40 beyond 450 km.

## 24. R17: centred, only pulling out, no corners (Nov 1, 21:30 UTC)

Round 16's critic: the dance clip passes ("he'd post it nearly as is"; its ending "the best frame in
the build"); the early pair's clip doesn't (the pair high with half the frame empty, V-corners, no
land); a dip in at the dance's start. Motion 5d38cc8: the pair centred a touch below the middle, a
journey that starts at its closest and only eases out, land within reach pulled into view. Visual
7b09e31: a clip's line a centripetal spline through ~2-point pieces, a reversal within a step eased
over a few samples each side. Engineering after them: `HOLD_DOWN` 0.25 (the lamps off the lower
third's edge in the braid), `COAST_PULL` 1.4 and `LAND_LOOK_KM` 500 (at 2.5 and 1000 the early pair
shrank to a tenth of the frame to show a coast).

## 25. R18: a place to start from (Nov 1, 23:10 UTC)

Round 17's critic: the dance clip passes without a caveat ("it has an arc: together, apart,
together, against a coastline"); the app's caveat "the open-ocean ones look a bit empty": faceted
streamlines close in, a still first frame, no place. Visual 5de935c: the stream function read as a
Catmull-Rom surface through 16 texels (still interpolating the measured values), so the streamlines
curve smoothly close in (32 texel loads a pixel in the sea pass, from 8: watch the device). Motion
fc72f33: a journey's first frame wide enough to show the nearest coast (to 3000 km) or the globe,
easing in to the pair over 2.4 s, then only pulling out.

## 26. R19: no pop (Nov 2, 00:20 UTC)

Round 18's critic: the dance clip postable once a camera pop is gone (zoom +21% in one frame ~1.3 s
in, where the opening's ease handed over to the following frame); the early clip's pair too small
mid-sea, its ending cut off. Motion 64e0113: a final limiter on the clip camera (zoom ≤ 0.55 a
second in its log, 1.8% a frame; pan ≤ 120 points a second), tested, and `lamp_in_clip.py
--max-step`; open sea (no land within 500 km) frames to zoom 70 (`FRAME_ZOOM_MAX`; 40 near land,
`FRAME_ZOOM_NEAR`); a journey's end beat 3.2 s, a 2 s pull-out then held.

## 27. R20: eased, the whole trail held, a glint (Nov 2, 01:45 UTC)

Round 19's critic: the dance clip passes ("Coldplay and post as is: yes"; "show it without a caveat?
Yes for dance.mp4"); the feature's caveat "some clips jerk at the start, and the camera sometimes
loses where the pair started". Motion: a second-order limiter (the zoom's acceleration ≤ 0.6 log a
second², the pan's ≤ 160 points a second², braking without overshoot); journeys hold the braid since
the clip began from 2 s after the opening, so the trail's start stays in frame (the dance's lamps
then ride its southern end, in the lower third more often: a trade the critic judges). Visual
5e8c7c2: in a clip each lamp's head a glass float 30% larger, a breathing glow of its hue on the
water, a glint with a small sparkle. Still to build, from the critic: a way to trim a journey's end
(the editing surface, with the speed and the trim of takes).

## 28. R21: seeds on the wind, the studio's second preset (Nov 2, 02:30 UTC)

Round 20's critic: the camera loses the dance from ~15 s (the whole-trail hold pulls back to the
Tasman, the pair a speck for the last 15 s) and the blue lamp sinks into the sea's blue. The clip was
called done after one small fix; the round's work is the run's plan (b): the wind's preset.

- *Engineering* (5af1913, the seam): `Leg.medium` (`model::Medium::{Sea, Wind}`, Sea left out of
  copied lamps, so every lamp's text and path is unchanged to the bit), `Held::Landed` / `Ditched`
  (a seed come down, for good; never cut into a new leg), `Intent::Medium` (what a put makes; kept in
  the snapshot and a take's start; the free pace and the conductor's tempo become the medium's:
  `App::free_rate`, `App::tempo`). `sea::medium::MediumRules`: put, flight (with the previous
  `Held`), velocity, may_put, answered by `Rules` and `AirRules`; `Media::of(medium)`. Paths,
  `conduct::Worlds` (generic), cues and the take's cut dispatch by the leg's medium. The wind shares
  the sea's wind and coast sheets (`Sheet.bytes` an `Arc`). With the wind's preset land answers the
  hit test as open. The iOS studio menu offers "Seeds on the wind" / "Lamps on the sea"; the web takes
  `?wind`. `plans/wind-seeds.json`.
- *Truth* (05a72ab): the wind's GPU cloud (`Maybes.air` over the same paths buffer: seed_at,
  step_many, keeps; a seed's cloud agrees with its CPU paths to 3 m, no fate differs in 9216); the
  preset's seed is a seed lofted (`AirRules::LIFE` 4: median 2–2.25 days aloft, ~7 s at 0.3 days a
  second; pollen measured 0.25–0.5); `AIR_RATE` 0.3 and `AIR_TEMPO` 0.45 measured and kept (a 1 s
  stroke spreads a seed's worlds 40 points against the sea's 34; sweeps match 17–19% of seeds up);
  the chooser never picks a world come down, and the wind's look-aheads (`AIR_TAP_AHEAD` 0.25 d,
  `AIR_CIRCLE_AHEAD` 0.5 d).
- *Visual* (853810d): a seed is a dandelion pappus from above (17 filaments of its hue round a dark
  seed, turning slowly, a shadow set off by its height; by night its own light), its line an airy
  spline since the lift, a quiet coming-down (a mark on land, one ring on the sea); the wind cloud in
  that look (flying a wash, landed marks, ditched rings). Wind streamlines on the ground skipped on
  purpose (they would muddy the currents; the seeds' lines are the wind seen). Cool hues (sky, lilac,
  mint) get a pale halo on their lines and a pale ring round the head, so a blue lamp no longer sinks
  into the sea; warm hues unchanged.
- *Motion* (bcb03f0): a seed set down over land; journeys of seeds by the wind's rules (the seam's
  `journey::pace` stepped them by the sea's: fixed), ending when the last comes down; the clip frame
  reads the medium (`Lean.scale`, `WIND_SCALE`); seed plans (put on land, carried to land, ridden,
  conducted). The critic's framing (the pair at 40–60% of the width) was tried three ways and
  reverted (0c97643: the journey framing is R20's): measured over the dance's last 15 s the pair's
  median span stayed 4–6% of the width and lamps left the frame, because at a close zoom the R19/R20
  pan limits (points a second) can't keep up with the pair. Next round's first job: pan limits scaled
  with zoom and a lead on the pair (`tools/adrift/pair_share.py` measures it).
- *Engineering, the owner's journey report* (5de4c9d, 2e6d585): a journey clip ran the days from the
  later put to today over its length (a 4-minute clip of a 214-day pair crawled at 0.9 days a second
  and stopped short of the loop he had seen). It now runs at the app's own pace (2 days a second;
  seeds 0.3) for its whole length, reaching the app's date 80% of the way through
  (`JOURNEY_ARRIVES`) and carrying the lamps on past it; test `a_journey_reaches_the_apps_date_and_goes_on`.
  Clips are HEVC at 2 Mbit/s (was H.264 at 12): the dance's 34 s are 9.3 MB, a 4-minute clip about
  66 MB (was ~360); a song is laid under by passing the video through.
- *Visual, the owner's "which are chosen"* (a248c3b): chosen as itself a solid ring of its hue on a
  pale rim; as its cloud a ring of ten dots of its hue; kept a small gold bead at the shoulder (the
  gold ring, read as another "chosen", gone); the others step back to half their light while any is
  chosen; the panel's marks alike. Live only (a clip keeps R20's look). `Scene.keep` for plans.

Next, from the exit interviews: a seed come down on the sea becoming a lamp (the one cross-medium
call); the dials that differ (lifetime, memory, kinship, wander, trail) recorded in a take; a seed
in the panel's marks (it draws as a lamp mark there); `GpuPath.measured` means rain for wind clouds
(a per-medium meaning on one field).
