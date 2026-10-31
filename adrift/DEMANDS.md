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

### D1 Truth → Engineering (R0)  [open]
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
ashore (`tools/adrift/data/sea-harness`, `cargo run`), and the Sargasso spread test still holds].
I could give back [the step depending only on (at, day, dt, seed, k): with memory it depends on
the velocity carried too, still deterministic for a seed; and 8 bytes a path on the GPU].

## Answered

(none yet)
