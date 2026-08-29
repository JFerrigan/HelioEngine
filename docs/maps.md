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
`heliobound-core`. It is a chunked collection of voxel coordinates and
`VoxelMaterial` cells with tracked bounds. `VoxelWorld` supports Serde, so it
can be serialized, but the project does not currently define a stable,
versioned map-file format or load gameplay maps from a dedicated map folder.

Most maps are generators or stamping functions in Rust. The city and Doomlike
arena generators live in `crates/heliobound-core/src/city.rs`; the other map
builders currently live in `crates/heliobound-cli/src/main.rs`. By contrast,
viewer assets use the dedicated, versioned `*.hbasset.json` format under
`assets/voxel-assets`. Those asset files are visual resources and are not
gameplay maps.
