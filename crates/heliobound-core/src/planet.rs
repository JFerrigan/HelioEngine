use crate::{Ray, Vec3, VoxelCell, VoxelCoord, VoxelMaterial, VoxelWorld};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetConfig {
    pub seed: u64,
    pub radius: i32,
    pub crust_depth: i32,
    pub terrain_amplitude: f32,
}

impl Default for PlanetConfig {
    fn default() -> Self {
        Self {
            seed: 0xA11C_E5EED,
            radius: 42_000_000,
            crust_depth: 3,
            terrain_amplitude: 5_000_000.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetSurfaceSample {
    pub direction: Vec3,
    pub radius: f32,
    pub elevation: f32,
    pub material: VoxelMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetHit {
    pub distance: f32,
    pub position: Vec3,
    pub normal: Vec3,
    pub sample: PlanetSurfaceSample,
}

#[derive(Clone, Copy, Debug)]
pub struct ProceduralPlanet {
    pub config: PlanetConfig,
}

impl ProceduralPlanet {
    pub fn new(config: PlanetConfig) -> Self {
        Self { config }
    }

    pub fn radius(&self) -> f32 {
        self.config.radius as f32
    }

    pub fn outer_radius(&self) -> f32 {
        self.radius() + self.config.terrain_amplitude.abs()
    }

    pub fn sample_surface(&self, direction: Vec3) -> PlanetSurfaceSample {
        let direction = direction.normalized();
        let elevation = elevation(self.config, direction);
        PlanetSurfaceSample {
            direction,
            radius: self.radius() + elevation,
            elevation,
            material: material_at(self.config, direction, elevation, 0.0),
        }
    }

    pub fn raycast(&self, ray: Ray, max_distance: f32) -> Option<PlanetHit> {
        let distance = intersect_sphere(ray, self.outer_radius(), max_distance)?;
        let position = ray.point_at(distance);
        let normal = position.normalized();
        let sample = self.sample_surface(normal);

        Some(PlanetHit {
            distance,
            position: normal * sample.radius,
            normal,
            sample,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlanetGenerator {
    pub config: PlanetConfig,
}

impl PlanetGenerator {
    pub fn new(config: PlanetConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self) -> VoxelWorld {
        assert!(
            self.config.radius <= 256,
            "voxel shell generation is for local bodies only; use ProceduralPlanet for planet-scale rendering"
        );
        let mut world = VoxelWorld::new();
        let outer_limit =
            (self.config.radius as f32 + self.config.terrain_amplitude.abs()).ceil() as i32 + 1;

        for z in -outer_limit..=outer_limit {
            for y in -outer_limit..=outer_limit {
                for x in -outer_limit..=outer_limit {
                    let coord = VoxelCoord::new(x, y, z);
                    if let Some(cell) = self.cell_at(coord) {
                        world.set(coord, cell);
                    }
                }
            }
        }

        world
    }

    pub fn cell_at(&self, coord: VoxelCoord) -> Option<VoxelCell> {
        let pos = Vec3::new(coord.x as f32, coord.y as f32, coord.z as f32);
        let distance = pos.length();
        if distance <= f32::EPSILON {
            return None;
        }

        let direction = pos / distance;
        let elevation = elevation(self.config, direction);
        let surface_radius = self.config.radius as f32 + elevation;
        let crust_depth = self.config.crust_depth.max(1) as f32;

        if distance > surface_radius || distance < surface_radius - crust_depth {
            return None;
        }

        let material = material_at(self.config, direction, elevation, surface_radius - distance);
        Some(VoxelCell::new(material))
    }
}

fn elevation(config: PlanetConfig, direction: Vec3) -> f32 {
    let continental = value_noise(direction, 5.0, config.seed);
    let ridges = (value_noise(direction, 14.0, config.seed ^ 0x52A3) * 2.0 - 1.0).abs();
    let crater = crater_depression(direction, config.seed);
    ((continental - 0.5) * 1.4 + ridges * 0.45 - crater) * config.terrain_amplitude
}

fn material_at(config: PlanetConfig, direction: Vec3, elevation: f32, depth: f32) -> VoxelMaterial {
    if depth > 1.2 {
        return VoxelMaterial::Basalt;
    }

    let latitude = direction.y.abs();
    let climate_noise = value_noise(direction, 9.0, config.seed ^ 0xC11A_7E);
    let life_noise = value_noise(direction, 17.0, config.seed ^ 0xB10F);
    let scaled_elevation = elevation / config.terrain_amplitude.max(1.0);

    if latitude > 0.72 || (scaled_elevation < -0.3 && climate_noise > 0.62) {
        VoxelMaterial::Ice
    } else if life_noise > 0.86 && latitude < 0.62 {
        VoxelMaterial::CarbonLife
    } else if life_noise < 0.08 && scaled_elevation > 0.2 {
        VoxelMaterial::SiliconLife
    } else if scaled_elevation > 0.5 {
        VoxelMaterial::Basalt
    } else {
        VoxelMaterial::Regolith
    }
}

fn intersect_sphere(ray: Ray, radius: f32, max_distance: f32) -> Option<f32> {
    let b = ray.origin.dot(ray.direction);
    let c = ray.origin.dot(ray.origin) - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }

    let root = discriminant.sqrt();
    let near = -b - root;
    let far = -b + root;
    let distance = if near >= 0.0 { near } else { far };

    if distance >= 0.0 && distance <= max_distance {
        Some(distance)
    } else {
        None
    }
}

fn value_noise(direction: Vec3, scale: f32, seed: u64) -> f32 {
    let x = (direction.x * scale).floor() as i32;
    let y = (direction.y * scale).floor() as i32;
    let z = (direction.z * scale).floor() as i32;
    unit_hash(hash_coord(x, y, z, seed))
}

fn crater_depression(direction: Vec3, seed: u64) -> f32 {
    let mut depression = 0.0;
    for index in 0..9 {
        let center = crater_center(seed, index);
        let alignment = direction.dot(center);
        if alignment > 0.965 {
            let size = 0.035 + unit_hash(hash_coord(index, 17, 43, seed)) * 0.03;
            depression += ((alignment - 0.965) / size).clamp(0.0, 1.0);
        }
    }
    depression.min(1.6)
}

fn crater_center(seed: u64, index: i32) -> Vec3 {
    let a = unit_hash(hash_coord(index, 1, 9, seed)) * std::f32::consts::TAU;
    let y = unit_hash(hash_coord(index, 2, 11, seed)) * 2.0 - 1.0;
    let r = (1.0 - y * y).sqrt();
    Vec3::new(r * a.cos(), y, r * a.sin()).normalized()
}

fn hash_coord(x: i32, y: i32, z: i32, seed: u64) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    h ^= (x as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = h.rotate_left(27);
    h ^= (y as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    h = h.rotate_left(31);
    h ^= (z as i64 as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn unit_hash(value: u64) -> f32 {
    ((value >> 40) as f32) / ((1_u64 << 24) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_planet_is_deterministic() {
        let config = PlanetConfig {
            seed: 123,
            radius: 12,
            crust_depth: 2,
            terrain_amplitude: 2.0,
        };

        let a = PlanetGenerator::new(config).generate();
        let b = PlanetGenerator::new(config).generate();

        assert_eq!(a.voxel_count(), b.voxel_count());
        assert_eq!(a.bounds(), b.bounds());
    }

    #[test]
    fn generated_planet_stores_shell_not_full_volume() {
        let config = PlanetConfig {
            seed: 456,
            radius: 14,
            crust_depth: 2,
            terrain_amplitude: 2.0,
        };

        let world = PlanetGenerator::new(config).generate();
        let full_cube = (config.radius * 2 + 1).pow(3) as usize;

        assert!(world.voxel_count() > 1_000);
        assert!(world.voxel_count() < full_cube / 2);
    }

    #[test]
    fn generated_planet_has_global_bounds() {
        let config = PlanetConfig {
            seed: 789,
            radius: 10,
            crust_depth: 2,
            terrain_amplitude: 1.0,
        };

        let world = PlanetGenerator::new(config).generate();
        let bounds = world.bounds().expect("planet should contain voxels");

        assert!(bounds.min.x < -8);
        assert!(bounds.max.x > 8);
        assert!(bounds.min.y < -8);
        assert!(bounds.max.y > 8);
        assert!(bounds.min.z < -8);
        assert!(bounds.max.z > 8);
    }

    #[test]
    fn procedural_planet_supports_large_scale_without_generation() {
        let planet = ProceduralPlanet::new(PlanetConfig::default());

        assert_eq!(planet.radius(), 42_000_000.0);
        assert!(planet.outer_radius() > planet.radius());

        let sample = planet.sample_surface(Vec3::new(0.0, 1.0, 0.0));
        assert!(sample.radius > 35_000_000.0);
    }

    #[test]
    fn procedural_planet_raycast_hits_large_body() {
        let planet = ProceduralPlanet::new(PlanetConfig::default());
        let hit = planet
            .raycast(
                Ray::new(
                    Vec3::new(0.0, 0.0, -125_000_000.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                200_000_000.0,
            )
            .expect("ray should hit large planet");

        assert!(hit.distance > 70_000_000.0);
        assert!(hit.distance < 90_000_000.0);
    }
}
