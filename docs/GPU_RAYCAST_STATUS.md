# GPU Raycaster Status

## Current state: direct GPU terrain foundation / CPU runtime remains active

- `VoxelWorld` now exposes deterministic dense chunk snapshots restricted to a
  caller-supplied inclusive chunk range.
- Every actual cell mutation increments that chunk's monotonic revision; no-op
  sets and clears do not. Revisions survive clones and empty-chunk removal.
- `heliobound-gpu` now owns a direct `wgpu 29` surface with resize,
  suboptimal/outdated/lost-surface recovery, a fullscreen triangle pipeline,
  and a Naga-validated WGSL DDA terrain pass.
- The GPU terrain cache contains a 128-byte camera/bounds/table uniform, a
  bounded chunk-coordinate-to-slot lookup buffer, and dense `16³` slot
  storage. Only newly resident or revision-changed slots are written; buffer
  growth copies existing resident storage and no normal-frame readback exists.
- WGSL matches the CPU DDA's finite bounds slab, outside-entry epsilon,
  Euclidean negative chunk addressing, axis-aligned handling, and X/Y/Z tie
  ordering. Unit tests cover uniform layout, negative-coordinate table lookup,
  slot reuse, revision uploads, and WGSL validation.
- The interactive CLI intentionally still uses `pixels` and the CPU renderer.
  The new terrain pass currently outputs a diagnostic normal/distance image to
  a physical surface; routing normal gameplay to it before logical targets,
  glyph/UI passes, and the `AppState` render-request split would remove the
  current visual/game-mode presentation.

Next: refactor the CLI's fused `AppState::frame` into simulation plus a terrain
render request and separate UI scene. Then make the terrain pass render to the
160×90 logical target, add material/glyph output and GPU UI cells, and use the
CPU renderer only as the selectable reference fallback.
