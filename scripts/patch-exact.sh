#!/usr/bin/env bash
# Apply Ansuz's fixes to the immutable SDK revision.
set -euo pipefail
cd "$(dirname "$0")/.."
exact_sdk="$PWD/tooling/exact2"
if [[ "$(git -C "$exact_sdk" rev-parse HEAD)" != 4e9000abbdaa8315ae864cdb8253af143aac510c ]]; then
  echo "Revisit the Ansuz patches before changing the Exact SDK pin." >&2
  exit 1
fi
for exact_patch in "$PWD/tooling/patches/exact-xcode26.patch" "$PWD/tooling/patches/exact-contact-catch.patch" "$PWD/tooling/patches/exact-canvas-contact.patch"; do
  if git -C "$exact_sdk" apply --reverse --check "$exact_patch" 2>/dev/null; then
    continue
  fi
  git -C "$exact_sdk" apply --check "$exact_patch"
  git -C "$exact_sdk" apply "$exact_patch"
done
