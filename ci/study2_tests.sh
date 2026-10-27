#!/usr/bin/env bash
set -euo pipefail
mkdir -p out/study2-shots
while read -r KIND UDID _ <&3; do
  RESULT="build/study2-$KIND.xcresult"
  xcodebuild test -project "$PROJECT" -scheme "$SCHEME" -configuration Debug \
    -destination "platform=iOS Simulator,id=$UDID" -destination-timeout 180 \
    -derivedDataPath build/study2-debug -resultBundlePath "$RESULT" \
    -only-testing:WaterUITests -parallel-testing-enabled NO \
    -maximum-concurrent-test-simulator-destinations 1 -quiet \
    ARCHS=arm64 COMPILER_INDEX_STORE_ENABLE=NO CODE_SIGNING_ALLOWED=NO \
    2>&1 | tee "out/study2-$KIND.log" | tail -90
  xcrun xcresulttool export attachments --path "$RESULT" --output-path "out/study2-shots/$KIND"
  xcrun xcresulttool get test-results summary --path "$RESULT" > "out/study2-$KIND-summary.json"
done 3< out/sims.txt
