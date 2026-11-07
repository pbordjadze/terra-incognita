#!/usr/bin/env bash
# Prepares a Linux machine (or CI runner) to build and check Adrift headless: Mesa's Vulkan software
# driver (lavapipe), which wgpu renders on without a GPU, and the wasm target for the web build.
#
#     bash tools/adrift/linux-setup.sh
set -euo pipefail
if ! ls /usr/share/vulkan/icd.d/lvp_icd* > /dev/null 2>&1; then
  export DEBIAN_FRONTEND=noninteractive
  SUDO=""
  if [ "$(id -u)" != 0 ] && command -v sudo > /dev/null; then SUDO="sudo -n"; fi
  $SUDO apt-get update -qq
  $SUDO apt-get install -y -qq mesa-vulkan-drivers > /dev/null
fi
rustup target add wasm32-unknown-unknown > /dev/null 2>&1 || true
# naga for tools/adrift/wgsl_check.sh, and wasm-bindgen for tools/adrift/web-build.sh (R6: both were
# missing on a fresh container; a few minutes of compiling the first time, nothing after)
command -v naga > /dev/null || cargo install -q naga-cli --version 30.0.1 --locked > /dev/null 2>&1 || echo "naga-cli: install failed (wgsl_check.sh needs it)"
command -v wasm-bindgen > /dev/null || cargo install -q wasm-bindgen-cli --version 0.2.130 > /dev/null 2>&1 || echo "wasm-bindgen-cli: install failed (web-build.sh needs it)"
lvp=(/usr/share/vulkan/icd.d/lvp_icd*)
echo "lavapipe: ${lvp[0]}"
echo "rust: $(rustc --version)"
