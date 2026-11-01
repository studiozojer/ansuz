# Ansuz recording history

**Identity correction:** the password flow described below is the current implementation, but David has rejected it in favor of existing ATproto / everything account authorization. [PRODUCT.md](../PRODUCT.md) is authoritative. Login replacement is pending; storage/replay tests do not establish ATproto acceptance.

Private gesture archives backed by Snapback **0.4.33**, deployed separately from Jera at **https://ansuz.jera.ink**. The Exact app uses Snapback's authenticated HTTP and file APIs. This first version is explicit archival sync, not a Snapback device replica or an extracted Expo package.

In the app: **Lab → Earth → describe the intended response → Record → perform the gesture → Stop**. Record the intended motion even if today's recognizer rejects it. **Takes → Cloud → Create account / Sign in → Sync** uploads saved device takes and refreshes history. **Load for replay** downloads and verifies the original file, then returns to the field; press **Replay**. **Share history with studio agent** grants read/download access to the public principal in `ansuz/data/src/cloud.rs`; the same button revokes it.

Device recordings remain in `gesture-takes.json`, independently of the reading library. Offline capture needs no account. The existing 12-take / 8 MiB device limit remains; Sync does not delete local takes. Device Delete only removes the local copy. Cloud records are immutable, and this prototype has no cloud deletion UI. Downloads load into the replay field for the current app session; they are not yet an offline cache. After reopening, Sync refreshes cloud history. No automatic background upload or agent processing runs.

Each cloud row records its owner, captured timestamp, experiment, model version, expected behavior, frame count and SHA-256 of the exact uploaded JSON bytes. The trace is an authenticated file capped at 2 MiB. History pages contain metadata and a file reference, not all frames. The server enforces unique `(owner, digest)`; before each upload, the client checks that key. An uncertain write outcome is resolved by that lookup on the next Sync, including after restarting the app. Uploads left unplaced by a failed archive are handled by Snapback's asset cleanup. A client's digest is a claimed content identity; downloads independently verify it. The original trace is never changed by a replay or a new recognizer. Future analysis records should reference this identity and their model version.

Password accounts are specific to Ansuz; no ATproto/PDS integration is introduced. Exact's secret store holds the session, with one fixed production origin and a dedicated secret name. Sync refreshes its seven-day session; an expired/rejected session requires signing in again. Disconnect forgets the device session; it does not revoke previously issued server sessions or an already-granted agent reader. Revoke agent access before disconnecting if desired. Revocation stops future downloads; it cannot erase files already downloaded by a reader.

## Develop and verify

```sh
npm ci
npm run check
npm test
./node_modules/.bin/snapback4 audit --json
./node_modules/.bin/snapback4 test --fixture-only
npm run dev
```

The integration test runs a real temporary server with password sessions and a full-size file. It covers byte preservation, retry, owner isolation, grants, revocation, duplicate refusal and server restart. Security invariants state privacy intent independently; Snapback's structural proof remains unproved for some read/disclosure paths. Generated finite checks and the HTTP tests are evidence, not a universal security proof. `ansuz/tools/check-cloud.mjs` exercises the phone-sized UI against the live isolated service with a synthetic account; set `ANSUZ_PROOF_CREDENTIALS_FILE` to a new private path. Its test uses no human recordings.

## Agent access

Use a separate account, not the phone's bearer. Provision it once with `ANSUZ_CREDENTIALS_FILE=/private/path/agent.json node scripts/provision-agent.mjs`, then put only its returned public principal in the app. Existing installations must be rebuilt if this designated reader changes. Protect the credential file with mode 0600; do not commit or copy it into builds. The agent account can discover only owners who have granted it access.

```sh
export ANSUZ_CREDENTIALS_FILE=/private/path/agent.json
node scripts/agent.mjs identity
node scripts/agent.mjs libraries
node scripts/agent.mjs history '<owner-principal>'
node scripts/agent.mjs fetch '<owner-principal>' '<sha256>' /private/path/original-take.json
```

The CLI refreshes its session (or signs in with its saved recovery credentials), paginates history and checks downloaded bytes before writing a new file. Its stdout contains records and public identities, never credentials. One credential file should have one active CLI writer; use separate accounts/files for concurrent agents. Replay observes the captured trajectory and simulation response, not physical finger sensation or raw digitizer samples.

Deployment and recovery are in [ops/README.md](ops/README.md).
