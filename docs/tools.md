# Terra's shared tools

The tools outside the toy kit: the builders of the Earth's data that Adrift and the toys borrow, the
checks, and delivery. The kit, the toys' and Adrift's own tools have their table in `docs/kit.md` › The
tools. Read it before you build; add to it when you do.

**The rules**

1. **Picture your experience first.** Write down what the reader should see, touch and feel before
   reading this page, so what exists doesn't shrink what you picture.
2. **A tool that falls short is the tool's bug.** Fix it, extend it, or build a new one beside it.
   Never settle for less than you pictured because a tool can't do it yet.
3. **Keep this page true in the same commit.** A commit that adds a tool adds its line; a commit
   that starts using one adds its user; a commit that changes a limit changes it here.

The Xcode-built Terra (its Metal globe, workbench, packs, playground, previews and their tools) was
retired on November 13, 2026; it is whole at the tag `before-xcode-retirement-2026-11-13`.

## Data: builders from sources

Each reads public sources (named in its docstring) and writes a file the apps' manifests name.
Rebuild, never hand-edit.

| Tool | Writes | Run | Used by |
| --- | --- | --- | --- |
| Globe | `Globe/` (distance fields `geo.png`, `terrain.png`, `ids.png`, `globe.json`, fonts) | `python3 tools/globe/build.py <cache>` | Adrift, the toys (`manifest.json`) |
| Ground | `Globe/ground/` (twelve months' land, sea ice, sea floor), `regions.json` | `python3 tools/globe/ground.py <cache>` | Adrift, the toys |
| Night | `Globe/ground/night.jpg` (Black Marble) | `python3 tools/globe/night.py <cache>` | Adrift, the toys |
| Ocean | `Globe/ocean/OceanCurrents.png`, `OceanStream.png` (NOAA drifters) | `python3 tools/ocean/build.py <cache>` | Adrift |

## Checks and tests

| Tool | What it checks | Run | Where it runs |
| --- | --- | --- | --- |
| Linters | rustfmt and clippy `-D warnings` (host, iOS, wasm32), ruff, shellcheck, actionlint, SwiftLint, each stock but for the rules loosened with reasons in `ruff.toml`, `.swiftlint.yml`, `.github/actionlint.yaml` | `bash ci/lint.sh [rust\|python\|shell\|actions\|swift\|fix]` (pinned tools: `tools/requirements.txt` in `.venv`) | `lint.yml`, first in every workflow |
| Same pictures | every toy's plans and Adrift's scenes played in two checkouts, compared pixel for pixel: a dependency bump changes nothing on screen | `python3 tools/toys/same_pictures.py play <checkout> <out>`, `compare <a> <b>` | by hand (Linux, Mesa's Vulkan) |

## Delivery

| Tool | What it does | Run |
| --- | --- | --- |
| Versions | Rust (`rust-toolchain.toml`, installed by `ci/rust.sh`), Xcode (`ci/xcode.sh`), crates (`Cargo.toml` + `Cargo.lock`), actions, Python (`tools/requirements.txt`); Dependabot's weekly grouped bumps | CLAUDE.md › Staying current; `.github/dependabot.yml` |
| SideStore | the source JSON listing every app's newest IPAs (`ci/sidestore_apps.json`) | `ci/sidestore_source.py`, `ci/sidestore_publish.sh`; `toys.yml` and `adrift.yml` after a release |
