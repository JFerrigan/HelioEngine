# Heliobound map prompt

Copy the text below into browser ChatGPT. Replace the bracketed request, save
its JSON response as `<stable-id>.hbmap.json` in this directory, and restart
Heliobound after the map loader is available.

---

Create one Heliobound map:

`[DESCRIBE THE MAP, MODE, ARCHITECTURE, OBJECTIVES, AND MOOD HERE]`

Return only one valid JSON object: no Markdown fence, explanation, comments,
trailing commas, or extra fields. Name the file `<stable-id>.hbmap.json`, using
the exact JSON `id` as `<stable-id>`.

Use this complete version 1 shape:

```json
{
  "format_version": 1,
  "id": "lowercase-stable-id",
  "name": "Human-readable Map Name",
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
    { "kind": "fill_box", "min": [-32, 0, -32], "max": [32, 0, 32], "material": "Stone" },
    { "kind": "fill_box", "min": [-6, 1, -6], "max": [6, 7, 6], "material": "Habitat" },
    { "kind": "clear_box", "min": [-2, 1, 5], "max": [2, 4, 6] },
    { "kind": "place_asset", "id": "airlock", "asset_id": "spaceship-airlock-door", "position": [0, 1, 5], "yaw_degrees": 180 }
  ],
  "markers": [
    { "id": "main-exit", "kind": "exit", "position": [0.5, 1.7, 5.5], "target": "menu" }
  ]
}
```

Rules:

- `format_version` is exactly `1`. `id` and `name` are non-empty. `id` uses
  only ASCII letters, digits, `-`, and `_`; use lowercase kebab-case.
- `mode` is exactly one of `city`, `doom`, `corn_maze`, `bar`, `sandbox`,
  `zombies`, `liminal`, `drone_gate`, or `echolocation`.
- `category` is `authored`, `procedural`, or `hybrid`. Use `authored` unless
  you use `generator_region`.
- Coordinates are `[x,y,z]`: x right/east, y up, z forward/south. Geometry
  coordinate values are integers. Box `min` and `max` are inclusive. Every
  operation, asset extent, and marker must lie inside top-level `bounds`.
- `position` on player/markers may be fractional. `player_start` is the one
  and only `player_spawn`; do not add a `player_spawn` to `markers`.
- Yaw is `0`, `90`, `180`, or `270`: `0` faces `+z`, then turns clockwise from
  above. Asset positions are integer anchors.
- Every direct geometry material is exactly one of: `Regolith`, `Basalt`,
  `Ocean`, `Ice`, `Grass`, `Dirt`, `Stone`, `Sand`, `Wood`, `Leaves`, `Zombie`,
  `CornStalk`, `CarbonLife`, `SiliconLife`, `Habitat`, `ShipHull`, `Glass`,
  `Beacon`, `Gate`, `Receiver`, `SignalPipe`, `PuzzleDoor`. Never use RGB,
  hex colors, `Custom`, or an invented material.
- Operations are applied in list order. Use `fill_box` for solid inclusive
  boxes and `clear_box` for openings. Later operations intentionally overwrite
  or clear earlier geometry.
- `place_asset` needs a unique `id`, an existing `asset_id` from
  `assets/voxel-assets`, integer `position`, and quarter-turn `yaw_degrees`.
  Use `spaceship-airlock-door` only if that asset file is available.
- `generator_region` has unique `id`, `generator`, `bounds`, unsigned integer
  `seed`, and exact `parameters`. Allowed generators are `city` with
  `{ "road_width": integer, "block_size": integer, "max_height": integer }`,
  `corn_maze` with `{ "cell_size": integer, "wall_height": integer, "density": number }`,
  and `sandbox_terrain` with `{ "ground_y": integer, "height_variation": integer, "material": "NamedMaterial" }`.
  Use no generator for a small authored map.
- Every marker has a unique `id`, `kind`, and finite `position`. Allowed marker
  kinds and their extra fields are: `exit` (`target`: `menu` or a map ID),
  `enemy_spawn` (`enemy_type`, `spawn_group`), `pickup` (`pickup_type`,
  `amount`), `interactable_door` (`door_type`, `closed_bounds`, `open_cost`),
  `wall_weapon` (`weapon_type`, `cost`, `yaw_degrees`), `liminal_objective`
  (`objective_type`, `room_id`), `echo_receiver` (`output_seconds`,
  `puzzle_id`), `echo_pipe` (`puzzle_id`, `sequence`), and `echo_door`
  (`puzzle_id`, `closed_bounds`, `normal`, `starting_side_anchor`,
  `far_side_anchor`).
- Only use markers compatible with the selected mode. An Echolocation puzzle
  needs matching receiver, one-or-more ordered pipe, and door markers using the
  same `puzzle_id`. `normal` is one of `[1,0,0]`, `[-1,0,0]`, `[0,1,0]`,
  `[0,-1,0]`, `[0,0,1]`, `[0,0,-1]`.
- Keep any bounds axis at or below 1024, any box/region at or below 1,000,000
  addressed voxels, no more than 10,000 operations, and the final world at or
  below 4,000,000 occupied voxels.

Before responding, silently verify: the JSON parses; all IDs are unique;
the filename matches `id`; every box is ordered and within bounds; materials,
marker kinds, asset IDs, yaws, and generator parameters are allowed; the
player start has open standing space; and the map is non-empty. Return only
the JSON object.
