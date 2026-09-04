# Heliobound

Heliobound is a native Rust prototype for exploring hard-science-fiction
worlds through a 3D voxel simulation rendered as a crisp 2D ASCII interface.
It is designed as an engine foundation: a spatial substrate that can grow into
systems for exploration, environments, authored scenarios, and simulation.

## Highlights

- A sparse, chunked voxel world with typed materials and negative-space support.
- Deterministic procedural worlds, including planet-scale virtual terrain that
  avoids materializing invisible interiors.
- Camera-driven DDA raycasting that projects 3D scenes into a 160×90 logical
  ASCII display.
- GPU-default presentation through `wgpu`, with a CPU reference renderer kept
  available for validation and automatic fallback.
- Data-driven maps and mixed-resolution voxel assets for reusable authored
  environments.
- Native tools for browsing assets, inspecting maps, editing finite maps, and
  testing unsaved work in place.

## Architecture

The workspace keeps durable simulation state separate from visual projection:

| Crate | Responsibility |
| --- | --- |
| `heliobound-core` | Spatial math, voxel storage, materials, map compilation, and procedural terrain. |
| `heliobound-gfx` | Scene composition, CPU reference rendering, voxel traversal, and material-to-glyph mapping. |
| `heliobound-gpu` | GPU terrain caching, logical glyph composition, sprites, UI, and surface presentation. |
| `heliobound-cli` | Native application bootstrap, controls, modes, tools, and demo composition. |

This separation keeps rendering downstream from authoritative world state and
makes the visual path testable without opening a desktop window.

## Run

```bash
. ./scripts/heliobound-env.sh
cargo run -p heliobound-cli
```

For an optimized build:

```bash
cargo run --release -p heliobound-cli
```

GPU presentation is selected by default. Set `HELIOBOUND_RENDERER=cpu` to run
the software `pixels` reference renderer explicitly.

## Explore and author

The application includes multiple voxel environments, a map viewer, an asset
viewer, and an in-progress keyboard-first map editor. Reusable assets are
loaded from [`assets/voxel-assets`](assets/voxel-assets); authored map
blueprints live in [`assets/voxel-maps`](assets/voxel-maps).

For authoring contracts and controls, start with:

- [Voxel assets](docs/voxel-assets.md)
- [Maps and map viewer](docs/maps.md)
- [Data-driven map authoring](docs/map-authoring.md)
- [Map editor roadmap](docs/map-editor.md)

## Technical notes

The GPU renderer maintains bounded chunk residency and renders one terrain ray
per logical cell without interactive GPU-to-CPU terrain readback. Its output
is checked against the CPU renderer with adapter-backed parity tests, including
final presentation color handling.

See the [foundation](docs/foundation.md), [graphics](docs/graphics.md), and
[GPU renderer status](docs/GPU_RAYCAST_STATUS.md) for deeper design and
validation detail.
