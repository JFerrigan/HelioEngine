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
    pub sea_level: f32,
    pub moisture: f32,
    pub ruggedness: f32,
    pub detail: f32,
    pub terrain: PlanetTerrainClass,
    pub material: VoxelMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanetTerrainClass {
    DeepOcean,
    ShallowOcean,
    Coast,
    Plains,
    Hills,
    Mountains,
    IceCap,
    CarbonBloom,
    SiliconField,
    Crater,
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
        let profile = terrain_profile(self.config, direction);
        PlanetSurfaceSample {
            direction,
            radius: self.radius() + profile.elevation.max(profile.sea_level),
            elevation: profile.elevation,
            sea_level: profile.sea_level,
            moisture: profile.moisture,
            ruggedness: profile.ruggedness,
            detail: profile.detail,
            terrain: profile.terrain,
            material: material_at(profile, 0.0),
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
        let profile = terrain_profile(self.config, direction);
        let surface_radius = self.config.radius as f32 + profile.elevation.max(profile.sea_level);
        let crust_depth = self.config.crust_depth.max(1) as f32;

        if distance > surface_radius || distance < surface_radius - crust_depth {
            return None;
        }

        let material = material_at(profile, surface_radius - distance);
        Some(VoxelCell::new(material))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TerrainProfile {
    elevation: f32,
    sea_level: f32,
    moisture: f32,
    ruggedness: f32,
    detail: f32,
    terrain: PlanetTerrainClass,
}

fn terrain_profile(config: PlanetConfig, direction: Vec3) -> TerrainProfile {
    let amplitude = config.terrain_amplitude.max(1.0);
    let continental = fbm(
        direction,
        config.seed,
        &[(2.5, 0.8), (5.0, 0.45), (11.0, 0.22)],
    );
    let hills = fbm(
        direction,
        config.seed ^ 0xA71D,
        &[(18.0, 0.35), (39.0, 0.18)],
    );
    let fine = fbm(
        direction,
        config.seed ^ 0xFACE,
        &[(75.0, 0.09), (161.0, 0.045)],
    );
    let ridges = ridge_noise(direction, config.seed ^ 0x52A3);
    let crater = crater_depression(direction, config.seed);
    let sea_level = -0.18 * amplitude;
    let elevation =
        ((continental - 0.5) * 1.35 + (hills - 0.5) * 0.42 + ridges * 0.52 + (fine - 0.5) * 0.18
            - crater * 0.65)
            * amplitude;
    let moisture = fbm(
        direction,
        config.seed ^ 0xC11A_7E,
        &[(7.0, 0.7), (23.0, 0.3)],
    )
    .clamp(0.0, 1.0);
    let ruggedness = (ridges * 0.75 + (hills - 0.5).abs() * 0.45 + crater * 0.55).clamp(0.0, 1.0);
    let detail = detail_noise(direction, config.seed);
    let terrain = terrain_class(
        config, direction, elevation, sea_level, moisture, ruggedness, crater,
    );

    TerrainProfile {
        elevation,
        sea_level,
        moisture,
        ruggedness,
        detail,
        terrain,
    }
}

fn terrain_class(
    config: PlanetConfig,
    direction: Vec3,
    elevation: f32,
    sea_level: f32,
    moisture: f32,
    ruggedness: f32,
    crater: f32,
) -> PlanetTerrainClass {
    let latitude = direction.y.abs();
    let amplitude = config.terrain_amplitude.max(1.0);
    let relative_height = (elevation - sea_level) / amplitude;
    let life_noise = value_noise(direction, 17.0, config.seed ^ 0xB10F);

    if latitude > 0.74 {
        PlanetTerrainClass::IceCap
    } else if crater > 0.72 {
        PlanetTerrainClass::Crater
    } else if relative_height < -0.18 {
        PlanetTerrainClass::DeepOcean
    } else if relative_height < 0.03 {
        PlanetTerrainClass::ShallowOcean
    } else if relative_height < 0.09 {
        PlanetTerrainClass::Coast
    } else if life_noise > 0.86 && moisture > 0.55 && latitude < 0.62 {
        PlanetTerrainClass::CarbonBloom
    } else if life_noise < 0.08 && relative_height > 0.25 {
        PlanetTerrainClass::SiliconField
    } else if relative_height > 0.58 || ruggedness > 0.72 {
        PlanetTerrainClass::Mountains
    } else if relative_height > 0.22 || ruggedness > 0.38 {
        PlanetTerrainClass::Hills
    } else {
        PlanetTerrainClass::Plains
    }
}

fn material_at(profile: TerrainProfile, depth: f32) -> VoxelMaterial {
    if depth > 1.2 {
        return VoxelMaterial::Basalt;
    }

    match profile.terrain {
        PlanetTerrainClass::DeepOcean | PlanetTerrainClass::ShallowOcean => VoxelMaterial::Ocean,
        PlanetTerrainClass::Coast => VoxelMaterial::Regolith,
        PlanetTerrainClass::IceCap => VoxelMaterial::Ice,
        PlanetTerrainClass::CarbonBloom => VoxelMaterial::CarbonLife,
        PlanetTerrainClass::SiliconField => VoxelMaterial::SiliconLife,
        PlanetTerrainClass::Mountains | PlanetTerrainClass::Crater => VoxelMaterial::Basalt,
        PlanetTerrainClass::Hills | PlanetTerrainClass::Plains => VoxelMaterial::Regolith,
    }
}

fn fbm(direction: Vec3, seed: u64, layers: &[(f32, f32)]) -> f32 {
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (scale, weight) in layers {
        total += (value_noise(direction, *scale, seed ^ scale.to_bits() as u64) - 0.5) * *weight;
        weight_sum += weight;
    }

    if weight_sum <= f32::EPSILON {
        0.5
    } else {
        (total / weight_sum + 0.5).clamp(0.0, 1.0)
    }
}

fn ridge_noise(direction: Vec3, seed: u64) -> f32 {
    let primary = (value_noise(direction, 15.0, seed) * 2.0 - 1.0).abs();
    let secondary = (value_noise(direction, 33.0, seed ^ 0xD4A3) * 2.0 - 1.0).abs();
    (1.0 - primary.min(secondary)).clamp(0.0, 1.0)
}

fn detail_noise(direction: Vec3, seed: u64) -> f32 {
    fbm(
        direction,
        seed ^ 0xD37A_11ED,
        &[(91.0, 0.48), (223.0, 0.31), (487.0, 0.21)],
    )
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
    use std::collections::HashSet;

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
    fn procedural_planet_samples_diverse_surface_classes() {
        let planet = ProceduralPlanet::new(PlanetConfig::default());
        let samples = globe_samples(&planet);
        let classes: HashSet<PlanetTerrainClass> =
            samples.iter().map(|sample| sample.terrain).collect();

        assert!(classes.len() >= 5);
        assert!(
            classes.contains(&PlanetTerrainClass::DeepOcean)
                || classes.contains(&PlanetTerrainClass::ShallowOcean)
        );
        assert!(classes.contains(&PlanetTerrainClass::Mountains));
    }

    #[test]
    fn procedural_planet_detail_changes_across_nearby_surface_points() {
        let planet = ProceduralPlanet::new(PlanetConfig::default());
        let a = planet.sample_surface(Vec3::new(0.120, 0.310, 0.943));
        let b = planet.sample_surface(Vec3::new(0.124, 0.313, 0.941));

        assert_ne!(a.terrain, PlanetTerrainClass::IceCap);
        assert!((a.detail - b.detail).abs() > 0.01);
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

    fn globe_samples(planet: &ProceduralPlanet) -> Vec<PlanetSurfaceSample> {
        let mut samples = Vec::new();
        for lat_step in -8..=8 {
            let y = lat_step as f32 / 8.0;
            let radius = (1.0 - y * y).max(0.0).sqrt();
            for lon_step in 0..32 {
                let angle = lon_step as f32 / 32.0 * std::f32::consts::TAU;
                samples.push(planet.sample_surface(Vec3::new(
                    radius * angle.cos(),
                    y,
                    radius * angle.sin(),
                )));
            }
        }
        samples
    }
}
