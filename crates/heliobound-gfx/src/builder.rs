use crate::scene::{Layer, Overlay, RenderAsset, Scene, SceneCell, TextStyle, Viewport};
use heliobound_core::{
    Camera, PlanetHit, PlanetTerrainClass, ProceduralPlanet, Ray, Vec3, VoxelBounds, VoxelCell,
    VoxelCoord, VoxelMaterial, VoxelWorld,
};

#[derive(Clone, Copy, Debug)]
pub struct GraphicsConfig {
    pub viewport: Viewport,
    pub max_distance: f32,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            viewport: Viewport {
                width: 160,
                height: 90,
            },
            max_distance: 96.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelHit {
    pub coord: VoxelCoord,
    pub cell: VoxelCell,
    pub distance: f32,
    pub normal: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct MaterialGlyphMap;

impl MaterialGlyphMap {
    pub fn glyph_for(&self, hit: VoxelHit) -> char {
        self.glyph_for_material(hit.cell.material, hit.normal, hit.distance)
    }

    pub fn style_for(&self, hit: VoxelHit) -> TextStyle {
        self.style_for_material(hit.cell.material, hit.normal)
    }

    pub fn glyph_for_planet(&self, hit: PlanetHit) -> char {
        let lit = hit
            .normal
            .dot(Vec3::new(-0.35, 0.75, -0.55).normalized())
            .max(0.0);
        let detail = (hit.sample.detail * 0.65 + hit.sample.ruggedness * 0.35).clamp(0.0, 1.0);
        let distance_fade = (1.0 - (hit.distance / 8_000_000.0)).clamp(0.25, 1.0);
        let intensity = (lit * 0.45 + detail * 0.35 + distance_fade * 0.2).clamp(0.0, 1.0);

        match hit.sample.terrain {
            PlanetTerrainClass::DeepOcean => shade(intensity, "~~~=+"),
            PlanetTerrainClass::ShallowOcean => shade(intensity, "~~-:."),
            PlanetTerrainClass::Coast => shade(intensity, ".,:;="),
            PlanetTerrainClass::Plains => shade(intensity, ".,;\"+"),
            PlanetTerrainClass::Hills => shade(intensity, "nrvx#"),
            PlanetTerrainClass::Mountains => shade(intensity, "^/A#M"),
            PlanetTerrainClass::IceCap => shade(intensity, "`'*I#"),
            PlanetTerrainClass::CarbonBloom => shade(intensity, "vxyY&"),
            PlanetTerrainClass::SiliconField => shade(intensity, "^*%AX"),
            PlanetTerrainClass::Crater => shade(intensity, " .oO@"),
        }
    }

    pub fn style_for_planet(&self, hit: PlanetHit) -> TextStyle {
        self.style_for_material(hit.sample.material, hit.normal)
    }

    fn glyph_for_material(&self, material: VoxelMaterial, normal: Vec3, distance: f32) -> char {
        let lit = normal
            .dot(Vec3::new(-0.35, 0.75, -0.55).normalized())
            .max(0.0);
        let distance_fade = (1.0 - (distance / 180_000.0)).clamp(0.2, 1.0);
        let intensity = lit * 0.7 + distance_fade * 0.3;

        match material {
            VoxelMaterial::Regolith => shade(intensity, ".,:;"),
            VoxelMaterial::Basalt => shade(intensity, "-=+#"),
            VoxelMaterial::Ocean => shade(intensity, "~=-+"),
            VoxelMaterial::Ice => shade(intensity, "`'*I"),
            VoxelMaterial::Grass => shade(intensity, ".,;\""),
            VoxelMaterial::Dirt => shade(intensity, ".,:="),
            VoxelMaterial::Stone => shade(intensity, "-=+#"),
            VoxelMaterial::Sand => shade(intensity, ".,:~"),
            VoxelMaterial::Wood => shade(intensity, ":-|H"),
            VoxelMaterial::Leaves => shade(intensity, ".,*%"),
            VoxelMaterial::Zombie => shade(intensity, "zZ&@"),
            VoxelMaterial::CornStalk => shade(intensity, ".,!|Y"),
            VoxelMaterial::CarbonLife => shade(intensity, "vxyY"),
            VoxelMaterial::SiliconLife => shade(intensity, "^*%A"),
            VoxelMaterial::Habitat => shade(intensity, "[]HM"),
            VoxelMaterial::ShipHull => shade(intensity, "<>XZ"),
            VoxelMaterial::Glass => shade(intensity, "'.oO"),
            VoxelMaterial::Beacon => shade(intensity, "i!*@"),
            VoxelMaterial::Gate => shade(intensity, "()0@"),
            VoxelMaterial::Receiver => shade(intensity, ".:rR"),
            VoxelMaterial::SignalPipe => shade(intensity, ".-=+"),
            VoxelMaterial::PuzzleDoor => shade(intensity, "|#HM"),
            VoxelMaterial::PressurePlate => shade(intensity, "._=+"),
            VoxelMaterial::Custom(_) => shade(intensity, ".:+#@"),
        }
    }

    fn style_for_material(&self, material: VoxelMaterial, normal: Vec3) -> TextStyle {
        let lit = normal
            .dot(Vec3::new(-0.35, 0.75, -0.55).normalized())
            .max(0.0);
        let brightness = (0.48 + lit * 0.52).clamp(0.35, 1.0);
        TextStyle {
            fg: Some(scale_hex_color(material_base_color(material), brightness)),
            bg: None,
            bold: brightness > 0.82,
        }
    }
}

fn material_base_color(material: VoxelMaterial) -> [u8; 3] {
    match material {
        VoxelMaterial::Regolith => [0xa8, 0x86, 0x62],
        VoxelMaterial::Basalt => [0x55, 0x5a, 0x60],
        VoxelMaterial::Ocean => [0x2d, 0x7d, 0xc9],
        VoxelMaterial::Ice => [0xb8, 0xeb, 0xff],
        VoxelMaterial::Grass => [0x67, 0xb8, 0x47],
        VoxelMaterial::Dirt => [0x8a, 0x5b, 0x36],
        VoxelMaterial::Stone => [0x8e, 0x93, 0x96],
        VoxelMaterial::Sand => [0xd8, 0xc2, 0x7a],
        VoxelMaterial::Wood => [0xa0, 0x63, 0x32],
        VoxelMaterial::Leaves => [0x34, 0x8f, 0x45],
        VoxelMaterial::Zombie => [0x8a, 0xd1, 0x67],
        VoxelMaterial::CornStalk => [0xb8, 0xc9, 0x44],
        VoxelMaterial::CarbonLife => [0xdf, 0x75, 0x9e],
        VoxelMaterial::SiliconLife => [0xa8, 0x9d, 0xff],
        VoxelMaterial::Habitat => [0x9c, 0xa7, 0xb2],
        VoxelMaterial::ShipHull => [0xc2, 0xc8, 0xd2],
        VoxelMaterial::Glass => [0x9f, 0xf5, 0xff],
        VoxelMaterial::Beacon => [0xff, 0xda, 0x63],
        VoxelMaterial::Gate => [0xff, 0x74, 0x39],
        VoxelMaterial::Receiver => [0x54, 0x68, 0x72],
        VoxelMaterial::SignalPipe => [0x3d, 0x54, 0x61],
        VoxelMaterial::PuzzleDoor => [0x65, 0x70, 0x78],
        VoxelMaterial::PressurePlate => [0xd7, 0x9a, 0x3a],
        VoxelMaterial::Custom(color) => color,
    }
}

fn scale_hex_color(color: [u8; 3], brightness: f32) -> String {
    let [r, g, b] = color;
    format!(
        "#{:02x}{:02x}{:02x}",
        scale_channel(r, brightness),
        scale_channel(g, brightness),
        scale_channel(b, brightness)
    )
}

fn scale_channel(channel: u8, brightness: f32) -> u8 {
    ((channel as f32 * brightness).round()).clamp(0.0, 255.0) as u8
}

#[derive(Clone, Debug)]
pub struct SceneBuilder {
    pub config: GraphicsConfig,
    pub materials: MaterialGlyphMap,
}

impl SceneBuilder {
    pub fn new(config: GraphicsConfig, materials: MaterialGlyphMap) -> Self {
        Self { config, materials }
    }

    pub fn build(&self, world: &VoxelWorld, camera: &Camera, tick: u64) -> Scene {
        self.build_with_visibility(world, camera, tick, |_| true)
    }

    /// Renders ordinary map voxels together with mixed-resolution asset
    /// voxels. The latter are tested directly as AABBs, preserving their
    /// authored size instead of rounding them into map-sized cubes.
    pub fn build_with_render_assets(
        &self,
        world: &VoxelWorld,
        render_assets: &[RenderAsset],
        camera: &Camera,
        tick: u64,
    ) -> Scene {
        self.build_with_visibility_and_render_assets(world, render_assets, camera, tick, |_| true)
    }

    /// Builds against the complete voxel world while allowing a caller to hide
    /// individual hit faces. Hidden hits remain blank occluders, so geometry
    /// behind them cannot leak into view.
    pub fn build_with_visibility(
        &self,
        world: &VoxelWorld,
        camera: &Camera,
        _tick: u64,
        is_visible: impl Fn(VoxelHit) -> bool,
    ) -> Scene {
        self.build_with_visibility_and_render_assets(world, &[], camera, _tick, is_visible)
    }

    fn build_with_visibility_and_render_assets(
        &self,
        world: &VoxelWorld,
        render_assets: &[RenderAsset],
        camera: &Camera,
        _tick: u64,
        is_visible: impl Fn(VoxelHit) -> bool,
    ) -> Scene {
        let mut scene = Scene::new(self.config.viewport);
        let height = self.config.viewport.height as i32;

        let mut background = Layer {
            name: "background".to_string(),
            z: 0,
            cells: Vec::with_capacity(self.config.viewport.width * self.config.viewport.height),
        };
        let mut voxels = Layer {
            name: "voxels".to_string(),
            z: 10,
            cells: Vec::new(),
        };
        let ray_grid = RayGrid::new(*camera, self.config.viewport);

        for y in 0..self.config.viewport.height {
            for x in 0..self.config.viewport.width {
                let ray = ray_grid.ray_for_cell(x, y);
                background.cells.push(SceneCell {
                    x: x as i32,
                    y: y as i32,
                    glyph: background_glyph_for_direction(ray.direction),
                    style: TextStyle::default(),
                });
                let world_hit = raycast(
                    world,
                    ray,
                    self.config.max_distance.min(camera.max_distance),
                );
                // Terrain is authoritative for occlusion. Do not spend time
                // walking an asset whose first possible detail lies behind it.
                let asset_hit = raycast_render_assets(
                    render_assets,
                    ray,
                    world_hit.map_or(self.config.max_distance.min(camera.max_distance), |hit| {
                        hit.distance
                    }),
                );
                if let Some(hit) = world_hit
                    .filter(|hit| asset_hit.is_none_or(|asset| hit.distance <= asset.distance))
                {
                    let visible = is_visible(hit);
                    voxels.cells.push(SceneCell {
                        x: x as i32,
                        y: y as i32,
                        glyph: if visible {
                            self.materials.glyph_for(hit)
                        } else {
                            ' '
                        },
                        style: if visible {
                            self.materials.style_for(hit)
                        } else {
                            TextStyle::default()
                        },
                    });
                } else if let Some(hit) = asset_hit {
                    let mut style = self.materials.style_for_material(hit.material, hit.normal);
                    if hit.ghost {
                        style.fg = style.fg.map(|color| dim_hex_color(&color));
                    }
                    voxels.cells.push(SceneCell {
                        x: x as i32,
                        y: y as i32,
                        glyph: self.materials.glyph_for_material(
                            hit.material,
                            hit.normal,
                            hit.distance,
                        ),
                        style,
                    });
                }
            }
        }

        scene.layers.push(background);
        scene.layers.push(voxels);
        scene.overlays.push(Overlay {
            x: 2,
            y: height - 3,
            z: 100,
            text: format!(
                "voxels {}  camera {:.1},{:.1},{:.1}",
                world.voxel_count(),
                camera.position.x,
                camera.position.y,
                camera.position.z
            ),
            style: TextStyle::default(),
        });

        scene
    }

    pub fn build_planet(&self, planet: &ProceduralPlanet, camera: &Camera, _tick: u64) -> Scene {
        let mut scene = Scene::new(self.config.viewport);
        let height = self.config.viewport.height as i32;

        let mut background = Layer {
            name: "background".to_string(),
            z: 0,
            cells: Vec::with_capacity(self.config.viewport.width * self.config.viewport.height),
        };
        let mut planet_layer = Layer {
            name: "planet".to_string(),
            z: 10,
            cells: Vec::new(),
        };
        let ray_grid = RayGrid::new(*camera, self.config.viewport);

        for y in 0..self.config.viewport.height {
            for x in 0..self.config.viewport.width {
                let ray = ray_grid.ray_for_cell(x, y);
                background.cells.push(SceneCell {
                    x: x as i32,
                    y: y as i32,
                    glyph: background_glyph_for_direction(ray.direction),
                    style: TextStyle::default(),
                });
                if let Some(hit) =
                    planet.raycast(ray, self.config.max_distance.min(camera.max_distance))
                {
                    planet_layer.cells.push(SceneCell {
                        x: x as i32,
                        y: y as i32,
                        glyph: self.materials.glyph_for_planet(hit),
                        style: self.materials.style_for_planet(hit),
                    });
                }
            }
        }

        scene.layers.push(background);
        scene.layers.push(planet_layer);
        scene.overlays.push(Overlay {
            x: 2,
            y: height - 3,
            z: 100,
            text: format!(
                "planet radius {:.0}  camera {:.0},{:.0},{:.0}  roll {:.1}",
                planet.radius(),
                camera.position.x,
                camera.position.y,
                camera.position.z,
                camera.roll_radians.to_degrees()
            ),
            style: TextStyle::default(),
        });

        scene
    }
}

#[derive(Clone, Copy)]
struct RenderVoxelHit {
    material: VoxelMaterial,
    distance: f32,
    normal: Vec3,
    ghost: bool,
}

fn raycast_render_assets(
    assets: &[RenderAsset],
    ray: Ray,
    max_distance: f32,
) -> Option<RenderVoxelHit> {
    assets
        .iter()
        .filter_map(|asset| {
            // The broad-phase slab test rejects whole instances. Detail then
            // uses local DDA, rather than scanning every occupied voxel.
            intersect_render_asset_bounds(ray, asset.min, asset.max, max_distance)?;
            raycast_render_asset_local(asset, ray, max_distance)
        })
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

fn raycast_render_asset_local(
    asset: &RenderAsset,
    ray: Ray,
    max_distance: f32,
) -> Option<RenderVoxelHit> {
    let origin = rotate_asset_vector(ray.origin - asset.anchor, inverse_yaw(asset.yaw_degrees));
    let direction = rotate_asset_vector(ray.direction, inverse_yaw(asset.yaw_degrees));
    let local_ray = Ray::new(origin, direction);
    let size = asset.voxel_size;
    let min = Vec3::new(
        -asset.pivot[0] * size,
        -asset.pivot[1] * size,
        -asset.pivot[2] * size,
    );
    let max = min
        + Vec3::new(
            asset.dimensions[0] as f32 * size,
            asset.dimensions[1] as f32 * size,
            asset.dimensions[2] as f32 * size,
        );
    let (mut distance, exit, mut normal) =
        intersect_asset_bounds(local_ray, min, max, max_distance)?;
    distance = distance.max(0.0);
    let point = local_ray.origin + local_ray.direction * (distance + 0.0001);
    let mut cell = VoxelCoord::new(
        ((point.x / size) + asset.pivot[0]).floor() as i32,
        ((point.y / size) + asset.pivot[1]).floor() as i32,
        ((point.z / size) + asset.pivot[2]).floor() as i32,
    );
    let step = |direction: f32| if direction >= 0.0 { 1 } else { -1 };
    let sx = step(local_ray.direction.x);
    let sy = step(local_ray.direction.y);
    let sz = step(local_ray.direction.z);
    let next = |coord: i32, pivot: f32, direction: f32, step: i32| {
        if direction.abs() <= f32::EPSILON {
            f32::INFINITY
        } else {
            (((if step > 0 { coord + 1 } else { coord }) as f32 - pivot) * size
                - local_ray.origin.x)
                / direction
        }
    };
    let mut tx = next(cell.x, asset.pivot[0], local_ray.direction.x, sx);
    let mut ty = if local_ray.direction.y.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        (((if sy > 0 { cell.y + 1 } else { cell.y }) as f32 - asset.pivot[1]) * size
            - local_ray.origin.y)
            / local_ray.direction.y
    };
    let mut tz = if local_ray.direction.z.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        (((if sz > 0 { cell.z + 1 } else { cell.z }) as f32 - asset.pivot[2]) * size
            - local_ray.origin.z)
            / local_ray.direction.z
    };
    // `next` is x-specific because it closes over the origin; keep the other
    // axes explicit and advance by a fixed physical-cell interval.
    let dx = if local_ray.direction.x.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        size / local_ray.direction.x.abs()
    };
    let dy = if local_ray.direction.y.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        size / local_ray.direction.y.abs()
    };
    let dz = if local_ray.direction.z.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        size / local_ray.direction.z.abs()
    };
    while distance <= exit && distance <= max_distance {
        if cell.x >= 0
            && cell.y >= 0
            && cell.z >= 0
            && cell.x < asset.dimensions[0]
            && cell.y < asset.dimensions[1]
            && cell.z < asset.dimensions[2]
        {
            if let Some(&material) = asset.voxels.get(&cell) {
                return Some(RenderVoxelHit {
                    material,
                    distance,
                    normal: rotate_asset_vector(normal, asset.yaw_degrees),
                    ghost: asset.ghost,
                });
            }
        }
        if tx <= ty && tx <= tz {
            distance = tx;
            tx += dx;
            cell.x += sx;
            normal = Vec3::new(-(sx as f32), 0.0, 0.0);
        } else if ty <= tz {
            distance = ty;
            ty += dy;
            cell.y += sy;
            normal = Vec3::new(0.0, -(sy as f32), 0.0);
        } else {
            distance = tz;
            tz += dz;
            cell.z += sz;
            normal = Vec3::new(0.0, 0.0, -(sz as f32));
        }
    }
    None
}

