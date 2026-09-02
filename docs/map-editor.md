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
- `heliobound-core` now exposes `CompiledMap::editable` and `export_map`.
  The exporter turns a mutable voxel working world into deterministic,
  coalesced `fill_box` operations and retains typed markers plus placed assets.
  It deliberately does not preserve a historical sequence of handwritten
  fills, clears, or generator operations.
- Unit-scale asset cells are owned by their retained asset entity during
  export. A future editor must move or remove the entity before painting its
  cells; this prevents custom asset colors from leaking into direct map
  materials.
- Round-trip tests load, export, reload, and compare geometry, markers,
  player start, and asset instances. The exporter is deterministic.
- Add editor metadata only where it is source data (display name, authoring
  bounds, presentation defaults), never runtime game state.

## Phase 3: evolve the Map Viewer into an editor foundation

Implemented as a trustworthy read-only inspector. Compiled previews are built
from `CompiledMap::fresh_session`, exactly as gameplay starts are. The HUD
shows the stable map ID and source, declared bounds, static collision voxel
count, retained asset instances, and typed marker labels. Bad map files remain
non-blocking and appear in the viewer HUD while valid entries stay selectable.
Legacy Corn Maze, Sandbox, and Drone Gate previews retain their explicit
`CLI generator` source and derived bounds until their procedural contracts are
ready for migration.

## Phase 4: minimum viable editor

Phase 4 turns the read-only Map Viewer into a safe authoring workflow for the
already compiled `hbmap` entries. It is deliberately a voxel-map editor, not a
general-purpose 3D modelling package: direct map voxels, placed reusable
assets, typed map markers, and the source contract are its only authority.

### User workflow

1. Open `Dev Tools`, select **Map Editor**, then select a valid `hbmap` map.
   Legacy procedural entries remain visible as previews but are read-only and
   explain why they cannot be edited.
2. The editor creates an in-memory `EditableMap` from the selected compiled
   blueprint. The original catalog entry and its on-disk file are never
   mutated while editing.
3. Navigate in the existing collision-free inspector camera. Ray-pick a voxel
   face, choose a tool and material, and make a local edit. A selection outline
   and concise HUD always show the target coordinate, active tool, material,
   dirty state, and undo depth.
4. Validate and preview the working copy at any time. The preview is compiled
   through the same core compiler gameplay uses, not through an editor-only
   rendering shortcut.
5. Save a new map with **Save As**, or explicitly replace the source with
   **Save**. Both paths validate, serialize deterministically, write a
   temporary sibling file, and atomically replace only the chosen destination
   after all checks succeed.
6. Leave through `M` or Escape. A dirty map requires an explicit choice to
   discard, save, or cancel; keyboard commands may never silently discard it.

### Editor scope and non-goals

Phase 4 supports only maps whose source is an `*.hbmap.json` file. It edits a
coalesced static representation; it does not attempt to preserve the original
hand-authored operation order, clears, or generator regions. Exported geometry
is deterministic `fill_box` output, as defined by Phase 2.

It does not:

- edit legacy Corn Maze, Voxel Sandbox, or Drone Gate generation;
- edit runtime overlays such as enemies, opened doors, purchased weapons,
  anomaly mutations, echo signals, or pursuer state;
- create or modify `hbasset` voxel models; Asset Viewer remains the inspection
  tool for those files;
- add arbitrary JSON fields, free-form marker kinds, scripted behaviors, or a
  second physics/collision model; or
- promise multi-user editing, history/version-control UI, copy/paste between
  maps, terrain sculpting, or mouse-driven gizmos in the first release.

### Editor data model

`heliobound-core` remains the owner of all durable editor data and validation.
The CLI owns only input, panels, current selection, and pending confirmation
state.

