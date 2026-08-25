# Core Foundation

Heliobound now starts at the physical layer: a 3D voxel substrate that can later host societies, ecologies, ships, habitats, and large-scale simulation systems.

## 1. Spatial Primitives

The core crate owns the stable math and addressing types:

- `Vec3`
- `Ray`
- `Camera`
- `VoxelCoord`
- `ChunkCoord`

These types should stay free of rendering and game-rule assumptions.

## 2. Voxel World

The voxel world is the first runtime truth:

- chunks store sparse voxel cells
- voxel coordinates support negative space
- materials are typed, not stringly typed
- empty cells are implicit

This gives us a physical model that can scale before we add higher-level behavior.

## 3. Procedural Planets

Planet generation is part of the core spatial layer. Heliobound supports two planet representations:

- `ProceduralPlanet`: virtual planet-scale rendering and sampling
- `PlanetGenerator`: small voxel-shell generation for local bodies and future detailed regions

The virtual planet creates:

- a full spherical planet
- deterministic terrain from a seed
- polar ice and lowland ice
- basalt highlands and subsurface crust
- carbon and silicon life patches
- crater depressions

The performance rule is strict: sample planet-scale terrain procedurally and generate voxels only where local simulation actually needs them. Do not fill millions or billions of invisible cells just to say the planet is solid.

## 4. Material Layer

Voxel materials are engine-level facts:

- regolith
- basalt
- ice
- carbon life
- silicon life
- habitat
- ship hull
- glass
- beacon

Rendering maps these materials to glyphs. Simulation systems should read materials as structured data, not display characters.

## 5. Presentation

The graphics crate consumes spatial state and camera data, then creates a 2D scene.

The voxel renderer uses one ray per ASCII cell and DDA traversal to find the nearest visible voxel. The planet renderer uses analytic sphere intersection plus procedural surface sampling.

## 6. Future Simulation

Civilizations, species, chemistry, adaptation, and pressure systems should be rebuilt above the spatial core.

The future society layer should reference places in the voxel world rather than replacing it with a separate region graph.

## Foundation Rules

- Keep rendering downstream from spatial state.
- Keep high-level simulation downstream from the voxel substrate.
- Prefer typed engine data before mod-facing strings.
- Use composition and explicit services instead of inheritance trees.
- Make each layer testable without opening a window.
