# touch-standalone

Builds Adrift's touch module (`adrift/crates/adrift-core/src/touch/`) on its own, against the real
`model.rs` and a stand-in `App` (the state the gestures probe, with the intents applied as `app.rs`
applies them; a sea of circles for land; floats that drift only as a plan says; no GPU), so the
gesture state machine, the camera and the curves are tested in milliseconds; and plays the module's
plans to prove every gesture, then exports them for the real build:

    cargo test --manifest-path tools/adrift/touch-standalone/Cargo.toml
    python3 tools/adrift/touch-standalone/plans.py                       # writes touch/plans/*.json
    cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- \
        --export adrift/crates/adrift-core/src/touch/plans/check \
        adrift/crates/adrift-core/src/touch/plans/*.json                 # checks each; writes plans/check/
    cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- -v plan.json   # every event and intent
    bash tools/adrift/touch-standalone/play_all.sh                       # every plan on the real build, checked

A plan: pointer steps over time (`down`/`move`/`up`/`cancel`: a finger id, with `xy` points or `at`
a place projected by the camera as it is), `shot`s, `shake`, `key`, and `expect`s the replay checks
(floats, the hand's state, intents seen since the last check, the camera). The exported form is what
`adrift-check -- play` takes (xy resolved, `advance` steps, every expectation also a shot named
`expect-NN` with the expectation kept beside it), and `check_play.py <plan> <out-dir>` reads the
play.log against it. Exit code 1 on a failure. `src/app.rs` here must be kept in step with
`adrift-core`'s `App::intend` by hand: it is the one copy.
