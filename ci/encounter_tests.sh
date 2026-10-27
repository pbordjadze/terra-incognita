#!/usr/bin/env bash
# Real UI interactions in the same macOS job as the ordinary builds. Each simulator runs
# sequentially; screenshots are XCTest attachments, exported for the normal CI report.
set -euo pipefail
mkdir -p out/encounter-shots
rc=0
while read -r KIND UDID _ <&3; do
  RESULT="build/encounter-$KIND.xcresult"
  if xcodebuild test -project "$PROJECT" -scheme "$SCHEME" -configuration Debug \
      -destination "platform=iOS Simulator,id=$UDID" -destination-timeout 180 \
      -derivedDataPath build/dd -resultBundlePath "$RESULT" \
      -only-testing:"${UI_TEST_TARGET:-TerraIncognitaUITests}" -parallel-testing-enabled NO \
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
done 3< out/sims.txt
exit "$rc"
