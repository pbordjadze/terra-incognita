#!/usr/bin/env bash
# Installs the app on a booted simulator and screenshots the film at moments of it.
#   ci/sim_shots.sh <TerraIncognita.app> <outdir> <udid> <t1> <t2> ...
# Each moment is a launch with `-film <T>` (the film held at T seconds; Debug builds only), a wait,
# and a screenshot; with LOOKS="id id ..." in the environment, each moment of each look (`-look`).
# Every simulator command runs under a watchdog, so one that hangs costs a minute, not the job.
set -uo pipefail
APP="$1"; OUT="$2"; UDID="$3"; shift 3
BUNDLE_ID=$(/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" "$APP/Info.plist")
mkdir -p "$OUT"
exec > >(tee -a "$OUT/sim-steps.log") 2>&1
step() { echo "$(date +%T) $*"; }
# run <seconds> <command...>: the command, killed if it takes longer
run() {
  local limit="$1"; shift
  "$@" & local pid=$!
  # (its sleep writes nowhere: left running, it would hold a caller's $(...) open to the end)
  ( sleep "$limit" > /dev/null 2>&1; kill -9 "$pid" 2>/dev/null && echo "$(date +%T) killed after ${limit} s: $*" ) & local dog=$!
  wait "$pid"; local rc=$?
  kill "$dog" 2>/dev/null; wait "$dog" 2>/dev/null
  return $rc
}
run 30 xcrun simctl boot "$UDID" 2>/dev/null || true
run 300 xcrun simctl bootstatus "$UDID" -b > /dev/null; step "booted ($?)"
run 120 xcrun simctl install "$UDID" "$APP"; step "installed ($?)"
shot() {
  run 60 xcrun simctl terminate "$UDID" "$BUNDLE_ID" 2>/dev/null || true
  step "launch $1: $(run 120 xcrun simctl launch "$UDID" "$BUNDLE_ID" "${@:2}" 2>&1 | tr '\n' ' ')"
  sleep "$WAIT"
  run 90 xcrun simctl io "$UDID" screenshot --type=png "$OUT/sim-$1.png" > /dev/null 2>&1; step "captured $1 ($?)"
}
WAIT=20
SINCE=$(date '+%Y-%m-%d %H:%M:%S')
if [ -n "${LOOKS:-}" ]; then
  for L in $LOOKS; do for T in "$@"; do shot "$L-t$(printf '%03d' "$T")" -look "$L" -film "$T"; done; done
else
  for T in "$@"; do shot "t$(printf '%03d' "$T")" -film "$T"; done
fi
run 60 xcrun simctl spawn "$UDID" log show --start "$SINCE" --style compact --predicate 'process == "TerraIncognita"' > "$OUT/sim-app.log" 2>&1 || true
step done
