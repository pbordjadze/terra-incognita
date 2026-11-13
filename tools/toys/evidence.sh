#!/usr/bin/env bash
# Plays every plan of a toy and films each, into one directory a critic is pointed at (with the plans
# beside them, and LOG.md of earlier verdicts if given): the evidence of one critic round.
#
#     bash tools/toys/evidence.sh <toy> <out-dir> [LOG.md]
set -euo pipefail
toy=$1
out=$(realpath -m "$2")
log=${3:+$(realpath "$3")}
here=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$out/plans"
cd "$here/toys"
cargo build --release -q -p "$toy" --bin "$toy-check"
for plan in "$toy"/plans/*.json; do
  name=$(basename "$plan" .json)
  rm -rf "${out:?}/${name:?}"
  cp "$plan" "$out/plans/"
  "target/release/$toy-check" play "$plan" "$out/$name" | tail -1
  if [ -d "$out/$name/film" ]; then
    bash "$here/tools/toys/film.sh" "$out/$name" >/dev/null
  fi
done
if [ -n "$log" ]; then
  cp "$log" "$out/LOG.md"
fi
