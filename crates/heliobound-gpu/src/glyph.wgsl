// Logical-cell glyph composition. All texture accesses are explicit loads so
// the 8x8 font and 160x90 terrain are always nearest-neighbour crisp.
struct Presentation {
  physical_size: vec2f,
  logical_size: vec2f,
  scale_and_origin: vec4f,
};
struct FullscreenOut { @builtin(position) position: vec4f };

@group(0) @binding(0) var glyph_ids: texture_2d<u32>;
@group(0) @binding(1) var glyph_colours: texture_2d<f32>;
@group(0) @binding(2) var glyph_atlas: texture_2d<f32>;
@group(0) @binding(3) var<uniform> presentation: Presentation;

@vertex fn vs_fullscreen(@builtin(vertex_index) index: u32) -> FullscreenOut {
  var positions = array<vec2f, 3>(vec2f(-1.0, -3.0), vec2f(3.0, 1.0), vec2f(-1.0, 1.0));
  return FullscreenOut(vec4f(positions[index], 0.0, 1.0));
}
fn atlas_bit(glyph: u32, pixel: vec2u) -> f32 {
  let atlas = vec2i(i32((glyph & 255u) % 16u) * 8 + i32(pixel.x), i32((glyph & 255u) / 16u) * 8 + i32(pixel.y));
  return textureLoad(glyph_atlas, atlas, 0).r;
}
fn physical_to_cell(pixel: vec2f) -> vec2i {
  return vec2i(floor((pixel - presentation.scale_and_origin.yz) / presentation.scale_and_origin.x));
}
@fragment fn fs_glyph(@builtin(position) position: vec4f) -> @location(0) vec4f {
  let cell = physical_to_cell(position.xy);
  if (any(cell < vec2i(0)) || cell.x >= i32(presentation.logical_size.x) || cell.y >= i32(presentation.logical_size.y)) { return vec4f(0.0, 0.0, 0.0, 1.0); }
  let local = vec2u(u32(i32(floor(position.x - presentation.scale_and_origin.y)) % i32(presentation.scale_and_origin.x)), u32(i32(floor(position.y - presentation.scale_and_origin.z)) % i32(presentation.scale_and_origin.x)));
  // At a window smaller than the logical grid, cell scale is one and this
  // still selects a deterministic subset of the 8x8 glyph.
  let glyph_pixel = (local * vec2u(8u)) / u32(presentation.scale_and_origin.x);
  let glyph = textureLoad(glyph_ids, cell, 0).r;
  let colour = textureLoad(glyph_colours, cell, 0);
  return vec4f(colour.rgb * atlas_bit(glyph, glyph_pixel), 1.0);
}

struct UiCell { x: i32, y: i32, glyph: u32, flags: u32, foreground_rgba: u32, background_rgba: u32 };
struct UiOut {
  @builtin(position) position: vec4f,
  @location(0) @interpolate(flat) cell: vec2i,
  @location(1) @interpolate(flat) glyph: u32,
  @location(2) @interpolate(flat) flags: u32,
  @location(3) @interpolate(flat) foreground: u32,
  @location(4) @interpolate(flat) background: u32,
};
@group(0) @binding(0) var<storage, read> ui_cells: array<UiCell>;
@group(0) @binding(1) var ui_atlas: texture_2d<f32>;
@group(0) @binding(2) var<uniform> ui_presentation: Presentation;
fn unpack_rgba(value: u32) -> vec4f {
  return vec4f(f32((value >> 24u) & 255u), f32((value >> 16u) & 255u), f32((value >> 8u) & 255u), f32(value & 255u)) / 255.0;
}
@vertex fn vs_ui(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> UiOut {
  let source = ui_cells[instance];
  var corners = array<vec2f, 6>(vec2f(0.0,0.0),vec2f(1.0,0.0),vec2f(0.0,1.0),vec2f(0.0,1.0),vec2f(1.0,0.0),vec2f(1.0,1.0));
  let pixel = ui_presentation.scale_and_origin.yz + (vec2f(f32(source.x), f32(source.y)) + corners[vertex]) * ui_presentation.scale_and_origin.x;
  var out: UiOut;
  out.position = vec4f(pixel.x / ui_presentation.physical_size.x * 2.0 - 1.0, 1.0 - pixel.y / ui_presentation.physical_size.y * 2.0, 0.0, 1.0);
  out.cell = vec2i(source.x, source.y);
  out.glyph = source.glyph;
  out.flags = source.flags;
  out.foreground = source.foreground_rgba;
  out.background = source.background_rgba;
  return out;
}
@fragment fn fs_ui(in: UiOut) -> @location(0) vec4f {
  let local = vec2u((vec2f(in.position.xy) - ui_presentation.scale_and_origin.yz - vec2f(in.cell) * ui_presentation.scale_and_origin.x) * 8.0 / ui_presentation.scale_and_origin.x);
  let atlas = vec2i(i32((in.glyph & 255u) % 16u) * 8 + i32(local.x), i32((in.glyph & 255u) / 16u) * 8 + i32(local.y));
  if (textureLoad(ui_atlas, atlas, 0).r > 0.5) { return unpack_rgba(in.foreground); }
  if ((in.flags & 1u) != 0u) { return unpack_rgba(in.background); }
  return vec4f(0.0);
}

// Pixel sprites retain their CPU framebuffer coordinates (1280 by 720) and
// scale with the logical 8-by-8 glyph grid. They are painter-ordered between
// scene cells and text overlays.
struct PixelSprite {
  x: i32, y: i32, scale: u32, flags: u32,
  foreground_rgba: u32, background_rgba: u32,
  rows: array<u32, 16>,
};
struct SpriteOut {
  @builtin(position) position: vec4f,
  @location(0) @interpolate(flat) sprite_index: u32,
};
@group(0) @binding(0) var<storage, read> pixel_sprites: array<PixelSprite>;
@group(0) @binding(1) var<uniform> sprite_presentation: Presentation;
@vertex fn vs_sprite(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> SpriteOut {
  let source = pixel_sprites[instance];
  var corners = array<vec2f, 6>(vec2f(0.0,0.0),vec2f(1.0,0.0),vec2f(0.0,1.0),vec2f(0.0,1.0),vec2f(1.0,0.0),vec2f(1.0,1.0));
  let factor = sprite_presentation.scale_and_origin.x / 8.0;
  let origin = sprite_presentation.scale_and_origin.yz + vec2f(f32(source.x), f32(source.y)) * factor;
  let extent = vec2f(16.0 * f32(source.scale)) * factor;
  let pixel = origin + corners[vertex] * extent;
  var out: SpriteOut;
  out.position = vec4f(pixel.x / sprite_presentation.physical_size.x * 2.0 - 1.0, 1.0 - pixel.y / sprite_presentation.physical_size.y * 2.0, 0.0, 1.0);
  out.sprite_index = instance;
  return out;
}
@fragment fn fs_sprite(in: SpriteOut) -> @location(0) vec4f {
  let source = pixel_sprites[in.sprite_index];
  let factor = sprite_presentation.scale_and_origin.x / 8.0;
  let origin = sprite_presentation.scale_and_origin.yz + vec2f(f32(source.x), f32(source.y)) * factor;
  let source_pixel = vec2u(floor((in.position.xy - origin) / (factor * f32(source.scale))));
  let bit = (source.rows[source_pixel.y] & (1u << (15u - source_pixel.x))) != 0u;
  if (bit) { return unpack_rgba(source.foreground_rgba); }
  if ((source.flags & 1u) != 0u) { return unpack_rgba(source.background_rgba); }
  return vec4f(0.0);
}
