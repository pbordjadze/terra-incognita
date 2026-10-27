#!/usr/bin/env bash
# Real UI interactions in the same macOS job as the ordinary builds. Each simulator runs
# sequentially; screenshots are XCTest attachments, exported for the normal CI report.
set -euo pipefail
mkdir -p out/encounter-shots
rc=0
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
  if xcodebuild test -project "$PROJECT" -scheme "$SCHEME" -configuration Debug \
      -destination "platform=iOS Simulator,id=$UDID" -destination-timeout 180 \
      -derivedDataPath build/dd -resultBundlePath "$RESULT" \
      -only-testing:TerraIncognitaUITests -parallel-testing-enabled NO \
      -maximum-concurrent-test-simulator-destinations 1 -quiet \
      ARCHS=arm64 COMPILER_INDEX_STORE_ENABLE=NO CODE_SIGNING_ALLOWED=NO \
      2>&1 | tee "out/encounter-$KIND.log" | tail -80; then
    echo "UI interactions passed: $KIND"
  else
    rc=1
  fi
  if [ -d "$RESULT" ]; then
    xcrun xcresulttool export attachments --path "$RESULT" \
      --output-path "out/encounter-shots/$KIND" || rc=1
    xcrun xcresulttool get test-results summary --path "$RESULT" \
      > "out/encounter-$KIND-summary.json" || rc=1
  else
    echo "::error::No UI test result for $KIND"
    rc=1
  fi
  shutdown_simulator
done 3< out/sims.txt
exit "$rc"
