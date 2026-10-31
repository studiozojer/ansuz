# Ansuz

An experimental surface for divination, questions and notes, built with Exact's Rust game/GPU surface and Contract UI.

The app starts in an angled atmospheric 3D view above the upper hemisphere of the empty voxel landscape, where top faces are visible and selectable. Drag or scroll to rotate around it, with momentum that a new touch catches. **Descend** opens the top-down surface editor; **Ascend** returns to the atmospheric viewpoint you left. Select a terrain face before descending to focus that face, or descend without a selection to start at the sphere’s top. In the editor, hover an actual terrain square to highlight it; click to select it, then **Add a note**. A saved note colors that square. Click it again to read or edit. No raised topic cubes, 3×3 clusters, sample readings, floating holograms or particles are seeded.

Selecting a top face reveals a single voxel clay tablet above that square. The stepped slab rises into view and can be turned with the arrow buttons or Q/E. Click the tablet itself to open that surface’s note, in either navigation view. A drag moves the scene without opening the editor; Add a note/Edit remains a button alternative. It works in both views; **Close** or Escape clears it. This is a temporary presentation of the selected surface, not another root or saved record. Notes still appear in the adjacent text panel and editor; text is not yet inscribed into the 3D mesh.

In the surface subview, drag to pan and scroll to zoom. **Recenter** returns to the starting atmospheric view without deleting notes. Arrow keys/WASD move in either view, F descends, R ascends, Enter or **Select centre** selects a face, and Escape clears selection. Navigation pauses while writing. Transitions follow an arc outside the sphere, and the atmospheric viewpoint is retained while you work in the subview.

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

Objects have stable IDs, content and separate typed relationships. A placement links an object ID to a voxel face; the coordinate is not its identity. Selecting an empty face creates nothing. Saving creates a note and placement; editing preserves its ID and relationships. Failed writes retain the draft, and unreadable existing data is not silently replaced with an empty library.

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

`ansuz/tools/make-terrain.py` samples a filled 3D voxel volume, removes internal faces and greedily merges coplanar exposed faces into a static GLB. The repeating grayscale grid still identifies individual 2.5-unit cells. `terrain_pick.rs` intersects the actual triangles, rejects a nearest side/underside, and resolves a merged top face back to its individual cell. A derived BVH accelerates picking; it is not a content store.

The field contains 463,788 occupied cells, 43,590 exposed cell faces and 19,092 merged quads (38,184 triangles). These counts describe meshing, not measured runtime improvement. This is not a runtime voxel editor, streaming system or raymarcher. Regenerate with `python3 ansuz/tools/make-terrain.py`; regenerate the voxel clay slab with `python3 ansuz/tools/make-tablet.py`. Meshing tests are `python3 ansuz/tools/test-terrain.py`.

The tablet is a filled 0.25-unit voxel volume: 1,116 occupied cells, 888 exposed faces and 1,776 triangles. Hidden faces are removed and the result is one static mesh with flat face normals and quantized clay tones. It is not one scene entity per cube or a runtime voxel sculpting tool.

The automated look check's near-black warning is intentionally silenced for the requested dark treatment. GPU-frame completion, visible-geometry coverage and remaining look checks still run; screenshots are inspected separately.
