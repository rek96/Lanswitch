#!/usr/bin/env bash
# Build signed-ready macOS disk image for LANSwitch.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Ad-hoc sign when no Apple Developer identity is configured (unsigned CI builds).
if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  export APPLE_SIGNING_IDENTITY="-"
fi

if [[ "${SKIP_HELPER:-}" != "1" ]]; then
  bash "$ROOT/scripts/prepare-binaries.sh" release
fi

echo "Building LANSwitch bundle..."
cargo tauri build

BUNDLE="$ROOT/target/release/bundle"
echo ""
echo "Done. Artifacts:"
find "$BUNDLE" -maxdepth 3 \( -name "*.dmg" -o -name "LANSwitch.app" \) 2>/dev/null || true
echo ""
echo "Sign, notarize, and staple before sharing. See docs/DISTRIBUTION.md."
