# Ansuz identity and recording contract

David's correction after the recording-history build: “We don't want email/password, we want atproto / everything account login.” Separate Ansuz and Jera services do not mean separate user identities. This supersedes the agent-authored password-account design in the first recording-history implementation.

People authorize an existing ATproto / everything account. Its verified DID is their stable identity for session ownership and permissions. Use compliant ATproto OAuth; entering a DID, resolving a handle or finding an account suggestion does not authenticate it. Do not substitute Snapback password accounts because an example or integration test uses them.

Snapback remains the private recording archive on Ansuz's separate endpoint. Login does not publish recordings or notes to an ATproto repository. Local recording stays available offline. Preserve original traces, expected behavior and provenance separately from later recognizer interpretations. Agents act with their own attributable identity and explicit read grants, never by borrowing the human's credentials.

The Jera implementation checkpoint is `zojercommons/projects/jera/specs/2026-10-31-atproto-login.md` in the composed workspace; the corresponding code is commit `aafa291` on Jera's `feat/build-delivery`, with subsequent account discovery at `f87d871`. Reuse its verified-DID session boundary and official OAuth SDK approach, adapting endpoint, callback, client metadata and secret namespaces for Ansuz. Review its documented acceptance limits rather than treating fixtures or native installation as proof of real-account login.

Before calling Ansuz login aligned, complete a real existing-account authorization on the phone, verify the DID and recording ownership, test account switching and relaunch, and reject unverified identity claims. Inventory existing cloud ownership before migration. Preserve old recordings and local files; never guess an email-to-DID mapping or reset the service. Separately define authenticated operator onboarding before retiring its legacy account.

Current status: the deployed Ansuz recording service and installed phone build still use the rejected password flow. Storage, replay, sharing and restore checks passed under that mechanism; those results do not satisfy ATproto login acceptance. This contract records the correction; it does not claim the runtime migration is complete.
