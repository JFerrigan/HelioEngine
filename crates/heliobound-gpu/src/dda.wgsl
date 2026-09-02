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
struct RayHit { hit: bool, distance: f32, coord: vec3i, normal: vec3f, material: u32 };

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> chunk_lookup: array<u32>;
@group(0) @binding(2) var<storage, read> chunk_voxels: array<u32>;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
  var positions = array<vec2f, 3>(vec2f(-1.0, -3.0), vec2f(3.0, 1.0), vec2f(-1.0, 1.0));
  return VertexOut(vec4f(positions[index], 0.0, 1.0));
}
fn no_hit() -> RayHit { return RayHit(false, 0.0, vec3i(0), vec3f(0.0), 0u); }
fn floor_div_16(value: i32) -> i32 {
  // WGSL division truncates; CPU uses div_euclid for negative chunks.
  if (value < 0) { return -((-value + 15) / CHUNK_EDGE); }
  return value / CHUNK_EDGE;
}
fn lookup_material(coord: vec3i) -> u32 {
  let chunk = vec3i(floor_div_16(coord.x), floor_div_16(coord.y), floor_div_16(coord.z));
  let local_chunk = chunk - camera.table_origin_and_padding.xyz;
  let dimensions = camera.table_dimensions_and_padding.xyz;
  if (any(local_chunk < vec3i(0)) || u32(local_chunk.x) >= dimensions.x || u32(local_chunk.y) >= dimensions.y || u32(local_chunk.z) >= dimensions.z) { return 0u; }
  let lookup_index = u32(local_chunk.x) + u32(local_chunk.y) * dimensions.x + u32(local_chunk.z) * dimensions.x * dimensions.y;
  let encoded_slot = chunk_lookup[lookup_index];
  if (encoded_slot == 0u) { return 0u; }
  let local = coord - chunk * CHUNK_EDGE;
  let voxel_index = u32(local.x) + u32(local.y) * 16u + u32(local.z) * 256u;
  return chunk_voxels[(encoded_slot - 1u) * CHUNK_VOLUME + voxel_index];
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
    if (material != 0u) { return RayHit(true, distance, coord, normal, material); }
    if (t_max.x <= t_max.y && t_max.x <= t_max.z) { coord.x = coord.x + step.x; distance = t_max.x; t_max.x = t_max.x + t_delta.x; normal = vec3f(f32(-step.x), 0.0, 0.0); }
    else if (t_max.y <= t_max.z) { coord.y = coord.y + step.y; distance = t_max.y; t_max.y = t_max.y + t_delta.y; normal = vec3f(0.0, f32(-step.y), 0.0); }
    else { coord.z = coord.z + step.z; distance = t_max.z; t_max.z = t_max.z + t_delta.z; normal = vec3f(0.0, 0.0, f32(-step.z)); }
  }
  return no_hit();
}
fn ray_for_cell(cell: vec2f) -> vec3f {
  let dimensions = vec2f(160.0, 90.0);
  let sensor_x = ((cell.x / dimensions.x) * 2.0 - 1.0) * camera.forward_and_aspect.w * camera.right_and_tan_half_fov.w;
  let sensor_y = (1.0 - (cell.y / dimensions.y) * 2.0) * camera.right_and_tan_half_fov.w;
  return normalize(camera.forward_and_aspect.xyz + camera.right_and_tan_half_fov.xyz * sensor_x + camera.up_and_padding.xyz * sensor_y);
}
// Temporary terrain diagnostic: normal and distance output. Glyph/material
// passes consume this DDA result later without CPU readback.
@fragment fn fs_terrain(@builtin(position) position: vec4f) -> @location(0) vec4f {
  let hit = cast_world(camera.position_and_max_distance.xyz, ray_for_cell(position.xy));
  if (!hit.hit) { return vec4f(0.0, 0.0, 0.0, 1.0); }
  let fade = 1.0 - clamp(hit.distance / camera.position_and_max_distance.w, 0.0, 0.65);
  return vec4f((hit.normal * 0.5 + vec3f(0.5)) * fade, 1.0);
}
@fragment fn fs_diagnostic() -> @location(0) vec4f { return vec4f(0.0, 0.0, 0.0, 1.0); }
