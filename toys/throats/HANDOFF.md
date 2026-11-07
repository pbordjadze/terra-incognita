# Throats: handoff (Nov 7, 2026; painting-company row 257)

A sunny California hillside from above; each territory held by a side-blotched lizard with an orange,
blue or yellow throat. Orange takes blue's rock, blue yellow's, yellow orange's; the colours turn in
spirals. A finger pours a stroke of one colour (each stroke the next colour); two fingers pinch and pan.
It opens on the whole hill's spirals (3 pt a lizard, every lizard a speck); pinch in and the specks
become lizards on their rocks, running in when they win one, off when they lose it.

## How it works

- `src/sim.rs`: the model on the CPU, 512 x 512 on a torus. A stochastic cyclic automaton (range 2,
  threshold 5, take 0.5; SOURCES.md), 2.2 steps a second; each step's flips are shown at their own
  moment inside the step (`pending`, `reveal`), so lizards arrive continuously. Opens on pinwheels
  warmed 220 steps. The ground no lizard holds: outcrops, shrubs, and grass meadows (24% of the hill, a
  smooth noise, thinner near outcrops; patches under 400 rocks turned to grass so none freezes on one
  colour). `relief()` blurs them into a filtered texture for round shapes.
- `src/hill.wgsl`: one full-screen fragment. Ground, gravel, outcrops, shrubs, meadows, the hill's folds
  lit; then per pixel the 3 x 3 territories round it: each its rock and its lizard (SDF body, legs,
  banded back, the colour on chin, shoulders and flanks; a newcomer flashes it). Far out a lizard is a
  speck (`specks`), close in a lizard (`near`), by its length in pixels.
- `src/lib.rs`: the camera (the opening window is the hill's rockiest, `opening`), fingers, the pour
  (24 pt radius), the per-frame upload (512² RGBA32F, 4 MB: halve it with RGBA16 if the phone is slow).
- `examples/cca.rs`: the tuner used to choose range, threshold and take (writes the field as PNG).
- `tools/toys/throats/critic.md` and `evidence.sh`: the critic's brief and its evidence.

## Not done

- No sound. No save/restore (the game starts fresh each launch, which suits it).
- The phone's GPU cost is unmeasured: about 20 SDF lizards a pixel close in. If slow, skip the 3 x 3
  loop when specks are full, or draw lizards as instanced sprites.
- Round 6's critic never saw the 3 pt opening it asked for (the six-round limit).
