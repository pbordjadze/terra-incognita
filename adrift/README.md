# Adrift

Tap the open sea and it rings; rest your finger a beat longer and the ring thickens into a glass
float, lamp amber, the one warm thing on a cool sea. Let go and the sea has it: the month's real
current, the eddy wander drifters measured there, its leeway on the month's wind. Nothing scripts
where it goes.

Written from scratch in Rust on wgpu: its own app beside Terra Incognita in the same SideStore
source, and a web build opened by link. Nothing here is a web view; nothing is fetched at run time.

- `DECISIONS.md`: the rulings every craft reads each round (module boundaries, the seams, the
  rounds). `DEMANDS.md`: the cascade. `SOURCES.md`: every dataset, what is measured and what is
  drawn, and the credits owed.
- `crates/adrift-core`: the library. `crates/adrift-check`: headless renders and played sessions
  to PNG on Linux (`tools/adrift/linux-setup.sh` installs Mesa's Vulkan software driver first).
  `crates/adrift-web`: the WebGPU build. `ios/`: the device shell.
- `data/`: the sheets the app loads, built by `tools/adrift/data/`; never hand-edited.

Build and check on Linux (about two seconds a render on the software driver):

    bash tools/adrift/linux-setup.sh
    python3 tools/adrift/assets.py adrift/build/assets --link
    cd adrift && cargo run -p adrift-check -- render crates/adrift-check/scenes/atlantic.json build/out/a.png --size 1179x2556 --ppp 3
    cargo run -p adrift-check -- play crates/adrift-check/plans/put-and-drift.json build/out/put

The web build: `bash tools/adrift/web-build.sh` (a directory for any static server; WebGPU: Safari
26, Chrome, Edge). The device: `adrift/ios/build.sh` on a Mac, or `[adrift-ipa]` in a branch commit's
first line for CI's unsigned IPA (`.github/workflows/adrift.yml`); on `main` every build is published
to the SideStore source beside Terra.
