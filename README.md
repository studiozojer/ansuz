# Ansuz

An experimental atlas for divination: I Ching, runes, tarot, and the notes and questions that grow around readings.

This first prototype explores a curved landscape with fixed clusters of sample readings. Drag to glide, touch a clearing to select it, then press **Descend**. **Ascend** restores the viewpoint you left. Arrow keys/WASD and **Next clearing** provide alternatives; **Gentle motion** removes the long camera transitions and coasting.

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

The Rust tests exercise selection, descent/return, stable geography, reduced motion, and catching a coast with a new touch. The browser checks exercise the visible controls and render the real GPU scene. `pins.json` has no native proof baseline yet; do not run the SDK's full `prove` command locally because it also builds native hosts.

The scene and camera live in `ansuz/logic/src/lib.rs`; the interface is `ansuz/app.contract`. SDK revision: `4e9000abbdaa8315ae864cdb8253af143aac510c`.
