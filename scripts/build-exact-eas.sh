#!/usr/bin/env bash
# This workflow runs on EAS; build:ios:local is the separate physical-phone path.
set -euo pipefail
set +x
if [[ "${EAS_BUILD:-}" != "true" ]]; then
  echo "This workflow runs on EAS. Use npm run build:ios or npm run build:simulator; for a paired phone use npm run build:ios:local." >&2
  exit 1
fi
mode="${1:-}"
case "$mode" in device|simulator) ;; *) echo "Expected device or simulator." >&2; exit 1 ;; esac
if [[ -z "${EXACT_GITHUB_TOKEN:-}" ]]; then
  echo "Add the preview Secret EXACT_GITHUB_TOKEN in the Ansuz EAS project, with read access to expo/snapback." >&2
  exit 1
fi
if [[ "$mode" == device && ( -z "${EXACT_IDENTITY:-}" || -z "${EXACT_PROFILE:-}" ) ]]; then
  echo "The device workflow must supply EAS signing credentials." >&2
  exit 1
fi
cd "$(dirname "$0")/.."
export GIT_TERMINAL_PROMPT=0 CARGO_NET_GIT_FETCH_WITH_CLI=true
exact_auth_dir=$(mktemp -d)
trap 'rm -rf "$exact_auth_dir"' EXIT
cat > "$exact_auth_dir/askpass" <<'ASKPASS'
#!/bin/sh
case "$1" in
  *Username*github.com*) printf '%s\n' x-access-token ;;
  *Password*github.com*) printf '%s\n' "$EXACT_GITHUB_TOKEN" ;;
  *) exit 1 ;;
esac
ASKPASS
chmod 700 "$exact_auth_dir/askpass"
export GIT_ASKPASS="$exact_auth_dir/askpass"
exact_revision=4e9000abbdaa8315ae864cdb8253af143aac510c
if [[ ! -f tooling/exact2/scripts/exact.mjs ]]; then
  mkdir -p tooling/exact2
  git -C tooling/exact2 init
  git -C tooling/exact2 fetch --depth 1 https://github.com/ccheever/exact2.git "$exact_revision"
  git -C tooling/exact2 checkout --detach FETCH_HEAD
fi
if [[ "$(git -C tooling/exact2 rev-parse HEAD)" != "$exact_revision" ]]; then
  echo "Exact SDK revision differs from the tested pin." >&2
  exit 1
fi
export PATH="$HOME/.cargo/bin:$PATH"
bash scripts/patch-exact.sh
if ! command -v rustup >/dev/null; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$exact_auth_dir/rustup.sh"
  sh "$exact_auth_dir/rustup.sh" -y --profile minimal --default-toolchain none
fi
export BUN="$(command -v bun)"
export EXACT_VERSION="$(node -p 'require("./app.json").expo.version')"
export EXACT_BUILD_NUMBER="$(node -p 'require("./app.json").expo.ios.buildNumber')"
# SDK setup installs its pinned Rust, Hermes, and build dependencies.
bun tooling/exact2/scripts/exact.mjs setup
if [[ "$mode" == device ]]; then
  exact_target=aarch64-apple-ios
elif [[ "$(uname -m)" == arm64 ]]; then
  exact_target=aarch64-apple-ios-sim
else
  exact_target=x86_64-apple-ios
fi
(cd tooling/exact2 && rustup target add "$exact_target")
mkdir -p exact-artifacts
exact_artifact_dir="$(pwd)/exact-artifacts"
if [[ "$mode" == device ]]; then
  (cd ansuz && bun exact.mjs ios --device --archive "$exact_artifact_dir/Ansuz.ipa")
else
  (cd ansuz && bun exact.mjs ios --no-install)
  python3 - <<'ARTIFACT'
from pathlib import Path
import shutil
candidates = [p for root in [Path('ansuz/target'), Path('tooling/exact2/target')] for p in root.glob('clients/**/ios-simulator/**/standalone/Ansuz.app')]
if len(candidates) != 1:
    raise SystemExit(f'Expected one Exact simulator app, found {len(candidates)}')
shutil.copytree(candidates[0], 'exact-artifacts/Ansuz.app')
# The SDK's simulator plist uses fixed development versions. Stamp the EAS
# app version before re-signing this copy for the simulator.
import plistlib, os
plist = Path('exact-artifacts/Ansuz.app/Info.plist')
info = plistlib.loads(plist.read_bytes())
info['CFBundleVersion'] = os.environ['EXACT_BUILD_NUMBER']
info['CFBundleShortVersionString'] = os.environ['EXACT_VERSION']
plist.write_bytes(plistlib.dumps(info))
ARTIFACT
  codesign --force --sign - exact-artifacts/Ansuz.app
fi
python3 scripts/check-apple-artifact.py "$mode"
