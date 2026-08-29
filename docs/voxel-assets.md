# Voxel Assets

Heliobound supports drop-in, mixed-resolution voxel assets for the asset
viewer. Imported assets are currently visual resources only: they are not
placed in gameplay maps and do not participate in movement collision or
echolocation.

## Add an asset

1. Create a version 1 `*.hbasset.json` file. The ready-to-copy
   [ChatGPT asset prompt](../assets/voxel-assets/CHATGPT_ASSET_PROMPT.md)
   contains the complete schema, orientation rules, scale guidance, and quality
   checklist.
2. Put the file in [`assets/voxel-assets`](../assets/voxel-assets).
3. Start or restart Heliocrisis. Asset discovery happens once during startup,
   so files added or edited while the application is running will not appear
   until the next launch.
4. Press `V` to open the asset viewer. Use the left and right arrow keys to
   browse assets and `M` to return to the simulation.

Valid imported assets appear after the built-in catalog in deterministic
filename order. The viewer shows each asset's voxel scale, physical dimensions,
and source type. Invalid files are skipped without stopping startup, and a
summary of loading errors appears in the viewer HUD.

## Format overview

Every asset contains:

- `format_version`, currently `1`.
- A stable `id` and human-readable `name`.
- A `voxel_size` of exactly `1`, `0.5`, `0.25`, or `0.125`.
- Integer `[x, y, z]` dimensions.
- An optional pivot in local voxel coordinates.
- A one-character alphanumeric palette mapped to opaque `#RRGGBB` colors.
- ASCII layers ordered bottom-to-top. Rows run front-to-back and characters run
  left-to-right. A period (`.`) represents empty space.

The default pivot is the center of the bottom footprint: `[x / 2, 0, z / 2]`.
See the packaged
[`example-signal-beacon.hbasset.json`](../assets/voxel-assets/example-signal-beacon.hbasset.json)
for a working asset loaded through the same discovery path as user-created
files.

## Validation

Heliocrisis rejects unsupported versions or scales, malformed colors, invalid
dimensions, inconsistent layer or row sizes, undefined palette symbols, empty
models, duplicate IDs, and oversized input. Errors include the relevant
filename to make problematic assets easy to locate.