| Data | Core representation | Rules |
| --- | --- | --- |
| Static geometry | `EditableMap` working `VoxelWorld` | Direct cells hold only named `VoxelMaterial`; empty is erase. |
| Bounds and identity | editable map metadata | Bounds remain inclusive and all edits must stay inside them. ID and filename rules are checked on save. |
| Player start | typed player-start metadata | There is exactly one; its clearance and finite transform are compiler-validated. |
| Assets | retained typed asset instances | An instance owns unit-scale expanded cells. Paint/erase cannot partially overwrite it; move or delete the complete instance first. |
| Markers | typed marker metadata | Marker IDs are globally unique and every kind is validated against the selected mode. |
| History | editor command records | Commands contain before/after data sufficient for exact undo and redo; history is never serialized. |
| File state | source path plus dirty/clean state | File paths are UI metadata, not map schema data; invalid or unsaved state never enters the runtime catalog. |

An edit command is one atomic user action: a single-cell paint, a box fill,
an erase, a marker move/property change, or an asset place/move/delete. A box
operation must be undoable in one step rather than one history entry per cell.
The initial history limit is 128 commands or 32 MiB of retained cell diffs,
whichever is reached first; discard oldest commands with an HUD notice rather
than growing without bound.

### Phase 4A: editor shell and safe working copies

Build the editor as a new Dev Tools menu entry, reusing Map Viewer catalog
selection, camera, rendering, asset discovery, and diagnostic presentation.

- Show only valid compiled `hbmap` files as editable; show the stable ID,
  bounds, asset count, marker count, static voxel count, and catalog errors.
- Load via `CompiledMap::editable` into an independent working copy. Starting,
  restarting, or returning to the viewer must always use the untouched compiled
  blueprint, proving the editor cannot leak edits into gameplay by reference.
- Add explicit editor modes: Browse, Geometry, Markers, Assets, Validate, and
  Save. The active mode is visible in the HUD and has a short key-help line.
- Provide a deterministic reset-to-source action that replaces the working
  copy only after dirty-discard confirmation.
- Keep the first UI keyboard-first and ASCII-friendly. Mouse ray picking is a
  convenience, not a required path: keyboard nudge/select controls must make
  every supported operation reachable without a mouse.

Acceptance: opening, leaving, or resetting an editor session cannot alter the
catalogue blueprint; a dirty session is visibly distinguishable from a clean
one; invalid catalog files remain non-blocking.

Implementation status: the Phase 4A shell is available from `Dev Tools` as
`4  MAP EDITOR`. It lists valid compiled `hbmap` maps in catalog order,
creates a clean `EditableMap` only when a map is opened, and renders that
working copy through the Map Viewer free-flight/ceiling-inspection path. Use
Up/Down then Enter to open, `R` to reset to source, `C` to show/hide ceilings,
`M` to return to Dev Tools (confirm `Y`/`N`
before discarding a dirty copy), and Escape to release mouse capture. The map
list is shown only before a working copy is opened. Geometry, marker, asset, validation, save, and
playtest tools remain intentionally unavailable; the clean-only state still
reserves dirty, active-panel, selected-ID, and confirmation boundaries for
later phases. Corn Maze, Voxel Sandbox, and Drone Gate are displayed with
their procedural read-only status and cannot be selected for editing.

The free-flight controls retain `Space` for rise and `Ctrl` for drop; Enter
opens a listed map or applies the active edit tool. The backtick key (`` ` ``)
requests a whole-map save and opens a confirmation prompt.

### Phase 4B: geometry tools

Implementation status: direct geometry editing is available for an opened
working copy. `I` ray-picks the center-facing voxel; a left click selects a
new voxel and clicking the selected voxel again applies the active tool; arrow
keys move the selected voxel one cell forward, backward, left, or right
relative to the horizontal camera view, and `U`/`O` nudge y. `[`/`]` cycle the allowed named materials, or browse assets when the Asset tool is active.
Choose `1` Move, `2` Add, `3` Paint, `4` Erase, `5` Replace, `6` Box Fill,
`7` Box Erase, or `8` Asset; `,`/`.` cycle the same tools. The Asset ghost follows the live center cursor ray and a click drops it on the targeted face. Add places the active material on
the ray-hit face immediately, Minecraft-style. The active tool has a distinct
80×80-pixel icon at the bottom center, and a compact
top-right preview shows the material under the center reticle. While a box tool is active,
use `B` to set/clear the first box corner; the inclusive box is shown as a
dim cyan wireframe until applied. Enter applies the tool.
Backtick (`` ` ``) requests a save of the complete working map; a
centered confirmation prompt requires `Y` to save or `N` to cancel.
Paint fills only empty direct cells, erase clears direct cells, and replace
updates occupied direct cells. All edits stay within declared bounds, protect
the player standing volume and unit-scale asset-owned cells, respect the
per-box/final-world limits, render immediately, and set the working copy
dirty. Undo/redo and validation remain later phases.

