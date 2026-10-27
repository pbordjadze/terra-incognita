#!/usr/bin/env bash
set -euo pipefail
mkdir -p out/study2-shots
rc=0
while read -r KIND UDID _ <&3; do
  RESULT="build/study2-$KIND.xcresult"
  if xcodebuild test -project "$PROJECT" -scheme "$SCHEME" -configuration Debug \
      -destination "platform=iOS Simulator,id=$UDID" -destination-timeout 180 \
      -derivedDataPath build/study2-debug -resultBundlePath "$RESULT" \
      -only-testing:WaterUITests -parallel-testing-enabled NO \
      -maximum-concurrent-test-simulator-destinations 1 \
      -test-timeouts-enabled YES -default-test-execution-time-allowance 180 \
      -maximum-test-execution-time-allowance 240 \
      ARCHS=arm64 COMPILER_INDEX_STORE_ENABLE=NO CODE_SIGNING_ALLOWED=NO \
      2>&1 | tee "out/study2-$KIND.log" | tail -90; then
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
done 3< out/sims.txt
exit "$rc"
