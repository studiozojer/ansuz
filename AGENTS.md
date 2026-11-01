# Ansuz

Read `PRODUCT.md` before changing identity, storage, sharing or account UI. Work on feature branches and submit PRs. The app lives in `ansuz/`; read its generated `AGENTS.md` for Exact authoring instructions.

David explicitly authorized local iPhone builds for Ansuz on 2026-10-31, superseding the earlier EAS-only rule for this app. Use `npm run build:ios:local` for a paired physical iPhone, or EAS for cloud builds. Simulator checks still use EAS Simulator; do not run the full Exact `prove` matrix, which invokes additional local native hosts. Hostless Rust tests and browser wasm builds are allowed locally.

The entry experience is the Exact orbital gesture study, with the native Contract card playground retained under “2D study”. Rust owns the sphere, camera, cursor hit test and orbital motion; Contract owns the controls. The sample is ephemeral; do not load or rewrite the existing surface-note file as part of playground work. The voxel implementation is recoverable through STRATA.爻. Do not connect to or migrate Zhouyi's database.

The repository-root `app.json`, `eas.json`, and `package.json` configure Expo EAS delivery of the native Exact app. They do not introduce an Expo Go or React Native runtime. Run `npm run check:delivery` after changes to the cloud build/signing scripts. Never commit signing files or the private SDK dependency token.
