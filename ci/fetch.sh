#!/usr/bin/env bash
# Waits for the CI report of a commit and extracts it.
#   [CI_BRANCH=<pushed-branch>] [TIMEOUT=<minutes>] ci/fetch.sh [<commit-sha>] [<outdir>]   (defaults: HEAD, ./ci-report)
# The report (published by .github/workflows/ci.yml to the ref refs/ci-shots/<branch>, outside
# refs/heads) contains STATUS.md, logs (*.log, long ones trimmed), <job>/errors.txt, test results,
# and screenshots under */shots/*.png (ci/shots.py finds one by name).
# Polls every 30 s for up to TIMEOUT minutes (180: the macOS runners are shared with paint-by-number
# and pigeon, so a run can queue behind theirs), and every 5 minutes says how the commit's run stands
# (GitHub's API, unauthenticated: the repository is public). It stops on its own, with status 2, when
# the run has ended and the branch's report isn't this commit's: a newer push cancels a branch's
# running check, and its own report replaces this one. Run it in the background and read what it prints.
set -euo pipefail
SHA="$(git rev-parse "${1:-HEAD}")"
OUT="${2:-ci-report}"
BRANCH="${CI_BRANCH:-$(git rev-parse --abbrev-ref HEAD)}"
REF="refs/ci-shots/${BRANCH}"
REPO="${CI_REPO:-$(git remote get-url origin | sed -E 's#(\.git)?/*$##; s#.*[/:]([^/:]+/[^/]+)$#\1#')}"
POLLS=$(( ${TIMEOUT:-180} * 2 ))
# how the commit's runs stand: "<status> <conclusion> <url>" a line, newest first (nothing if unknown)
runs() {
  curl -fsS -m 20 "https://api.github.com/repos/${REPO}/actions/runs?head_sha=${SHA}&per_page=5" 2>/dev/null |
    python3 -c '
import json, sys
for r in json.load(sys.stdin).get("workflow_runs", []):
    if r.get("name") == "CI":
        print(r["status"], r.get("conclusion") or "-", r["html_url"])
' 2>/dev/null || true
}
ended=0
for i in $(seq 1 "$POLLS"); do
  if git fetch -q origin "+${REF}:${REF}" 2>/dev/null; then
    if [[ "$(git show "${REF}:COMMIT" 2>/dev/null)" == "$SHA" ]]; then
      rm -rf "$OUT" && mkdir -p "$OUT"
      git archive "${REF}" | tar -x -C "$OUT"
      cat "$OUT/STATUS.md"
      find "$OUT" -name errors.txt -size +0 -exec sh -c 'echo "== $1"; head -60 "$1"' _ {} \;
      find "$OUT" \( -name '*.png' -o -name '*.jpg' \) | sort
      exit 0
    fi
  fi
  if (( i % 10 == 1 )); then
    state="$(runs | head -1)"
    echo "$(date +%T) ${SHA:0:8} on ${BRANCH}: ${state:-no run seen yet}"
    # ended with no report for this commit: look once more in 5 minutes, then say so and stop
    if [[ "$state" == completed* ]]; then
      ended=$((ended + 1))
      if (( ended > 1 )); then
        echo "The run ended (${state}) and there is no report for ${SHA:0:8} on ${REF}: a newer push's run cancelled it or replaced its report, or it failed before the report." >&2
        exit 2
      fi
    fi
  fi
  sleep 30
done
echo "Timed out after ${TIMEOUT:-180} minutes waiting for the CI report of ${SHA:0:8} on $REF ($(runs | head -1))" >&2
exit 1
