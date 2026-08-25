use crate::{VoxelCell, VoxelCoord, VoxelMaterial, VoxelWorld};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CityConfig {
    pub seed: u64,
    pub half_extent: i32,
    pub block_size: i32,
    pub road_width: i32,
    pub max_height: i32,
}

impl Default for CityConfig {
    fn default() -> Self {
        Self {
            seed: 0xC17A_DE1A,
            half_extent: 72,
            block_size: 16,
            road_width: 5,
            max_height: 30,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CityGenerator {
    pub config: CityConfig,
}

impl CityGenerator {
    pub fn new(config: CityConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self) -> VoxelWorld {
        let mut world = VoxelWorld::new();
        self.place_ground(&mut world);
        self.place_buildings(&mut world);
        world
    }

    pub fn is_road(&self, x: i32, z: i32) -> bool {
        let block_size = self.config.block_size.max(4);
        let road_radius = self.config.road_width.max(1) / 2;
        distance_to_grid_line(x, block_size) <= road_radius
            || distance_to_grid_line(z, block_size) <= road_radius
    }

    fn place_ground(&self, world: &mut VoxelWorld) {
        let extent = self.config.half_extent.max(8);
        for z in -extent..=extent {
            for x in -extent..=extent {
                let material = if self.is_road(x, z) {
                    VoxelMaterial::Basalt
                } else {
                    VoxelMaterial::Regolith
                };
                world.set(VoxelCoord::new(x, 0, z), VoxelCell::new(material));
            }
        }
    }

    fn place_buildings(&self, world: &mut VoxelWorld) {
        let extent = self.config.half_extent.max(8);
        let block_size = self.config.block_size.max(8);
        let road_radius = self.config.road_width.max(1) / 2;
        let min_block = (-extent).div_euclid(block_size) - 1;
        let max_block = extent.div_euclid(block_size) + 1;

        for block_z in min_block..=max_block {
            for block_x in min_block..=max_block {
                let min_x = (block_x * block_size + road_radius + 2).max(-extent);
                let max_x = ((block_x + 1) * block_size - road_radius - 3).min(extent);
                let min_z = (block_z * block_size + road_radius + 2).max(-extent);
                let max_z = ((block_z + 1) * block_size - road_radius - 3).min(extent);

                if min_x > max_x || min_z > max_z {
                    continue;
                }

                let footprint = self.footprint(block_x, block_z, min_x, max_x, min_z, max_z);
                self.place_building(world, footprint);
            }
        }
    }

    fn footprint(
        &self,
        block_x: i32,
        block_z: i32,
        min_x: i32,
        max_x: i32,
        min_z: i32,
        max_z: i32,
    ) -> BuildingFootprint {
        let width_limit = (max_x - min_x + 1).max(4);
        let depth_limit = (max_z - min_z + 1).max(4);
        let hash = hash_pair(block_x, block_z, self.config.seed);
        let width = (5 + (hash & 0x5) as i32).min(width_limit);
        let depth = (5 + ((hash >> 3) & 0x5) as i32).min(depth_limit);
        let x_span = (max_x - min_x + 1 - width).max(0);
        let z_span = (max_z - min_z + 1 - depth).max(0);
        let x = min_x + (unit_range(hash >> 11, x_span + 1));
        let z = min_z + (unit_range(hash >> 19, z_span + 1));
        let height = 6 + unit_range(hash >> 27, (self.config.max_height - 5).max(1));

        BuildingFootprint {
            min: VoxelCoord::new(x, 1, z),
            max: VoxelCoord::new(x + width - 1, height, z + depth - 1),
        }
    }

    fn place_building(&self, world: &mut VoxelWorld, footprint: BuildingFootprint) {
        for y in footprint.min.y..=footprint.max.y {
            for z in footprint.min.z..=footprint.max.z {
                for x in footprint.min.x..=footprint.max.x {
                    let on_wall = x == footprint.min.x
                        || x == footprint.max.x
                        || z == footprint.min.z
                        || z == footprint.max.z;
                    let on_roof = y == footprint.max.y;

                    if !on_wall && !on_roof {
                        continue;
                    }

                    let material = if on_roof {
                        VoxelMaterial::ShipHull
                    } else if y > 2 && (x + y + z).rem_euclid(4) == 0 {
                        VoxelMaterial::Glass
                    } else {
                        VoxelMaterial::Habitat
                    };
                    world.set(VoxelCoord::new(x, y, z), VoxelCell::new(material));
                }
            }
        }

        let center_x = (footprint.min.x + footprint.max.x) / 2;
        let center_z = (footprint.min.z + footprint.max.z) / 2;
        world.set(
            VoxelCoord::new(center_x, footprint.max.y + 1, center_z),
            VoxelCell::new(VoxelMaterial::Beacon),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BuildingFootprint {
    min: VoxelCoord,
    max: VoxelCoord,
}

fn distance_to_grid_line(coord: i32, spacing: i32) -> i32 {
    let local = coord.rem_euclid(spacing);
    local.min(spacing - local)
}

fn unit_range(value: u64, upper_exclusive: i32) -> i32 {
    if upper_exclusive <= 1 {
        0
    } else {
        (value % upper_exclusive as u64) as i32
    }
}

fn hash_pair(x: i32, z: i32, seed: u64) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    h ^= (x as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
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

    #[test]
    fn city_generation_is_deterministic() {
        let config = CityConfig::default();

        let a = CityGenerator::new(config).generate();
        let b = CityGenerator::new(config).generate();

        assert_eq!(a.voxel_count(), b.voxel_count());
        assert_eq!(a.bounds(), b.bounds());
    }

    #[test]
    fn road_grid_uses_basalt_at_major_crossings() {
        let generator = CityGenerator::new(CityConfig::default());
        let city = generator.generate();

        assert!(generator.is_road(0, 0));
        assert_eq!(
            city.get(VoxelCoord::new(0, 0, 0)),
            Some(VoxelCell::new(VoxelMaterial::Basalt))
        );
    }

    #[test]
    fn city_contains_buildings_above_ground() {
        let config = CityConfig::default();
        let city = CityGenerator::new(config).generate();
        let ground_cells = (config.half_extent * 2 + 1).pow(2) as usize;
        let bounds = city.bounds().expect("generated city should have bounds");

        assert!(city.voxel_count() > ground_cells);
        assert!(bounds.max.y > 8);
    }
}