`F9` opens a centered playtest-mode picker for the current in-memory working
world: `1` Explorer, `2` Flight, or `3` Shooter. It does not save or update
the catalog; its HUD is labelled **UNSAVED EDITOR PREVIEW**, and `M` returns to
the same editor copy. Explorer has grounded `WASD` movement and `Space` to
jump; Flight uses `WASD` plus `Space`/`Ctrl` for free vertical movement; and
Shooter has the grounded controls, a visible viewmodel weapon, and left-click
firing. Editor maps do not yet author encounters, so Shooter supplies the
weapon and firing presentation without importing another map's enemies.

Press `X` from the unopened Map Editor list to create an unsaved `Untitled
Map` with a 20×20 grass ground plane and a valid centered player start. The
patch is a starting area, not a build boundary: move the selection and add
geometry freely in every direction, and the editable map expands its declared
bounds to contain the new work. It is immediately editable and visibly dirty;
while a working copy is open, `X` is refused rather than replacing it. Phase
4D Save As will provide the durable filename/ID workflow. Once confirmed,
saving adds the compiled map to the Map Editor and Map Viewer catalogs without
requiring a restart.

Provide the smallest set that can make a useful room or repair a map.

| Tool | Action | Constraints |
| --- | --- | --- |
| Move | Position the selected voxel with ray-pick or view-relative arrow keys. | Never mutates. |
| Add | Place the active material in the empty cell adjoining a ray-hit face. | Refuse outside bounds, player volume, or asset-owned cells. |
| Paint | Fill one selected empty/direct cell with the active material. | Refuse outside bounds, player volume, or asset-owned cells. |
| Erase | Clear one selected direct cell. | Refuse partial asset erasure; warn when it removes required door/marker support geometry. |
| Replace | Change a selected direct cell from one material to the active material. | One undoable command. |
| Box fill | Set every direct cell in an inclusive selected box to the active material. | Preview count; enforce existing per-box and final-world limits before commit. |
| Box erase | Clear every direct cell in an inclusive selected box. | Preview count; same asset and bounds protections. |
| Flood fill | Fill connected cells of the picked material/empty state. | Optional after box tools; hard cap at the map operation limit and show affected count before commit. |

The initial material picker includes every `VoxelMaterial` allowed by the map
schema, grouped by architecture, terrain, gameplay-device, and decorative
use. It does not expose `Custom` or raw colours. Geometry edits immediately
render in the working preview and mark the document dirty. They need not
re-compile after every individual cell edit; a full compiler validation occurs
on demand and before saving.

Picking uses the same DDA world ray as rendering. A face normal determines the
default target: Add selects the neighboring empty cell and Erase/Replace
selects the hit solid cell. If the ray misses, keyboard coordinate entry and
selection-box endpoints remain usable.

Acceptance: single cells and boxes render correctly; undo restores materials
and occupancy exactly; illegal edits neither modify the world nor consume a
history entry; exported/reloaded geometry equals the working geometry.

### Phase 4C: history, selection, and validation feedback

- `Ctrl+Z` undoes and `Ctrl+Y`/`Ctrl+Shift+Z` redoes. Any new edit after an
  undo clears redo history.
