# GPU Raycaster Status

## Current state: GPU-default presentation / CPU reference remains authoritative

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
- GPU presentation is the default. `HELIOBOUND_RENDERER=cpu` explicitly selects
  the CPU `pixels` reference backend. GPU initialization failure or any later
  frame-submission failure permanently selects CPU for that process and retains
  the observed reason in the status overlay.

## Latest integration check

- The CLI now has an explicit post-simulation `FramePresentation` boundary.
  Corn Maze, Bar, Voxel Sandbox, and Liminal can supply an authoritative
  static-world terrain request without cloning their world; modes with dynamic
  voxel actors, mixed-resolution assets, or Echolocation face filtering remain
  on the CPU reference path.
- `HELIOBOUND_RENDERER=gpu` initializes the direct surface and exercises the
  GPU terrain cache, logical 160×90 glyph target, font8x8 atlas compositor,
  and UI-cell pass for those static-world requests. An initialization or frame
  error prints its reason and presents the CPU fallback instead.
- Successful GPU frames for those modes no longer call `SceneBuilder` for
  terrain: simulation creates a UI-only scene and the CPU reference scene is
  built lazily only if a GPU submission fails. This removes the 14,400 CPU
  terrain rays from that successful path while retaining the fallback.
- The application was launched in GPU mode on 2026-09-03. A UI bind-layout
  validation error was found and fixed; the repeat launch produced no `wgpu`
  validation or shader errors. This is a smoke check, not visual-parity proof.
- A subsequent native-window smoke launch exposed the terrain background-cell
  binding as four bytes although its WGSL element is two `u32` values. The
  binding minimum is now eight bytes; the corrected GPU launch ran for 30
  seconds with no `wgpu` validation or surface errors. This observed result
  covers initialization/menu presentation only: Corn Maze, Bar, Voxel Sandbox,
  Liminal, resize/minimize, and live static-geometry editing still require an
  interactive manual pass before eligibility expands.
- `heliobound-gpu` now has an adapter-backed, test-only offscreen readback
  harness for the logical terrain targets. It compares every 160×90 glyph and
  shaded RGBA terrain cell against `heliobound_gfx::raycast` plus
  `MaterialGlyphMap` across deterministic enclosed-room, pillars/corners,
  stairs, corridor, open-view, chunk-boundary, negative-coordinate, dense,
  and edited-world fixtures. The current fixture comparisons pass.
- The terrain shader's material glyph selection now uses the CPU reference
  renderer's rounded ramp indexing, including both four- and five-character
  material ramps.
- The static GPU path now uploads the CPU-composed deterministic sky cells as
  a logical 160×90 background contract. Terrain misses retain those starfield
  cells, and terrain hits replace them, matching CPU blank/occlusion behavior
  without interactive readback. Adapter-backed terrain parity fixtures passed
  after this change, including background glyph and RGBA checks.
- The CLI now reports GPU logical rays, resident/dirty chunks, upload bytes,
  voxel cache capacity, and the retained fallback reason. A GPU-frame error
  switches subsequent frames to the CPU reference backend.
- Static pixel sprites now have a compact GPU pass using their existing 16 by
  16 bit rows and CPU framebuffer coordinates. GPU painter order is scene
  cells, sprites, then text overlays; style foregrounds and opaque sprite/UI
  backgrounds retain the CPU values.
- Every menu, gameplay, viewer, editor, playtest, and Freeplay state now uses
  the GPU presentation surface by default. Static Corn Maze, Bar, Voxel
  Sandbox, and Liminal retain direct GPU DDA terrain. All other states use a
  generic complete-scene GPU compositor request: their authoritative CPU scene
  is converted once into painter-ordered logical cells, sprites, and overlays,
  then presented by the GPU. This includes analytic planet terrain, dynamic
  actor worlds, render assets, and Echolocation's face-filtered result without
  creating a second simulation world.
- GPU submission now accepts one typed `RenderRequest` containing the terrain
  source/background, painter-ordered logical cells, pixel sprites, and final
  text overlays. The direct-DDA and complete-scene compatibility paths both
  use that boundary, so no interactive caller can accidentally retain a prior
  frame's transient UI/sprite/overlay buffers. This is a renderer-boundary
  hardening step, not direct-terrain parity for the compatibility modes.
- Direct terrain requests now include an ordered, frame-local dynamic voxel
  buffer with separate GPU capacity/count telemetry. It replaces transient
  solids every submitted frame and never invalidates static chunk residency.
  This has shader/unit validation only; dynamic-mode eligibility and
  asset-local DDA parity remain pending.

This verifies the current static terrain glyph/color contract on the covered
fixtures only. The generic compositor routes all application modes through the
GPU by default, but complete adapter-backed parity fixtures for dynamic worlds,
assets, procedural planets, and Echolocation remain future verification work.

Latest automated check: `cargo test --workspace` passed on 2026-09-03,
including 201 CLI tests and 6 GPU tests; 2 documented CLI tests remain
ignored.

Next: manually exercise Corn Maze, Bar, Voxel Sandbox, and Liminal with
`HELIOBOUND_RENDERER=gpu`, including resize/minimize recovery and edited
static geometry, before expanding GPU eligibility.

## Runtime validation attempt — 2026-09-03

- Ran `HELIOBOUND_RENDERER=gpu cargo run -p heliobound-cli` in this validation
  environment. The native process initialized and then exited normally before
  menu input could be delivered. Its output contained only macOS input-method
  startup logs; no `wgpu`, shader, or surface error was emitted.
- No eligible mode was entered, no GPU status overlay was observed, and no
  visual-painter-order comparison was performed. Corn Maze, Bar, Voxel
  Sandbox, and Liminal therefore remain pending.
- Resize, minimize/restore, sustained-play surface recovery, and duplicate
  simulation checks remain pending. Voxel Sandbox placement/removal and its
  GPU-cache update check also remain pending.
- A repeat launch from the same environment again built successfully, opened
  the native process, and exited before input could reach the menu. It emitted
  no `wgpu`, shader, or surface error; only the macOS input-method startup
  logs were observed. This likewise provides no mode, status-overlay,
  painter-order, recovery, or cache-edit result.

This unattended launch is not an interactive GPU validation result and does
not change GPU eligibility, default-backend behavior, or CPU fallback
semantics.