fn inverse_yaw(yaw: u16) -> u16 {
    (360 - yaw) % 360
}
fn rotate_asset_vector(v: Vec3, yaw: u16) -> Vec3 {
    match yaw {
        0 => v,
        90 => Vec3::new(v.z, v.y, -v.x),
        180 => Vec3::new(-v.x, v.y, -v.z),
        270 => Vec3::new(-v.z, v.y, v.x),
        _ => unreachable!("asset yaw is validated"),
    }
}

fn intersect_asset_bounds(
    ray: Ray,
    min: Vec3,
    max: Vec3,
    max_distance: f32,
) -> Option<(f32, f32, Vec3)> {
    let mut enter = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    let mut normal = Vec3::ZERO;
    for (origin, direction, lo, hi, axis) in [
        (
            ray.origin.x,
            ray.direction.x,
            min.x,
            max.x,
            Vec3::new(-1.0, 0.0, 0.0),
        ),
        (
            ray.origin.y,
            ray.direction.y,
            min.y,
            max.y,
            Vec3::new(0.0, -1.0, 0.0),
        ),
        (
            ray.origin.z,
            ray.direction.z,
            min.z,
            max.z,
            Vec3::new(0.0, 0.0, -1.0),
        ),
    ] {
        if direction.abs() <= f32::EPSILON {
            if origin < lo || origin > hi {
                return None;
            }
            continue;
        }
        let mut near = (lo - origin) / direction;
        let mut far = (hi - origin) / direction;
        let mut n = axis;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            n = n * -1.0;
        }
        if near > enter {
            enter = near;
            normal = n;
        }
        exit = exit.min(far);
        if enter > exit {
            return None;
        }
    }
    (exit >= 0.0 && enter <= max_distance).then_some((enter, exit, normal))
}

