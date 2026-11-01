# Ansuz recording history

Private gesture archives use Snapback **0.4.33** on **https://ansuz.jera.ink**, separately from Jera. People sign in with an existing **ATproto / everything account**. The verified DID owns recordings and permissions; Ansuz has no email/password signup. [PRODUCT.md](../PRODUCT.md) defines the identity contract and current acceptance status.

On the sign-in screen, start typing a name or handle and select an account suggestion, then press **Log in**. You can also enter a full handle or DID directly. Suggestions are public discovery, not proof of account control.

In the app: **Lab → Earth → describe the intended response → Record → perform the gesture → Stop**. Record the intended motion even if today's recognizer rejects it. **Takes → Cloud → Log in → Sync** uploads only that account's device takes. **Load for replay** downloads and checks the original file, then returns to the field; press **Replay**. **Share history with studio agent** grants read/download access to the separate recorder identity; the same button revokes it.

Offline capture needs no account. Signed-out takes are device-only. After signing in, **Device → Add device-only takes to this account → Add to this account** explicitly assigns a chosen take. Login and Sync never claim them automatically. Account switching clears the loaded replay, note and cloud results. Previously saved takes stay in their owner's collection. A late response from a prior account cannot load its replay into the new session.

The owner-aware file is `gesture-takes-v2.json`. The original `gesture-takes.json` remains untouched: when no v2 file exists, its recordings are read as unassigned and the first successful edit writes the new file. The reading library `surface-notes.json` is separate. Per-account device capacity is 12 takes, with an overall 256-take / 8 MiB bound and 2 MiB per trace. All account collections share one app-private file; this is application-level ownership isolation, not separate encrypted vaults. Device Delete removes only the local copy; Sync does not delete it. Cloud records are immutable, and this prototype has no cloud deletion UI.

Each cloud row records its DID owner, captured timestamp, experiment, model version, expected behavior, frame count and SHA-256 of the uploaded JSON. History pages contain metadata and a file reference rather than all frames. The server enforces unique `(owner, digest)`. An uncertain archive-write outcome is resolved by looking up that key on the next Sync, including after restart. Downloads independently verify the checksum. Replay and recognizer changes never rewrite original evidence; future analysis should reference its content identity and model version. Downloads are session replay inputs, not a durable offline cloud cache.

Exact keeps the session under an Ansuz-specific secret namespace. Startup checks the cached DID's authority; offline restoration permits local use of the previously verified account. Remote operations recheck authority. Sign-out forgets the device session immediately and attempts server logout. It does not remove recordings or revoke an already-granted reader. Revoking a reader stops future downloads but cannot erase previously downloaded files. See [auth/README.md](auth/README.md) for protocol and lifetime limits.

## Develop and verify

From `recordings/`:

```sh
npm ci
npm ci --prefix auth
npm run check
npm test
npm test --prefix auth
./node_modules/.bin/snapback4 audit --json
./node_modules/.bin/snapback4 test --fixture-only
```

The integration test uses a disposable real Snapback server with trusted synthetic DID and ordinary operator sessions. It checks bytes, retries, owner isolation, reader grants and revocation, duplicate refusal, restart, disabled passwords and denied identity minting by ordinary sessions. Structural privacy invariants remain unproved for some disclosure paths; finite tests are evidence, not a universal proof.

From the repo root, build the web app and run `bun ansuz/tools/check-accounts.mjs` for phone-sized UI tests against a simulated provider and actual local auth/storage handlers. `check-cloud.mjs` now delegates to that test; the old password-based live test is retired. Real provider approval on the phone is a separate acceptance check.

## Agent access

The designated reader is `operator:ansuz-recorder`, an ordinary machine identity, not a human DID and not the service's minting authority. On the VPS, `python3 /opt/ansuz-recordings/issue-reader.py` provisions a 30-day session into the private `auth/recorder-agent.json`. Import that JSON through stdin with `ANSUZ_CREDENTIALS_FILE=/private/path/agent.json node scripts/provision-agent.mjs`; do not print credentials. The importer preserves the previous file under a retired name and writes the new file mode 0600. Renew through the same server-controlled issuance before expiry. The CLI refuses expired or legacy password credentials.

```sh
export ANSUZ_CREDENTIALS_FILE=/private/path/agent.json
node scripts/agent.mjs identity
node scripts/agent.mjs libraries
node scripts/agent.mjs history '<owner-DID>'
node scripts/agent.mjs fetch '<owner-DID>' '<sha256>' /private/path/original-take.json
```

The agent discovers only libraries explicitly shared with its principal. Downloads are checksum-checked before writing a new file. No automatic agent processing runs. Replay measures captured trajectories and simulation response; it does not reproduce physical finger sensation or raw digitizer samples.

Deployment and recovery are in [ops/README.md](ops/README.md).

## Evaluating Earth changes against a recording

The Earth sweep recognizer (`ansuz.elements/2`) interprets an upward gather followed by a downward return. It accepts curved paths and any starting location in the lab field. This is an editable, hand-authored interpretation of demonstrations, not a trained gesture model. Other elements still use model 1. Model-1 Earth recordings retain their original pose recognizer during ordinary replay; new Earth recordings identify model 2.

To compare both recognizers against a private downloaded take without changing its source experiment, metadata, or bytes:

```sh
cargo run --manifest-path ansuz/logic/Cargo.toml --example evaluate_earth -- /private/path/original-take.json
```

Run from the repository root with the SDK's pinned Rust toolchain. The report lists trigger timestamps and phase transitions separately for each model. Keep downloaded human traces and derived reports outside the repository. Passing a demonstration is calibration evidence; use negative examples and physical-device feedback before treating a gesture vocabulary as settled. Multitouch capture/replay remains a separate pending upgrade.

## Reliable account archival

Signed-in captures and explicitly adopted takes enter a durable upload queue in the same atomic device file as their source trace. Guest takes remain device-only. While the app is running, queued takes retry every 15 seconds and resume after account restoration or restart. Closing the app suspends transfer; it does not remove the queued source. Each attempt first checks the authenticated owner's content digest. A lost acknowledgement is reconciled against that existing cloud row before retrying an upload. A confirmation is persisted only after the row's owner and digest have been checked. Local deletion of a queued source requires explicitly pausing retries first; an in-flight request may still finish after pausing.

The public `/m/archive` endpoint is handled by the authorization service. It authenticates the account with Snapback, downloads the referenced file with that account's bearer, checks the exact SHA-256, validates the versioned trace, and compares its expected response, frame count, experiment, model, and capture timestamp with the submitted metadata. It archives an identical service-owned file copy under the verified account's DID. The Snapback mutation itself permits only `service:atproto-auth`, so direct HTTP mutations or queued sync intents cannot bypass validation. Upload/download bytes remain unchanged; interpretation is separate. Existing archive rows and read grants remain valid.

Deploy the schema, auth modules, and Caddy fragment together using the existing backed-up adoption workflow. Deploy the server before distributing the new app. The existing app's archive request shape remains supported through the gateway. `auth/trace.mjs` implements the server contract corresponding to `ansuz/shared/gesture_trace.rs`; keep their version support and bounds aligned when evolving the format.
