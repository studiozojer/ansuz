# Ansuz

An Exact interaction playground for exploring how divination objects should feel in the hand. The opening screen is a quiet, flat field with one sample card. The voxel landscape is preserved in [stratum 001](strata/001-voxel-surfaces.爻) at commit `21f38cc`; [stratum 002](strata/002-interaction-playground.爻) records this change of direction.

Drag moves the card and leaves it where released. Glide projects a bounded destination from release velocity and springs toward it. Spring returns to the center. Pinch resizes; tap toggles selection. Touch the visible card during a release animation to catch it immediately where it is, even before moving your finger. Drag from that position to take control. Lifting without dragging resumes the spring and still counts as a selection tap; cancellation resumes it without selecting. Reset, resize buttons and directional controls remain outside the moving object. Reduced motion disables release animation and glide projection.

Recognition, pinch anchoring and the held transform use Exact's native `transformDragFor` binding. Contract supplies the destination on release; there is no per-frame application pointer loop and no transition input lock. The current glide is a bounded spring projection, not a physical friction model. The sample is ephemeral and resets on relaunch. It is not an authored reading.

## Run

Initialize the pinned SDK and install its tools (Bun 1.4.2 was used):

```sh
git submodule update --init --recursive
cd tooling/exact2
bun install --frozen-lockfile
bun scripts/exact.mjs setup --check
```

Install missing browser/Rust tooling reported by that check, then apply the tracked SDK patches from the repository root before starting the browser preview:

```sh
bash scripts/patch-exact.sh
bun ansuz/exact.mjs web
```

Use its localhost URL in a WebGPU-capable browser. This is a standalone Exact app; Zhouyi is unchanged. Local iPhone builds are authorized for Ansuz; simulator checks still use EAS Simulator.

## Build locally for a paired iPhone

Local iPhone builds were explicitly authorized on 2026-10-31. Use Xcode, Bun 1.4.2+, the SDK's pinned Rust toolchain, a paired iPhone in Developer Mode, and a development certificate/provisioning profile covering `studio.zojer.ansuz` and that phone. A valid team wildcard profile can cover this app without registering a new identifier. No EAS secret is needed locally when the SDK's dependencies are already cached or your local Git authentication can fetch them.

After the SDK setup above, install its iPhone Rust target once:

```sh
(cd tooling/exact2 && rustup target add aarch64-apple-ios)
npm run build:ios:local
```

To choose a particular phone, append `-- --phone "your phone name"`. If the right Bun version isn't on PATH, set `BUN=/path/to/bun` for the npm command. The underlying command is `bun ansuz/exact.mjs ios --device --run`: Exact builds, signs, installs, and launches the native app. It does not use `expo run:ios`, Metro, or Expo Go. This installs alongside Zhouyi, with separate local storage.

Both local and EAS build commands first run `scripts/patch-exact.sh`. The tracked `exact-xcode26.patch` fixes a Swift name collision and removes unsupported iOS 27 audio APIs while retaining the existing interruption handler. `exact-contact-catch.patch` makes exclusive manipulation handles (`touch-action="none"`) hold their displayed transform on contact in iOS and web, while preserving tap/pan recognition. On iOS, transform targets are hit-tested at their animated position. These patches deliberately leave the SDK checkout modified; the gitlink remains pinned. The script accepts already-applied patches and refuses a different SDK revision. Revisit both patches when upgrading Exact.

## Install on iPhone with Expo EAS

