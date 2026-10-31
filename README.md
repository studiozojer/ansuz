# Ansuz

An experimental atlas for divination: I Ching, runes, tarot, and the notes and questions that grow around readings.

This first prototype explores a dark, gridded voxel landscape with fixed clusters of sample readings. Drag to glide, touch a region to select it, then press **Descend**. **Ascend** restores the viewpoint you left. Arrow keys/WASD and **Next region** provide alternatives; **Gentle motion** removes the long camera transitions and coasting.

On desktop, hover the top of one of the five topic platforms to illuminate it and reveal a floating wireframe Platonic solid. Scroll over that top to cycle tetrahedron, cube, octahedron, dodecahedron and icosahedron. **Next region** / N focuses a topic without a mouse; **Previous solid** / Q and **Next solid** / E change its representation. Gentle motion also stops hologram bobbing. Decorative terrain cells are not yet editable topics.

Built with Exact's Rust game/GPU surface and a Contract interface. This is a new standalone app, not an Expo screen or a migration of Zhouyi. It has no account, journal persistence, or backend connection yet. The sample cards are landmarks, not working readings.

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

Use the localhost URL it prints in a WebGPU-capable browser. This browser preview is the first test surface; native phone performance is not yet verified. Native builds must use EAS Build and simulator checks EAS Simulator; do not use the SDK's local native build commands for this project.

## Check

From `ansuz/`:

```sh
bun exact.mjs test-rust
bun exact.mjs contract build app.contract --json
bun exact.mjs test web
bun proof.mjs web
```

The Rust tests exercise selection, descent/return, stable geography, reduced motion, catching a coast with a new touch, root records and hover geometry. The browser checks exercise the visible controls, preservation of records through shape changes, and the real GPU scene. `pins.json` has no native proof baseline yet; do not run the SDK's full `prove` command locally because it also builds native hosts.

With the web build already served, `bun tools/check-hover.mjs http://localhost:8766` checks real desktop mouse hover, wheel cycling, leaving the top and reaching the shape buttons. Pass the actual preview URL if its port differs. Screenshots go to `artifacts/desktop-hover/`.

The scene and camera live in `ansuz/logic/src/lib.rs`; the interface is `ansuz/app.contract`. SDK revision: `4e9000abbdaa8315ae864cdb8253af143aac510c`.

## Records and views

`ansuz/logic/src/records.rs` holds fixture objects with stable IDs and separate containment, derivation and association links. Derivation links can name a source block. Shapes and positions belong to presentation, so changing a hologram cannot change a reading's identity or provenance. Pure surface and provenance projections demonstrate resolving the same records through different layouts; a visible sphere-to-pyramid transition is not implemented yet.

Weights are authored sample values. The provisional aggregation counts each reachable contained object once, including the root; associations and derivations do not add weight. The current motion experiment makes heavier aggregates bob more slowly. Both rules are tunable assumptions, not an implementation of the cited P-tree paper or a claim about sacred geometry. Shapes carry no fixed divination-type meaning.

The source of this slice is David's written root-node outline and subsequent multiple-geometries north star, preserved separately in the armillary commons. The audio transcript is a separate source and has not been used as evidence for this implementation.

## Voxel terrain

`ansuz/tools/make-terrain.py` samples a filled 3D voxel volume, removes faces between occupied cells, and greedily merges coplanar exposed faces. It bakes a static GLB for Exact's mesh renderer, with a repeating grayscale cell grid. This is voxel-derived geometry, not a runtime voxel editor, raymarcher, streaming system, or new graphics backend. The underlying curved coordinate system remains for navigation; there are no ocean, continent, or sun entities.

The current field has 463,788 occupied cells, 43,590 exposed cell faces and 19,092 merged quads (38,184 triangles). These counts compare meshing strategies for this field; they do not establish a frame-rate improvement over the previous scene. Runtime chunk streaming and distance-based detail are not implemented. Reading anchors are independent of the terrain. Ambient square particles drift around regions; Gentle motion stops new particles and lets existing ones fade.

Regenerate with `python3 ansuz/tools/make-terrain.py`; check face removal, merging, winding and cavity preservation with `python3 ansuz/tools/test-terrain.py`.

The automated look check's `near-black` warning is intentionally silenced for this user-requested dark treatment after inspecting both screen orientations and the descended view. GPU-frame completion, visible-geometry coverage, and the remaining look checks still run. No native performance result is claimed.
