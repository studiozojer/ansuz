# Ansuz ATproto authorization

People authorize an existing everything account through ATproto OAuth. The official `@atproto/oauth-client-node` SDK handles discovery, PAR, PKCE, DPoP and the verified subject. The service requests only the `atproto` identity scope. It does not publish gesture recordings to a PDS.

This implementation derives from Jera's `foundation/auth` at `aafa291`, with account-directory work from `f87d871`. Ansuz has its own client metadata, callback, signing key, OAuth store, Snapback service credential and device secret namespace. Shared hosting does not share app sessions or recordings. The sign-in picker reuses Jera’s 250 ms typing delay, public directory lookup, popover dismissal and keyboard/tap selection, with Ansuz’s grayscale styling. Search begins at two characters; direct DIDs skip search. Suggestions show public display names and handles. Selecting one fills the handle without authenticating it; Log in still opens OAuth. Search text goes to the public Bluesky index and the Jera PDS directory. This is an incomplete directory, so a full handle or DID remains usable when search has no match or is unavailable.

The device prepares a random verifier and state before the user presses Log in. The system authorization browser receives only a challenge. The callback must match the original browser cookie and SDK-verified flow; a short-lived, single-use handoff code returns to `studio.zojer.ansuz:/oauth`. Exchanging it requires the original device verifier. Only then does the private service mint an ordinary Snapback session whose principal is the verified DID. A handle or DID typed in the field is never sufficient proof.

The public proxy blocks `/auth/session`, `/auth/revoke` and `/auth/delete-account`. Password and guest identities are disabled in the schema. `/oauth/check` checks a saved session; `/auth/refresh` checks underlying OAuth authority before rotating the Snapback bearer. Definite revocation retires that DID's Snapback sessions. A transport failure preserves local recordings and the previously verified identity for offline capture. A rejected session clears account views and requires authorization again.

The server uses a private SQLite store and signing key under `/var/lib/ansuz-recordings/auth`. Whole-service backups include both that directory and the Snapback deployment. The server-only minting token must never be placed in an app or agent credential file. The separately provisioned `operator:ansuz-recorder` has an ordinary rule-checked session and sees only explicitly shared libraries.

## Verification and limits

`npm ci && npm test` runs the HTTP handoff, browser binding, expiry, origin, replay, revocation and discovery tests. These substitute already-verified provider evidence; they do not prove human account authorization. `bun ansuz/tools/check-accounts.mjs` from the repo root additionally exercises the real Exact browser runtime against a disposable local service after `bun ansuz/exact.mjs web-build`.

The browser fixture owns ports 4435, 4436 and 8768. Serve Exact's extensionless `/.exact/auth/callback` as HTML; a generic static server may treat it as a download and strand the popup. The production callback allowlist currently supports the native app and the local web preview at port 8768. Adding a hosted web origin requires deliberate matching changes to the app grants and service allowlist.

The app is currently a single-runtime client. Snapback refresh rotates and invalidates the previous bearer. If the app is killed between rotation and durable receipt, a fresh authorization may be required; recordings are preserved. Sign-out immediately forgets the device credential and attempts remote logout; offline logout cannot promise server revocation. Sign-out does not revoke a previously granted agent reader. No automatic background synchronization or account deletion UI is implemented.

Real existing-account authorization, callback return and relaunch on the physical phone remain a human acceptance check, tracked in PRODUCT.md. Installation and simulated provider tests do not substitute for that check.
