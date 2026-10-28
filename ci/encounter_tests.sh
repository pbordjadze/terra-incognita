#!/usr/bin/env bash
# Real UI interactions in the same macOS job as the ordinary builds. A failed test is run once
# more (a busy shared simulator sometimes times out launching the app); failing twice fails the job. Each simulator runs
# sequentially; screenshots are XCTest attachments, exported for the normal CI report, with each
# test's duration (encounter-<device>-durations.txt).
set -euo pipefail
mkdir -p out/encounter-shots
rc=0
# UI_TEST_TARGET: one or more test classes, space-separated (default: the whole UI test target)
ONLY=()
for target in ${UI_TEST_TARGET:-TerraIncognitaUITests}; do ONLY+=("-only-testing:$target"); done
active_simulator=""
shutdown_simulator() {
  [ -n "$active_simulator" ] || return 0
  python3 - "$active_simulator" <<'PY' || true
import subprocess, sys
try:
    subprocess.run(["xcrun", "simctl", "shutdown", sys.argv[1]], timeout=60, check=False)
except subprocess.TimeoutExpired:
    print("Simulator shutdown exceeded 60 seconds", file=sys.stderr)
PY
  active_simulator=""
}
trap shutdown_simulator EXIT
# xcodebuild test on the active simulator: <result bundle> <log> <-only-testing:…>…
run_tests() {
  local result="$1" log="$2"
  shift 2
  xcodebuild test -project "$PROJECT" -scheme "$SCHEME" -configuration Debug \
      -destination "platform=iOS Simulator,id=$UDID" -destination-timeout 180 \
      -derivedDataPath build/dd -resultBundlePath "$result" \
      "$@" -parallel-testing-enabled NO \
      -retry-tests-on-failure -test-iterations 2 \
      -maximum-concurrent-test-simulator-destinations 1 -quiet \
      ARCHS=arm64 COMPILER_INDEX_STORE_ENABLE=NO CODE_SIGNING_ALLOWED=NO \
      2>&1 | tee "$log" | tail -80
}
# The result's screenshots, summary and durations into the report.
report() {
  local result="$1" name="$2"
  xcrun xcresulttool export attachments --path "$result" --output-path "out/encounter-shots/$name" || rc=1
  xcrun xcresulttool get test-results summary --path "$result" > "out/encounter-$name-summary.json" || rc=1
  xcrun xcresulttool get test-results tests --path "$result" 2>/dev/null | python3 -c '
import json, sys
rows = []
def walk(node):
    if node.get("nodeType") == "Test Case":
        rows.append((node.get("durationInSeconds", 0), node.get("duration", ""), node.get("result", ""), node.get("nodeIdentifier", node.get("name", ""))))
    for child in node.get("children", []):
        walk(child)
for node in json.load(sys.stdin).get("testNodes", []):
    walk(node)
for seconds, shown, result, name in sorted(rows, reverse=True):
    print(f"{shown:>9}  {result:<8} {name}")
' > "out/encounter-$name-durations.txt" || true
}
# XCTest runs a failed test again itself, but not one whose test runner died under it: those (only
# when they are all the failures) get their one more run in a fresh runner. Prints their -only-testing.
RUNNER_DIED='
import json, re, sys
failures = json.load(sys.stdin).get("testFailures", [])
died = re.compile(r"test runner (exited|crashed|never began)|lost connection to the test", re.I)
if failures and all(died.search(f.get("failureText", "")) for f in failures):
    for name in sorted({f["targetName"] + "/" + f["testIdentifierString"].removesuffix("()") for f in failures}):
        print("-only-testing:" + name)
'
while read -r KIND UDID _ <&3; do
  # Compilation finishes before any simulator boots. Keep only the device being tested
  # alive, so two iOS runtimes do not compete with builds or each other's interactions.
  active_simulator="$UDID"
  if ! python3 - "$UDID" > "out/boot-$KIND.log" 2>&1 <<'PY'
import subprocess, sys
udid = sys.argv[1]
# An existing boot is harmless; bootstatus is the definitive readiness check.
subprocess.run(["xcrun", "simctl", "boot", udid], timeout=60, check=False)
subprocess.run(["xcrun", "simctl", "bootstatus", udid, "-b"], timeout=300, check=True)
PY
  then
    echo "::error::Simulator boot failed for $KIND"
    cat "out/boot-$KIND.log"
    rc=1
    shutdown_simulator
    continue
  fi
  RESULT="build/encounter-$KIND.xcresult"
  if run_tests "$RESULT" "out/encounter-$KIND.log" "${ONLY[@]}"; then
    echo "UI interactions passed: $KIND"
  else
    RERUN=()
    if [ -d "$RESULT" ]; then
      while read -r arg; do
        [ -n "$arg" ] && RERUN+=("$arg")
      done < <(xcrun xcresulttool get test-results summary --path "$RESULT" | python3 -c "$RUNNER_DIED")
    fi
    if [ "${#RERUN[@]}" -gt 0 ]; then
      echo "::warning::$KIND: the test runner died during ${RERUN[*]#-only-testing:}; running it once more"
      if run_tests "build/encounter-$KIND-rerun.xcresult" "out/encounter-$KIND-rerun.log" "${RERUN[@]}"; then
        echo "UI interactions passed: $KIND (after the rerun)"
      else
        rc=1
      fi
      if [ -d "build/encounter-$KIND-rerun.xcresult" ]; then report "build/encounter-$KIND-rerun.xcresult" "$KIND-rerun"; fi
    else
      rc=1
    fi
  fi
  if [ -d "$RESULT" ]; then
    report "$RESULT" "$KIND"
  else
    echo "::error::No UI test result for $KIND"
    rc=1
  fi
  shutdown_simulator
done 3< out/sims.txt
exit "$rc"
