# Two media: what differs between the sea and the wind

For the preset seam (round 11): what a preset is in the rules, and the dials that fall out of the
difference between the sea's lamps (`Rules::flight`) and the wind's things (`AirRules::step`).
Both are the same machine: **a mean field, plus a remembered wander drawn from a world's dice,
plus what stops a thing.** A medium is the sheets that fill those three slots; a dial scales one of
them. Every number below is read from the sheets (`adrift/data/`), medians over the sea or the
globe; `adrift/SOURCES.md` says what is measured.

## The machine both share

```
v_wander(t + dt) = a v_wander(t) + sigma sqrt(1 - a^2) kick_k(place, quarter day)   a = exp(-dt / T)
x(t + dt)        = x(t) + (carry * mean(place, day) + v_wander) dt
stop?            = the medium's holds (coast, ice) or its fates (settle, rain)
```

`kick_k` is world k's field: unit Gaussian draws on a lattice `cell` wide, a fresh lattice each
quarter day, bilinear between nodes (`rules::world_field`, `wind::air_field`). Path k of every
thing lives in world k.

## The two media, side by side

| | The sea (lamps) | The wind (seeds) | Ratio |
| --- | --- | --- | --- |
| Mean field | the month's current, `currents.png` (+3% of the wind, the leeway) | the month's 10 m wind, `wind.png` (carry 1) | |
| Typical mean speed | 0.1–0.2 m/s (Gulf Stream 1.5) | 2.8 m/s median (0.7–7, 10th–90th) | ~20x |
| Wander sigma | 0.14 m/s median (0.09–0.26), from drifters' K / T | 3.0 m/s median (1.6–5.5), measured gusts | ~20x |
| Wander / mean | ~1 | ~1 | **the same** |
| Memory T | 2.8 days (2.2–4.1) | 0.94 days (0.55–1.4) | 1/3 |
| Diffusivity K = sigma^2 T | 4900 m^2/s (1600–19000) | 850 000 m^2/s (160 000–2 400 000) | ~170x |
| Memory length sigma T | ~33 km | ~240 km | ~7x |
| World lattice (kinship) | 1 degree (~110 km, an eddy) | 15 x 8 degrees (~1000 x 900 km, a weather system) | ~9x |
| Lattice / memory length | ~3 | ~4 | about the same |
| Spread in 10 days (RMS) | ~150–300 km | ~1500–1900 km, held to real paths (`wind_tests`) | ~8x |
| What holds it | coast (held at the shore), ice (held till it retreats) | nothing: it crosses coasts and mountains | |
| Lifetime | unbounded: a lamp drifts for years | ~1.2 days dry (settling), hours in heavy rain (rain-out) | |
| Its fate | aground or frozen in, and freed again | down on land, or down on the sea (where a lamp could take over) | |
| A step | a quarter day: a turn of ~13° median | a quarter day: kinks of sqrt(2 dt / T) = 0.7 sigma (draw it as a spline, below) | |

What the table says: **the two media share their shape and differ in scale.** Wander against
mean is about 1 in both, and the world's lattice is about 3–4 memory lengths in both. Mostly the
wind is the sea run ~20x faster, with a third of the memory and a short life. So the studio's
dials are dimensionless, with each medium supplying its own units.

## The dials (what the seam exposes; 1 = the measured value everywhere)

| Dial | What it scales in the rules | Sea field | Wind field | Range worth offering | What it looks like |
| --- | --- | --- | --- | --- | --- |
| **pace** (days a second of clip) | the clock's rate, not the rules | ~3 days/s shows a lamp's month in 10 s | ~0.3 days/s (a seed's day in 3 s) | 0.1x–10x around the medium's default | slow drift vs. a gust's flight |
| **wander** (stochasticity) | sigma (and so K): `Rules.wander`, `AirRules.gust` | 1 | 1 | 0 (rails: the mean field alone) – 2 | ribbons → frayed bundles → smoke |
| **memory** | T, the decay time, with sigma kept, so K scales with it | `eddy_time` x m | `Weather.time` x m | 0.25–4 | twitchy wander → long sweeping arcs; 4 in the wind gives loops like a balloon's |
| **kinship** | the world lattice's cell (x m), and a share s of the kick from the world vs the thing's own dice: `kick = sqrt(s) world_k + sqrt(1 - s) own` | cell 1°, s = 1 | cell 15x8°, s = 1 | cell 0.25–4x; s 0 (strangers) – 1 (flock) | a flock that flies as one (and parts at fronts) vs. a scatter |
| **lifetime** | the settle rate's inverse (`AirRules.fall`, `washout` x 1/m); the sea has none | infinite (offer a fade-out instead) | ~1.2 days dry; rain shortens it | 0.25x – infinite | a dandelion clock's burst vs. a long voyage |
| **holds** | which stops apply: coast, ice (sea); come-down onto land/sea (air) | coast + ice | none in flight | on/off each | lamps piling on a shore; seeds crossing it |
| **trail** | the look's persistence, best tied to memory: a trail of ~3 T reads as the thing's "intent" | ~8 days | ~3 days | 0.5–5 T | |

Mapping notes for the seam:

- **wander** and **memory** are independent only if the dial on memory keeps sigma. Then the 10-day
  spread goes as sqrt(memory) and the look changes from twitchy to sweeping. Keeping K instead
  (sigma = sqrt(K / T)) would make memory change the texture and leave the spread alone. That's
  the subtler dial, and probably a better one for a music edit: the cloud's size stays where it was
  framed. I suggest **memory keeps K** and wander scales K.
- **kinship's share s** needs one new draw per step (the thing's own Gaussian: `rules::gaussian`
  on `hash_seed(seed, k, day)`, already in both shaders). s < 1 breaks the R6 property that one
  path alone sees exactly what it saw before (it still has the same law).
