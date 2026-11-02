#!/bin/bash
# Sets the version everywhere it is kept: the Rust workspace and the app. Merged to main, a new
# version makes GitHub build and publish a release (.github/workflows/release.yml).
#   scripts/set-version.sh 0.3.0
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
V="${1:?usage: scripts/set-version.sh X.Y.Z}"
[[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not X.Y.Z: $V" >&2; exit 1; }
# Only the workspace's own version line, under [workspace.package].
sed -i '' -E "/^\[workspace\.package\]/,/^\[/ s/^version = \"[^\"]*\"/version = \"$V\"/" Cargo.toml
sed -i '' -E "s/MARKETING_VERSION: \"[^\"]*\"/MARKETING_VERSION: \"$V\"/" apps/macos/project.yml
cargo metadata --format-version 1 --offline > /dev/null
echo "version $V"
