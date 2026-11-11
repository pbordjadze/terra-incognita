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
#     STORE=1 VERSION=1.0 BUILD_NUMBER=7.1 bash toys/ios/build.sh <toy> [out-dir]
#         # the same app as App Store Connect wants it (.github/workflows/testflight.yml; painting-company
#         # row 312): its icon compiled into an asset catalog (Assets.car, the 1024 px App Store icon in it),
#         # the SDK and Xcode it was built with in Info.plist (DT*), the version and build as given; still
#         # unsigned (the workflow's export signs it), and no .ipa: <out>/Payload/<Exec>.app
# Every app carries toys/ios/PrivacyInfo.xcprivacy, the privacy manifest.
set -euo pipefail
TOY="${1:?usage: build.sh <toy> [out-dir]}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${2:-$ROOT/toys/build/$TOY/ios}")"
BUILD="${BUILD:-1}"
MIN_IOS=17.0
meta() { python3 -c "import json,sys; print(json.load(open('$ROOT/toys/$TOY/toy.json'))[sys.argv[1]])" "$1"; }
NAME="$(meta name)"; BUNDLE="$(meta bundleIdentifier)"
# a toy.json's optional versionBase lifts its version and build above an app it replaces (Terra's
# bundle id was built by Xcode as 1.0.<run>, with run numbers in the hundreds)
BASE="$(python3 -c "import json; print(json.load(open('$ROOT/toys/$TOY/toy.json')).get('versionBase', 0))")"
BUILD=$((BUILD + BASE))
EXEC="$(echo "$TOY" | python3 -c "import sys; print(sys.stdin.read().strip().title().replace('-', ''))")"
SIM="${SIM:-0}"
STORE="${STORE:-0}"
VERSION="${VERSION:-1.0.$BUILD}"
BUILD_NUMBER="${BUILD_NUMBER:-$BUILD}"
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
sed -e "s/TOY_VERSION/$VERSION/" -e "s/TOY_BUILD/$BUILD_NUMBER/" -e "s/TOY_BUNDLE/$BUNDLE/" \
    -e "s/TOY_EXEC/$EXEC/" \
    -e "s/TOY_NAME/$NAME/g" "$ROOT/toys/ios/Info.plist" > "$APP/Info.plist"
if [ "$STORE" = 1 ]; then
  echo "== store: asset catalog, toolchain keys"
  CAT="$OUT/store/Assets.xcassets"
  rm -rf "$OUT/store"; mkdir -p "$CAT/AppIcon.appiconset"
  echo '{"info":{"author":"xcode","version":1}}' > "$CAT/Contents.json"
  # one 1024 px icon (opaque: App Store Connect refuses an icon with alpha); actool draws every size from it
  sips -s format png "$ROOT/toys/$TOY/icon.png" --out "$CAT/AppIcon.appiconset/icon.png" > /dev/null
  echo '{"images":[{"filename":"icon.png","idiom":"universal","platform":"ios","size":"1024x1024"}],"info":{"author":"xcode","version":1}}' \
    > "$CAT/AppIcon.appiconset/Contents.json"
  xcrun actool "$CAT" --compile "$APP" --platform "$SDKNAME" --minimum-deployment-target "$MIN_IOS" \
    --app-icon AppIcon --target-device iphone --target-device ipad \
    --output-partial-info-plist "$OUT/store/icons.plist" --output-format human-readable-text --errors --warnings
  ls -l "$APP/Assets.car"
  # the catalog's own icon keys replace the loose files' (CFBundleIcons, CFBundleIcons~ipad)
  /usr/libexec/PlistBuddy -c "Delete :CFBundleIcons" "$APP/Info.plist"
  /usr/libexec/PlistBuddy -c "Merge $OUT/store/icons.plist" "$APP/Info.plist"
  # what Xcode writes into every app it builds: App Store Connect reads which SDK and Xcode made it
  SDKVER="$(xcrun --sdk $SDKNAME --show-sdk-version)"; SDKBUILD="$(xcrun --sdk $SDKNAME --show-sdk-build-version)"
  XCVER="$(xcodebuild -version | awk '/^Xcode/ {print $2}')"; XCBUILD="$(xcodebuild -version | awk '/^Build version/ {print $3}')"
  DTXCODE="$(echo "$XCVER" | awk -F. '{printf "%d%d%d", $1, $2, $3}')"
  P() { /usr/libexec/PlistBuddy -c "$1" "$APP/Info.plist"; }
  P "Add :CFBundleSupportedPlatforms array"; P "Add :CFBundleSupportedPlatforms:0 string iPhoneOS"
  P "Add :DTPlatformName string $SDKNAME"; P "Add :DTPlatformVersion string $SDKVER"
  P "Add :DTPlatformBuild string $SDKBUILD"; P "Add :DTSDKName string $SDKNAME$SDKVER"
  P "Add :DTSDKBuild string $SDKBUILD"; P "Add :DTXcode string $DTXCODE"; P "Add :DTXcodeBuild string $XCBUILD"
  P "Add :DTCompiler string com.apple.compilers.llvm.clang.1_0"; P "Add :BuildMachineOSBuild string $(sw_vers -buildVersion)"
fi
plutil -convert binary1 "$APP/Info.plist"
cp "$ROOT/toys/ios/PrivacyInfo.xcprivacy" "$APP/PrivacyInfo.xcprivacy"
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
if [ "$STORE" = 1 ]; then
  du -sh "$APP" | cut -f1 | xargs echo "app (store, unsigned):"
  exit 0
fi
rm -f "$OUT/$TOY.ipa"
(cd "$OUT" && zip -qry "$TOY.ipa" Payload)
ls -l "$OUT/$TOY.ipa"
du -sh "$APP" | cut -f1 | xargs echo "app:"
