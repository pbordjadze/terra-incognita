# Ghost Jam: handoff (Nov 4, 2026)

painting-company row 218. Traffic on the real ring of Sugiyama et al. 2008 (230 m, 22 cars, Nagoya),
from above at dusk; tap a car and it stops for a moment, and a jam of brake lights crawls backward round
the ring while every car drives forward. ROUNDS.md has a line a round; SOURCES.md what is real and drawn.

## Where it stands

- Merged to main at 2bb910dc (Nov 4, 5:02 PM ET); main's run released `jam-build-144` (5:43 PM ET)
  and puts Ghost Jam in the SideStore source (`ci/sidestore_apps.json`, after Carol). The branch
  IPA (round 1) built and opened in the iOS Simulator with Metal's validation on. Not yet seen on a phone.
- Checks: `cargo test -p jam` (the paper's numbers: even flow about 31 km/h holding 10 minutes under
  the drivers' wobble; one tap a jam running backward at about 20 km/h that stays; every tap a jam,
  three at once and later ones too; no car touching); `bash tools/toys/launch_check.sh jam` passes
  (the iPhone's limits, 2 shaders to MSL 2.4, peak RSS 140 MB of 1400); plans start, taps, info,
  rules, ipad, every touch doing something.
- Five rounds, each judged by a fresh critic from renders at phone size: R1 between ok and neat, R2
  probably neat, R3–R5 neat but only just.

## How it works

- `src/traffic.rs`: the ring and its drivers (IDM), the tap (brake 6.5 m/s² to a stop, stand 0.8 s),
  brake lights (braking harder than 1.8 m/s² or standing). Constants and why are in its header and
  tests; two ignored probes (`probe_many`, `probe_late`) print what many taps and a late tap do.
- `src/lib.rs`: the drawing (a ground `Backdrop`, everything else `paint`): cars drawn 1.25x long and
  1.45x wide on a wider road; inside the ring the last 40 s wound inward (each car's faint track, a
  soft red glow where its brake lights were on, which merges into one arm curling backward); a halo
  on one car until the first touch. Anchors: `invited`, `clear` (the moving car farthest from any
  braking one), `near`, `far`, `centre`, `car0`..`car21`.
- `src/sound.rs`: 22 engine buzzes heard from the centre (pitch with speed, loud pulling out of a
  stop), tyre hiss, a chirp for each car braking hard, a thump for a tap.

## What the last critic said (round 5, after the merge)

Borderline "oh huh neat": the first jam earns it. Weakest: the inner spiral still reads as the biggest
red on screen at phone size, and nothing says it is the jam's history (two critics loved it, two found
it took over: R4 halved it). Then: the jam never ends, and after a minute with more taps the ring is
mostly red; the sound is a steady rumble and the chirps are faint. Keep: the invited single tap, the
backward band, the halo, the night look, the inert empty ground.

## What I'd do next

1. The spiral: show it only after the jam has gone round once, or fainter still; or let the reader
   find it (it's the paper's figure, and the most striking still).
2. An ending: drivers growing attentive after a minute or two untouched (a slowly rising `A`), so a
   jam melts and the ring comes back to even flow, ready for the next tap. Say so in SOURCES.md (drawn).
3. Many taps at once can leave no jam (all cars pull away together in IDM); a slow-to-start rule
   fixed it but broke the paper's 20 km/h (SOURCES.md › Known). A reaction delay might do both.
4. Louder, clearer chirps; check on a phone speaker (the hum's fundamental is under 200 Hz).
5. Terra's door: its session has the line to add (Nagoya [136.91, 35.18], night, red tint).
