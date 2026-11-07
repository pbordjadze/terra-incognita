#!/usr/bin/env bash
# The Rust that rust-toolchain.toml names (with rustfmt and clippy), and the targets a job asks for,
# so every job builds with the one version named there and nowhere else (CLAUDE.md › Staying current).
# Installs rustup first on a machine without it.
#   bash ci/rust.sh                                  # a Linux job
#   bash ci/rust.sh aarch64-apple-ios wasm32-unknown-unknown
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v rustup > /dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
  echo "$HOME/.cargo/bin" >> "${GITHUB_PATH:-/dev/null}"
  export PATH="$HOME/.cargo/bin:$PATH"
fi
rustup toolchain install --profile minimal   # reads rust-toolchain.toml
if [ "$#" -gt 0 ]; then rustup target add "$@"; fi
rustc --version
cargo --version
