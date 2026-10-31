# touch-standalone

Builds Adrift's touch module (`adrift/crates/adrift-core/src/touch/`) on its own, with a stand-in
`model.rs` (the types Motion proposed to Engineering), so the gesture state machine, the camera and
the curves can be tested on Linux before or beside the skeleton, and plays its plans through a tiny
stand-in sea (a disc with one land mass, floats that do not drift) to prove every gesture:

    cargo test --manifest-path tools/adrift/touch-standalone/Cargo.toml
    cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- \
        adrift/crates/adrift-core/src/touch/plans/*.json           # every plan: intents, and its expectations checked
    cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- -v plan.json   # every intent as it happens

A plan is the same JSON `adrift-check -- play` takes (VOCABULARY.md › Plans): pointer events over
time, `tick`s, `shot`s, and `expect`s this replay checks (the floats' count, the camera, the state of
the hand, which intents were seen). Exit code 1 if an expectation fails.

When `adrift-core`'s `model.rs` lands, `src/model.rs` here is to be replaced by a `#[path]` include
of it, and this crate kept as the fast test of the touch module alone.
