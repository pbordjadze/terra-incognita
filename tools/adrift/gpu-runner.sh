#!/usr/bin/env bash
# Adrift's GPU runner: on a machine with a real GPU (Vulkan), checks out a named branch of this
# repository, runs only the repository's own checks (tests, every scene rendered, every plan
# played, the measured scrubs' frame timings, the take's export), and prints one compact text
# summary at the end (between `== SUMMARY` and `== END`), which is what a runner session sends back:
# pass/fail, the GPU, and the frame timings. The PNGs and mp4s stay in <workdir>/out/<branch>.
# Nothing else runs: no network beyond git fetch, no code but the repo's own.
#
#     bash tools/adrift/gpu-runner.sh <branch> [workdir]       # one run, the summary at the end
#     GPU_RUNNER_PUSH=1 bash tools/adrift/gpu-runner.sh <branch> # also push the results to
#                                                                # gpu-results/<branch> (needs push access)
#     bash tools/adrift/gpu-runner.sh --watch [workdir]         # every minute: run any changed
#                                                                # gpu-request/* branch
# Needs: git with push access to origin, rustup (stable), ffmpeg, python3 with Pillow, a Vulkan GPU
# driver. The software driver works too (it is what CI uses), only slower: set ADRIFT_SOFTWARE=1 to
# install it with tools/adrift/linux-setup.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MODE="${1:?usage: gpu-runner.sh <branch> | --watch [workdir]}"
WORK="${2:-$HOME/adrift-gpu-runner}"
mkdir -p "$WORK"

run_one() {
  local branch="$1" name="${1#gpu-request/}"
  local src="$WORK/src" out="$WORK/out/$name"
  if [ ! -d "$src/.git" ]; then git clone -q "$(git -C "$ROOT" remote get-url origin)" "$src"; fi
  git -C "$src" fetch -q origin "$branch"
  git -C "$src" checkout -q --detach FETCH_HEAD
  local sha; sha=$(git -C "$src" rev-parse --short HEAD)
  rm -rf "$out"; mkdir -p "$out"
  echo "== $branch at $sha ($(date -u +%H:%M:%S) UTC)" | tee "$out/run.log"
  (
    cd "$src"
    if [ "${ADRIFT_SOFTWARE:-0}" = 1 ]; then bash tools/adrift/linux-setup.sh; fi
    python3 tools/adrift/assets.py adrift/build/assets --link > /dev/null
    cd adrift
    cargo test --release -q -p adrift-core 2>&1 | grep -E "test result|FAILED|panicked" || true
    cargo build --release -q -p adrift-check
    # the adapter wgpu itself chose (a laptop's discrete GPU, not the first device vulkaninfo lists)
    cargo run --release -q -p adrift-check -- info 2>&1 | grep "^adapter:" || echo "adapter: none found"
    for s in crates/adrift-check/scenes/*.json crates/adrift-core/src/look/scenes/*.json; do
      [ -f "$s" ] || continue
      cargo run --release -q -p adrift-check -- render "$s" "$out/scenes/$(basename "$s" .json).png" --size 1179x2556 --ppp 3 2>&1 | grep -E "drawn|rror" || true
    done
    for p in crates/adrift-check/plans/*.json crates/adrift-core/src/touch/plans/*.json crates/adrift-core/src/look/plans/*.json crates/adrift-core/src/sea/plans/*.json; do
      [ -f "$p" ] || continue
      grep -q '"steps"' "$p" || continue
      if cargo run --release -q -p adrift-check -- play "$p" "$out/plans/$(basename "$p" .json)" > "$out/plans-$(basename "$p" .json).log" 2>&1; then
        echo "ok   $(basename "$p")"
      else
        echo "FAIL $(basename "$p")"; tail -3 "$out/plans-$(basename "$p" .json).log"
      fi
      grep -hE "scrub .* frames|export " "$out/plans-$(basename "$p" .json).log" || true
    done
  ) 2>&1 | tee -a "$out/run.log"
  # keep the results small: the PNGs at half size where they are big, the mp4s as they are
  python3 - "$out" <<'PY' || true
import sys, pathlib
from PIL import Image
for p in pathlib.Path(sys.argv[1]).rglob("*.png"):
    im = Image.open(p)
    if im.width > 800:
        im.resize((im.width // 2, im.height // 2)).save(p)
PY
  # the summary: what a runner session sends back as text
  {
    echo "== SUMMARY $branch $sha"
    grep -E "^adapter:" "$out/run.log" | head -1
    grep -E "test result" "$out/run.log" | head -1
    echo "plans: $(grep -c '^ok ' "$out/run.log") ok, $(grep -c '^FAIL ' "$out/run.log") failed"
    grep '^FAIL ' "$out/run.log" || true
    grep -hE "scrub .* frames|export " "$out/run.log" | sed 's/^ *[0-9]* t= *[0-9.]*s //' || true
    echo "== END"
  } | tee "$out/summary.txt"
  [ "${GPU_RUNNER_PUSH:-0}" = 1 ] || return 0
  # one commit on gpu-results/<name>
  local res="$WORK/results"
  rm -rf "$res"; git -C "$src" worktree prune
  git -C "$src" worktree add -q --detach "$res" HEAD
  ( cd "$res" && git checkout -q --orphan "gpu-results/$name" && git rm -rqf . && cp -r "$out"/. . && git add -A && \
    git -c user.name="Adrift GPU runner" -c user.email="adrift-gpu-runner@localhost" commit -qm "GPU results for $branch at $sha [skip ci]" && \
    git push -qf origin "HEAD:refs/heads/gpu-results/$name" )
  git -C "$src" worktree remove --force "$res"
  echo "== pushed gpu-results/$name ($sha)"
}

if [ "$MODE" = "--watch" ]; then
  declare -A last=()
  while true; do
    while read -r sha ref; do
      b="${ref#refs/heads/}"
      if [ "${last[$b]:-}" != "$sha" ]; then
        run_one "$b" || echo "run of $b failed"
        last[$b]="$sha"
      fi
    done < <(git -C "$ROOT" ls-remote origin 'refs/heads/gpu-request/*')
    sleep 60
  done
else
  run_one "$MODE"
fi
