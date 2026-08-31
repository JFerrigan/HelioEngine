# Maps and the map viewer

Press `V` on the main menu to open the map viewer. It presents canonical,
freshly generated snapshots of every finite voxel gameplay environment:

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

Press `O` to switch to the original orbit view. In that view, mouse movement
orbits the camera, `WASD` and arrow keys pan relative to the current camera
view, `Q` and `E` roll, and `Space` and `Ctrl` zoom. Press `C` to hide or show
dense ceiling layers, `R` to reset the current view, and `M` to return to the
menu.

The planet-flight environment is not listed because it is analytic virtual
terrain rather than a finite voxel map.

## Current map format

Finite environments share the in-memory `VoxelWorld` representation from
`heliobound-core`. City, Bar, Doomlike arena, Zombies, Liminal office, and
Echolocation blueprints are loaded at startup from versioned `*.hbmap.json`
files in `assets/voxel-maps/`. Bad files are reported independently and do not
hide other maps. The map viewer labels these entries `hbmap`; Corn Maze,
Sandbox, and Drone Gate retain their legacy generators for now.

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