fn intersect_render_asset_bounds(ray: Ray, min: Vec3, max: Vec3, max_distance: f32) -> Option<()> {
    let mut enter = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    for (origin, direction, min, max) in [
        (ray.origin.x, ray.direction.x, min.x, max.x),
        (ray.origin.y, ray.direction.y, min.y, max.y),
        (ray.origin.z, ray.direction.z, min.z, max.z),
    ] {
        if direction.abs() <= f32::EPSILON {
            if origin < min || origin > max {
                return None;
            }
            continue;
        }
        let mut near = (min - origin) / direction;
        let mut far = (max - origin) / direction;
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        enter = enter.max(near);
        exit = exit.min(far);
        if enter > exit {
            return None;
        }
    }
    (exit >= 0.0 && enter <= max_distance).then_some(())
}

fn dim_hex_color(color: &str) -> String {
    let channel = |offset| u8::from_str_radix(&color[offset..offset + 2], 16).unwrap_or(128);
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(1) / 2,
        channel(3) / 2,
        channel(5) / 2
    )
}

#[derive(Clone, Copy, Debug)]
struct RayGrid {
    camera: Camera,
    viewport: Viewport,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    aspect: f32,
    tan_half_fov: f32,
}

impl RayGrid {
    fn new(camera: Camera, viewport: Viewport) -> Self {
        let forward = camera.forward();
        let right = camera.right();
        let up = camera.up();
        Self {
            camera,
            viewport,
            forward,
            right,
            up,
            aspect: viewport.width.max(1) as f32 / viewport.height.max(1) as f32,
            tan_half_fov: (camera.fov_y_radians * 0.5).tan(),
        }
    }

