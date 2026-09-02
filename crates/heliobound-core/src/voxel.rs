use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CHUNK_SIZE: i32 = 16;
const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

/// A dense, renderer-facing copy of one sparse world chunk.
///
/// Cells use the same x-fastest layout as the world chunk storage.  This is a
/// deliberately owned snapshot: renderers cannot retain references into the
/// authoritative simulation world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkSnapshot {
    pub coord: ChunkCoord,
    pub revision: u64,
    pub cells: Vec<Option<VoxelCell>>,
}

impl ChunkSnapshot {
    pub const VOLUME: usize = CHUNK_VOLUME;

    pub fn get(&self, local: VoxelCoord) -> Option<VoxelCell> {
        self.cells[local_index(local)]
    }
}

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
    Receiver,
    SignalPipe,
    PuzzleDoor,
    PressurePlate,
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
    /// Kept independently of resident chunks so removing and later restoring
    /// a chunk cannot make a GPU cache mistake new data for an old upload.
    #[serde(default)]
    chunk_revisions: HashMap<ChunkCoord, u64>,
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
        let (was_empty, changed) = {
            let chunk = self.chunks.entry(chunk_coord).or_insert_with(Chunk::new);
            let previous = chunk.get(local);
            chunk.set(local, Some(cell));
            (previous.is_none(), previous != Some(cell))
        };
        if was_empty {
            self.filled_voxels += 1;
        }
        if changed {
            self.bump_chunk_revision(chunk_coord);
        }
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
        let changed = chunk.get(local).is_some();
        if changed {
            self.filled_voxels -= 1;
        }
        chunk.set(local, None);
        if chunk.is_empty() {
            self.chunks.remove(&chunk_coord);
        }
        if changed {
            self.bump_chunk_revision(chunk_coord);
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

    /// The current revision for a chunk. Chunks that have never been changed
    /// report zero; occupied chunks always have a positive revision.
    pub fn chunk_revision(&self, coord: ChunkCoord) -> u64 {
        self.chunk_revisions.get(&coord).copied().unwrap_or(0)
    }

    /// Copies occupied chunks within the inclusive chunk-coordinate range in
    /// deterministic coordinate order. This bounds renderer work without
    /// exposing the sparse hash-map implementation.
    pub fn chunk_snapshots_in(&self, min: ChunkCoord, max: ChunkCoord) -> Vec<ChunkSnapshot> {
        let mut snapshots = self
            .chunks
            .iter()
            .filter(|(coord, _)| {
                (min.x..=max.x).contains(&coord.x)
                    && (min.y..=max.y).contains(&coord.y)
                    && (min.z..=max.z).contains(&coord.z)
            })
            .map(|(&coord, chunk)| ChunkSnapshot {
                coord,
                revision: self.chunk_revision(coord),
                cells: chunk.cells.clone(),
            })
            .collect::<Vec<_>>();
        snapshots.sort_by_key(|snapshot| (snapshot.coord.z, snapshot.coord.y, snapshot.coord.x));
        snapshots
    }

    /// Returns every occupied cell in a stable world-coordinate order.
    ///
    /// This is intentionally a snapshot instead of exposing the chunk storage:
    /// callers such as map export must not inherit the hash-map iteration order
    /// of the sparse world implementation.
    pub fn voxels(&self) -> Vec<(VoxelCoord, VoxelCell)> {
        let mut cells = Vec::with_capacity(self.filled_voxels);
        for (chunk_coord, chunk) in &self.chunks {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let local = VoxelCoord::new(x, y, z);
                        if let Some(cell) = chunk.get(local) {
                            cells.push((
                                VoxelCoord::new(
                                    chunk_coord.x * CHUNK_SIZE + x,
                                    chunk_coord.y * CHUNK_SIZE + y,
                                    chunk_coord.z * CHUNK_SIZE + z,
                                ),
                                cell,
                            ));
                        }
                    }
                }
            }
        }
        cells.sort_by_key(|(coord, _)| (coord.z, coord.y, coord.x));
        cells
    }

    fn bump_chunk_revision(&mut self, coord: ChunkCoord) {
        let revision = self.chunk_revisions.entry(coord).or_insert(0);
        *revision = revision.checked_add(1).expect("chunk revision overflow");
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

    #[test]
    fn snapshots_are_dense_ordered_and_only_change_on_real_mutations() {
        let mut world = VoxelWorld::new();
        let left = VoxelCoord::new(-1, 0, 0);
        let right = VoxelCoord::new(16, 0, 0);
        let basalt = VoxelCell::new(VoxelMaterial::Basalt);

        world.set(left, basalt);
        let left_chunk = ChunkCoord::new(-1, 0, 0);
        assert_eq!(world.chunk_revision(left_chunk), 1);
        world.set(left, basalt);
        assert_eq!(world.chunk_revision(left_chunk), 1);
        world.set(left, VoxelCell::new(VoxelMaterial::Ice));
        assert_eq!(world.chunk_revision(left_chunk), 2);
        world.set(right, basalt);

        let snapshots =
            world.chunk_snapshots_in(ChunkCoord::new(-2, -1, -1), ChunkCoord::new(1, 1, 1));
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].coord, left_chunk);
        assert_eq!(snapshots[0].cells.len(), ChunkSnapshot::VOLUME);
        assert_eq!(
            snapshots[0].get(VoxelCoord::new(15, 0, 0)),
            Some(VoxelCell::new(VoxelMaterial::Ice))
        );

        world.clear(left);
        assert_eq!(world.chunk_revision(left_chunk), 3);
        world.set(left, basalt);
        assert_eq!(world.chunk_revision(left_chunk), 4);
    }
}
