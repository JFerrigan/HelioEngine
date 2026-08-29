# Heliobound voxel asset prompt

Copy everything below into browser ChatGPT, replace the bracketed description,
and save its response in this directory. Heliocrisis scans this directory only
at startup, so restart the application after adding or changing a file.

---

Create one Heliobound voxel asset representing:

`[DESCRIBE THE OBJECT, SILHOUETTE, COLORS, AND DESIRED DETAIL HERE]`

Return only the contents of a single valid JSON file. After the JSON, state the
filename on a separate line only if the interface cannot provide a downloadable
file. Name it `<stable-id>.hbasset.json`, where `<stable-id>` is the same value
as the JSON `id`.

Use this complete version 1 schema:

```json
{
  "format_version": 1,
  "id": "lowercase-stable-id",
  "name": "Human-readable Name",
  "voxel_size": 0.5,
  "dimensions": [5, 4, 3],
  "pivot": [2.5, 0.0, 1.5],
  "palette": {
    "A": "#RRGGBB",
    "1": "#RRGGBB"
  },
  "layers": [
    [".....", ".AAA.", "....."],
    [".....", ".A.A.", "....."],
    [".....", ".AAA.", "....."],
    [".....", "..A..", "....."]
  ]
}
```

Rules:

- `format_version` must be the integer `1`.
- `id` must be non-empty and contain only ASCII letters, digits, hyphens, or
  underscores. Prefer lowercase kebab-case and keep it stable across revisions.
- `name` must be non-empty.
- `voxel_size` is the physical edge length of every voxel and must be exactly
  one of `1`, `0.5`, `0.25`, or `0.125`.
- `dimensions` is `[x, y, z]` using positive integers. Keep every axis at or
  below 256 and the product at or below 1,000,000. Prefer much smaller assets.
- `pivot` is optional and uses local voxel coordinates. If omitted, it defaults
  to the center of the bottom footprint: `[x / 2, 0, z / 2]`. Use that default
  for objects that stand on terrain. A voxel at coordinate `[x, y, z]` occupies
  the half-open local box from `[x, y, z]` to `[x+1, y+1, z+1]`.
- Each palette key must be exactly one ASCII alphanumeric character. Each value
  must be an opaque six-digit RGB color in `#RRGGBB` form. Palette symbols are
  case-sensitive. The period `.` is reserved for empty space and must not be in
  the palette.
- `layers` contains exactly `dimensions[1]` layers, ordered bottom-to-top.
- Each layer contains exactly `dimensions[2]` rows, ordered front-to-back.
- Each row contains exactly `dimensions[0]` characters, left-to-right along the
  positive x axis.
- Every non-period character used in a row must exist in the palette. The model
  must contain at least one non-empty voxel.
- Do not add comments, trailing commas, Markdown fences, extra fields, or text
  inside the JSON file.

Scale guidance:

- Use `1` for map-scale architecture and large block forms.
- Use `0.5` for furniture, characters, and medium props.
- Use `0.25` for compact props with recognizable detail.
- Use `0.125` only when fine silhouette or color detail is important; keep the
  total voxel count modest.

Before answering, verify all of the following silently:

1. The filename ends in `.hbasset.json` and begins with the exact stable `id`.
2. The number of layers equals y, rows per layer equals z, and characters per
   row equals x.
3. Orientation is bottom-to-top, front-to-back, then left-to-right.
4. Every visible symbol has a palette entry and every color is valid `#RRGGBB`.
5. The object has no accidental floating fragments unless requested.
6. Its silhouette is legible from the front, side, and three-quarter views.
7. The chosen voxel tier represents the intended real-world physical size.
8. The JSON parses without comments or trailing commas.
