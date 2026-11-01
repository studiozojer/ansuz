# Ansuz

An Exact interaction playground for exploring how divination objects should feel in the hand. The opening screen is a greyscale wireframe sphere seen from low orbit. The card playground remains implemented; its entry button is hidden in the streamlined main view. The voxel landscape is preserved in [stratum 001](strata/001-voxel-surfaces.爻) at commit `21f38cc`; [stratum 002](strata/002-interaction-playground.爻) records this change of direction.

The implementation supports **Spin sphere** (fixed camera, rotating globe) and **Fly camera** (fixed globe, camera moving at constant altitude). The main view uses the default spin mode and hides mode switching and reset controls. Switching modes in the retained implementation preserves the relative view and the cell under the cursor. These are deliberately equivalent viewpoints with different world transforms; the mode readout identifies what moves.

One finger or a held mouse drag revolves in both axes. Release adds bounded, decaying drift; a new stationary contact catches it immediately. Cancellation stops drift, and reduced motion keeps direct dragging without inertia. Arrow keys and fixed directional buttons provide alternatives. The retained reset action restores the initial orientation. A fixed center reticle raycasts the sphere and outlines the underlying cell in white. Highlighting is provisional: tapping does not select, open, create or save a reading.

The sphere is a cubed-sphere grid with 40 × 40 quadrilateral cells on each of six faces. Cell addresses are geometric coordinates, not content identities or provenance roots. Quaternion navigation crosses seams and poles without a latitude clamp. Camera altitude is 1.4 world units above a radius-10 sphere, and its local viewing angle remains fixed. Two meshes are generated once at setup; 16 short segments outline the target cell. Rendering uses native pixels for thin lines. There are no terrain textures, voxels, lighting effects or per-frame geometry uploads.

The original card study remains implemented with its controls, while its main-screen entry is hidden. Drag moves the card and leaves it where released. Glide projects a bounded destination from release velocity and springs toward it. Spring returns to the center. Pinch resizes; tap toggles selection. Touch the visible card during a release animation to catch it immediately where it is, even before moving your finger. Drag from that position to take control. Lifting without dragging resumes the spring and still counts as a selection tap; cancellation resumes it without selecting. Reset, resize buttons and directional controls remain outside the moving object. Reduced motion disables release animation and glide projection.

Recognition, pinch anchoring and the held transform use Exact's native `transformDragFor` binding. Contract supplies the destination on release; there is no per-frame application pointer loop and no transition input lock. The current glide is a bounded spring projection, not a physical friction model. The sample is ephemeral and resets on relaunch. It is not an authored reading.

## Gesture lab

The lab also contains four **elemental studies** in its 2D field. Choose an element and grab the cube:

- **Fire:** hold still for about half a second, begin a small slow stroke, then strike quickly. The cube ignites and cools back to normal.
- **Water:** circle around the original center. Consistent winding dissolves the cube into stirred blue pixels, attracted back toward that center.
- **Earth:** brace still, lift to the four-corner target, then hold the finishing pose. The block rises on a segmented column. Off-axis motion and premature release require another attempt.
- **Air:** fan the cube back and forth. It continuously pushes nearby field particles; momentum dissipates after release.

