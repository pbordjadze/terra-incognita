# Terra Incognita — engineering notes

**Terra Incognita is the toy that holds the other toys: `toys/terra/`** (Rust on the toy kit, built and
released by `.github/workflows/toys.yml` as `terra-build-<n>` under Terra's own bundle id; painting-company
row 217). It opens on the whole Earth with a living light for each toy (`doors.rs`); a touch dives into
the toy itself, linked into Terra's one library (`rooms.rs`); the Earth mark or a pinch out comes back.
Beside it are the other toys (`toys/`) and Adrift (`adrift/`), each its own app in the same SideStore
source. This file is the map and the rules.

The Xcode-built Terra that came first (`App/`: a Metal globe with clouds and a moving sea, the quiet
globe's rules, a workbench of prototypes and runtime-3 packs, the playground and its previews, the
Persian and Wikipedia styles) was retired on November 13, 2026 with its CI, packs pipeline and tools; it
is whole on the branch `before-xcode-retirement-2026-11-13`. The experiences retired before it are at
`before-retirement-2026-10-31`.

## Documentation scope

This file describes what is built and its engineering constraints. For product direction, read
`terra-incognita/DESIGN.md` and its linked latest conversation capture in `pbordjadze/painting-company`
before design work. Keep design discussions and plans in that private repository.

## Map

- **The toys and Terra** (`toys/`): the toy kit (`toys/crates/toy-kit`: the `Toy` trait, the shared
  globe, paint, sound, the headless check), the one iOS shell (`toys/ios/`), each toy's crate, and Terra, the host that
  links them. `toys/README.md` is their map and what a toy should be; `docs/kit.md` the kit's tools.
- **Terra on the web** (`toys/terra-web/`): the same library behind the iPhone shell's calls for JavaScript, and a page
  (`index.html`) doing what `main.swift` does; built by `tools/toys/web_build.sh`, pushed by CI to the `web` branch
  (CI owns it), which the owner's site serves at an unlisted path. Browsers' WebAssembly has no threads or clock:
  a toy uses `toy_kit::wall_clock()` and never spawns a thread on wasm32.
- **Adrift** (`adrift/`): glass floats on the real sea, Rust on wgpu; `adrift/README.md`, its rulings
  in `DECISIONS.md`, its sources in `SOURCES.md`.
