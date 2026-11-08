#!/bin/bash
# Build the isolated test app. Generate the native framework before Xcode resolves/caches it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
cd "$ROOT"
scripts/build-core.sh
(cd apps/macos && xcodegen -q && xcodebuild -project CabinetStudio.xcodeproj \
    -scheme CabinetStudio -configuration Release \
    -derivedDataPath "$ROOT/target/xcode-benchtop" \
    -destination 'platform=macOS,arch=arm64' clean build -quiet)
mkdir -p "$ROOT/target/OpenAI Testing"
# The project builds the real app; this copy gets its own name and id, so it sits beside it.
APP="$ROOT/target/OpenAI Testing/Cabinet Studio OpenAI.app"
rm -rf "$APP"
ditto "$ROOT/target/xcode-benchtop/Build/Products/Release/Cabinet Studio.app" "$APP"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier io.github.brendan779.cabinetstudio.openai" \
    -c "Set :CFBundleName Cabinet Studio OpenAI" -c "Set :CFBundleDisplayName Cabinet Studio OpenAI" \
    "$APP/Contents/Info.plist"
codesign --force --deep --sign - "$APP"
echo "OpenAI test app: $ROOT/target/OpenAI Testing/Cabinet Studio OpenAI.app"
