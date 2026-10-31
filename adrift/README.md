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

Build and check on Linux:

    bash tools/adrift/linux-setup.sh
    cargo build --manifest-path adrift/Cargo.toml
    cargo run --manifest-path adrift/Cargo.toml -p adrift-check -- render adrift/crates/adrift-check/scenes/atlantic.json out.png
