# Data-driven map authoring

This document defines the canonical map format proposed for Heliobound. It is
an implementation contract. Version 1 is implemented for static authored
maps; see [maps and the map viewer](maps.md) for the deferred generators.

## Decision

Use versioned `*.hbmap.json` **hybrid blueprints** for every finite gameplay
map. A blueprint describes an immutable starting world. Rust game-mode code
owns simulation and turns that blueprint into a fresh mutable play session.

Three approaches were considered:

| Approach | Good at | Limitation |
| --- | --- | --- |
| Flat voxel layers | Small, hand-authored scenes; simple generated text | Full-map ASCII becomes unwieldy for architecture, terrain, repetition, and generated maps. |
| Procedural recipes | Compact deterministic city, corn, and sandbox worlds | Has poor direct authorial control and grows an engine-specific operation language. |
| Hybrid blueprints | Concise authored spaces plus reusable assets and deterministic regions | Requires a validator/compiler. This is the v1 choice. |

The format is deliberately declarative. It describes static geometry,
placements, deterministic generator regions, and semantic markers; it does
not contain scripts, arbitrary code, colors, AI rules, or frame-to-frame
state.

## Discovery and identity

- Maps live in `assets/voxel-maps/` and use the exact name
  `<stable-id>.hbmap.json`.
- The loader scans this directory once at startup, alongside voxel assets.
  Files are processed in deterministic filename order.
- `id` must exactly match the filename stem, be globally unique, and contain
  only ASCII letters, digits, `_`, and `-`. Lowercase kebab-case is preferred.
- A bad file produces a file-local error and is skipped. It must never prevent
  other valid maps (or assets) from loading.
- Imported assets and maps have independent error collections: a bad asset
  does not block maps that do not reference it, and a map with an unresolved
  asset is the only map rejected for that reference.

## Version 1 format

All coordinates use the right-handed voxel world: `x` increases right/east,
`y` increases upward, and `z` increases forward/south. A voxel at `[x,y,z]`
occupies `[x,x+1) × [y,y+1) × [z,z+1)`. Geometry coordinates are integers.
Box `min` and `max` are inclusive.

`position` is the player or marker's point in world units and may have finite
fractional components. Map-authored placements use integer voxel anchors.
Yaw is clockwise when viewed from above: `0` faces `+z`, `90` faces `+x`,
`180` faces `-z`, and `270` faces `-x`.

```json
{
  "format_version": 1,
  "id": "example-outpost",
  "name": "Example Outpost",
  "mode": "bar",
  "category": "authored",
  "bounds": { "min": [-32, 0, -32], "max": [32, 24, 32] },
  "player_start": {
    "id": "player-start",
    "kind": "player_spawn",
    "position": [0.5, 1.7, 8.5],
    "yaw_degrees": 180
  },
  "operations": [
    {
      "kind": "fill_box",
      "min": [-32, 0, -32],
      "max": [32, 0, 32],
      "material": "Stone"
    },
    {
      "kind": "fill_box",
      "min": [-6, 1, -6],
      "max": [6, 7, 6],
      "material": "Habitat"
    },
    {
      "kind": "clear_box",
      "min": [-2, 1, 5],
      "max": [2, 4, 6]
    },
    {
      "kind": "place_asset",
      "id": "airlock",
      "asset_id": "spaceship-airlock-door",
      "position": [0, 1, 5],
      "yaw_degrees": 180
    }
  ],
  "markers": [
    {
      "id": "main-exit",
      "kind": "exit",
      "position": [0.5, 1.7, 5.5],
      "target": "menu"
    }
  ]
}
```

All fields shown at the top level are required. Unknown fields are rejected in
v1, including unknown fields on an operation or marker. This makes a typo a
load error instead of silently changing a level.

### Top-level fields

