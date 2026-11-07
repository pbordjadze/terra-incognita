#!/usr/bin/env bash
# The standard linters over the whole repository, each with its stock configuration (loosened only where
# a rule is plainly wrong for this code, in the tool's own config file with the reason):
#   rustfmt and clippy (-D warnings)  the toys' and Adrift's workspaces (rust-toolchain.toml)
#   ruff                              the Python tools and checks (ruff.toml)
#   shellcheck                        every shell script
#   actionlint                        the workflows (.github/actionlint.yaml), with shellcheck on their scripts
#   SwiftLint                         the Swift (.swiftlint.yml), when `swiftlint` is on PATH
# ci/qa/ and qa-explore.yml are generated from painting-company's shared copy and linted there.
#
#   bash ci/lint.sh            # everything; the exit status says whether all passed
#   bash ci/lint.sh rust       # one of: rust, python, shell, actions, swift
#   bash ci/lint.sh fix        # rustfmt and ruff's safe fixes in place, then everything
# The versions are pinned: Rust by rust-toolchain.toml, ruff, shellcheck and actionlint by
# tools/requirements.txt (python3 -m venv .venv && .venv/bin/pip install -r tools/requirements.txt), SwiftLint
# by SWIFTLINT in ci.yml's lint job.
set -uo pipefail
cd "$(dirname "$0")/.."
only="${1:-all}"
failed=()
run() { local name="$1"; shift; echo "== $name"; "$@" || failed+=("$name"); }
want() { [ "$only" = all ] || [ "$only" = fix ] || [ "$only" = "$1" ]; }
[ -d .venv/bin ] && PATH="$PWD/.venv/bin:$PATH"

if [ "$only" = fix ]; then
  (cd toys && cargo fmt --all) && (cd adrift && cargo fmt --all)
  ruff check --fix .
fi

if want rust; then
  run "rustfmt (toys)" bash -c 'cd toys && cargo fmt --all -- --check'
  run "rustfmt (adrift)" bash -c 'cd adrift && cargo fmt --all -- --check'
  run "clippy (toys)" bash -c 'cd toys && cargo clippy --workspace --all-targets -- -D warnings'
  run "clippy (adrift)" bash -c 'cd adrift && cargo clippy -p adrift-core -p adrift-check --all-targets -- -D warnings'
  # the phone's shells and the web's build for iOS and wasm32 only (ci/rust.sh aarch64-apple-ios wasm32-unknown-unknown)
  run "clippy (ios)" bash -c 'cd toys && cargo clippy -p toy-kit --target aarch64-apple-ios -- -D warnings &&
    cd ../adrift && cargo clippy -p adrift-core --target aarch64-apple-ios -- -D warnings'
  run "clippy (web)" bash -c 'cd adrift && cargo clippy -p adrift-web --target wasm32-unknown-unknown -- -D warnings &&
    cd ../toys && cargo clippy -p toy-kit --no-default-features --features web --target wasm32-unknown-unknown -- -D warnings'
fi
if want python; then
  run ruff ruff check .
fi
if want shell; then
  run shellcheck bash -c "git ls-files -z '*.sh' ':!ci/qa/*' | xargs -0 shellcheck"
fi
if want actions; then
  run actionlint bash -c "actionlint \$(git ls-files '.github/workflows/*.yml' ':!.github/workflows/qa-explore.yml')"
fi
if want swift; then
  if command -v swiftlint > /dev/null; then
    run swiftlint swiftlint lint --strict --quiet
  else
    echo "== swiftlint: not on PATH, skipped (ci.yml's lint job runs it)"
  fi
fi

if [ ${#failed[@]} -gt 0 ]; then
  echo "lint: failed: ${failed[*]}"
  exit 1
fi
echo "lint: all passed"
