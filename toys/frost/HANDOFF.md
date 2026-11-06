# Frost: handoff (Nov 6, 2026; row 242 over row 218's)

Row 242 answered the owner's "it just keeps filling in". What a finger does now lasts:

- **The glaze** (`frost.wgsl`: binding 7, `glaze`, one f32 a mask texel as u32 bits): where a melt froze
  smooth, how deep inside its edge (points; `melt_pane` takes the max). `grow` treats a cell whose glaze
  is deeper than `fringe_at` (FRINGE 14 pt, ragged by the flaws) as a sink, so frost creeps back only a
  fringe deep. `relax` makes every glaze shallower by RELAX_PER_S (0.55 pt/s), so a hole heals in about
  two minutes; `renew` (lib.rs) seeds a healing hole's edge every ~0.9 s from the CPU's `marks`.
- **Seeds on the melt's true edge**: `edge_point` mirrors `melt_radius`'s wobble (change both together),
  with each path point's own melt width (`Finger::radii`).
- **What is under a finger** is read at touch-down (`probe`: ice in a 16 pt disc and the glaze under it,
  stats words 48–50, read in the same frame). On frost or bare glass: Melt. Deep inside a clear patch:
  Poised; moved, Melt (widens the wipe); still for PRESS_S 0.4 s, Pen (`pen`: a granular frozen core
  owned by HOAR in the mask, the glaze given back three print-widths out, seeds one side then the other
  every 30 pt).
- **A shake** (`shaken` → `sweep`): `clear` sweeps a wet front from the lamp's side at 900 pt/s, the
  mask and glaze reset; the creep from the frame restarts (`creep_from`).
- Plan `play` (54 s) shows all of it; `full` was retired.

# Frost: row 218's handoff (Nov 4, 2026)

painting-company row 218. A cabin window at Oymyakon at about −50 °C, the night outside. Frost grows by
itself from the frame by Reiter's hexagonal automaton for snow crystals (GPU compute), each crystal at its
own lattice angle, meeting its neighbours along seams; the air wetter low on the glass (dense ferns),
drier high (sparser, thinner arms). The finger is warm: it melts its way (wider where it rests); lifted,
the wet glass freezes again from the melt's edge. ROUNDS.md has a line a round; SOURCES.md the model.

## How it works

- `src/frost.wgsl`: twelve slots, each a 512² axial hex grid (ping-pong buffers), stepped 230 times a
  second; `grow` (Reiter's step, the far air held at the pane's β a little beyond the tips), `claim`
  (a newly frozen cell claims the pane mask three cells round it, lowest id wins: deterministic), `seed`,
  `melt_cells`/`melt_pane` (a capsule along the finger's way this frame), `paint` (the pane field, two
  texels a point, from the grids of the live crystal owning each texel; finished crystals stay in it).
- `src/lib.rs`: slots, the frame's creeping seeds, the finger, the regrowth seeds, the stats readback
  (blocking, a few bytes a frame: frozen counts and reach drive retirement, crackle and haptics).
- `src/pane.wgsl`: the night, the frame, the meltwater, the ice's light. `src/sound.rs`: the crackle.
- `tools/toys/frost/reiter.py`: the numpy reference used to choose β and γ.

## Checks

- `bash tools/toys/launch_check.sh frost`: passes (183 MB peak of 1400, all three shaders to MSL 2.4).
- Plans: start, full (left alone 22 s, then tap, rest, winding wipe), drag, meet, melt, ipad, info, rules;
  every touch changes the frame (`--tap-check`). The 44 s `full` plan takes several minutes on lavapipe.
- On CI the plans take about 23 minutes (the 44 s `full` plan with the touch check's shadow runs on
  a 2-core runner); shorten it if CI time matters.
- The phone's GPU cost is unmeasured: up to 12 live grids × 262k cells × ~4 steps a frame, most threads
  exiting early. If it is slow, drop SLOTS or N, or STEPS_PER_S.

## What the last critic said (round 5): "a good ok, not yet oh huh neat"

1. **First, the refreeze.** It nucleates whole six-armed stars inside the melt (a drag on clear glass
   becomes a column of stars). Seed only on the melt boundary (in `regrow`, radius ×1.0 instead of
   ×0.94, nearer the surviving ice), so each grows inward as a one-sided feathery front and the fronts
   meet in one seam down the stroke. Hold the melt clear and wet 3–4 s before the regrowth starts (the
   `after` times in `regrow`).
2. Upper crystals read flat and chalky beside the fine lower ones; the rim highlight reads as an
   outline (pane.wgsl: `rim`, the alpha ramp).
3. A lone tiny crystal at the top frame early reads as a "^" button; let early frame seeds branch at
   once or avoid singles there.
Keep: growth from the frame and crossbar meeting along seams, the bottom frosting first, the resting
melt's dark hole with its wet rim and the stars through it, the night and the frame, one gesture.

## Not done

- No run on Paul's phone yet. The IPA built and opened in the iOS Simulator with Metal's validation on
  (run 41920531959, started by hand; the branch's push run was cancelled behind another toy's stuck
  check).
- Save/restore: the pane isn't kept when the app goes to the background (it frosts over again).
- Terra (the toy that holds the toys) may want Frost's place: Oymyakon, 63.46°N 142.79°E.