- **The Earth's data** (`Globe/`): the coast's and borders' distance fields (`geo.png`), terrain,
  `ids.png` and `globe.json`, twelve months' ground and sea ice and the sea floor (`Globe/ground/`), the
  night's lights (`night.jpg`), the month's ocean currents (`Globe/ocean/`) and Libertinus. Adrift and
  the toys name what they borrow in their `manifest.json`. Built by `tools/globe/build.py`,
  `ground.py`, `night.py` (Natural Earth, GEBCO, NASA's Blue and Black Marble, NSIDC) and
  `tools/ocean/build.py` (the NOAA Global Drifter Program); their docstrings are the layouts. Rebuild,
  never hand-edit; the textures are read as decoded, without colour management, which would bend the
  distances.
- **Tools** (`tools/`): `tools/toys/` and `tools/kit/` (the toys and the kit), `tools/adrift/` (Adrift's
  data and studies), `tools/globe/` and `tools/ocean/` (the data above). Every one is listed in
  `docs/tools.md` or `docs/kit.md`: read them before building one.
- **CI** (`.github/workflows/`): `toys.yml` (the toys and Terra: Linux checks of every plan, then on
  `main` or with `[toys-ipa <toy>]` an IPA on macOS, released as `<toy>-build-<n>`), `adrift.yml`
  (Adrift, the same shape, `[adrift-ipa]`), and `lint.yml` (Linux, first in both; `ci/lint.sh`). Each
  regenerates the SideStore source on the `sidestore` branch after a release (`ci/sidestore_publish.sh`,
  `ci/sidestore_apps.json`); CI owns that branch. Every macOS job builds with the one Xcode `ci/xcode.sh`
  names.

## Staying current

Every version is named in one place: Rust in `rust-toolchain.toml` (`ci/rust.sh` installs it in CI), Xcode
in `ci/xcode.sh`, crates in each workspace's `Cargo.toml` with `Cargo.lock`, actions at their major tag
in the workflows, Python's in `tools/requirements.txt`; naga-cli, wasm-bindgen-cli and SwiftLint at their
install lines (`toys.yml`, `adrift.yml`, `lint.yml`).
- Dependabot (`.github/dependabot.yml`) opens one grouped pull request a week per ecosystem (cargo,
  actions, pip). Take one onto a branch with a `[toys-ipa terra]` tag in its first line, let the Linux
  checks and lint pass, then merge as usual; a wgpu major goes on its own branch (it moves toy-kit's
  iPhone limits in `budget.rs` and the launch check's naga), proven by `tools/toys/same_pictures.py`.
- By hand, about monthly: Rust's newest stable into `rust-toolchain.toml`, Xcode's into `ci/xcode.sh`,
  then `bash ci/lint.sh` and fix what a newer clippy or ruff finds.
- Lint before pushing: `bash ci/lint.sh` (`fix` formats). Formatting goes in its own commit.
- The pre-commit hook (`ci/hooks/pre-commit`: rustfmt, ruff, ShellCheck, the ratchet and silenced lints on what
  the commit changes, in seconds) is on in every Claude Code session (`.claude/settings.json`, which also
  rustfmts a Rust file as soon as it is written); elsewhere, `git config core.hooksPath ci/hooks`. Don't
  `--no-verify` past it: CI's lint job fails on the same things.
- The ratchet (`ci/ratchet.py`, in `ci/lint.sh`): no source file past 800 lines, no function past 100 lines of
  code or a complexity of 20, no type error in the Python (pyright). Split a file or function by what it does
  before it gets there. What was already past is in `ci/ceilings/` and may only get better (`--tighten` lowers a
  ceiling in the commit that improves it); raising one takes a reason on its line.
- A lint is silenced only with its reason beside it: `#[allow(...)] // why` on the same line (or the line above, where rustfmt moves it), `# noqa: CODE why`,
  and a comment line with the reason directly above a `# shellcheck disable=` or `// swiftlint:disable`; a rule
  loosened for a whole tool goes in its config file with its reason. `ci/silenced.py` (in `ci/lint.sh`) holds to it.

## Review

The same rules as the owner's other apps (painting-company's CLAUDE.md › Review, `process/critic.md`).
- **A fresh reader before every merge.** Before merging into `main`, a reviewer that didn't write the code
  reads `git diff origin/main...HEAD` (the `/code-review` skill, or a subagent given the diff and the goal,
  not the maker's reasoning). Fix what it confirms; anything large or design-level goes to the owner. The
  merge commit's message lists what the review found and what was fixed.
- **A fresh critic each build** for anything the owner will look at (a new toy or screen, visuals, measured
  performance): judged from screenshots or a clip only, not the code, with the log of earlier verdicts in
  `critics/LOG.md` on the branch. Done after 3 rounds, or two rounds running of only polish; the maker calls
  it, with the reason in the log. Bug fixes skip the critic; the reviewer is enough.

## Rules

- Public repository. Plans, briefs and the backlog live in the owner's private company repository;
  a commit message says what changed and why and quotes no one.
- No network at run time, no analytics, no accounts; nothing is a web view. The one exception to
  bundling everything is Screw's song, the reader's own file (`toys/README.md`).
- The macOS runners are shared with paint-by-number and pigeon: look at their Actions runs before
  a push, one branch at a time while paint-by-number's `main` runs.
- A tool you write to check or measure your own work (a harness, a preview, a shim) is committed
  before you finish, in `tools/` (or painting-company's when it holds anything private), its
  header saying how to run it: a scratch directory dies with its session (painting-company row 159).
- Name, icon and listing are the owner's to decide.
- Times given to the owner are Eastern (ET).