- A selection may be one voxel or an inclusive box. Display both endpoints,
  dimensions, and addressed voxel count before mutating a box.
- Validation runs after an explicit command and automatically before Save or
  Playtest. Report errors with map ID, stable marker/asset ID where applicable,
  and coordinates/bounds where applicable. Do not collapse multiple errors
  into a generic failure.
- Compiler errors leave the working copy intact. The HUD retains the most
  recent successful validation timestamp separately from the current dirty
  state, so a user cannot mistake an old success for validation of later edits.
- Warnings are non-blocking only when the core validator classifies them as
  warnings. Schema, bounds, limits, asset resolution, and mode-required marker
  failures are always errors.

### Phase 4D: save, save-as, and playtest

Save behavior is a correctness feature, not merely UI plumbing.

1. Export the current editable map deterministically in memory.
2. Parse/validate/compile that exact serialized content with the normal map
   compiler and discovered asset catalog.
3. For **Save As**, require a new lowercase stable ID and filename
   `<id>.hbmap.json`; reject an occupied destination unless the user confirms
   replacement. For **Save**, retain the original source filename and reject
   non-`hbmap` or read-only sources.
4. Write the validated bytes to a unique temporary file in the destination
   directory, flush it, then rename it over the selected file. Clean up a
   failed temporary write when possible and preserve the old source untouched.
5. Re-discover or replace the in-memory catalog entry only after the rename
   succeeds. Mark the editor clean only after this step.

The first playtest command compiles a fresh session from the working export in
memory and launches the map's normal mode startup with a clearly visible
**UNSAVED EDITOR PREVIEW** HUD label. It creates no file, never changes the
catalogue blueprint, and returns to the same editor working copy on exit.

Acceptance: valid Save As produces a deterministic, loadable sibling map;
failed validation and failed writes preserve the original file byte-for-byte;
playtest starts from the edited snapshot while ordinary game launches retain
the last saved blueprint.

### Phase 4E: assets and generic typed markers

Asset and marker support follows core geometry editing so the editor can make
complete playable maps without teaching it mode mechanics prematurely.

**Assets**

- Browse discovered assets in deterministic catalog order, showing ID, name,
  voxel size, dimensions, pivot, collision participation, and transformed
  footprint at the candidate yaw.
- Place only at integer anchors with yaw `0`, `90`, `180`, or `270`; preview
  the rotated footprint before committing.
- Select by ray or stable placement ID; move, rotate, duplicate, or delete a
  whole instance as one history command.
- Reject unknown assets, duplicate placement IDs, transformed extents outside
  map bounds, and collisions that violate the core asset contract. Generate a
  stable suggested placement ID from asset ID plus a numeric suffix, while
  allowing an explicit edit.

**Generic map metadata and markers**

- Edit map display name, category, bounds (only through an explicit resize
  command), player start position/yaw, and generic `exit` markers.
- A bounds shrink previews the cells, markers, and assets it would exclude and
  refuses to commit until the user resolves them. A bounds expansion starts
  empty and does not fabricate geometry.
- Marker list navigation shows stable ID, kind, position, and a concise
  type-specific summary. Selecting a marker highlights it in the world.
- Create, move, rename, and delete only marker kinds allowed for the map mode.
  Required markers cannot be deleted into a valid saved map; the command may
  create a temporary invalid draft but must show its error immediately.
- Property forms are typed controls for numbers, IDs, enum choices, yaw, boxes,
  vectors, and references. They never expose an unvalidated free-form JSON
  editor.

Phase 4 ends with basic marker forms sufficient for player start, exits,
assets, and inspecting all other marker types. Full authoring of gameplay
semantics belongs to Phase 5.

### Phase 4 test plan

Core tests:

- Editable-map clone isolation, deterministic export, and export/reload
  equivalence after paint, erase, replace, and box edits.
- Undo/redo round trips for every command type, redo invalidation, history
  cap behavior, and no-op/failed command behavior.
- Asset ownership protection, placement transforms, rotation, duplicate IDs,
  and bounds rejection.
