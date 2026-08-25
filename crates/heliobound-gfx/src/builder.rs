use crate::scene::{Layer, Overlay, Scene, SceneCell, TextStyle, Viewport};
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
            VoxelMaterial::CarbonLife => shade(intensity, "vxyY"),
            VoxelMaterial::SiliconLife => shade(intensity, "^*%A"),
            VoxelMaterial::Habitat => shade(intensity, "[]HM"),
            VoxelMaterial::ShipHull => shade(intensity, "<>XZ"),
            VoxelMaterial::Glass => shade(intensity, "'.oO"),
            VoxelMaterial::Beacon => shade(intensity, "i!*@"),
        }
    }
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
                    glyph: star_for_direction(ray.direction),
                    style: TextStyle::default(),
                });
                if let Some(hit) = raycast(
                    world,
                    ray,
                    self.config.max_distance.min(camera.max_distance),
                ) {
                    voxels.cells.push(SceneCell {
                        x: x as i32,
                        y: y as i32,
                        glyph: self.materials.glyph_for(hit),
                        style: TextStyle::default(),
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
                "HELIOBOUND  tick {}  voxels {}  chunks {}  camera {:.1},{:.1},{:.1}",
                tick,
                world.voxel_count(),
                world.chunk_count(),
                camera.position.x,
                camera.position.y,
                camera.position.z
            ),
            style: TextStyle::default(),
        });

        scene
    }

    pub fn build_planet(&self, planet: &ProceduralPlanet, camera: &Camera, tick: u64) -> Scene {
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
                    glyph: star_for_direction(ray.direction),
                    style: TextStyle::default(),
                });
                if let Some(hit) =
                    planet.raycast(ray, self.config.max_distance.min(camera.max_distance))
                {
                    planet_layer.cells.push(SceneCell {
                        x: x as i32,
                        y: y as i32,
                        glyph: self.materials.glyph_for_planet(hit),
                        style: TextStyle::default(),
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
                "HELIOBOUND  tick {}  planet radius {:.0}  camera {:.0},{:.0},{:.0}  roll {:.1}",
                tick,
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

    let start_distance = match world.bounds() {
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

    while distance <= max_distance {
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

fn intersect_bounds(bounds: VoxelBounds, ray: Ray, max_distance: f32) -> Option<f32> {
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
        Some(enter.max(0.0))
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

fn star_for_direction(direction: Vec3) -> char {
    let direction = direction.normalized();
    let x = (direction.x * 4096.0).round() as i32;
    let y = (direction.y * 4096.0).round() as i32;
    let z = (direction.z * 4096.0).round() as i32;
    let hash = hash_sky_cell(x, y, z);

    if hash % 997 == 0 {
        '*'
    } else if hash % 431 == 0 {
        '+'
    } else if hash % 149 == 0 {
        '.'
    } else {
        ' '
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