    fn ray_for_cell(self, x: usize, y: usize) -> Ray {
        let width = self.viewport.width.max(1) as f32;
        let height = self.viewport.height.max(1) as f32;
        let sensor_x = ((((x as f32 + 0.5) / width) * 2.0) - 1.0) * self.aspect * self.tan_half_fov;
        let sensor_y = (1.0 - (((y as f32 + 0.5) / height) * 2.0)) * self.tan_half_fov;
        Ray::new(
            self.camera.position,
            self.forward + self.right * sensor_x + self.up * sensor_y,
        )
    }
}

pub fn raycast(world: &VoxelWorld, ray: Ray, max_distance: f32) -> Option<VoxelHit> {
    if ray.direction.length() <= f32::EPSILON || max_distance <= 0.0 {
        return None;
    }

    let (start_distance, end_distance) = match world.bounds() {
        Some(bounds) => intersect_bounds(bounds, ray, max_distance)?,
        None => return None,
    };
    let start_distance = if start_distance > 0.0 {
        start_distance + 0.0001
    } else {
        0.0
    };
    let start = ray.point_at(start_distance);
    let mut coord = VoxelCoord::new(
        start.x.floor() as i32,
        start.y.floor() as i32,
        start.z.floor() as i32,
    );
    let step = VoxelCoord::new(
        axis_step(ray.direction.x),
        axis_step(ray.direction.y),
        axis_step(ray.direction.z),
    );
    let mut t_max = Vec3::new(
        start_distance + initial_t_max(start.x, ray.direction.x, step.x),
        start_distance + initial_t_max(start.y, ray.direction.y, step.y),
        start_distance + initial_t_max(start.z, ray.direction.z, step.z),
    );
    let t_delta = Vec3::new(
        t_delta(ray.direction.x),
        t_delta(ray.direction.y),
        t_delta(ray.direction.z),
    );
    let mut distance = start_distance;
    let mut normal = Vec3::ZERO;

    // The world is finite, so its bounds are an intrinsic endpoint even when
    // the caller intentionally supplies an unlimited view distance.
    while distance <= end_distance {
        if let Some(cell) = world.get(coord) {
            return Some(VoxelHit {
                coord,
                cell,
                distance,
                normal,
            });
        }

        if t_max.x <= t_max.y && t_max.x <= t_max.z {
            coord.x += step.x;
            distance = t_max.x;
            t_max.x += t_delta.x;
            normal = Vec3::new(-(step.x as f32), 0.0, 0.0);
        } else if t_max.y <= t_max.z {
            coord.y += step.y;
            distance = t_max.y;
            t_max.y += t_delta.y;
            normal = Vec3::new(0.0, -(step.y as f32), 0.0);
        } else {
            coord.z += step.z;
            distance = t_max.z;
            t_max.z += t_delta.z;
            normal = Vec3::new(0.0, 0.0, -(step.z as f32));
        }
    }

    None
}

