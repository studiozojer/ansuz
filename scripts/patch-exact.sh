#!/usr/bin/env bash
# Keep Ansuz's pinned SDK buildable with Xcode 26.
set -euo pipefail
cd "$(dirname "$0")/.."
exact_sdk="$PWD/tooling/exact2"
exact_patch="$PWD/tooling/patches/exact-xcode26.patch"
if [[ "$(git -C "$exact_sdk" rev-parse HEAD)" != 4e9000abbdaa8315ae864cdb8253af143aac510c ]]; then
  echo "Revisit the Xcode compatibility patch before changing the Exact SDK pin." >&2
  exit 1
fi
if git -C "$exact_sdk" apply --reverse --check "$exact_patch" 2>/dev/null; then
  exit 0
fi
git -C "$exact_sdk" apply --check "$exact_patch"
git -C "$exact_sdk" apply "$exact_patch"
