# Ansuz

An experimental surface for divination, questions and notes, built with Exact's Rust game/GPU surface and Contract UI.

The app starts in an angled atmospheric 3D view above an empty voxel landscape. Drag or scroll to rotate around it, with momentum that a new touch catches. **Click any exposed face** to move smoothly into a close, perpendicular view of that face: top, bottom, and all four sides work. **Landscape** returns to the atmospheric viewpoint you left. Selecting a face creates nothing; **Add a note** opens the editor. The tablet experiment is retained in git history, with no floating object in the active scene.

A saved note gives its face a dark slate fill and a small pale content marker. Hover adds a thin cyan outline; selection adds a thicker amber outline. Selection takes precedence over hover on the same face. A selected note's title and body preview lie directly on the face; the editor retains the complete text.

In the focused subview, drag to pan along that face’s plane and scroll to zoom. **Recenter** returns to the starting atmospheric view without deleting notes. Arrow keys/WASD move in either view, R returns to the landscape, Enter/F or **Focus centre** focuses a face, and Escape clears selection. Navigation pauses while writing. Distant camera transitions arc around the landscape, and the atmospheric viewpoint is retained while you work in the subview.

## Run

Initialize the pinned SDK and install its tools (Bun 1.4.2 was used):

```sh
git submodule update --init --recursive
cd tooling/exact2
bun install --frozen-lockfile
bun scripts/exact.mjs setup --check
```

Install missing browser/Rust tooling reported by that check, then from `ansuz/`:

```sh
bun exact.mjs web
```

Use its localhost URL in a WebGPU-capable browser. This is a standalone Exact app; Zhouyi is unchanged. Phone behavior and performance remain unverified. Native builds must use EAS Build and simulator checks EAS Simulator; do not use the SDK's local native build commands here.

## Records and local saving

`ansuz/data` saves a small JSON library through Exact's device-local storage adapter at `app:/data/surface-notes.json`. Notes survive reload on the same browser origin/profile. There is no account, cloud sync, import/export or multi-device ID namespace yet. Clearing browser site data removes this local library; changing preview ports uses a different browser origin. This prototype allows up to 4 MiB of library JSON.

Objects have stable IDs, content and separate typed relationships. A placement links an object ID to a voxel face `(x, y, z, face)`; the coordinate is not its identity. Face directions are 0:+Y, 1:−Y, 2:+X, 3:−X, 4:+Z, 5:−Z. Older placements without a face remain +Y; loading does not rewrite the saved file. Different faces of one cell can hold independent notes. Selecting an empty face creates nothing. Saving creates a note and placement; editing preserves its ID and relationships. Failed writes retain the draft, and unreadable existing data is not silently replaced with an empty library.

The north star remains one dataset with many spatial projections. The current UI has atmospheric and top-down navigation over the same terrain and records. The earlier raised-cluster and hologram interpretation was explicitly rejected in favor of working directly on terrain surfaces; that experiment remains in git history. Record-level projection tests remain as groundwork, without claiming a visible sphere/pyramid/tree transformation.

## Check

From `ansuz/`:

```sh
bun exact.mjs test-rust
bun exact.mjs contract build app.contract --json
bun exact.mjs test web
bun proof.mjs web
bun tools/check-surfaces.mjs http://localhost:8766
```

The last command requires an already served browser build; pass its actual URL. It uses real mouse selection and checks create, edit, reload, adjacent faces and body-only notes. Screenshots go to `artifacts/surfaces/`. Hostless tests cover picking, camera transitions, orbit/coast/selection, identity, projections and storage failures. Browser proof covers rendering, deterministic save/restore and the surface/editor flow. Its overall status remains UNVERIFIED without a cross-host baseline; do not run the full SDK `prove` command locally because it builds native hosts.

SDK revision: `4e9000abbdaa8315ae864cdb8253af143aac510c`.

## Terrain

`ansuz/tools/make-terrain.py` samples a filled 3D voxel volume, removes internal faces and greedily merges coplanar exposed faces into a static GLB. The repeating grayscale grid still identifies individual 2.5-unit cells. `terrain_pick.rs` intersects the actual triangles and resolves the nearest exposed face back to its individual cell and orientation. Back faces occlude selection rather than allowing clicks through terrain. A derived BVH accelerates picking; it is not a content store.

The field contains 463,788 occupied cells, 43,590 exposed cell faces and 19,092 merged quads (38,184 triangles). These counts describe meshing, not measured runtime improvement. This is not a runtime voxel editor, streaming system or raymarcher. Regenerate with `python3 ansuz/tools/make-terrain.py`. Meshing tests are `python3 ansuz/tools/test-terrain.py`.

The automated look check's near-black warning is intentionally silenced for the requested dark treatment. GPU-frame completion, visible-geometry coverage and remaining look checks still run; screenshots are inspected separately.
