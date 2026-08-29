# Heliobound

Heliobound is a native hard-science-fiction simulation prototype built around a 3D voxel world rendered as 2D ASCII.

The current goal is not a playable game loop. The goal is a clean engine foundation:

- chunked voxel storage
- camera and ray primitives
- procedural planet generation
- per-cell ASCII projection through voxel traversal
- room for future simulation systems above the spatial layer
- ASCII first, then low-color and pixel-art rendering later

## Current Shape

- `crates/heliobound-core`: camera math, rays, voxel coordinates, materials, chunked voxel world, virtual procedural planets
- `crates/heliobound-gfx`: scene model, ASCII renderer, voxel raycaster, virtual planet renderer, material-to-glyph mapping
- `crates/heliobound-cli`: native window bootstrap and deterministic demo scene
- `docs/`: design notes and architecture decisions

Start with [docs/foundation.md](docs/foundation.md) for the current root structure.
For the graphics layer, read [docs/graphics.md](docs/graphics.md).

## Run

```bash
. ./scripts/heliobound-env.sh
cargo run -p heliobound-cli
```

For smoother rendering, run the optimized build:

```bash
cargo run --release -p heliobound-cli
```

The current planet is rendered as a virtual procedural body at 1000x the original prototype scale. Local voxel shells are still supported for smaller bodies and future landing-zone detail, but planet-scale rendering does not materialize billions of surface voxels.

For the voxel asset viewer and drop-in `*.hbasset.json` authoring workflow, see
the dedicated [voxel asset usage guide](docs/voxel-assets.md). Assets belong in
[`assets/voxel-assets`](assets/voxel-assets) and are discovered at startup.

For the pan-and-orbit gameplay map viewer and an overview of how maps are
currently represented, see [maps and the map viewer](docs/maps.md).

## Controls

- Click the window to capture mouse look.
- `W` / `S`: thrust forward and backward.
- `A` / `D`: strafe left and right.
- `Space` / `Ctrl`: move up and down.
- `Q` / `E`: roll.
- `Shift`: boost.
- `Escape`: release the mouse; press again to quit.
