# Salt Flat (was Ghost Jam): handoff (Nov 6, 2026)

painting-company row 244 (Ghost Jam was row 218). Cars at night on the Bonneville Salt Flats, from
above, with no roads. Each driver only follows the nearest tail lights ahead, keeps a safe gap (IDM, the
rule Ghost Jam tuned to the Nagoya ring), turns no tighter than a car, dabs the brakes when brake lights
come on ahead, and gets bored and peels off. Convoys emerge, snake, merge and split; tyre tracks fade on
the salt. Tap a car: it stands 2 s and its red passes down its convoy. Drag a car: it rides the finger
like on a string, out of its convoy. Hold the open salt: a light the cars come to circle, a galaxy of
convoys. ROUNDS.md has a line a round; SOURCES.md what is real and drawn. The Nagoya ring is in `main`'s
history at 3032c9a.

The crate, directory, bundle id (`com.pbordjadze.toy.jam`), release tag (`jam-build-<n>`) and Terra's
link (`jam::Jam`) are unchanged; only the display name (`toy.json`, a working title) and the SideStore
listing's words changed. Terra's door (row 245 owns it) moves to Bonneville [-113.86, 40.76] once this is
on main.

## How it works

- `src/flat.rs`: the drivers (header: the rules). Leaders are chosen a few times a second by a greedy
  match, one follower a car; strangers ahead are swerved round and braked for only when about to touch,
  with a creep when held up; a finger's `Hand::Lead` pulls a car on a string, `Hand::Lamp` makes drivers
  without a leader circle it, each on its own orbit. Tests: convoys form and keep forming; no gridlock;
  no car through another; cars stay on the field; a tap stops cars behind; the chain lights in turn;
  untouched red stays rare; a held light gathers cars; a led car reaches the finger; rings come and go.
- `src/lib.rs`: the drawing. The salt crust (a Voronoi backdrop) by night; the tracks in a `Layer`
  faded by a ReverseSubtract blend pass (times a constant, less a hair, so nothing sticks at the last
  8-bit step); headlight cones, cars and lamps with `paint`. The field is 1.45 screens on every side
  (`BEYOND`); one car to 1,700 m² (`AREA`); the night is 12 s under way at open (`BEFORE`), laid as a
  few long convoys. Anchors: `invited`, `head` (longest convoy on screen), `middle`, `open` (the salt
  farthest from cars), `centre`, `car<i>`.
- `src/sound.rs`: the 24 engines nearest the middle (Ghost Jam's hum, chirps and thump).

## Where it stands

- Six rounds, each judged by a fresh critic from phone-size renders and frame sequences: R1 and R2 "ok",
  R3–R5 "oh huh neat" (see ROUNDS.md and the critic log in the final report).
- `cargo test -p jam` (10 tests), every plan with the tap check, `tools/toys/launch_check.sh jam`
  (4 shaders to MSL, ~150 MB peak) pass on Linux. The branch IPA (round 1's) was built in CI.

## What I'd do next

1. Feel it on a phone: the drag's string, the frame rate with ~85 cars and their glows, the sound.
2. Knots where convoys cross at right angles still read a little like a pile-up; drivers could yield
   earlier (the one closer to the crossing goes, the other aims behind it).
3. The light's ring keeps circling a while after the finger lifts (the embers fade, the drivers' aim
   carries on): lovely or stuck, a phone will tell.
4. The sound was Ghost Jam's, untuned for an open field: engines could pan with the convoys.
