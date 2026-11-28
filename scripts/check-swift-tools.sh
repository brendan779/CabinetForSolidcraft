#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/target/swift-tests"
swiftc "$ROOT/apps/macos/CabinetStudio/Model/ToolRunner.swift" \
  "$ROOT/apps/macos/Tests/ToolRunnerTests.swift" -o "$ROOT/target/swift-tests/tool-runner"
"$ROOT/target/swift-tests/tool-runner"

swiftc -module-cache-path "$ROOT/target/swift-tests/module-cache" \
  "$ROOT/apps/macos/CabinetStudio/Model/StoneGeometry.swift" \
  "$ROOT/apps/macos/Tests/StoneGeometryTests.swift" -o "$ROOT/target/swift-tests/stone-geometry"
"$ROOT/target/swift-tests/stone-geometry"

swiftc -module-cache-path "$ROOT/target/swift-tests/module-cache" \
  "$ROOT/apps/macos/CabinetStudio/Model/OffcutStore.swift" \
  "$ROOT/apps/macos/Tests/OffcutStoreTests.swift" -o "$ROOT/target/swift-tests/offcut-store"
"$ROOT/target/swift-tests/offcut-store"
