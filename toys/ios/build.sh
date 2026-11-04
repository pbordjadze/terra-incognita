#!/usr/bin/env bash
# Builds one toy for the device without an Xcode project: its Rust crate as a static archive for
# aarch64-apple-ios, the shared Swift shell (toys/ios/Sources) linked against it, the toy's data
# bundled, an unsigned .app in Payload/ zipped into <Name>.ipa (SideStore signs it when installing).
# The toy's name and bundle id come from toys/<toy>/toy.json. macOS with Xcode's command-line tools
# and rustup (`rustup target add aarch64-apple-ios`).
#
#     BUILD=<n> bash toys/ios/build.sh <toy> [out-dir]     # <out>/<toy>.ipa; version 1.0.<n>, build <n>
#     SIM=1 bash toys/ios/build.sh <toy> [out-dir]         # <out>/Payload/<Exec>.app for the iOS Simulator
#                                                          # (ad-hoc signed; tools/toys/sim_launch.sh runs it)
set -euo pipefail
TOY="${1:?usage: build.sh <toy> [out-dir]}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${2:-$ROOT/toys/build/$TOY/ios}")"
BUILD="${BUILD:-1}"
MIN_IOS=17.0
meta() { python3 -c "import json,sys; print(json.load(open('$ROOT/toys/$TOY/toy.json'))[sys.argv[1]])" "$1"; }
NAME="$(meta name)"; BUNDLE="$(meta bundleIdentifier)"
EXEC="$(echo "$TOY" | python3 -c "import sys; print(sys.stdin.read().strip().title().replace('-', ''))")"
SIM="${SIM:-0}"
if [ "$SIM" = 1 ]; then
  RTARGET=aarch64-apple-ios-sim; SDKNAME=iphonesimulator; STARGET="arm64-apple-ios$MIN_IOS-simulator"
else
  RTARGET=aarch64-apple-ios; SDKNAME=iphoneos; STARGET="arm64-apple-ios$MIN_IOS"
fi
SDK="$(xcrun --sdk $SDKNAME --show-sdk-path)"
export SDKROOT="$SDK"
mkdir -p "$OUT"
cd "$ROOT/toys"
echo "== rust: $TOY ($RTARGET, release)"
cargo build --release --target $RTARGET -p "$TOY" --lib
LIB="target/$RTARGET/release/lib$(echo "$TOY" | tr - _).a"
ls -l "$LIB"
APP="$OUT/Payload/$EXEC.app"
rm -rf "$OUT/Payload"
mkdir -p "$APP/data"
echo "== swift"
swiftc -O -target $STARGET -sdk "$SDK" \
  -I "$ROOT/toys/ios/Sources" -import-objc-header "$ROOT/toys/ios/Sources/toy.h" \
  -parse-as-library -module-name "$EXEC" \
  "$ROOT/toys/ios/Sources/main.swift" "$LIB" \
  -framework UIKit -framework QuartzCore -framework Metal -framework CoreGraphics -framework Foundation \
  -framework AVFoundation \
  -lc++ -o "$APP/$EXEC"
echo "== bundle"
sed -e "s/TOY_VERSION/1.0.$BUILD/" -e "s/TOY_BUILD/$BUILD/" -e "s/TOY_BUNDLE/$BUNDLE/" \
    -e "s/TOY_EXEC/$EXEC/" \
    -e "s/TOY_NAME/$NAME/g" "$ROOT/toys/ios/Info.plist" > "$APP/Info.plist"
plutil -convert binary1 "$APP/Info.plist"
ICON="$ROOT/toys/$TOY/icon.png"     # 1024 px, drawn from code (tools/toys/icon.py, or the toy's own)
sips -z 120 120 "$ICON" --out "$APP/AppIcon60x60@2x.png" > /dev/null
sips -z 180 180 "$ICON" --out "$APP/AppIcon60x60@3x.png" > /dev/null
sips -z 152 152 "$ICON" --out "$APP/AppIcon76x76@2x.png" > /dev/null
python3 "$ROOT/tools/toys/assets.py" "$TOY" "$APP/data" > /dev/null
echo "APPL????" > "$APP/PkgInfo"
if [ "$SIM" = 1 ]; then
  # the Simulator runs only signed apps; an ad-hoc signature does
  codesign --force --sign - "$APP" > /dev/null
  du -sh "$APP" | cut -f1 | xargs echo "app (simulator):"
  exit 0
fi
rm -f "$OUT/$TOY.ipa"
(cd "$OUT" && zip -qry "$TOY.ipa" Payload)
ls -l "$OUT/$TOY.ipa"
du -sh "$APP" | cut -f1 | xargs echo "app:"
