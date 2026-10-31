# Ansuz

Work on feature branches and submit PRs. The app lives in `ansuz/`; read its generated `AGENTS.md` for Exact authoring instructions.

Use EAS Build for native builds and EAS Simulator for simulator checks. Do not run local native build commands or the full Exact `prove` matrix, which invokes them. Hostless Rust tests and browser wasm builds are allowed locally.

Notes are local to this prototype. Do not connect to or migrate Zhouyi's database as part of this work.

The repository-root `app.json`, `eas.json`, and `package.json` configure Expo EAS delivery of the native Exact app. They do not introduce an Expo Go or React Native runtime. Run `npm run check:delivery` after changes to the cloud build/signing scripts. Never commit signing files or the private SDK dependency token.
