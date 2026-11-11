#!/usr/bin/env bash
# Holds a toy's store build (toys/ios/build.sh with STORE=1) to what an App Store Connect upload needs,
# before it is signed (painting-company row 312; .github/workflows/testflight.yml runs it). macOS.
#   bash tools/toys/check_store.sh <path/to/Exec.app>
# The export-compliance answer (no non-exempt encryption) and the privacy manifest; the icon compiled into
# an asset catalog with its name in Info.plist; the SDK and Xcode keys; a version and build App Store
# Connect accepts; an arm64 executable linking only the system's libraries; nothing that is a debug build's.
set -euo pipefail
APP="${1:?usage: check_store.sh <path/to/Exec.app>}"
failed=0
fail() { echo "FAIL: $*"; failed=1; }
key() { /usr/libexec/PlistBuddy -c "Print $1" "$APP/Info.plist" 2> /dev/null || true; }
BINARY="$APP/$(key CFBundleExecutable)"

[[ "$(key ITSAppUsesNonExemptEncryption)" == false ]] || fail "ITSAppUsesNonExemptEncryption is not false"
[[ -f "$APP/PrivacyInfo.xcprivacy" ]] || fail "PrivacyInfo.xcprivacy is not in the app"
plutil -lint "$APP/PrivacyInfo.xcprivacy" > /dev/null || fail "PrivacyInfo.xcprivacy is not a valid plist"
[[ -s "$APP/Assets.car" ]] || fail "no asset catalog (Assets.car): App Store Connect wants the icon in one"
[[ "$(key CFBundleIcons:CFBundlePrimaryIcon:CFBundleIconName)" == AppIcon ]] || fail "CFBundleIcons names no AppIcon"
[[ "$(key CFBundleIcons~ipad:CFBundlePrimaryIcon:CFBundleIconName)" == AppIcon ]] || fail "CFBundleIcons~ipad names no AppIcon"
if xcrun --find assetutil > /dev/null 2>&1; then
  xcrun assetutil --info "$APP/Assets.car" | grep -q '"Name" : "AppIcon"' || fail "the asset catalog holds no AppIcon"
fi
for k in DTPlatformName DTPlatformVersion DTSDKName DTSDKBuild DTXcode DTXcodeBuild CFBundleSupportedPlatforms:0 MinimumOSVersion; do
  [[ -n "$(key "$k")" ]] || fail "Info.plist has no $k"
done
[[ "$(key DTPlatformName)" == iphoneos ]] || fail "not built for the device (DTPlatformName $(key DTPlatformName))"
[[ "$(key CFBundleShortVersionString)" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || fail "CFBundleShortVersionString $(key CFBundleShortVersionString) isn't one to three integers"
[[ "$(key CFBundleVersion)" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] || fail "CFBundleVersion $(key CFBundleVersion) isn't one to three integers"
[[ "$(lipo -archs "$BINARY")" == arm64 ]] || fail "the executable isn't arm64 alone ($(lipo -archs "$BINARY"))"
# only the system's own libraries (nothing embedded or from a path an installed app can't reach)
bad="$(otool -L "$BINARY" | tail -n +2 | awk '{print $1}' | grep -vE '^(/System/Library/|/usr/lib/)' || true)"
[[ -z "$bad" ]] || fail "links what isn't the system's: $bad"
[[ -z "$(find "$APP" -name '*.mobileprovision' -o -name '_CodeSignature' | head -1)" ]] || fail "already signed or provisioned: the export signs it"
[[ $failed -eq 0 ]] && echo "Store build ok: $(key CFBundleIdentifier) $(key CFBundleShortVersionString) ($(key CFBundleVersion)), SDK $(key DTSDKName), Xcode $(key DTXcode), $(du -sh "$APP" | cut -f1)"
exit $failed
