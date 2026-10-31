#!/usr/bin/env bash
# Builds Adrift for the device without an Xcode project: the Rust library as a static archive for
# aarch64-apple-ios, the Swift shell compiled and linked against it, the data bundled, an unsigned
# .app in Payload/ zipped into Adrift.ipa (SideStore signs it when installing). macOS with Xcode's
# command-line tools and rustup (`rustup target add aarch64-apple-ios`).
#
#     BUILD=<n> bash adrift/ios/build.sh [out-dir]       # out/Adrift.ipa; version 1.0.<n>, build <n>
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-$ROOT/adrift/build/ios}"
BUILD="${BUILD:-1}"
MIN_IOS=17.0
SDK="$(xcrun --sdk iphoneos --show-sdk-path)"
mkdir -p "$OUT"
cd "$ROOT/adrift"
echo "== rust (aarch64-apple-ios, release)"
cargo build --release --target aarch64-apple-ios -p adrift-core --features ffi
LIB="target/aarch64-apple-ios/release/libadrift_core.a"
ls -l "$LIB"
APP="$OUT/Payload/Adrift.app"
rm -rf "$OUT/Payload"
mkdir -p "$APP/data"
echo "== swift"
swiftc -O -target arm64-apple-ios$MIN_IOS -sdk "$SDK" \
  -I "$ROOT/adrift/ios/Sources" -import-objc-header "$ROOT/adrift/ios/Sources/adrift.h" \
  -parse-as-library -module-name Adrift \
  "$ROOT/adrift/ios/Sources/main.swift" "$LIB" \
  -framework UIKit -framework QuartzCore -framework Metal -framework CoreGraphics -framework Foundation \
  -lc++ -o "$APP/Adrift"
echo "== bundle"
sed -e "s/ADRIFT_VERSION/1.0.$BUILD/" -e "s/ADRIFT_BUILD/$BUILD/" "$ROOT/adrift/ios/Info.plist" > "$APP/Info.plist"
plutil -convert binary1 "$APP/Info.plist"
cp "$ROOT/adrift/ios/icon/AppIcon60x60@2x.png" "$APP/AppIcon60x60@2x.png"
cp "$ROOT/adrift/ios/icon/AppIcon60x60@3x.png" "$APP/AppIcon60x60@3x.png"
cp "$ROOT/adrift/ios/icon/AppIcon76x76@2x.png" "$APP/AppIcon76x76@2x.png"
python3 "$ROOT/tools/adrift/assets.py" "$APP/data" > /dev/null
echo "APPL????" > "$APP/PkgInfo"
rm -f "$OUT/Adrift.ipa"
(cd "$OUT" && zip -qry Adrift.ipa Payload)
ls -l "$OUT/Adrift.ipa"
du -sh "$APP" | cut -f1 | xargs echo "app:"
