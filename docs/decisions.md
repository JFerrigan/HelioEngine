# Major Decisions

## 1. Use Rust

Rust is the best default here because the project needs native performance, deterministic data handling, and clear modular boundaries.

## 2. Make Voxels the First Runtime Truth

The engine now starts with a 3D voxel substrate instead of an inherited civilization graph.

This gives the simulation a physical world before we add culture, species, or ecology.

## 3. Render 3D Through 2D ASCII

The first renderer uses camera rays and DDA voxel traversal to produce an ASCII frame.

That keeps the current display simple while still exercising real 3D world structure.

## 4. Use Typed Materials Internally

Engine systems should work with `VoxelMaterial`, not glyphs or string keys.

String identifiers can return later for mod loading, but the engine boundary should stay typed.

## 5. Keep Civilization Simulation as a Later Layer

Civilizations should still behave like adaptive systems under pressure, but they should sit above the spatial core.

The next version of that layer should attach to voxel places, ships, habitats, and resources rather than owning a separate world graph.
