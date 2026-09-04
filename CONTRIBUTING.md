# Contributing to Heliobound

Thanks for helping shape Heliobound. The project is a Rust voxel-simulation
foundation with an ASCII-first visual language, so contributions should keep
simulation state, rendering, and native presentation clearly separated.

## Getting started

Use the repository environment helper, then build and run the application:

```bash
. ./scripts/heliobound-env.sh
cargo run -p heliobound-cli
```

Run the full verification suite before opening a pull request:

```bash
cargo fmt --check
cargo test --workspace
git diff --check
```

Use `cargo run --release -p heliobound-cli` when evaluating performance or
interactive rendering.

## Project boundaries

- `heliobound-core` owns durable spatial state, voxel materials, map
  compilation, and procedural generation. It must not depend on windowing or
  rendering concepts.
- `heliobound-gfx` projects authoritative world state into scenes and retains
  the CPU reference renderer.
- `heliobound-gpu` owns GPU caching and presentation only; it must not become
  a second source of simulation truth.
- `heliobound-cli` owns application flow, controls, and native-window wiring.

Favor typed data, deterministic behavior, and small testable interfaces over
implicit state or presentation-specific shortcuts. Keep rendering downstream
from simulation state.

## Changes and tests

- Keep changes focused and explain the user-visible or architectural reason.
- Add or update deterministic tests with behavior changes, especially for
  voxel traversal, map compilation, rendering parity, controls, and game-mode
  rules.
- Preserve the CPU renderer as the visual authority when changing GPU paths.
  GPU work should include appropriate adapter-backed or shader validation where
  practical.
- Do not claim desktop visual parity from a logical-target test alone; verify
  the final compositor when color or painter ordering is involved.
- Update the relevant document in `docs/` when a public contract, control,
  renderer boundary, map format, or living design note changes.

## Maps and assets

Maps and voxel assets are versioned content contracts, not informal examples.
Use the authoring guides before changing their formats:

- [Map authoring](docs/map-authoring.md)
- [Voxel assets](docs/voxel-assets.md)

Keep IDs stable, make generated content deterministic, and validate new content
through the normal loader/compiler path.

## Pull requests

Describe what changed, why it changed, and how it was verified. Include
screenshots or a short capture for visual changes when possible. Call out any
known limitations, deferred desktop checks, or intentionally unchanged behavior
so reviewers can assess the scope accurately.