| Field | Contract |
| --- | --- |
| `format_version` | Integer exactly `1`. |
| `id`, `name` | Non-empty stable identifier and display name. |
| `mode` | One of `city`, `doom`, `corn_maze`, `bar`, `sandbox`, `zombies`, `liminal`, `drone_gate`, or `echolocation`. |
| `category` | `authored`, `procedural`, or `hybrid`. Static v1 migration files use `authored`; files with generator regions use `hybrid` or `procedural`. |
| `bounds` | Inclusive integer `{ "min": [x,y,z], "max": [x,y,z] }`; every min is no greater than its max. Geometry, asset extents, generated cells, and markers must lie within it. |
| `player_start` | The required typed `player_spawn` marker. Its ID participates in the map-wide unique marker-ID set. |
| `operations` | Ordered list of geometry operations. |
| `markers` | Ordered list of typed non-player markers. No entry may use `player_spawn`; there is exactly one player spawn, `player_start`. |

### Geometry operations

Operations are applied strictly in file order. Later fills can overwrite prior
cells, and later clears can deliberately cut openings. Every operation has a
`kind`.

| Kind | Required fields | Meaning |
| --- | --- | --- |
| `fill_box` | `min`, `max`, `material` | Fill every inclusive voxel in the box with a named engine material. |
| `clear_box` | `min`, `max` | Remove every voxel in the inclusive box. |
| `place_asset` | `id`, `asset_id`, `position`, `yaw_degrees` | Place a discovered `*.hbasset.json` asset at an integer anchor. `id` is unique among placement IDs. `yaw_degrees` is `0`, `90`, `180`, or `270`. |
| `generator_region` | `id`, `generator`, `bounds`, `seed`, `parameters` | Run a registered deterministic generator in its inclusive bounds. `id` is unique among generator-region IDs. |

`material` is exactly one of the existing named `VoxelMaterial` values:
`Regolith`, `Basalt`, `Ocean`, `Ice`, `Grass`, `Dirt`, `Stone`, `Sand`, `Wood`,
`Leaves`, `Zombie`, `CornStalk`, `CarbonLife`, `SiliconLife`, `Habitat`,
`ShipHull`, `Glass`, `Beacon`, `Gate`, `Receiver`, `SignalPipe`, or
`PuzzleDoor`. `Custom` is not valid in a map file. Arbitrary RGB colors remain
an asset-palette feature, never direct map geometry.

An asset reference must resolve by asset ID after asset discovery. The compiler
rotates the asset around its declared pivot, checks its final extents against
map bounds, and records it as an asset instance for mixed-resolution rendering.
At the collision layer, only an asset representation that can be exactly
expanded onto the integer voxel grid participates in `VoxelWorld`; other
asset instances remain visual until a future collision representation is
explicitly defined. This avoids silently rounding 0.5/0.25 voxel assets into
incorrect collision.

V1 reserves these built-in generator names and parameter objects:

| Generator | Parameters |
| --- | --- |
| `city` | `{ "road_width": integer, "block_size": integer, "max_height": integer }` |
| `corn_maze` | `{ "cell_size": integer, "wall_height": integer, "density": number }` |
| `sandbox_terrain` | `{ "ground_y": integer, "height_variation": integer, "material": "NamedMaterial" }` |

Each generator validates its own exact parameter keys and ranges. A seed is an
unsigned 64-bit integer encoded as a JSON number. The same map file, generator
version, and seed must compile to identical geometry.

### Markers

Every marker has a stable non-empty `id`, an allowed `kind`, and a finite
`position: [x,y,z]`. Marker IDs are unique across `player_start` and
`markers`. Marker positions may be fractional, but gameplay code decides any
mode-specific snapping or clearance check.

