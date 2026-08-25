# Graphics System

The graphics layer turns 3D voxel state into a 2D ASCII scene.

## Current Pipeline

1. The app provides a `VoxelWorld`, `Camera`, viewport, and tick.
2. `SceneBuilder` generates one camera ray per ASCII cell.
3. The builder uses DDA traversal to walk the voxel grid.
4. The nearest solid voxel becomes a `SceneCell`.
5. `MaterialGlyphMap` converts typed materials into glyphs.
6. The native window paints those glyphs into a pixel buffer.

## Why DDA

Voxel rendering should not test every component in the world.

DDA traversal steps through only the grid cells crossed by a ray, which fits ASCII rendering well because each screen cell needs only the first visible voxel.

Planet-scale scenes depend on two additional constraints:

- the voxel world caches populated bounds so rays that miss the world skip immediately
- virtual planets use analytic sphere intersection plus procedural material sampling
- local voxel planets store crust/surface voxels first, not dense invisible interiors

## ASCII First

ASCII is the first renderer because it is:

- inspectable
- cheap to test
- compatible with simulation debugging
- a clean bridge to later low-color and pixel-art styles

The scene model should survive a later tile or sprite renderer.

## Extension Points

Near-term extension points:

- alternate material glyph maps
- camera controllers
- debug overlays
- alternate raycast budgets
- separate render modes for simulation inspection

Longer term, renderers should consume snapshots rather than mutable simulation state.
