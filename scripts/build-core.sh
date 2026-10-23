#!/bin/bash
# Builds the Rust core as a universal static library, generates the Swift bindings and packs both
# into apps/macos/Core/CabinetCore.xcframework. Xcode runs this as a pre-build phase; it is also
# safe to run by hand. Pass --debug for a faster unoptimised build.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"

PROFILE=release
FLAG=--release
if [[ "${1:-}" == "--debug" ]]; then PROFILE=debug; FLAG=; fi

TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
for t in "${TARGETS[@]}"; do
  rustup target list --installed | grep -qx "$t" || rustup target add "$t"
  MACOSX_DEPLOYMENT_TARGET=15.0 cargo build -q -p cabinet-ffi $FLAG --target "$t"
done

OUT="$ROOT/target/universal/$PROFILE"
mkdir -p "$OUT"
lipo -create -output "$OUT/libcabinet_ffi.a" \
  "target/aarch64-apple-darwin/$PROFILE/libcabinet_ffi.a" \
  "target/x86_64-apple-darwin/$PROFILE/libcabinet_ffi.a"

GEN="$ROOT/target/uniffi-swift"
rm -rf "$GEN" && mkdir -p "$GEN"
cargo run -q -p cabinet-ffi --bin uniffi-bindgen -- generate \
  --library "target/aarch64-apple-darwin/$PROFILE/libcabinet_ffi.a" \
  --language swift --out-dir "$GEN"

CORE="$ROOT/apps/macos/Core"
HEADERS="$GEN/headers"
mkdir -p "$HEADERS" "$CORE"
cp "$GEN"/*FFI.h "$HEADERS/"
# Xcode looks for module.modulemap inside an xcframework's headers.
cat "$GEN"/*FFI.modulemap > "$HEADERS/module.modulemap"
cp "$GEN"/cabinet_ffi.swift "$CORE/CabinetCore.swift"

rm -rf "$CORE/CabinetCore.xcframework"
xcodebuild -create-xcframework -library "$OUT/libcabinet_ffi.a" -headers "$HEADERS" \
  -output "$CORE/CabinetCore.xcframework" >/dev/null
echo "CabinetCore.xcframework and CabinetCore.swift are up to date ($PROFILE)"
