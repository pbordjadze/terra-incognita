#!/usr/bin/env bash
# A toy opened in the iOS Simulator with Metal's API validation on: the launch a Linux check can't
# make. The toy's shaders go through Apple's own Metal compiler and its every Metal call through
# Apple's validation layer (MTL_DEBUG_LAYER), which aborts on a misuse the phone's driver might crash
# on, naming it. Then it is left running and is asked whether it is still alive.
#
#     bash tools/toys/sim_launch.sh <toy> [out-dir]      # macOS with Xcode and rustup's aarch64-apple-ios-sim
#
# It builds the toy for the Simulator (SIM=1 toys/ios/build.sh), boots an iPhone, installs and launches
# it, waits SECONDS_ALIVE (default 70) and fails if the toy is gone or logged a GPU error. <out-dir> keeps what it saw: the toy's
# own toy-log.txt (the kit's launch steps, its GPU errors), its stdout and stderr (Metal's validation
# messages), any crash report, and screenshots: `screen.png` at rest, then the shell's rules card
# opened (TOY_OPEN, main.swift) at 30 s as `rules.png` and, if the toy has one, its origins card at
# 60 s as `origins.png`. (A hosted Mac's first `simctl io screenshot` took 140 s and later ones up to
# 22 s, so the Simulator's screen is warmed up first and the cards are 30 s apart.) A toy may keep a
# `toys/<toy>/sim.env` of KEY=value lines, passed to the app (Terra's TERRA_PASSAGE plays a passage into a
# held toy and out); SIM_DEVICE=iPad (the environment, or that file) opens it on an iPad instead; SIM_ALSO=iPad there
# has CI's IPA job open it on an iPad as well (out/sim-ipad).
# What it can't vouch for: the phone's memory limit
# (the Simulator has none of its own) and the phone's GPU (the Mac's draws for it): toy-log.txt from a
# phone stays the last word.
set -euo pipefail
TOY="${1:?usage: sim_launch.sh <toy> [out-dir]}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${2:-$ROOT/toys/build/$TOY/sim}")"
WAIT="${SECONDS_ALIVE:-70}"
mkdir -p "$OUT"
trap 'rm -rf "$OUT/build"' EXIT     # (the built app: only what was seen is kept)
SIM=1 bash "$ROOT/toys/ios/build.sh" "$TOY" "$OUT/build" | tail -5
APP="$(ls -d "$OUT"/build/Payload/*.app)"
EXEC="$(basename "$APP" .app)"
BUNDLE="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['bundleIdentifier'])" "$ROOT/toys/$TOY/toy.json")"

# the toy's own environment for the app (sim.env), and the device
SIM_ENV="$ROOT/toys/$TOY/sim.env"
if [ -f "$SIM_ENV" ]; then
  while IFS='=' read -r k v; do
    case "$k" in ''|\#*|SIM_ALSO) continue ;; SIM_DEVICE) SIM_DEVICE="${SIM_DEVICE:-$v}" ;; *) export "SIMCTL_CHILD_$k=$v" ;; esac
  done < "$SIM_ENV"
fi
DEVICE="${SIM_DEVICE:-iPhone}"
# an iPhone (or the device asked for), booted once and reused by the next toy in this job: the plainest
# of it first (an "iPhone 17 Pro", an "iPad Pro 13-inch"; not Xcode 27's two-screened "iPhone Duo",
# whose screenshots came back empty), on the newest runtime that has one
UDID="$(xcrun simctl list devices available -j | DEVICE="$DEVICE" python3 -c '
import json, os, re, sys
d = json.load(sys.stdin)["devices"]
D = re.escape(os.environ["DEVICE"])
runtimes = sorted((r for r in d if "iOS" in r), key=lambda r: [int(x) for x in r.rsplit("iOS-", 1)[-1].split("-") if x.isdigit()])
for tier in (D + r" \d+ Pro$", D + r" Pro\b", D + r" \d", D):
    found = None
    for r in reversed(runtimes):
        devs = [x for x in d[r] if re.match(tier, x["name"])]
        if devs:
            found = ([x for x in devs if x["state"] == "Booted"] or devs)[0]; break
    if found:
        print(found["udid"]); break
')"
[ -n "$UDID" ] || { echo "no $DEVICE simulator"; exit 1; }
xcrun simctl list devices | grep "$UDID" | sed 's/^ */simulator: /'
xcrun simctl boot "$UDID" 2> /dev/null || true
xcrun simctl bootstatus "$UDID" -b > /dev/null
xcrun simctl uninstall "$UDID" "$BUNDLE" 2> /dev/null || true
xcrun simctl install "$UDID" "$APP"

