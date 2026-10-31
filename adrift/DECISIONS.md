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
