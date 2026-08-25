# Architecture

Heliobound is organized as a small native Rust workspace. The core architectural decision is that the physical world comes first.

## Layers

1. Spatial core
2. Rendering and scene projection
3. Native app bootstrap
4. Future simulation systems
5. Future scenario and mod loading

## Spatial Core

`heliobound-core` owns durable, serializable engine state:

- vectors, rays, and cameras
- voxel and chunk coordinates
- typed voxel materials
- sparse chunked voxel storage
- virtual procedural planet rendering support

This layer does not know about ASCII, windows, pixels, or UI.

Planet-scale bodies are virtual procedural objects. The renderer sees a whole 1000x-scale planet, while the storage layer avoids dense interior voxels that would increase memory and ray traversal cost without improving the current view.

## Rendering

`heliobound-gfx` is a downstream consumer of the spatial core.

It currently provides:

- a scene model
- ASCII text rendering
- material-to-glyph mapping
- DDA voxel traversal from camera rays
- analytic planet intersection from camera rays

The renderer should not own simulation rules. It should turn snapshots into frames.

## App Bootstrap

`heliobound-cli` owns the native window and demo composition.

It should stay thin:

- create a world
- create a camera
- create a renderer
- run the event loop

As the project grows, demo scene construction should move out of `main.rs`.

## Future Simulation

Species, chemistry, civilization pressure, culture, migration, and adaptation systems should be rebuilt as a separate layer above the voxel world.

Those systems should reference physical locations and materials instead of becoming a separate authoritative world model.

## Why This Shape

The project needs a renderer soon, but the renderer should not become the engine. A voxel substrate gives us something concrete to simulate, inspect, and extend while leaving room for higher-level society behavior later.
