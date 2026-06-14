#!/usr/bin/env bash
# Copies the release helper binary into src-tauri/binaries/ with the target-triple
# suffix required by Tauri externalBin bundling.
set -euo pipefail

PROFILE="${1:-release}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "$TRIPLE" ]]; then
  echo "Could not detect Rust host triple." >&2
  exit 1
fi

echo "Building lanswitch-helper ($PROFILE) for $TRIPLE..."
cargo build -p lanswitch-helper --profile "$PROFILE"

SRC="$ROOT/target/$PROFILE/lanswitch-helper"
if [[ ! -f "$SRC" ]]; then
  echo "Helper binary not found at $SRC" >&2
  exit 1
fi

DEST_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$DEST_DIR"
DEST="$DEST_DIR/lanswitch-helper-$TRIPLE"
cp -f "$SRC" "$DEST"
echo "Prepared $DEST"
