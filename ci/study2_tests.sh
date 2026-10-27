#!/usr/bin/env bash
set -euo pipefail
mkdir -p out/study2-shots out/study2-opening build/study2-preview
APP=build/study2-debug/Build/Products/Debug-iphonesimulator/TerraIncognita.app
rc=0
# These are this job's two simulators; only one should consume memory while testing.
while read -r KIND UDID _ <&3; do
  xcrun simctl shutdown "$UDID" >/dev/null 2>&1 || true
done 3< out/sims.txt
while read -r KIND UDID _ <&3; do
  if ! python3 - "$UDID" <<'PY' 2>&1 | tee "out/study2-boot-$KIND.log"
import subprocess,sys
try:
    subprocess.run(['xcrun','simctl','bootstatus',sys.argv[1],'-b'],check=True,timeout=480)
except (subprocess.TimeoutExpired,subprocess.CalledProcessError) as error:
    print('Simulator startup failed:',error,flush=True)
    sys.exit(1)
PY
  then
    rc=1
    xcrun simctl shutdown "$UDID" >/dev/null 2>&1 || true
    continue
  fi
  xcrun simctl install "$UDID" "$APP"
  xcrun simctl launch --terminate-running-process "$UDID" com.pbordjadze.terraincognita -water
  sleep 10
  xcrun simctl terminate "$UDID" com.pbordjadze.terraincognita
  xcrun simctl launch "$UDID" com.pbordjadze.terraincognita -water
  sleep 15
  xcrun simctl io "$UDID" screenshot "out/study2-opening/$KIND.png"
  xcrun simctl terminate "$UDID" com.pbordjadze.terraincognita
  # Same report-ref mechanism as ordinary CI, published before the interaction stage.
  cp out/study2-opening/*.png build/study2-preview/
  echo "$GITHUB_SHA" > build/study2-preview/COMMIT
  (
    cd build/study2-preview
    git init -q
    git add -A
    git -c user.name=ci -c user.email=ci@users.noreply.github.com commit -qm "Native opening views"
    git push -qf "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "HEAD:refs/ci-shots/${GITHUB_REF_NAME}-opening"
  )
  RESULT="build/study2-$KIND.xcresult"
  if xcodebuild test -project "$PROJECT" -scheme "$SCHEME" -configuration Debug \
      -destination "platform=iOS Simulator,id=$UDID" -destination-timeout 180 \
      -derivedDataPath build/study2-debug -resultBundlePath "$RESULT" \
      -only-testing:WaterUITests -parallel-testing-enabled NO \
      -maximum-concurrent-test-simulator-destinations 1 \
      -test-timeouts-enabled YES -default-test-execution-time-allowance 180 \
      -maximum-test-execution-time-allowance 240 \
      ARCHS=arm64 COMPILER_INDEX_STORE_ENABLE=NO CODE_SIGNING_ALLOWED=NO \
      2>&1 | tee "out/study2-$KIND.log" | tail -100; then
    echo "Native interactions passed: $KIND"
  else
    rc=1
  fi
  if [ -d "$RESULT" ]; then
    xcrun xcresulttool export attachments --path "$RESULT" --output-path "out/study2-shots/$KIND" || rc=1
    xcrun xcresulttool get test-results summary --path "$RESULT" > "out/study2-$KIND-summary.json" || rc=1
    cat "out/study2-$KIND-summary.json"
  else
    echo "::error::Missing test result for $KIND"
    rc=1
  fi
  xcrun simctl shutdown "$UDID" >/dev/null 2>&1 || true
done 3< out/sims.txt
exit "$rc"
