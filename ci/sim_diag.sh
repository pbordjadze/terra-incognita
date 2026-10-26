#!/usr/bin/env bash
# Finds what stalls the page on the simulator: launches the app once per mode (`full`, or a part
# of the page left out, `no2d` or `nogl`: Web/index.html), lets it play, screenshots it, and
# keeps the page's start-up reports with WebKit's word on its GPU process, summed up in diag.txt.
#   ci/sim_diag.sh <TerraIncognita.app> <outdir> <udid> <mode> ...
# Every simulator command runs under a watchdog, as in ci/sim_shots.sh.
set -uo pipefail
APP="$1"; OUT="$2"; UDID="$3"; shift 3
BUNDLE_ID=$(/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" "$APP/Info.plist")
mkdir -p "$OUT"
exec > >(tee -a "$OUT/diag-steps.log") 2>&1
step() { echo "$(date +%T) $*"; }
run() {
  local limit="$1"; shift
  "$@" & local pid=$!
  ( sleep "$limit"; kill -9 "$pid" 2>/dev/null && echo "$(date +%T) killed after ${limit} s: $*" ) & local dog=$!
  wait "$pid"; local rc=$?
  kill "$dog" 2>/dev/null; wait "$dog" 2>/dev/null
  return $rc
}
run 30 xcrun simctl boot "$UDID" 2>/dev/null || true
run 300 xcrun simctl bootstatus "$UDID" -b > /dev/null; step "booted ($?)"
run 120 xcrun simctl install "$UDID" "$APP"; step "installed ($?)"
for MODE in "$@"; do
  run 60 xcrun simctl terminate "$UDID" "$BUNDLE_ID" 2>/dev/null || true
  START=$(date '+%Y-%m-%d %H:%M:%S')
  step "launch $MODE: $(run 120 xcrun simctl launch "$UDID" "$BUNDLE_ID" -diag "$MODE" 2>&1 | tr '\n' ' ')"
  sleep 60
  run 90 xcrun simctl io "$UDID" screenshot --type=png "$OUT/diag-$MODE.png" > /dev/null 2>&1; step "captured $MODE ($?)"
  run 120 xcrun simctl spawn "$UDID" log show --start "$START" --style compact \
    --predicate 'subsystem == "com.pbordjadze.terraincognita" OR eventMessage CONTAINS "became unresponsive" OR eventMessage CONTAINS "gpuProcessExited"' \
    > "$OUT/diag-$MODE.log" 2>&1 || true
  { echo "== $MODE"
    awk '/terraincognita:page\]|became unresponsive|gpuProcessExited/ { t = $2; sub(/.*\] /, ""); print t, $0 }' "$OUT/diag-$MODE.log" | cut -c1-170 | head -80
  } >> "$OUT/diag.txt"
done
step done
