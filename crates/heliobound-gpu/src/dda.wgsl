// CPU-reference DDA terrain pass. This shader raycasts one logical ASCII cell
// per fragment. Chunk lookup entries contain slot + 1; zero means absent.
const CHUNK_EDGE: i32 = 16;
const CHUNK_VOLUME: u32 = 4096u;
const ENTRY_EPSILON: f32 = 0.0001;
const AXIS_EPSILON: f32 = 1.1920929e-7;

struct CameraUniform {
  position_and_max_distance: vec4f,
  forward_and_aspect: vec4f,
  right_and_tan_half_fov: vec4f,
  up_and_padding: vec4f,
  bounds_min_and_padding: vec4i,
  // xyz are inclusive; w is the conservative traversal cap.
  bounds_max_and_max_steps: vec4i,
  table_origin_and_padding: vec4i,
  table_dimensions_and_padding: vec4u,
};
struct VertexOut { @builtin(position) position: vec4f };
struct RayHit { hit: bool, distance: f32, coord: vec3i, normal: vec3f, material: u32, ghost: u32 };

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> chunk_lookup: array<u32>;
struct BackgroundCell { glyph: u32, foreground_rgba: u32 };
@group(0) @binding(2) var<storage, read> background_cells: array<BackgroundCell>;
@group(0) @binding(3) var<storage, read> chunk_voxels: array<u32>;
struct DynamicVoxel { x: i32, y: i32, z: i32, material: u32 };
@group(0) @binding(4) var<storage, read> dynamic_voxels: array<DynamicVoxel>;
struct RenderAsset { min: vec4f, max: vec4f, anchor: vec4f, voxel_size: f32, yaw_degrees: f32, ghost: u32, voxel_offset: u32, dimensions: vec4u, pivot: vec4f };
struct AssetVoxel { x: i32, y: i32, z: i32, material: u32 };
@group(0) @binding(5) var<storage, read> render_assets: array<RenderAsset>;
@group(0) @binding(6) var<storage, read> asset_voxels: array<AssetVoxel>;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
  var positions = array<vec2f, 3>(vec2f(-1.0, -3.0), vec2f(3.0, 1.0), vec2f(-1.0, 1.0));
  return VertexOut(vec4f(positions[index], 0.0, 1.0));
}
fn no_hit() -> RayHit { return RayHit(false, 0.0, vec3i(0), vec3f(0.0), 0u, 0u); }
fn floor_div_16(value: i32) -> i32 {
  // WGSL division truncates; CPU uses div_euclid for negative chunks.
  if (value < 0) { return -((-value + 15) / CHUNK_EDGE); }
  return value / CHUNK_EDGE;
}
fn lookup_material(coord: vec3i) -> u32 {
  // Dynamic simulation stamping is ordered; the final matching cell wins.
  var dynamic_material = 0u;
  var has_dynamic = false;
  for (var index = 0u; index < camera.table_dimensions_and_padding.w; index = index + 1u) {
    let voxel = dynamic_voxels[index];
    if (all(vec3i(voxel.x, voxel.y, voxel.z) == coord)) { dynamic_material = voxel.material; has_dynamic = true; }
  }
  let chunk = vec3i(floor_div_16(coord.x), floor_div_16(coord.y), floor_div_16(coord.z));
  let local_chunk = chunk - camera.table_origin_and_padding.xyz;
  let dimensions = camera.table_dimensions_and_padding.xyz;
  if (any(local_chunk < vec3i(0)) || u32(local_chunk.x) >= dimensions.x || u32(local_chunk.y) >= dimensions.y || u32(local_chunk.z) >= dimensions.z) { return select(0u, dynamic_material, has_dynamic); }
  let lookup_index = u32(local_chunk.x) + u32(local_chunk.y) * dimensions.x + u32(local_chunk.z) * dimensions.x * dimensions.y;
  let encoded_slot = chunk_lookup[lookup_index];
  if (encoded_slot == 0u) { return select(0u, dynamic_material, has_dynamic); }
  let local = coord - chunk * CHUNK_EDGE;
  let voxel_index = u32(local.x) + u32(local.y) * 16u + u32(local.z) * 256u;
  let static_material = chunk_voxels[(encoded_slot - 1u) * CHUNK_VOLUME + voxel_index];
  return select(static_material, dynamic_material, has_dynamic);
}
fn axis_interval(origin: f32, direction: f32, minimum: f32, maximum: f32) -> vec2f {
  if (abs(direction) <= AXIS_EPSILON) {
    if (origin >= minimum && origin <= maximum) { return vec2f(-3.402823e38, 3.402823e38); }
    return vec2f(1.0, -1.0);
  }
  let a = (minimum - origin) / direction;
  let b = (maximum - origin) / direction;
  return vec2f(min(a, b), max(a, b));
}
fn axis_step(direction: f32) -> i32 { if (direction > 0.0) { return 1; } if (direction < 0.0) { return -1; } return 0; }
fn initial_t_max(origin: f32, direction: f32, step: i32) -> f32 {
  if (step == 0) { return 3.402823e38; }
  let boundary = select(floor(origin), floor(origin) + 1.0, step > 0);
  return (boundary - origin) / direction;
}
fn axis_t_delta(direction: f32) -> f32 { if (abs(direction) <= AXIS_EPSILON) { return 3.402823e38; } return abs(1.0 / direction); }
fn cast_world(origin: vec3f, direction: vec3f) -> RayHit {
  if (camera.position_and_max_distance.w <= 0.0 || length(direction) <= AXIS_EPSILON) { return no_hit(); }
  let bounds_min = vec3f(camera.bounds_min_and_padding.xyz);
  let bounds_max = vec3f(camera.bounds_max_and_max_steps.xyz) + vec3f(1.0);
  let x = axis_interval(origin.x, direction.x, bounds_min.x, bounds_max.x);
  let y = axis_interval(origin.y, direction.y, bounds_min.y, bounds_max.y);
  let z = axis_interval(origin.z, direction.z, bounds_min.z, bounds_max.z);
  let enter = max(max(x.x, y.x), z.x);
  let end_distance = min(min(x.y, y.y), min(z.y, camera.position_and_max_distance.w));
  if (end_distance < 0.0 || enter > end_distance || enter > camera.position_and_max_distance.w) { return no_hit(); }
  var distance = max(enter, 0.0);
  if (distance > 0.0) { distance = distance + ENTRY_EPSILON; }
  let start = origin + direction * distance;
  var coord = vec3i(floor(start));
  let step = vec3i(axis_step(direction.x), axis_step(direction.y), axis_step(direction.z));
  var t_max = vec3f(distance + initial_t_max(start.x, direction.x, step.x), distance + initial_t_max(start.y, direction.y, step.y), distance + initial_t_max(start.z, direction.z, step.z));
  let t_delta = vec3f(axis_t_delta(direction.x), axis_t_delta(direction.y), axis_t_delta(direction.z));
  var normal = vec3f(0.0);
  let max_steps = max(camera.bounds_max_and_max_steps.w, 1);
  // X then Y then Z tie ordering mirrors heliobound_gfx::raycast.
  for (var steps = 0; steps < max_steps && distance <= end_distance; steps = steps + 1) {
    let material = lookup_material(coord);
    if (material != 0u) { return RayHit(true, distance, coord, normal, material, 0u); }
    if (t_max.x <= t_max.y && t_max.x <= t_max.z) { coord.x = coord.x + step.x; distance = t_max.x; t_max.x = t_max.x + t_delta.x; normal = vec3f(f32(-step.x), 0.0, 0.0); }
    else if (t_max.y <= t_max.z) { coord.y = coord.y + step.y; distance = t_max.y; t_max.y = t_max.y + t_delta.y; normal = vec3f(0.0, f32(-step.y), 0.0); }
    else { coord.z = coord.z + step.z; distance = t_max.z; t_max.z = t_max.z + t_delta.z; normal = vec3f(0.0, 0.0, f32(-step.z)); }
  }
  return no_hit();
}
fn inverse_asset_rotate(v: vec3f, yaw: f32) -> vec3f {
  let turn = u32(round(yaw / 90.0)) % 4u;
  if (turn == 1u) { return vec3f(-v.z, v.y, v.x); }
  if (turn == 2u) { return vec3f(-v.x, v.y, -v.z); }
  if (turn == 3u) { return vec3f(v.z, v.y, -v.x); }
  return v;
}
fn asset_rotate(v: vec3f, yaw: f32) -> vec3f {
  let turn = u32(round(yaw / 90.0)) % 4u;
  if (turn == 1u) { return vec3f(v.z, v.y, -v.x); }
  if (turn == 2u) { return vec3f(-v.x, v.y, -v.z); }
  if (turn == 3u) { return vec3f(-v.z, v.y, v.x); }
  return v;
}
fn asset_material(asset: RenderAsset, coord: vec3i) -> u32 {
  for (var index = 0u; index < asset.dimensions.w; index = index + 1u) {
    let voxel = asset_voxels[asset.voxel_offset + index];
    if (all(vec3i(voxel.x, voxel.y, voxel.z) == coord)) { return voxel.material; }
  }
  return 0u;
}
fn cast_asset(asset: RenderAsset, origin: vec3f, direction: vec3f, max_distance: f32) -> RayHit {
  // World-space broad phase first; local DDA only runs for an intersected instance.
  let bx = axis_interval(origin.x, direction.x, asset.min.x, asset.max.x);
  let by = axis_interval(origin.y, direction.y, asset.min.y, asset.max.y);
  let bz = axis_interval(origin.z, direction.z, asset.min.z, asset.max.z);
  if (max(max(bx.x, by.x), bz.x) > min(min(bx.y, by.y), min(bz.y, max_distance))) { return no_hit(); }
  let local_origin = inverse_asset_rotate(origin - asset.anchor.xyz, asset.yaw_degrees);
  let local_direction = inverse_asset_rotate(direction, asset.yaw_degrees);
  let size = asset.voxel_size;
  let minimum = -asset.pivot.xyz * size;
  let maximum = minimum + vec3f(asset.dimensions.xyz) * size;
  let x = axis_interval(local_origin.x, local_direction.x, minimum.x, maximum.x);
  let y = axis_interval(local_origin.y, local_direction.y, minimum.y, maximum.y);
  let z = axis_interval(local_origin.z, local_direction.z, minimum.z, maximum.z);
  let enter = max(max(x.x, y.x), z.x);
  let end_distance = min(min(x.y, y.y), min(z.y, max_distance));
  if (end_distance < 0.0 || enter > end_distance) { return no_hit(); }
  var distance = max(enter, 0.0);
  let start = local_origin + local_direction * (distance + ENTRY_EPSILON);
  var coord = vec3i(floor(start / size + asset.pivot.xyz));
  let step = vec3i(axis_step(local_direction.x), axis_step(local_direction.y), axis_step(local_direction.z));
  var t_max = vec3f(distance + initial_t_max(start.x / size, local_direction.x / size, step.x), distance + initial_t_max(start.y / size, local_direction.y / size, step.y), distance + initial_t_max(start.z / size, local_direction.z / size, step.z));
  let t_delta = vec3f(axis_t_delta(local_direction.x / size), axis_t_delta(local_direction.y / size), axis_t_delta(local_direction.z / size));
  var normal = vec3f(0.0);
  let max_steps = i32(asset.dimensions.x + asset.dimensions.y + asset.dimensions.z + 3u);
  for (var steps = 0; steps < max_steps && distance <= end_distance; steps = steps + 1) {
    if (all(coord >= vec3i(0)) && all(vec3u(coord) < asset.dimensions.xyz)) {
      let material = asset_material(asset, coord);
      if (material != 0u) { return RayHit(true, distance, coord, asset_rotate(normal, asset.yaw_degrees), material, asset.ghost); }
    }
    if (t_max.x <= t_max.y && t_max.x <= t_max.z) { coord.x = coord.x + step.x; distance = t_max.x; t_max.x = t_max.x + t_delta.x; normal = vec3f(f32(-step.x), 0.0, 0.0); }
    else if (t_max.y <= t_max.z) { coord.y = coord.y + step.y; distance = t_max.y; t_max.y = t_max.y + t_delta.y; normal = vec3f(0.0, f32(-step.y), 0.0); }
    else { coord.z = coord.z + step.z; distance = t_max.z; t_max.z = t_max.z + t_delta.z; normal = vec3f(0.0, 0.0, f32(-step.z)); }
  }
  return no_hit();
}
fn cast_assets(origin: vec3f, direction: vec3f, max_distance: f32) -> RayHit {
  var closest = no_hit();
  let count = u32(camera.up_and_padding.w);
  for (var index = 0u; index < count; index = index + 1u) {
    let hit = cast_asset(render_assets[index], origin, direction, select(max_distance, closest.distance, closest.hit));
    if (hit.hit && (!closest.hit || hit.distance < closest.distance)) { closest = hit; }
  }
  return closest;
}
fn ray_for_cell(cell: vec2f) -> vec3f {
  let dimensions = vec2f(160.0, 90.0);
  let sensor_x = ((cell.x / dimensions.x) * 2.0 - 1.0) * camera.forward_and_aspect.w * camera.right_and_tan_half_fov.w;
  let sensor_y = (1.0 - (cell.y / dimensions.y) * 2.0) * camera.right_and_tan_half_fov.w;
  return normalize(camera.forward_and_aspect.xyz + camera.right_and_tan_half_fov.xyz * sensor_x + camera.up_and_padding.xyz * sensor_y);
}
struct TerrainOut { @location(0) glyph: u32, @location(1) colour: vec4f };
fn material_colour(material: u32) -> vec3f {
  switch material {
    case 1u: { return vec3f(0.659, 0.525, 0.384); } case 2u: { return vec3f(0.333, 0.353, 0.376); }
    case 3u: { return vec3f(0.176, 0.49, 0.788); } case 4u: { return vec3f(0.722, 0.922, 1.0); }
    case 5u: { return vec3f(0.404, 0.722, 0.278); } case 6u: { return vec3f(0.541, 0.357, 0.212); }
    case 7u: { return vec3f(0.557, 0.576, 0.588); } case 8u: { return vec3f(0.847, 0.761, 0.478); }
    case 9u: { return vec3f(0.627, 0.388, 0.196); } case 10u: { return vec3f(0.204, 0.561, 0.271); }
    case 11u: { return vec3f(0.541, 0.82, 0.404); } case 12u: { return vec3f(0.722, 0.788, 0.267); }
    case 13u: { return vec3f(0.875, 0.459, 0.62); } case 14u: { return vec3f(0.659, 0.616, 1.0); }
    case 15u: { return vec3f(0.612, 0.655, 0.698); } case 16u: { return vec3f(0.761, 0.784, 0.824); }
    case 17u: { return vec3f(0.624, 0.961, 1.0); } case 18u: { return vec3f(1.0, 0.855, 0.388); }
    case 19u: { return vec3f(1.0, 0.455, 0.224); } case 20u: { return vec3f(0.329, 0.408, 0.447); }
    case 21u: { return vec3f(0.239, 0.329, 0.38); } case 22u: { return vec3f(0.396, 0.439, 0.471); }
    case 23u: { return vec3f(0.843, 0.604, 0.227); }
    default: { if ((material & 0x80000000u) != 0u) { return vec3f(f32((material >> 16u) & 255u), f32((material >> 8u) & 255u), f32(material & 255u)) / 255.0; } return vec3f(1.0); }
  }
}
fn glyph_for(material: u32, intensity: f32) -> u32 {
  // This uses the same rounded ramp index as MaterialGlyphMap::shade.
  let clamped = clamp(intensity, 0.0, 1.0);
  let shade4 = min(u32(floor(clamped * 3.0 + 0.5)), 3u);
  let shade5 = min(u32(floor(clamped * 4.0 + 0.5)), 4u);
  switch material {
    case 2u, 7u: { return array<u32, 5>(45u,61u,43u,35u,35u)[shade5]; }
    case 12u: { return array<u32, 5>(46u,44u,33u,124u,89u)[shade5]; }
    case 1u: { return array<u32, 4>(46u,44u,58u,59u)[shade4]; }
    case 3u: { return array<u32, 4>(126u,61u,45u,43u)[shade4]; }
    case 4u: { return array<u32, 4>(96u,39u,42u,73u)[shade4]; }
    case 5u: { return array<u32, 4>(46u,44u,59u,34u)[shade4]; }
    case 6u: { return array<u32, 4>(46u,44u,58u,61u)[shade4]; }
    case 8u: { return array<u32, 4>(46u,44u,58u,126u)[shade4]; }
    case 9u: { return array<u32, 4>(58u,45u,124u,72u)[shade4]; }
    case 10u: { return array<u32, 4>(46u,44u,42u,37u)[shade4]; }
    case 11u: { return array<u32, 4>(122u,90u,38u,64u)[shade4]; }
    case 13u: { return array<u32, 4>(118u,120u,121u,89u)[shade4]; }
    case 14u: { return array<u32, 4>(94u,42u,37u,65u)[shade4]; }
    case 15u: { return array<u32, 4>(91u,93u,72u,77u)[shade4]; }
    case 16u: { return array<u32, 4>(60u,62u,88u,90u)[shade4]; }
    case 17u: { return array<u32, 4>(39u,46u,111u,79u)[shade4]; }
    case 18u: { return array<u32, 4>(105u,33u,42u,64u)[shade4]; }
    case 19u: { return array<u32, 4>(40u,41u,48u,64u)[shade4]; }
    case 20u: { return array<u32, 4>(46u,58u,114u,82u)[shade4]; }
    case 21u: { return array<u32, 4>(46u,45u,61u,43u)[shade4]; }
    case 22u: { return array<u32, 4>(124u,35u,72u,77u)[shade4]; }
    case 23u: { return array<u32, 4>(46u,95u,61u,43u)[shade4]; }
    default: { return array<u32, 5>(46u,58u,43u,35u,64u)[shade5]; }
  }
}
fn unpack_rgba(value: u32) -> vec4f {
  return vec4f(f32((value >> 24u) & 255u), f32((value >> 16u) & 255u), f32((value >> 8u) & 255u), f32(value & 255u)) / 255.0;
}
// Material, asset, and sky values are authored display-space sRGB colours.
// Rgba8UnormSrgb targets expect linear fragment output and return linear
// samples to the glyph compositor, preserving the CPU renderer's byte values.
fn srgb_to_linear(channel: f32) -> f32 {
  if (channel <= 0.04045) { return channel / 12.92; }
  return pow((channel + 0.055) / 1.055, 2.4);
}
fn srgb_to_linear_colour(colour: vec3f) -> vec3f {
  return vec3f(srgb_to_linear(colour.r), srgb_to_linear(colour.g), srgb_to_linear(colour.b));
}
@fragment fn fs_terrain(@builtin(position) position: vec4f) -> TerrainOut {
  let direction = ray_for_cell(position.xy);
  let world_hit = cast_world(camera.position_and_max_distance.xyz, direction);
  let asset_hit = cast_assets(camera.position_and_max_distance.xyz, direction, camera.position_and_max_distance.w);
  var hit = asset_hit;
  if (world_hit.hit && (!asset_hit.hit || world_hit.distance <= asset_hit.distance)) { hit = world_hit; }
  if (!hit.hit) {
    let cell = u32(position.x) + u32(position.y) * 160u;
    let background = background_cells[cell];
    let colour = unpack_rgba(background.foreground_rgba);
    return TerrainOut(background.glyph, vec4f(srgb_to_linear_colour(colour.rgb), colour.a));
  }
  let light = max(dot(hit.normal, normalize(vec3f(-0.35, 0.75, -0.55))), 0.0);
  let distance_fade = clamp(1.0 - hit.distance / 180000.0, 0.2, 1.0);
  let intensity = light * 0.7 + distance_fade * 0.3;
  let brightness = clamp(0.48 + light * 0.52, 0.35, 1.0);
  let colour = material_colour(hit.material) * brightness * select(1.0, 0.55, hit.ghost != 0u);
  return TerrainOut(glyph_for(hit.material, intensity), vec4f(srgb_to_linear_colour(colour), 1.0));
}
@fragment fn fs_diagnostic() -> @location(0) vec4f { return vec4f(0.0, 0.0, 0.0, 1.0); }