- Marker uniqueness, player-start preservation, typed property validation,
  bounds resize rejection, and mode compatibility.
- Save validation uses exactly the bytes to be written; invalid data never
  invokes replacement; a simulated rename/write failure leaves the source
  unchanged.

CLI tests:

- Menu and keyboard navigation enter/leave the editor, including dirty-state
  confirmation paths.
- Ray-pick face targeting and keyboard fallback select the expected voxel.
- HUD shows dirty state, active tool, selection, compiler errors, and unsaved
  playtest state.
- A playtest uses a fresh working snapshot, while a normal game session still
  starts from the catalog blueprint.

Manual checks:

- Make a doorway by erasing a box, save it, restart the application, inspect
  it in Map Viewer, and play it.
- Add a reusable asset, rotate it four times, save/reload, and confirm its
  visual footprint and collision behavior agree.
- Attempt invalid IDs, out-of-bounds boxes, a blocked player spawn, and a
  malformed mode-required marker; confirm the source file survives unchanged.

## Phase 5: mode-specific authoring

Phase 5 builds typed authoring panels on top of the generic marker system. A
panel is a view/controller for an existing core marker type, never an
alternative data model. Each panel must be driven by the same typed adapters
that mode startup consumes, so an editable map cannot save configurations the
game cannot interpret.

### Delivery order

1. **Zombies:** enemy spawn groups, `Building`/`CornField` doors, wall weapon,
   costs, and pickup placement. Include preview glyphs for door bounds and
   group labels.
2. **Liminal Office:** rooms, room bounds/types/signs, connection graph,
   objective, lights, and anchors. Provide graph validation and a room-link
   overlay; prevent a saved objective from referring to a missing room.
3. **Echolocation:** receivers, ordered pipes, puzzle doors, pressure plates,
   propagation links/delays, door transition anchors, and pursuer anchor data.
   Show sequence numbers and a non-runtime wire/path overlay. Extend the map
   schema only through a versioned, compiler-validated marker contract before
   adding a UI control.
4. **Doom and Bar:** encounter/pickup/exit authoring only where an existing
   typed adapter exists. Do not introduce generic enemy scripting.
5. **City and future procedural modes:** author generator-region parameters,
   landmarks, or explicit hybrid overlays only after each generator has a
   designed seeded contract. Corn Maze, Sandbox, and Drone Gate remain outside
   this milestone.

### Mode-panel requirements

- Panels derive choices from closed Rust enums and present the mode's required
  marker checklist: present, invalid, or missing.
- Marker references use pickers from currently valid IDs rather than manually
  typed arbitrary strings where possible.
- World overlays show doors, closed bounds, spawn group membership, liminal
  graph links, echo puzzle wiring, and affected anchors without changing the
  playable static world.
- A mode preview validates all required relationships before launch: Zombies
  can create its doors/groups/weapon; Liminal can build a connected objective
  graph; Echolocation can construct every receiver/pipe/door/plate route and
  its traversable room network.
- Runtime transitions remain runtime-only. Editing a closed door definition is
  allowed; saving an "open door" runtime state is not.

### Phase 5 tests and acceptance

- Per-mode editor tests create, edit, export, reload, and adapt the required
  marker sets without raw string inspection in gameplay.
- Negative tests cover dangling room links, duplicate echo pipe sequence,
  mismatched puzzle IDs, invalid door bounds/normals/anchors, missing Zombies
  groups or wall weapon, and unsupported mode-marker pairs.
- Map Viewer, playtest, and ordinary gameplay agree on static geometry,
  player start, assets, and marker locations after each saved edit.
- Update the affected living design notes—especially `docs/echolocation.md`—in
  the same change whenever a mode's authorable contract changes.

## Completion criteria

- Every finite game and viewer entry starts from one compiled map catalog.
- No legacy static builder remains after its parity test has passed.
- Editor saves are deterministic, validated, and round-trip tested.
- `cargo test --workspace` remains green throughout the migration.