fn intersect_bounds(bounds: VoxelBounds, ray: Ray, max_distance: f32) -> Option<(f32, f32)> {
    let min = Vec3::new(
        bounds.min.x as f32,
        bounds.min.y as f32,
        bounds.min.z as f32,
    );
    let max = Vec3::new(
        bounds.max.x as f32 + 1.0,
        bounds.max.y as f32 + 1.0,
        bounds.max.z as f32 + 1.0,
    );

    let (tx_min, tx_max) = axis_bounds(ray.origin.x, ray.direction.x, min.x, max.x)?;
    let (ty_min, ty_max) = axis_bounds(ray.origin.y, ray.direction.y, min.y, max.y)?;
    let (tz_min, tz_max) = axis_bounds(ray.origin.z, ray.direction.z, min.z, max.z)?;

    let enter = tx_min.max(ty_min).max(tz_min);
    let exit = tx_max.min(ty_max).min(tz_max);

    if exit < 0.0 || enter > exit || enter > max_distance {
        None
    } else {
        Some((enter.max(0.0), exit.min(max_distance)))
    }
}

fn axis_bounds(origin: f32, direction: f32, min: f32, max: f32) -> Option<(f32, f32)> {
    if direction.abs() <= f32::EPSILON {
        if origin >= min && origin <= max {
            Some((f32::NEG_INFINITY, f32::INFINITY))
        } else {
            None
        }
    } else {
        let a = (min - origin) / direction;
        let b = (max - origin) / direction;
        Some((a.min(b), a.max(b)))
    }
}

