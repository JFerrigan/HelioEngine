# Maps and the map viewer

Open `Dev Tools` from the main menu, then select `3` (or use the arrow keys
and `Enter`/`Space`) to open the map viewer. It presents canonical, freshly
generated snapshots of every finite voxel gameplay environment:

1. Procedural city
2. Doomlike arena
3. Corn maze
4. Starhusk bar
5. Voxel sandbox
6. Heliobound Zombies
7. Liminal office
8. Drone gate course
9. Echolocation

Use `1` through `9` to select a map or `N` and `P` to cycle. The default is
collision-free first-person flight: mouse movement looks, `WASD` moves on the
horizontal plane, `Space` and `Ctrl` rise and drop, and `Shift` boosts speed.
Flight moves at twice the sandbox's normal speed.

The read-only inspector HUD identifies the selected map's stable ID and
source, declared (or legacy-derived) bounds, static collision voxel count,
asset instances, and typed marker labels. A malformed map file is shown as a
non-blocking catalog error; it never removes the other selectable maps.

Press `O` to switch to the original orbit view. In that view, mouse movement
orbits the camera, `WASD` and arrow keys pan relative to the current camera
view, `Q` and `E` roll, and `Space` and `Ctrl` zoom. Press `C` to hide or show
dense ceiling layers, `R` to reset the current view, and `M` to return to the
menu.

The planet-flight environment is not listed because it is analytic virtual
terrain rather than a finite voxel map.

## Map Editor (Phase 4A)

`Dev Tools` now also has `4  MAP EDITOR`. It lists valid compiled `hbmap`
entries in catalog order and opens an isolated, clean `EditableMap` working
copy. Use Up/Down and Enter to select and open a map, `R` to reset the copy
to its immutable blueprint, `C`
to hide/show dense ceilings, `M` to return to Dev Tools (confirm `Y`/`N`
before discarding a dirty copy), and Escape to release the mouse. The map list
is shown only before a working copy is opened. Its preview uses the same
free-flight inspector camera as Map
Viewer, but renders the working copy rather than the catalog blueprint.
As in Map Viewer, `Space` rises and `Ctrl` drops; Enter opens a map from the
list or applies the active editor tool. The backtick key (`` ` ``) requests a
whole-map save.

Phase 4A was inspection-only: markers, assets, validation, save, and playtest
remain explicitly unavailable. Corn Maze, Voxel Sandbox, and Drone Gate remain
read-only because they are legacy procedural environments rather than compiled
`*.hbmap.json` sources.

Phase 4B enables direct working-copy geometry: `I` ray-picks; a left click
selects a new voxel, while clicking the selected voxel again applies the
active tool;
arrow keys move the selected voxel one cell forward, backward, left, or right
relative to the camera view; and `U`/`O` nudge it vertically. `[`/`]` choose a named material, and
`1` through `8` select Move, Add, Paint, Erase, Replace, Box Fill, Box
Erase, and Asset; `,`/`.` cycle those tools. In a box tool, the first world
click sets the root corner and the next click applies the box through its
opposite corner. Add places a voxel on the face under the
cursor, Minecraft-style. The active tool appears as a distinct 80×80-pixel,
bottom-center icon. A compact top-right voxel preview shows
the material currently under the center reticle. In a box tool, press `B` to set an inclusive box
corner; the active box is shown as a dim cyan wireframe. Enter applies the selected tool. Press backtick (`` ` ``) to save the whole working map; the
centered prompt requires `Y` to save or `N` to cancel. Geometry edits remain
in memory until saved, are constrained to map bounds,
and protect player and unit-scale asset cells; validation remains unavailable.
Choose the Asset tool with `8`, browse discovered assets with `[`/`]`, then
click a face (or press Enter) to place its complete colored voxel model. The
candidate follows the center mouse reticle over map faces as a dim ghost at
its true authored voxel size; it is not a placeholder map cube. Unit-scale assets retain their
collision cells, while smaller assets remain visual as defined by the map
format.

Press `P` to test the current working world without saving it. The centered
picker asks for `1` Explorer, `2` Flight, or `3` Shooter before opening the
preview; `Enter` returns to the same editor copy. Explorer uses grounded `WASD`
movement and `Space` to jump; Flight uses free `WASD` flight with `Space`/`Ctrl`
for vertical movement; Shooter uses grounded movement plus a visible weapon
and fires with a left click. The preview is explicitly labelled **UNSAVED
EDITOR PREVIEW**.

Press `X` from the unopened Map Editor list to begin an unsaved `Untitled
Map`: a 20×20 grass ground with a centered player start. This is an initial
patch only; building beyond it automatically expands the editable map in all
directions.
After confirmation, saving adds it to the Map Editor and Map Viewer catalogs
immediately; `X` is blocked while a working copy is already open.

## Current map format

Finite environments share the in-memory `VoxelWorld` representation from
`heliobound-core`. City, Bar, Doomlike arena, Zombies, Liminal office, and
Echolocation blueprints are loaded at startup from versioned `*.hbmap.json`
files in `assets/voxel-maps/`. Bad files are reported independently and do not
hide other maps. The map viewer labels these entries `hbmap`; Corn Maze,
Sandbox, and Drone Gate retain their legacy generators for now. They are
deliberately deferred until each has a pure seeded procedural contract, rather
than being converted into editor-authored map files.

Most remaining maps are generators or stamping functions in Rust. The City
generator is registered by the core map compiler; the other deferred map
builders currently live in `crates/heliobound-cli/src/main.rs`. By contrast,
viewer assets use the dedicated, versioned `*.hbasset.json` format under
`assets/voxel-assets`. Those asset files are visual resources and are not
gameplay maps.

The schema, validation limits, and marker contracts are documented in
[data-driven map authoring](map-authoring.md). City uses the registered
deterministic `generator_region` compiler; the remaining deferred generators
are still legacy paths. The staged work to migrate them and prepare an editor
is tracked in [the map migration and editor roadmap](map-editor.md).