# Metal's validation layer and the shell's environment for the toy (SIMCTL_CHILD_ passes it through)
export SIMCTL_CHILD_MTL_DEBUG_LAYER=1
export SIMCTL_CHILD_MTL_DEBUG_LAYER_ERROR_MODE=assert
export SIMCTL_CHILD_RUST_BACKTRACE=1
# (the rules card, then the origins card, open by themselves near the end, for their screenshots: the
# iOS text a Linux shot lacks; a toy without origins keeps showing its rules)
export SIMCTL_CHILD_TOY_OPEN=rules,origins SIMCTL_CHILD_TOY_OPEN_AFTER=30 SIMCTL_CHILD_TOY_OPEN_EVERY=30
xcrun simctl io "$UDID" screenshot /dev/null > /dev/null 2>&1 || true     # (the warm-up)
before="$(ls ~/Library/Logs/DiagnosticReports 2> /dev/null | grep -c "$EXEC" || true)"
xcrun simctl launch --terminate-running-process --stdout="$OUT/stdout.txt" --stderr="$OUT/stderr.txt" "$UDID" "$BUNDLE"
alive=1
# (by the clock, not by passes: on a busy hosted Mac a pass of this loop took 5 s, not 1)
start=$(date +%s); shot_screen=0; shot_rules=0
while :; do
  sleep 1
  i=$(( $(date +%s) - start ))
  # (a Simulator app is a process of the Mac's own: its executable's path names it)
  if ! pgrep -f "/$EXEC.app/$EXEC" > /dev/null; then alive=0; echo "gone after ${i} s"; break; fi
  if [ "$shot_screen" = 0 ] && [ "$i" -ge 3 ]; then shot_screen=1; xcrun simctl io "$UDID" screenshot "$OUT/screen.png" > /dev/null 2>> "$OUT/screenshots.txt" || echo "no screen.png" >> "$OUT/screenshots.txt"; fi
  if [ "$shot_rules" = 0 ] && [ "$i" -ge 33 ]; then shot_rules=1; xcrun simctl io "$UDID" screenshot "$OUT/rules.png" > /dev/null 2>> "$OUT/screenshots.txt" || echo "no rules.png" >> "$OUT/screenshots.txt"; fi
  [ "$i" -ge "$WAIT" ] && break
done
[ "$alive" = 1 ] && [ "$WAIT" -ge 63 ] && { xcrun simctl io "$UDID" screenshot "$OUT/origins.png" > /dev/null 2>> "$OUT/screenshots.txt" || echo "no origins.png" >> "$OUT/screenshots.txt"; }
DATA="$(xcrun simctl get_app_container "$UDID" "$BUNDLE" data 2> /dev/null || true)"
[ -n "$DATA" ] && cp "$DATA/Documents/toy-log.txt" "$OUT/toy-log.txt" 2> /dev/null || true
ls -t ~/Library/Logs/DiagnosticReports 2> /dev/null | grep "$EXEC" | head -n $(( $(ls ~/Library/Logs/DiagnosticReports 2> /dev/null | grep -c "$EXEC" || true) - before )) |
  while read -r f; do cp ~/Library/Logs/DiagnosticReports/"$f" "$OUT/"; echo "crash report: $f"; done || true
xcrun simctl terminate "$UDID" "$BUNDLE" 2> /dev/null || true
echo "== toy-log.txt"; cat "$OUT/toy-log.txt" 2> /dev/null || echo "(none)"
# a GPU error the toy only logged (a pipeline Apple's compiler refused draws nothing on the phone)
if grep -q "gpu: " "$OUT/toy-log.txt" 2> /dev/null && grep -q -E "gpu: .*(Error|error|lost)" "$OUT/toy-log.txt"; then
  echo "FAIL  $TOY logged a GPU error in the Simulator (toy-log.txt above)"; gpu_error=1
fi
echo "== stderr (last 40 lines)"; tail -40 "$OUT/stderr.txt" 2> /dev/null || true
if [ "${gpu_error:-0}" = 1 ]; then
  exit 1
elif [ "$alive" = 1 ]; then
  echo "ok    $TOY alive in the Simulator after ${WAIT} s, Metal validation on"
else
  echo "FAIL  $TOY closed in the Simulator (Metal validation on): toy-log.txt and stderr above say where"
  exit 1
fi
