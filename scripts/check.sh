#!/bin/bash
# The gate every change must pass: formatting, lints, Rust and tool tests, catalog validation,
# dependency licences and advisories (cargo deny) and the macOS app build. Overnight agents run
# this before every commit.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"

cargo fmt --all --check
cargo clippy --workspace --all-targets -q -- -D warnings
cargo test --workspace -q
python3 -m unittest discover -s tools -q
if ! command -v cargo-deny >/dev/null; then
  echo "check: cargo-deny is missing: brew install cargo-deny" >&2
  exit 1
fi
cargo deny --log-level error check

if [[ "$(uname)" == "Darwin" && "${SKIP_APP:-0}" != "1" ]]; then
  # The bindings and xcframework are generated, so a fresh checkout must build them before
  # xcodegen can find them.
  scripts/build-core.sh
  (cd apps/macos && xcodegen -q && xcodebuild -project CabinetStudio.xcodeproj -scheme CabinetStudio \
    -configuration Debug -derivedDataPath "$ROOT/target/xcode" build -quiet)
fi
echo "check: all green"
