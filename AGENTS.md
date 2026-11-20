# Ansuz

Read `PRODUCT.md` before changing identity, storage, sharing or account UI. Work on feature branches and submit PRs. Current web work is in `stratum-007/` using the pinned public Exact SDK in `tooling/exact`. Read its generated `AGENTS.md` and the [stratum record](strata/007-shared-reading.爻). The earlier Exact2 app lives in `ansuz/` with its own generated instructions and mobile delivery path.

David explicitly authorized local iPhone builds for Ansuz on 2026-10-31, superseding the earlier EAS-only rule for this app. Use `npm run build:ios:local` for a paired physical iPhone, or EAS for cloud builds. Simulator checks still use EAS Simulator; do not run the full Exact `prove` matrix, which invokes additional local native hosts. Hostless Rust tests and browser wasm builds are allowed locally.

The entry experience is the Exact orbital gesture study, with the native Contract card playground retained in code. The main sphere view hides the 2D entry, reset, and spin/fly mode buttons as part of UI streamlining. Rust owns the sphere, camera, cursor hit test and orbital motion; Contract owns the controls. The sample is ephemeral; do not load or rewrite the existing surface-note file as part of playground work. The voxel implementation is recoverable through STRATA.爻. Do not connect to or migrate Zhouyi's database.

The repository-root `app.json`, `eas.json`, and `package.json` configure Expo EAS delivery of the native Exact app. They do not introduce an Expo Go or React Native runtime. Run `npm run check:delivery` after changes to the cloud build/signing scripts. Never commit signing files or the private SDK dependency token.

The shared recording schema, OAuth gateway, server integration tests and deployment tools live in [jera-snapback](https://github.com/studiozojer/jera-snapback). Configure `JERA_SNAPBACK_DIR` for client integration checks as described in [recordings/README.md](recordings/README.md).