These are deliberate experiments with explicit element selection. “Pressure” means a steady hold, not force sensing. The water and air are bounded particle approximations rather than a full fluid solver. RUMBLE's published pose/base-pose and training-feedback descriptions inspired earth's deliberate sequence; the one-finger dwell thresholds are our own translation. See the [pose guide](https://rumble.wiki.gg/wiki/Poses) and [gym guide](https://rumble.wiki.gg/wiki/Gym). No RUMBLE art or code is used. The pixel effects are generated from a fixed pool of 240 reusable cube meshes; no sprite downloads or per-frame mesh generation are required.

Recording an elemental take resets the exercise to a known start. Trace version 3 adds the selected element, `ansuz.elements/1` model identifier, and per-frame phase, energy and object position. Replay reruns recognition and response from input rather than assigning the recorded output. Versions 1 and 2 remain readable. The original Motion target and 3D sphere remain available. A saved take keeps its original experiment; Replay selects that experiment again. Verification on 2026-10-31: 39 Rust tests pass, including successful and rejected elemental patterns, cooling/settling, cancellation, elemental replay and old trace imports. Real Chromium touch walkthroughs pass for all four effects and the original motion recording workflow; rendered screenshots were inspected. The updated signed build installed and launched on the paired iPhone 16 Pro. Physical feel remains unconfirmed.

```sh
bun ansuz/tools/check-elements.mjs http://localhost:8768
```

Open **Lab** from the orbital screen. Choose **2D target** or **3D sphere**, enter what should happen, press **Record**, perform the gesture in the field, then press **Stop**. A take stops automatically after 20 seconds and saves locally. **Replay** restores its starting state and runs the recorded primary-pointer input through the same motion function. Motion takes can be compared in either view; elemental takes replay in their 2D exercise. Touching the field interrupts replay. **Takes** lists saved recordings with **Load**, **Copy JSON**, and **Delete** controls. Capture a flick, a new contact, a tiny drag, and a stationary hold as one take to preserve their timing.

Each take separates canvas observer events from the processed 120 Hz input and simulated response. Version 2 also preserves each frame's viewport and reduced-motion preference; version 1 takes remain readable. Recording is limited to 2,400 frames, 4,800 observer events and twelve saved takes, in a separate `app:/data/gesture-takes.json` file capped at 8 MiB. Saves are atomic; failed writes preserve the previous library and leave the unsaved take available for retry/export. Unreadable storage is preserved. Neither the reading library nor Zhouyi is accessed.

This is a diagnostic prototype, not yet a reusable Expo module. Replay covers primary-pointer input; it does not capture keyboard controls, all multi-touch contacts, or raw digitizer data. Observer up includes cancellation. Recorded response is simulation state, not measured displayed frames. Replaying delivered input cannot diagnose a delay before the app received it; physical delivery needs its own trace. A community Expo package with a shared schema and automation adapters is being designed separately.

The `exact-canvas-contact.patch` disables the containing iOS scroll view's content-touch delay when hit-testing an input-owning canvas, preserving the default for other content. A pre-patch physical trace showed began events before the canvas received them. The patch compiles and is installed on the phone, but post-patch physical timing and the user's reported skipping still need confirmation.

Lab verification on 2026-10-31: 32 hostless Rust tests pass, including per-frame replay across viewport changes, legacy imports, malformed-import preservation, capture bounds, replay interruption and failed-write preservation. Contract compilation reports no diagnostics. A Chromium touch walkthrough checks a nine-second recording, 2D/3D replay, JSON export, loading beyond the engine message-size limit, persistence after reload and 320-point controls. The updated signed build installed and launched on the paired iPhone 16 Pro. Android capture and automation replay have not been implemented or tested. The retained card walkthrough currently fails its glide-velocity assertion in both this build and a separately built pre-lab baseline (`a3bb917`). A trace shows pauses over the engine’s 100 ms velocity window during individually driven input, but browser-timed retries also fail; the cause is not fully established. The lab/orbit checks are passing, but do not interpret this as a clean run of every retained card check.

```sh
bun ansuz/tools/check-lab.mjs http://localhost:8768
```

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

The recorder now writes version-4 multitouch traces. The tracked `exact-pointer-journal.patch` exposes a bounded, tick-local list of delivered pointer and blur events from Exact's existing canvas input stream. This includes every contact the host delivered, independent of the primary-pointer game API; it does not change recognition or claim raw digitizer input. See [the recording format](recordings/FORMAT.md) for timing, cancellation, replay and capture-limit semantics.

Both local and EAS build commands first run `scripts/patch-exact.sh`. The tracked `exact-xcode26.patch` fixes a Swift name collision and removes unsupported iOS 27 audio APIs while retaining the existing interruption handler. `exact-contact-catch.patch` makes exclusive manipulation handles (`touch-action="none"`) hold their displayed transform on contact in iOS and web, while preserving tap/pan recognition. On iOS, transform targets are hit-tested at their animated position. These patches deliberately leave the SDK checkout modified; the gitlink remains pinned. The script accepts already-applied patches and refuses a different SDK revision. Revisit all patches when upgrading Exact.

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

`ansuz/logic/src/records.rs` retains stable objects, explicit relationships and independent projections. The storage adapter and its tests remain as groundwork. The orbital canvas uses Exact's native game renderer and generated meshes; no old voxel assets are bundled. The 2D card still uses the native paired-transform binding. Neither study calls the storage adapter.

[STRATA.爻](STRATA.爻) is a chronological map of significant changes, following expocommons's practice. Git holds each implementation; strata explain what it was and why it changed. This is repository provenance, not a runtime snapshot database or a hierarchy for readings.

## Check

From the repository root, with the pinned Bun available:

```sh
bun ansuz/exact.mjs test-rust
bun ansuz/exact.mjs test web
bun ansuz/tools/check-interactions.mjs http://localhost:8768
bun ansuz/tools/check-orbit.mjs http://localhost:8768
npm run check:delivery
```

The browser walkthroughs require the current `ansuz/target/web-dist` served at that URL. It exercises real browser input; the authored suite tests selection, drag, pinch bounds, spring return, reduced motion and fallback controls. Exact's physical-phone agent can activate controls and inspect rendering, but cannot inject physical multi-touch; those calls do not prove finger feel. Verified on 2026-10-31: 12 retained Rust record/storage tests, five authored browser tests, five delivery tests, and the real Chromium walkthrough pass. The walkthrough covers finger drag versus tap, two-contact pinch and its focal point, bounded glide with release velocity, catching an in-flight spring with a stationary finger at its visible edge, resuming after stationary release/cancellation, cancellation recovery, reduced motion, 320-point layout and mouse input. A local Xcode 26.5 build was installed on iPhone 16 Pro; inspected the native screenshot and activated selection, size, movement, mode and reset controls. David reported that the playground feels good on the phone, but that the initial build could not catch a returning spring. After installing the contact-catch fix, David confirmed that it feels great on the phone and authorized merging the tested playground to main. VoiceOver and sustained performance remain unverified. This is a development build, not release-performance qualification.

Orbital verification: 24 Rust tests pass (12 retained record/storage tests and twelve geometry/input/render-history tests). The new tests cover all cell centers, repeated rotations through polar regions, fixed altitude, distinct camera/sphere transforms, touch without mouse buttons, stationary catch, cancellation, reduced motion, mode switching and reset. Quick-flick regressions include a complete down/move/up between ticks, terminal-only movement, release-tick travel, earlier hover exclusion, short tap/cancel catching, and reduced-motion flicks. Real Chromium/WebGPU input verifies both modes, changing cursor cells, fixed reticle, drift/catch, cancellation, reduced motion, fallback controls, 320-point layout, remounting the world after the 2D study, and desktop mouse navigation. Phone and desktop screenshots were inspected. The signed physical-iPhone build was installed and launched on iPhone 16 Pro after reconnecting the device; David subsequently reported that held drags work but quick flicks can be missed. The app now consumes contact edges and release movement even when a finger has already lifted before the next tick. David confirmed that the updated flick behavior feels good, then reported a jerk near grid-face joins, specifically −X / 26 · 02 → −Y / 02 · 25. Automated traces and a targeted landmark test show continuous navigation across that join. The outline now resets its edge interpolation when the selected cell changes, preventing unrelated edges from morphing across a face join. A per-tick regression fails when that fix is removed, and the browser checks cross −X → −Y in both modes. The follow-up build installed and launched on the iPhone. Whether this fixes the reported phone symptom remains unconfirmed; the cubed-sphere grid still has visible changes of line direction at face joins.

The fixed dark colors intentionally retain the requested visual direction and produce Exact's one-color-scheme warnings. Review the real screenshots; successful compilation alone is not a visual check.

## Guidance

Installed locally on 2026-10-31: [Expo animation](https://github.com/expo/skills/tree/main/plugins/expo/skills/expo-animation), [Expo native UI](https://github.com/expo/skills/tree/main/plugins/expo/skills/expo-native-ui), [Software Mansion React Native best practices](https://github.com/software-mansion-labs/skills/tree/main/skills/react-native-best-practices), and [Three.js interaction](https://github.com/CloudAI-X/threejs-skills/tree/main/skills/threejs-interaction). These are agent guidance, not application dependencies. Their interruption, gesture-ownership and device-verification principles inform this playground; React Native and Three.js code recipes do not run in Exact.

SDK revision: `4e9000abbdaa8315ae864cdb8253af143aac510c`, with the tracked Xcode compatibility, contact-catch and canvas-contact patches.