fn axis_step(direction: f32) -> i32 {
    if direction > 0.0 {
        1
    } else if direction < 0.0 {
        -1
    } else {
        0
    }
}

fn initial_t_max(origin: f32, direction: f32, step: i32) -> f32 {
    if step == 0 {
        return f32::INFINITY;
    }

    let next_boundary = if step > 0 {
        origin.floor() + 1.0
    } else {
        origin.floor()
    };
    (next_boundary - origin) / direction
}

fn t_delta(direction: f32) -> f32 {
    if direction.abs() <= f32::EPSILON {
        f32::INFINITY
    } else {
        (1.0 / direction).abs()
    }
}

fn shade(intensity: f32, ramp: &str) -> char {
    let chars: Vec<char> = ramp.chars().collect();
    let idx = ((chars.len() - 1) as f32 * intensity.clamp(0.0, 1.0)).round() as usize;
    chars[idx]
}

const SKY_CELL_RADIANS: f32 = 0.09;
const STAR_CORE_RADIANS: f32 = 0.008;
const STAR_HALO_RADIANS: f32 = 0.016;
const MIN_POLAR_WIDTH: f32 = 0.2;

fn star_for_direction(direction: Vec3) -> char {
    let (yaw, pitch) = sky_angles(direction);
    let (cell_x, cell_y) = sky_cell_for_angles(yaw, pitch);
    let mut closest: Option<(f32, char)> = None;

    for y in (cell_y - 1)..=(cell_y + 1) {
        for x in (cell_x - 1)..=(cell_x + 1) {
            let hash = hash_sky_cell(x, y, 0);
            if hash % 3 != 0 {
                continue;
            }

            let center_yaw =
                (x as f32 + hash_unit(hash, 0)) * SKY_CELL_RADIANS - std::f32::consts::PI;
            let center_pitch = ((y as f32 + hash_unit(hash, 16)) * SKY_CELL_RADIANS
                - std::f32::consts::FRAC_PI_2)
                .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
            let yaw_delta = short_angle_delta(yaw, center_yaw);
            let pitch_delta = pitch - center_pitch;
            let polar_width = pitch.cos().abs().max(MIN_POLAR_WIDTH);
            let distance = ((yaw_delta * polar_width).powi(2) + pitch_delta.powi(2)).sqrt();

            if distance <= STAR_HALO_RADIANS {
                let glyph = if distance <= STAR_CORE_RADIANS {
                    bright_star_glyph(hash)
                } else {
                    '.'
                };
                if closest
                    .map(|(closest_distance, _)| distance < closest_distance)
                    .unwrap_or(true)
                {
                    closest = Some((distance, glyph));
                }
            }
        }
    }

    closest.map(|(_, glyph)| glyph).unwrap_or(' ')
}

/// The starfield represents the sky, rather than a backdrop behind the world.
/// Rays at or below the horizon intentionally receive an empty background.
/// Deterministic logical sky used by both the CPU scene builder and GPU
/// presentation bridge. It is intentionally independent of frame tick.
pub fn background_glyph_for_direction(direction: Vec3) -> char {
    if direction.y > 0.0 {
        star_for_direction(direction)
    } else {
        ' '
    }
}

fn sky_angles(direction: Vec3) -> (f32, f32) {
    let direction = direction.normalized();
    let yaw = direction.x.atan2(direction.z);
    let pitch = direction.y.clamp(-1.0, 1.0).asin();
    (yaw, pitch)
}

#[cfg(test)]
fn sky_cell_for_direction(direction: Vec3) -> (i32, i32) {
    let (yaw, pitch) = sky_angles(direction);
    sky_cell_for_angles(yaw, pitch)
}

