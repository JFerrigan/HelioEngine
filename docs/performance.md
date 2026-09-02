# Performance Plan

## Purpose

Heliobound currently favors simple, inspectable CPU rendering over throughput.
That is appropriate for early iteration, but Zombies and Echolocation expose
several per-frame costs that make even small maps feel slower than their visual
complexity suggests. This document records the improvement plan without
changing gameplay or authored voxel detail.

## Current Frame Work

The normal voxel frame is software-rendered:

1. `SceneBuilder` traces one DDA ray for every 160 by 90 ASCII cell.
2. It creates background and visible-voxel scene cells.
3. The native app clones and sorts the scene, then draws glyphs into a 1280 by
   720 pixel buffer.

This means every frame starts with 14,400 camera rays and CPU pixel work,
regardless of whether the camera or scene changed. DDA prevents scanning all
voxels in the map, but it does not make a full-screen CPU renderer free.

### Zombies-specific work

- A complete navigation field is rebuilt from the player every frame.
- Runtime doors and other mutable geometry are copied into a fresh world every
  frame.
- A second render world is created with the active zombies stamped into it.
- Every active enemy performs movement and collision work after navigation.

### Echolocation-specific work

- The normal full-screen voxel render still runs for every frame.
- Pulses build and advance impact lists, reveal faces, and can spawn reflected
  waves.
- Footprints, wave impacts, and puzzle indicators add line-of-sight raycasts
  before being drawn as overlays.

## Improvement Order

### 1. Measure before changing behavior

Add lightweight frame timings for scene building, simulation, and software
painting. Track the median and worst frame time for an idle view and an active
combat/echo view. Use release builds for all comparisons.

Performance work should target frame-time percentiles, not only average FPS:

- 60 FPS target: 16.7 ms per frame
- 30 FPS minimum target: 33.3 ms per frame

### 2. Cache navigation and static world data

Rebuild a navigation field only when its inputs change: the player crosses a
navigation cell, a door opens/closes, or collision geometry changes. Enemies
can keep using the most recent field between those events.

Keep static maps immutable. Represent doors, enemies, and other dynamic
objects as compact render/simulation overlays rather than cloning the whole
`VoxelWorld` every frame.

### 3. Put explicit budgets on echolocation

Bound the amount of work a pulse can generate:

- cap active waves and reflected-wave sources;
- cap or prioritize visible impact markers per frame;
- perform line-of-sight checks only for markers near the camera or likely to
  project on screen;
- reuse visibility results for a short frame window when the camera has not
  moved materially.

The pulse should remain accurate at its source, but distant cosmetic markers
may be deferred or omitted when the frame budget is exhausted.

### 4. Reduce software renderer overhead

Avoid cloning the `Scene` during painting and avoid allocation churn in layers,
styles, and glyph cells. Reuse frame buffers and scene-layer capacity.

Offer an internal render scale or adaptive ASCII viewport. For example, a
temporary 120 by 68 viewport reduces primary camera rays substantially while
the final image can still be upscaled to the window.

### 5. Move the hot image path to the GPU

The durable solution for high-resolution gameplay is a GPU-backed renderer:

- keep DDA or use chunk/mesh rendering to generate the visible scene;
- upload compact glyph/tile data or geometry in batches;
- let the GPU handle rasterization, depth, and scaling;
- retain the existing `Scene` model as a debug and test-friendly fallback.

This is a larger architectural project, so it follows the caching and budgeting
work above rather than delaying immediate improvements.

## Asset Rendering

Placed mixed-resolution assets should use asset-local DDA and a world-space
spatial index. That eliminates scanning every voxel of every asset per camera
ray. It improves editor and map-viewer scalability, but it is separate from
the main Zombies and Echolocation costs described here.

## Verification

For each change, record release-build frame timings in these scenarios:

1. idle Zombies view at round start;
2. Zombies with a representative late-round enemy count;
3. idle Echolocation view;
4. Echolocation during a charged pulse with reflections and a nearby pursuer;
5. editor view with at least ten placed assets.

Also retain behavior tests for DDA hit correctness, enemy navigation,
echolocation reveals/puzzle behavior, and map rendering. An optimization is
not accepted if it changes collision, nearest-hit ordering, or authored voxel
silhouettes without an explicit design decision.
