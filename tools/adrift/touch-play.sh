#!/usr/bin/env bash
# Plays every touch plan (adrift/crates/adrift-core/src/touch/plans/*.json) on the real globe with
# adrift-check and reports which passed their expectations: one process (`play-all`), the plans
# N at a time, each one's lines in <out-dir>/<name>.log and its shots in <out-dir>/<name>/.
# Minutes on Linux's software Vulkan.
#
#     bash tools/adrift/touch-play.sh [out-dir] [-j N]   # default adrift/build/out/motion, N the cores (at most 4)
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT=${1:-adrift/build/out/motion}
shift || true
mkdir -p "$OUT"
case "$OUT" in /*) ;; *) OUT="../$OUT" ;; esac   # as adrift/ sees it
python3 tools/adrift/assets.py adrift/build/assets --link > /dev/null
(cd adrift && cargo build -q -p adrift-check)
# prints `ok   <name>`, or `FAIL <name>` and its first failing lines; non-zero if any failed
cd adrift && ./target/debug/adrift-check play-all crates/adrift-core/src/touch/plans "$OUT" "$@"