[EAS project: studiozojer/ansuz](https://expo.dev/accounts/studiozojer/projects/ansuz). EAS builds the native Exact application with UIKit and its GPU renderer. The root Expo config is delivery metadata; this app does not run in Expo Go. The pinned EAS CLI and `expo-modules-autolinking` are build/configuration tooling only. The latter is required by EAS's entitlement introspection for projects without a checked-in Xcode project; no Expo or React Native runtime is added.

From the repository root:

```sh
npm ci
npm run build:ios
```

The `preview` profile produces a signed, internally distributed `Ansuz.ipa`. Open the successful EAS build’s install link on a registered iPhone. It includes the native playground, so it does not need the browser preview server. The bundle ID is `studio.zojer.ansuz`; this installs alongside Zhouyi. Browser notes do not automatically transfer to the phone.

First-build prerequisites:

- Add `EXACT_GITHUB_TOKEN` as a **Secret** in this project's **preview** environment, with read access to `expo/snapback`. The pinned Exact SDK resolves that private Cargo dependency even though Ansuz uses local JSON notes. Do not put the token in source, app config, or a chat message.
- Accept the current Apple Developer Program License Agreement for Zojer Studio (`497QWRKRQH`), then run `npx eas credentials:configure-build --platform ios --profile preview`. Reuse the studio's distribution certificate and create an ad-hoc profile for Ansuz including your phone. If needed, `npx eas device:create` registers a new device before building.

As of 2026-10-31, the Expo project is linked and the registered iPhone `hazy` is visible. Apple signing setup stopped at Apple's “PLA Update available” error; the GitHub secret is also absent. No EAS IPA has been produced yet. These cloud setup blockers do not prevent a local development build when an existing certificate/profile covers the app and phone.

For EAS Simulator, use `npm run build:simulator`. This profile needs the GitHub secret but no Apple distribution profile. The studio account has EAS Simulator access. Use the resulting build with EAS Simulator for the native walkthrough; an `.app` simulator artifact cannot be installed on a physical iPhone. Simulator execution remains cloud-only.

The custom workflows fetch Exact at the committed SDK pin, install its toolchain, compile the same `ansuz/` sources, and check the artifact's identity, version and platform before upload. The phone workflow imports only that build's credentials into a temporary worker keychain and cleans it up afterward. `.easignore` excludes credentials, local SDK checkouts and generated caches. App versions/build numbers are local in root `app.json` and passed to Exact; increment `expo.ios.buildNumber` for a new distributable build. `npm run check:delivery` checks identity consistency, local-build refusal, credential validation, and artifact rejection without compiling natively.

## Records and provenance

The playground never loads, rewrites or deletes the older `app:/data/surface-notes.json`. The same app identity preserves that file across an ordinary update, but deleting the app can delete its local data. The old note editor is available only in stratum 001's source. No Zhouyi database or cloud service is involved.

`ansuz/logic/src/records.rs` retains stable objects, explicit relationships and independent projections. The storage adapter and its tests remain as groundwork. No game canvas is mounted, no voxel assets are bundled, and the small generated game shell is retained only because this project was scaffolded as an Exact game. It does not drive the playground's gestures.

[STRATA.爻](STRATA.爻) is a chronological map of significant changes, following expocommons's practice. Git holds each implementation; strata explain what it was and why it changed. This is repository provenance, not a runtime snapshot database or a hierarchy for readings.

## Check

From the repository root, with the pinned Bun available:

```sh
bun ansuz/exact.mjs test-rust
bun ansuz/exact.mjs test web
bun ansuz/tools/check-interactions.mjs http://localhost:8768
npm run check:delivery
```

The browser walkthrough requires the current `ansuz/target/web-dist` served at that URL. It exercises real browser input; the authored suite tests selection, drag, pinch bounds, spring return, reduced motion and fallback controls. Exact's physical-phone agent can activate controls and inspect rendering, but cannot inject physical multi-touch; those calls do not prove finger feel. Verified on 2026-10-31: 12 retained Rust record/storage tests, five authored browser tests, five delivery tests, and the real Chromium walkthrough pass. The walkthrough covers finger drag versus tap, two-contact pinch and its focal point, bounded glide with release velocity, catching an in-flight spring with a stationary finger at its visible edge, resuming after stationary release/cancellation, cancellation recovery, reduced motion, 320-point layout and mouse input. A local Xcode 26.5 build was installed on iPhone 16 Pro; inspected the native screenshot and activated selection, size, movement, mode and reset controls. David reported that the playground feels good on the phone, but that the initial build could not catch a returning spring. The contact-catch fix builds and is installed; its physical iPhone gesture behavior still needs hands-on confirmation. VoiceOver and sustained performance remain unverified. This is a development build, not release-performance qualification.

The fixed dark colors intentionally retain the requested visual direction and produce Exact's one-color-scheme warnings. Review the real screenshots; successful compilation alone is not a visual check.

## Guidance

Installed locally on 2026-10-31: [Expo animation](https://github.com/expo/skills/tree/main/plugins/expo/skills/expo-animation), [Expo native UI](https://github.com/expo/skills/tree/main/plugins/expo/skills/expo-native-ui), [Software Mansion React Native best practices](https://github.com/software-mansion-labs/skills/tree/main/skills/react-native-best-practices), and [Three.js interaction](https://github.com/CloudAI-X/threejs-skills/tree/main/skills/threejs-interaction). These are agent guidance, not application dependencies. Their interruption, gesture-ownership and device-verification principles inform this playground; React Native and Three.js code recipes do not run in Exact.

SDK revision: `4e9000abbdaa8315ae864cdb8253af143aac510c`, with the tracked Xcode compatibility and contact-catch patches.
