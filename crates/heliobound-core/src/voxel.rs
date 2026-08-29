use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CHUNK_SIZE: i32 = 16;
const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VoxelCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl VoxelCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoxelMaterial {
    Regolith,
    Basalt,
    Ocean,
    Ice,
    Grass,
    Dirt,
    Stone,
    Sand,
    Wood,
    Leaves,
    Zombie,
    CornStalk,
    CarbonLife,
    SiliconLife,
    Habitat,
    ShipHull,
    Glass,
    Beacon,
    Gate,
    /// A color supplied by an imported voxel asset's palette.
    Custom([u8; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelCell {
    pub material: VoxelMaterial,
}

impl VoxelCell {
    pub const fn new(material: VoxelMaterial) -> Self {
        Self { material }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelBounds {
    pub min: VoxelCoord,
    pub max: VoxelCoord,
}

impl VoxelBounds {
    pub const fn new(coord: VoxelCoord) -> Self {
        Self {
            min: coord,
            max: coord,
        }
    }

    pub fn include(&mut self, coord: VoxelCoord) {
        self.min.x = self.min.x.min(coord.x);
        self.min.y = self.min.y.min(coord.y);
        self.min.z = self.min.z.min(coord.z);
        self.max.x = self.max.x.max(coord.x);
        self.max.y = self.max.y.max(coord.y);
        self.max.z = self.max.z.max(coord.z);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Chunk {
    cells: Vec<Option<VoxelCell>>,
}

impl Chunk {
    fn new() -> Self {
        Self {
            cells: vec![None; CHUNK_VOLUME],
        }
    }

    fn get(&self, local: VoxelCoord) -> Option<VoxelCell> {
        self.cells[local_index(local)]
    }

    fn set(&mut self, local: VoxelCoord, cell: Option<VoxelCell>) {
        self.cells[local_index(local)] = cell;
    }

    fn is_empty(&self) -> bool {
        self.cells.iter().all(Option::is_none)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VoxelWorld {
    chunks: HashMap<ChunkCoord, Chunk>,
    filled_voxels: usize,
    bounds: Option<VoxelBounds>,
}

impl VoxelWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, coord: VoxelCoord) -> Option<VoxelCell> {
        let (chunk, local) = split_coord(coord);
        self.chunks.get(&chunk).and_then(|chunk| chunk.get(local))
    }

    pub fn set(&mut self, coord: VoxelCoord, cell: VoxelCell) {
        let (chunk_coord, local) = split_coord(coord);
        let chunk = self.chunks.entry(chunk_coord).or_insert_with(Chunk::new);
        if chunk.get(local).is_none() {
            self.filled_voxels += 1;
        }
        chunk.set(local, Some(cell));
        if let Some(bounds) = &mut self.bounds {
            bounds.include(coord);
        } else {
            self.bounds = Some(VoxelBounds::new(coord));
        }
    }

    pub fn clear(&mut self, coord: VoxelCoord) {
        let (chunk_coord, local) = split_coord(coord);
        let Some(chunk) = self.chunks.get_mut(&chunk_coord) else {
            return;
        };
        if chunk.get(local).is_some() {
            self.filled_voxels -= 1;
        }
        chunk.set(local, None);
        if chunk.is_empty() {
            self.chunks.remove(&chunk_coord);
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn voxel_count(&self) -> usize {
        self.filled_voxels
    }

    pub fn bounds(&self) -> Option<VoxelBounds> {
        self.bounds
    }
}

fn split_coord(coord: VoxelCoord) -> (ChunkCoord, VoxelCoord) {
    let chunk = ChunkCoord::new(
        coord.x.div_euclid(CHUNK_SIZE),
        coord.y.div_euclid(CHUNK_SIZE),
        coord.z.div_euclid(CHUNK_SIZE),
    );
    let local = VoxelCoord::new(
        coord.x.rem_euclid(CHUNK_SIZE),
        coord.y.rem_euclid(CHUNK_SIZE),
        coord.z.rem_euclid(CHUNK_SIZE),
    );
    (chunk, local)
}

fn local_index(local: VoxelCoord) -> usize {
    debug_assert!((0..CHUNK_SIZE).contains(&local.x));
    debug_assert!((0..CHUNK_SIZE).contains(&local.y));
    debug_assert!((0..CHUNK_SIZE).contains(&local.z));
    (local.x + local.y * CHUNK_SIZE + local.z * CHUNK_SIZE * CHUNK_SIZE) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_voxels_across_negative_chunk_boundaries() {
        let mut world = VoxelWorld::new();
        let coord = VoxelCoord::new(-1, -2, 17);
        let cell = VoxelCell::new(VoxelMaterial::Ice);

        world.set(coord, cell);

        assert_eq!(world.get(coord), Some(cell));
        assert_eq!(world.chunk_count(), 1);
        assert_eq!(world.voxel_count(), 1);
    }

    #[test]
    fn clearing_last_voxel_removes_empty_chunk() {
        let mut world = VoxelWorld::new();
        let coord = VoxelCoord::new(0, 0, 0);

        world.set(coord, VoxelCell::new(VoxelMaterial::Basalt));
        world.clear(coord);

        assert_eq!(world.get(coord), None);
        assert_eq!(world.chunk_count(), 0);
        assert_eq!(world.voxel_count(), 0);
    }

    #[test]
    fn bounds_expand_as_voxels_are_inserted() {
        let mut world = VoxelWorld::new();

        world.set(
            VoxelCoord::new(4, -2, 7),
            VoxelCell::new(VoxelMaterial::Basalt),
        );
        world.set(
            VoxelCoord::new(-8, 5, 2),
            VoxelCell::new(VoxelMaterial::Ice),
        );

        assert_eq!(
            world.bounds(),
            Some(VoxelBounds {
                min: VoxelCoord::new(-8, -2, 2),
                max: VoxelCoord::new(4, 5, 7),
            })
        );
    }
}
