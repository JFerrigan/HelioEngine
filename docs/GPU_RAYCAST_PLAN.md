# GPU DDA Raycaster Migration

This is the implementation plan for the GPU migration. The CPU `SceneBuilder`,
DDA raycaster, and glyph painter remain the visual reference and the fallback.
The GPU backend is direct `wgpu 29`, operates at the 160×90 logical-cell
resolution, and never reads interactive-frame terrain results back to CPU.

## Milestones

1. **Complete:** expose bounded, revisioned dense chunk snapshots from
   `heliobound-core`.
2. **Complete in the GPU crate:** direct surface creation/recovery and a
   fullscreen terrain pipeline; retain CPU presentation in the CLI.
3. **Complete in the GPU crate:** camera/bounds uniforms, bounded sparse
   lookup, fixed dense slots, and revision-only upload policy.
4. **Complete at shader-unit level:** camera rays and DDA semantic validation.
   Adapter-backed readback fixtures remain required before parity is claimed.
5. Add logical render targets, glyph atlas, GPU UI cells, and nearest upscale.
6. Refactor CLI frame construction into CPU simulation + terrain render request
   + UI scene, then select GPU/CPU backend at that boundary.
7. Enable static voxel modes only after glyph, colour, silhouette, and
   occlusion parity; dynamic overlays migrate independently and retain CPU
   fallback until their own parity work is complete.

GPU residency is bounded by the camera chunk range. Only occupied chunks in
that range get fixed storage slots; unchanged chunk revisions are not uploaded.