| Kind | Additional required fields | Owner |
| --- | --- | --- |
| `player_spawn` | `yaw_degrees` (only `player_start`) | Shared map startup. |
| `exit` | `target` (`menu` or a map ID) | Mode navigation. |
| `enemy_spawn` | `enemy_type`, `spawn_group` | Doom/Zombies encounter system. |
| `pickup` | `pickup_type`, `amount` | Mode inventory system. |
| `interactable_door` | `door_type`, `closed_bounds`, `open_cost` | Doom/Zombies/other door system. `closed_bounds` is an inclusive voxel box. |
| `wall_weapon` | `weapon_type`, `cost`, `yaw_degrees` | Zombies. |
| `liminal_objective` | `objective_type`, `room_id` | Liminal graph and anomaly system. |
| `liminal_room` | `room_id`, `bounds`, `room_type`, `sign` | Liminal topology. |
| `liminal_connection` | `from_room`, `to_room` | Liminal topology graph link. |
| `echo_receiver` | `output_seconds`, `puzzle_id` | Echolocation puzzle. |
| `echo_pipe` | `puzzle_id`, `sequence` | Echolocation puzzle; all pipes for a puzzle sort by `sequence`. |
| `echo_door` | `puzzle_id`, `closed_bounds`, `normal`, `starting_side_anchor`, `far_side_anchor` | Echolocation puzzle transition. `normal` is one axis unit vector. |

`enemy_type`, `pickup_type`, `door_type`, `weapon_type`, and `objective_type`
are mode-specific allowlists owned by Rust. The initial migration must expose
the existing values explicitly (for example Zombies `Building`/`CornField`
door kinds and its wall rifle) rather than interpreting arbitrary strings.
Mode validation rejects irrelevant marker kinds and requires the marker sets
needed by a mode. In particular, an Echolocation map needs one complete
receiver/pipe/door set per `puzzle_id`; a Zombies map needs its declared doors,
enemy spawn groups, and wall weapon; and a Liminal map needs a valid objective
that resolves to its room graph.

## Validation and compiler limits

The loader validates JSON before compiling any world:

- all numbers are finite; all grid coordinates are signed 32-bit integers;
- every coordinate/extent lies in the declared finite bounds;
- IDs, materials, kinds, yaw values, asset references, generator names, and
  generator parameters use their allowlists;
- bounds dimensions are at most `1024` on any axis, boxes and regions do not
  exceed `1,000,000` addressed voxels, a file has at most `10,000` operations,
  and the final static world has at most `4,000,000` occupied voxels;
- the file is at most 8 MiB, placement/generator IDs are unique in their own
  namespaces, and marker IDs are unique map-wide; and
- mode requirements, marker references, assets, door boxes, puzzle groups,
  and player-start clearance all validate before a map becomes available.

The compiler first validates, then applies geometry in order, expands eligible
assets, runs deterministic regions, and returns an immutable compiled
blueprint: `VoxelWorld`, renderable asset instances, the player transform, and
typed marker metadata. A new game session clones/builds from this blueprint.
Runtime overlays—including enemies, opened doors, consumed pickups, sound
waves, liminal anomalies, and Echolocation signals/door transitions—belong to
mode state and must not mutate the canonical blueprint.

## Migration plan

1. Add map discovery, the strict parser/validator, and a shared compiler.
   Preserve source filename and errors for the map-viewer HUD.
2. Move the static layouts for Bar, Doom, Zombies, Liminal Office, and
   Echolocation into canonical blueprint files. Replace their map-specific
   stamping calls with compiler output; retain each mode's behavioral Rust
   state, consuming markers instead of hard-coded coordinates.
3. Point the map viewer at the same compiled blueprint catalog, showing map
   ID, source type, and load errors.
4. Register the existing City, Corn Maze, and Sandbox algorithms as the three
   generator-region types above. Their recipe files then replace large voxel
   dumps without changing their deterministic output.

## Required tests and acceptance

- Parser/compiler tests: minimal valid map, ordered fill/clear overwrite,
  every material mapping, asset transforms, generator determinism, marker
  parsing, and duplicate or unknown references.
- Discovery tests: deterministic filename order and malformed-file errors that
  do not suppress other maps.
- Migration tests: every migrated map has a non-empty world, required markers,
  and a valid declared start; retain Doom collision, Zombies doors/spawns,
  Liminal graph/objective, and Echolocation receiver/door regressions.
- End-to-end: a browser-ChatGPT-created map using an imported reusable asset
  and typed markers validates, appears in the map viewer, and loads.

Use the copy-ready [map authoring prompt](../assets/voxel-maps/CHATGPT_MAP_PROMPT.md)
when the loader and `assets/voxel-maps/` directory are introduced.
