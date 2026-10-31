#!/usr/bin/env bash
# Installs the app on a booted simulator and screenshots it: each shot is a launch, a wait and a
# screenshot.
#   ci/sim_shots.sh <TerraIncognita.app> <outdir> <udid>
# With GLOBE=1, the globe at places (held with `-globe-at`, a country lit with `-globe-select`) and by
# night; always the playground's plain globe, workbench and the delivery check (`-workbench`,
# `-prototype <id>`), unless STAGE_ONLY=1; with PLACES="name:lon,lat,zoom ...", the stage at rest at each
# (ci/fast.py: the ground's month, the clouds' drift and the sea held).
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
# (a first launch on a simulator still settling can outlast the shot's wait: one unpictured, to warm it)
run 180 xcrun simctl launch "$UDID" "$BUNDLE_ID" > /dev/null 2>&1; step "warmed ($?)"
sleep 10
shot() {
  run 60 xcrun simctl terminate "$UDID" "$BUNDLE_ID" 2>/dev/null || true
  step "launch $1: $(run 120 xcrun simctl launch "$UDID" "$BUNDLE_ID" "${@:2}" 2>&1 | tr '\n' ' ')"
  sleep "$WAIT"
  run 90 xcrun simctl io "$UDID" screenshot --type=png "$OUT/sim-$1.png" > /dev/null 2>&1; step "captured $1 ($?)"
}
WAIT=20
SINCE=$(date '+%Y-%m-%d %H:%M:%S')
# the playground: the plain globe, the workbench, prototypes on the globe
if [ -n "${PLACES:-}" ]; then
  # (as tools/metal/metal.py's `shot` draws the same names: month 10, day 0.4 of the drift, the sea at 12 s)
  for P in $PLACES; do
    NAME="${P%%:*}"
    IFS=, read -r LON LAT ZOOM <<< "${P#*:}"
    shot "stage-$NAME" -globe-at "$LON" "$LAT" "$ZOOM" -globe-month 10 -clouds-days 0.4 -ocean-time 12
  done
fi
if [ "${STAGE_ONLY:-}" != 1 ]; then
  shot plain -globe-at -40 22 1
  shot workbench -workbench
  shot hello-pack -prototype hello-pack
fi
if [ "${GLOBE:-}" = 1 ]; then
  shot globe-whole -globe-at -83 28 1
  shot globe-italy -globe-at 12 44 2.6 -globe-select Italy
  shot globe-lakes -globe-at -84 41 4.5 -globe-select "United States of America"
  shot globe-night -prototype night -globe-at 78 26 3.2 -night-time 0
fi
run 60 xcrun simctl spawn "$UDID" log show --start "$SINCE" --style compact --predicate 'process == "TerraIncognita"' > "$OUT/sim-app.log" 2>&1 || true
step done