fn sky_cell_for_angles(yaw: f32, pitch: f32) -> (i32, i32) {
    let wrapped_yaw = (yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
    let clamped_pitch = (pitch + std::f32::consts::FRAC_PI_2).clamp(0.0, std::f32::consts::PI);
    (
        (wrapped_yaw / SKY_CELL_RADIANS).floor() as i32,
        (clamped_pitch / SKY_CELL_RADIANS).floor() as i32,
    )
}

fn hash_unit(hash: u64, shift: u32) -> f32 {
    (((hash >> shift) & 0xffff) as f32) / 65_535.0
}

fn short_angle_delta(a: f32, b: f32) -> f32 {
    (a - b + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn bright_star_glyph(hash: u64) -> char {
    if hash % 23 == 0 {
        '*'
    } else if hash % 11 == 0 {
        '+'
    } else {
        '.'
    }
}

fn hash_sky_cell(x: i32, y: i32, z: i32) -> u64 {
    let mut h = 0xD1B5_4A32_D192_ED03_u64;
    h ^= (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = h.rotate_left(21);
    h ^= (y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = h.rotate_left(27);
    h ^= (z as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use heliobound_core::{Camera, Vec3, VoxelCell, VoxelMaterial};
    use std::collections::HashMap;

    fn test_asset(
        anchor: Vec3,
        voxel_size: f32,
        voxels: &[(VoxelCoord, VoxelMaterial)],
    ) -> RenderAsset {
        let dimensions = voxels.iter().fold([0; 3], |mut dims, (v, _)| {
            dims[0] = dims[0].max(v.x + 1);
            dims[1] = dims[1].max(v.y + 1);
            dims[2] = dims[2].max(v.z + 1);
            dims
        });
        let max = anchor
            + Vec3::new(
                dimensions[0] as f32 * voxel_size,
                dimensions[1] as f32 * voxel_size,
                dimensions[2] as f32 * voxel_size,
            );
        RenderAsset {
            min: anchor,
            max,
            voxels: voxels.iter().copied().collect::<HashMap<_, _>>(),
            dimensions,
            voxel_size,
            pivot: [0.0; 3],
            anchor,
            yaw_degrees: 0,
            ghost: false,
        }
    }

    #[test]
    fn render_asset_broad_bounds_reject_a_miss_before_detail_raycast() {
        let asset = test_asset(
            Vec3::new(10.0, 0.0, 10.0),
            1.0,
            &[(VoxelCoord::new(0, 0, 0), VoxelMaterial::Beacon)],
        );
        let ray = Ray::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(0.0, 0.0, 1.0));
        assert!(raycast_render_assets(&[asset], ray, 100.0).is_none());
    }

    #[test]
    fn render_asset_raycast_hits_detail_after_broad_bounds_hit() {
        let asset = test_asset(
            Vec3::new(0.25, 0.25, 4.25),
            0.5,
            &[(VoxelCoord::new(0, 0, 0), VoxelMaterial::Beacon)],
        );
        let ray = Ray::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(0.0, 0.0, 1.0));
        let hit = raycast_render_assets(&[asset], ray, 100.0).expect("asset detail should hit");
        assert_eq!(hit.material, VoxelMaterial::Beacon);
        assert!((hit.distance - 3.75).abs() < f32::EPSILON);
    }

    #[test]
    fn raycast_hits_first_solid_voxel() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 0, 4),
            VoxelCell::new(VoxelMaterial::Regolith),
        );
        world.set(
            VoxelCoord::new(0, 0, 8),
            VoxelCell::new(VoxelMaterial::Beacon),
        );

        let hit = raycast(
            &world,
            Ray::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(0.0, 0.0, 1.0)),
            20.0,
        )
        .expect("ray should hit first voxel");

        assert_eq!(hit.coord, VoxelCoord::new(0, 0, 4));
        assert_eq!(hit.cell.material, VoxelMaterial::Regolith);
    }

    #[test]
    fn builder_projects_voxels_into_scene() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 0, 8),
            VoxelCell::new(VoxelMaterial::Habitat),
        );
        let camera = Camera::new(Vec3::new(0.5, 0.5, 0.5));
        let builder = SceneBuilder::new(
            GraphicsConfig {
                viewport: Viewport {
                    width: 9,
                    height: 9,
                },
                max_distance: 20.0,
            },
            MaterialGlyphMap,
        );

        let scene = builder.build(&world, &camera, 0);

        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "voxels" && !layer.cells.is_empty()));
    }

    #[test]
    fn face_filtered_builder_keeps_hidden_hits_as_blank_occluders() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 0, 4),
            VoxelCell::new(VoxelMaterial::Habitat),
        );
        world.set(
            VoxelCoord::new(0, 0, 8),
            VoxelCell::new(VoxelMaterial::Beacon),
        );
        let camera = Camera::new(Vec3::new(0.5, 0.5, 0.5));
        let builder = SceneBuilder::new(
            GraphicsConfig {
                viewport: Viewport {
                    width: 9,
                    height: 9,
                },
                max_distance: 20.0,
            },
            MaterialGlyphMap,
        );

        let scene = builder.build_with_visibility(&world, &camera, 0, |_| false);
        let voxels = scene
            .layers
            .iter()
            .find(|layer| layer.name == "voxels")
            .expect("voxel layer exists");
        assert!(!voxels.cells.is_empty());
        assert!(voxels.cells.iter().all(|cell| cell.glyph == ' '));
    }

    #[test]
    fn builder_colors_voxels_by_material() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 0, 8),
            VoxelCell::new(VoxelMaterial::Grass),
        );
        let camera = Camera::new(Vec3::new(0.5, 0.5, 0.5));
        let builder = SceneBuilder::new(
            GraphicsConfig {
                viewport: Viewport {
                    width: 9,
                    height: 9,
                },
                max_distance: 20.0,
            },
            MaterialGlyphMap,
        );

        let scene = builder.build(&world, &camera, 0);
        let voxel = scene
            .layers
            .iter()
            .find(|layer| layer.name == "voxels")
            .and_then(|layer| layer.cells.first())
            .expect("grass voxel should be drawn");

        assert!(voxel
            .style
            .fg
            .as_deref()
            .unwrap_or_default()
            .starts_with('#'));
        assert_ne!(voxel.style.fg, Some("#dfe8db".to_string()));
    }

    #[test]
    fn puzzle_materials_have_distinct_glyphs_and_subdued_idle_colors() {
        let materials = MaterialGlyphMap;
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let glyphs = [
            materials.glyph_for_material(VoxelMaterial::Receiver, normal, 4.0),
            materials.glyph_for_material(VoxelMaterial::SignalPipe, normal, 4.0),
            materials.glyph_for_material(VoxelMaterial::PuzzleDoor, normal, 4.0),
        ];
        assert!(glyphs.iter().all(|glyph| !glyph.is_whitespace()));
        assert_ne!(glyphs[0], glyphs[1]);
        assert_ne!(glyphs[1], glyphs[2]);
        for material in [
            VoxelMaterial::Receiver,
            VoxelMaterial::SignalPipe,
            VoxelMaterial::PuzzleDoor,
        ] {
            let color = material_base_color(material);
            assert!(color.iter().all(|channel| *channel < 0x80));
            assert!(materials.style_for_material(material, normal).fg.is_some());
        }
    }

    #[test]
    fn raycast_skips_rays_that_miss_world_bounds() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 0, 8),
            VoxelCell::new(VoxelMaterial::Habitat),
        );

        let hit = raycast(
            &world,
            Ray::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0)),
            20.0,
        );

        assert_eq!(hit, None);
    }

    #[test]
    fn starfield_is_direction_locked() {
        assert_eq!(
            star_for_direction(Vec3::new(0.1, 0.2, 1.0)),
            star_for_direction(Vec3::new(0.1, 0.2, 1.0))
        );
    }

    #[test]
    fn starfield_is_hidden_at_and_below_the_horizon() {
        let upward = Vec3::new(0.1, 0.2, 1.0);

        assert_eq!(
            background_glyph_for_direction(upward),
            star_for_direction(upward)
        );
        assert_eq!(
            background_glyph_for_direction(Vec3::new(0.1, 0.0, 1.0)),
            ' '
        );
        assert_eq!(
            background_glyph_for_direction(Vec3::new(0.1, -0.2, 1.0)),
            ' '
        );
    }

    #[test]
    fn starfield_uses_coarse_far_distance_cells() {
        let direction_at_yaw = |yaw: f32| Vec3::new(yaw.sin(), 0.0, yaw.cos());

        assert_eq!(
            sky_cell_for_direction(direction_at_yaw(0.2)),
            sky_cell_for_direction(direction_at_yaw(0.21))
        );
        assert_ne!(
            sky_cell_for_direction(direction_at_yaw(0.2)),
            sky_cell_for_direction(direction_at_yaw(0.35))
        );
    }

    #[test]
    fn starfield_uses_camera_direction_not_screen_position() {
        let builder = SceneBuilder::new(
            GraphicsConfig {
                viewport: Viewport {
                    width: 32,
                    height: 18,
                },
                max_distance: 1.0,
            },
            MaterialGlyphMap,
        );
        let world = VoxelWorld::new();
        let a = Camera::new(Vec3::ZERO);
        let b = Camera::new(Vec3::ZERO).looking_at(0.4, 0.0);

        let scene_a = builder.build(&world, &a, 1);
        let scene_b = builder.build(&world, &b, 1);
        let bg_a = scene_a
            .layers
            .iter()
            .find(|layer| layer.name == "background")
            .unwrap();
        let bg_b = scene_b
            .layers
            .iter()
            .find(|layer| layer.name == "background")
            .unwrap();

        assert_ne!(
            bg_a.cells.iter().map(|cell| cell.glyph).collect::<String>(),
            bg_b.cells.iter().map(|cell| cell.glyph).collect::<String>()
        );
    }

    #[test]
    fn planet_glyphs_use_terrain_classes() {
        let materials = MaterialGlyphMap;
        let ocean =
            materials.glyph_for_planet(fake_planet_hit(PlanetTerrainClass::DeepOcean, 0.4, 0.1));
        let mountain =
            materials.glyph_for_planet(fake_planet_hit(PlanetTerrainClass::Mountains, 0.4, 0.9));

        assert_ne!(ocean, mountain);
    }

    #[test]
    fn planet_glyphs_use_fine_detail_within_same_class() {
        let materials = MaterialGlyphMap;
        let low_detail =
            materials.glyph_for_planet(fake_planet_hit(PlanetTerrainClass::Plains, 0.0, 0.1));
        let high_detail =
            materials.glyph_for_planet(fake_planet_hit(PlanetTerrainClass::Plains, 1.0, 0.1));

        assert_ne!(low_detail, high_detail);
    }

    fn fake_planet_hit(terrain: PlanetTerrainClass, detail: f32, ruggedness: f32) -> PlanetHit {
        PlanetHit {
            distance: 100_000.0,
            position: Vec3::new(0.0, 1.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            sample: heliobound_core::PlanetSurfaceSample {
                direction: Vec3::new(0.0, 1.0, 0.0),
                radius: 42_000_000.0,
                elevation: 0.0,
                sea_level: 0.0,
                moisture: 0.5,
                ruggedness,
                detail,
                terrain,
                material: VoxelMaterial::Regolith,
            },
        }
    }
}
