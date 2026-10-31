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
echo "lavapipe: $(ls /usr/share/vulkan/icd.d/lvp_icd* 2>/dev/null | head -1)"
echo "rust: $(rustc --version)"
