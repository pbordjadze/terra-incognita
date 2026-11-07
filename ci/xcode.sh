#!/usr/bin/env bash
# The one Xcode every macOS job builds with, named here and nowhere else, so staying current is a
# one-line bump (it lists the runner's other Xcodes: take the newest stable one) (Paul's own Mac, a
# self-hosted runner for every app, keeps the same one at /Applications/Xcode_27.2.app; GitHub's
# `xcode-27` image has it too). It selects it for the job's later steps (DEVELOPER_DIR in
# $GITHUB_ENV), fails by name on a runner without it, and fetches the Metal toolchain when that
# Xcode lacks it (since Xcode 26 it is a separate download; the app's .metal files need it).
#   ci/xcode.sh            # a workflow's first step after checkout
#   ci/xcode.sh --version  # prints the version alone
# (The QA explore workflow, generated from the shared copy, names its version itself:
# `ci/qa/xcode.sh 27.2` in qa-explore.yml; keep the two the same.)
set -euo pipefail
VERSION=27.2
[[ "${1:-}" == --version ]] && { echo "$VERSION"; exit 0; }
DIR="/Applications/Xcode_$VERSION.app/Contents/Developer"
installed() { for a in /Applications/Xcode*.app; do [ -e "$a" ] && printf '%s ' "${a##*/}"; done; }
[[ -d "$DIR" ]] || { echo "::error::Xcode $VERSION is not on this runner ($DIR); installed: $(installed)"; exit 1; }
echo "DEVELOPER_DIR=$DIR" >> "${GITHUB_ENV:-/dev/null}"
export DEVELOPER_DIR="$DIR"
xcodebuild -version
echo "Xcodes on this runner: $(installed)"
if xcrun -sdk iphoneos metal --version > /dev/null 2>&1; then
  echo "Metal toolchain: $(xcrun -sdk iphoneos metal --version 2>&1 | head -1)"
else
  echo "Metal toolchain missing: downloading it"
  xcodebuild -downloadComponent MetalToolchain
  xcrun -sdk iphoneos metal --version | head -1
fi
