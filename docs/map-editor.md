# Map migration and editor roadmap

## Goal

Every finite gameplay environment has one checked-in, validated
`*.hbmap.json` blueprint. The map viewer, a future editor, and game startup
all compile that same blueprint. Rust owns only per-run state and runtime
overlays.

## Phase 1: migrate authoritative map sources

The first phase removes the remaining split between legacy builders and map
files without changing their initial layouts.

1. Migrate only environments with an already-stable authored or seeded source.
   City Walk is the first such migration.
2. Represent deterministic environments with registered
   `generator_region` operations rather than serializing their entire voxel
   output. The map records its seed, region, and exact generator parameters.
3. For each migration, add a parity test against the retained legacy builder:
   occupied coordinates, materials, voxel count, bounds, and player start
   transform must match exactly.
4. Switch game startup and the map viewer to fresh sessions cloned from the
   compiled map. Delete the legacy static builder only after parity and
   gameplay regression tests pass.
5. Keep Corn Maze, Voxel Sandbox, and Drone Gate on their legacy procedural
   paths for now. Each needs a deliberately designed, pure seeded procedural
   interface before migration; do not serialize its current generated output
   or make it an editor-authored blueprint as a shortcut. Drone Gate also
   intentionally generates a new randomized course every run.

The first increment is City Walk: it is a deterministic core generator with a
stable seed and is therefore the lowest-risk proof that `generator_region`
can be authoritative.

Corn Maze, Voxel Sandbox, and Drone Gate are therefore outside the current
blueprint migration scope. Their future work is procedural-engine design, not
map-editor work.

## Phase 2: make map data editor-ready

- Keep strict versioned validation and stable IDs for markers and asset
  placements.
- Add a canonical exporter from a mutable voxel working world to deterministic
  map operations. The editor edits a world and structured entities; it does
  not preserve a historical sequence of hand-written operations.
- Add round-trip tests: load, export, reload, and compare world and markers.
- Add editor metadata only where it is source data (display name, authoring
  bounds, presentation defaults), never runtime game state.

## Phase 3: evolve the Map Viewer into an editor foundation

First make the viewer a trustworthy read-only inspector: compiled geometry,
asset instances, bounds, collision, marker labels, and non-blocking catalog
errors. It must launch the selected map through the same fresh-session path as
gameplay.

## Phase 4: minimum viable editor

Add voxel paint/erase/material and box tools, undo/redo, temporary working
copies, save-as, and validated direct save. Then add marker and asset placement
with mode-specific property panels. Invalid content must never overwrite a
valid source file.

## Phase 5: mode-specific authoring

Expose typed authoring panels for each mode: City landmarks; Corn Maze goal
and encounters; Zombies doors, groups, and wall weapons; Liminal rooms,
connections, signs, objectives, and lights; and Echolocation receivers,
pipes, doors, pressure plates, and pursuer anchors. Gameplay reads typed
adapters rather than raw string markers.

## Completion criteria

- Every finite game and viewer entry starts from one compiled map catalog.
- No legacy static builder remains after its parity test has passed.
- Editor saves are deterministic, validated, and round-trip tested.
- `cargo test --workspace` remains green throughout the migration.