- **lifetime** for the sea should be a fade, not a fall: lamps don't sink. For the wind, the
  measured value is short (about a day). That's true for pollen near the ground, but a studio
  default of ~4x (a seed lifted higher, a balloon) will read better. Default to it and say so.
- **A seed that comes down on the sea** (`Aloft::Sea`) could become a lamp (`Rules::put` at its
  place and day): one medium hands a thing to the other, a small unexplained surprise.

## What a preset is, then

```
Preset {
  medium: Sea | Wind,          // which sheets fill: mean, wander (sigma, T), holds/fates, lattice
  things: how they are put     // a lamp on the water; a seed lifted anywhere (land too)
  dials: { pace, wander, memory, kinship (cell, share), lifetime, holds, trail }
}
```

Both media step at a quarter day. For the wind, a quarter-day step is coarse against T (~1 day):
draw the path as a centripetal Catmull–Rom through the steps and glide between them
(`AirRules::velocity`). Don't step finer than the quarter day: steps that span no quarter-day
middle reread the same world draw, which correlates the kicks (the sea's `world_kick` has the same
property).

The flat plane (a later preset) fits the same machine: an analytic mean field (a vortex, a
shear), the wander's sigma and T as plain numbers, a lattice in plane units, no holds.

## The wind preset's seam (sketch for round 13)

What the wind needs that the sea's lamps don't, so `Preset` can hold both without a branch at every
call:

- **The rules behind a trait, not `Rules`.** Both media answer the same few calls. `put(at, day, k)`
  is `Rules::put` or `AirRules::rise`. `step(state, day, dt, seed, k)` is `Rules::flight` or
  `AirRules::step`. `velocity(state, day)` drives the glide. A state carries a place, the wander's
  velocity and a status: the sea's `Held` {No, Coast, Ice}, the air's `Aloft` {Flying, Land, Sea}.
  `conduct::Worlds` already only needs put and step: make it generic over the medium, and
  conducting works for seeds unchanged.
- **The GPU maybes per medium.** `Maybes` (`rules.wgsl`, bindings: currents, wind, eddy, ice,
  coast, eddy time) and `AirMaybes` (`wind.wgsl`: wind, gusts, coast) share `GpuPath` and the
  params' layout up to their medium's fields. The seam picks the pipeline; the look reads the same
  buffer.
- **Where a thing may be put.** The sea refuses land and unrecorded water (`Rules::may_put`). The
  wind takes anywhere. `Preset::may_put(at) -> Put`.
- **The ending.** A lamp never ends (it is held, and freed). A seed comes down: the look needs
  "down on land / on the sea" (`Aloft`), and the take may hand a seed that came down on the sea to
  the sea's rules as a lamp (`Rules::put` at its place and day). That's the one cross-medium call.
- **The clock.** The wind runs about 20x faster: its default pace and its conducting tempo are
  the medium's (about 0.3 days a second, against the sea's 2 free and 3 conducting). The step stays
  a quarter day in both (the worlds' grid); the wind's line is drawn as a spline through the steps.
- **The dials** (table above) as one struct the preset owns: wander, memory, kinship (cell,
  share), lifetime (the wind's fall and washout; the sea: none), trail. Each scales one field the
  rules already read (`Rules.wander`, `AirRules.gust`, `fall`, `washout`), so a dial is a field set
  on the medium's rules at a take's start and recorded in the take.
- **Sheets.** The wind's `gusts.png` and its `wind.png` are already in FILES. Nothing more to
  bundle.
