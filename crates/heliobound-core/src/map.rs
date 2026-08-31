//! Strict, startup-loadable v1 map blueprints.  The catalog deliberately owns
//! no mutable game state: callers clone `CompiledMap::world` for a session.
use crate::{
    CityConfig, CityGenerator, Vec3, VoxelBounds, VoxelCell, VoxelCoord, VoxelMaterial, VoxelWorld,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const MAX_FILE: u64 = 8 * 1024 * 1024;
const MAX_BOX: i64 = 1_000_000;
const MAX_OCCUPIED: usize = 4_000_000;

#[derive(Clone, Debug)]
pub struct MapLoadError {
    pub path: PathBuf,
    pub message: String,
}
impl std::fmt::Display for MapLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

#[derive(Clone, Debug)]
pub struct AssetDefinition {
    pub id: String,
    pub name: String,
    pub voxel_size: f32,
    pub pivot: [f32; 3],
    pub voxels: Vec<(VoxelCoord, VoxelMaterial)>,
}
#[derive(Clone, Debug, Default)]
pub struct AssetCatalog {
    pub assets: BTreeMap<String, AssetDefinition>,
    pub errors: Vec<MapLoadError>,
}
impl AssetCatalog {
    pub fn discover(directory: impl AsRef<Path>) -> Self {
        let directory = directory.as_ref();
        let mut out = Self::default();
        let Ok(read) = fs::read_dir(directory) else {
            return out;
        };
        let mut paths: Vec<_> = read
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|x| x.to_str())
                    .is_some_and(|n| n.ends_with(".hbasset.json"))
            })
            .collect();
        paths.sort();
        for path in paths {
            match Self::load(&path) {
                Ok(asset) => {
                    if out.assets.insert(asset.id.clone(), asset).is_some() {
                        out.errors.push(err(&path, "duplicate asset id"));
                    }
                }
                Err(message) => out.errors.push(err(&path, message)),
            }
        }
        out
    }
    fn load(path: &Path) -> Result<AssetDefinition, String> {
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_FILE {
            return Err("file exceeds 8 MiB limit".into());
        }
        let raw: AssetFile =
            serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if raw.format_version != 1 || raw.id.trim().is_empty() || raw.name.trim().is_empty() {
            return Err("format_version must be 1 and id/name must be non-empty".into());
        }
        let stem = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".hbasset.json"))
            .unwrap_or("");
        if raw.id != stem {
            return Err("id must match filename".into());
        }
        if ![1.0, 0.5, 0.25, 0.125].contains(&raw.voxel_size)
            || raw.dimensions.iter().any(|d| !(1..=128).contains(d))
        {
            return Err("invalid voxel_size or dimensions".into());
        }
        let pivot = raw.pivot.unwrap_or([
            raw.dimensions[0] as f32 / 2.,
            0.,
            raw.dimensions[2] as f32 / 2.,
        ]);
        if pivot.iter().any(|v| !v.is_finite()) {
            return Err("pivot must be finite".into());
        }
        if raw.layers.len() != raw.dimensions[1] as usize || raw.palette.is_empty() {
            return Err("invalid layers or palette".into());
        }
        let mut palette = HashMap::new();
        for (symbol, color) in raw.palette {
            let mut ch = symbol.chars();
            let Some(c) = ch.next() else {
                return Err("invalid palette symbol".into());
            };
            if ch.next().is_some() || !c.is_ascii_alphanumeric() {
                return Err("invalid palette symbol".into());
            };
            palette.insert(c, color_hex(&color)?);
        }
        let mut voxels = Vec::new();
        for (y, layer) in raw.layers.iter().enumerate() {
            if layer.len() != raw.dimensions[2] as usize {
                return Err("incorrect layer rows".into());
            };
            for (z, row) in layer.iter().enumerate() {
                if row.chars().count() != raw.dimensions[0] as usize {
                    return Err("incorrect row width".into());
                };
                for (x, c) in row.chars().enumerate() {
                    if c != '.' {
                        let color = *palette
                            .get(&c)
                            .ok_or_else(|| "undefined palette symbol".to_owned())?;
                        voxels.push((
                            VoxelCoord::new(x as i32, y as i32, z as i32),
                            VoxelMaterial::Custom(color),
                        ));
                    }
                }
            }
        }
        if voxels.is_empty() {
            return Err("asset must contain voxels".into());
        };
        Ok(AssetDefinition {
            id: raw.id,
            name: raw.name,
            voxel_size: raw.voxel_size,
            pivot,
            voxels,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetFile {
    format_version: u32,
    id: String,
    name: String,
    voxel_size: f32,
    dimensions: [i32; 3],
    #[serde(default)]
    pivot: Option<[f32; 3]>,
    palette: HashMap<String, String>,
    layers: Vec<Vec<String>>,
}
fn color_hex(s: &str) -> Result<[u8; 3], String> {
    if s.len() != 7 || !s.starts_with('#') {
        return Err("invalid palette color".into());
    };
    Ok([
        u8::from_str_radix(&s[1..3], 16).map_err(|_| "invalid palette color")?,
        u8::from_str_radix(&s[3..5], 16).map_err(|_| "invalid palette color")?,
        u8::from_str_radix(&s[5..7], 16).map_err(|_| "invalid palette color")?,
    ])
}

#[derive(Clone, Debug)]
pub struct MapMetadata {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub category: String,
    pub bounds: VoxelBounds,
}
#[derive(Clone, Debug)]
pub struct PlacedAsset {
    pub id: String,
    pub asset_id: String,
    pub position: VoxelCoord,
    pub yaw_degrees: u16,
    pub voxel_size: f32,
}
#[derive(Clone, Debug)]
pub enum MapOperation {
    FillBox {
        min: VoxelCoord,
        max: VoxelCoord,
        material: VoxelMaterial,
    },
    ClearBox {
        min: VoxelCoord,
        max: VoxelCoord,
    },
    PlaceAsset(PlacedAsset),
    GeneratorRegion {
        id: String,
        generator: String,
    },
}
#[derive(Clone, Debug)]
pub enum MapMarker {
    PlayerSpawn {
        id: String,
        position: Vec3,
        yaw_degrees: u16,
    },
    Exit {
        id: String,
        position: Vec3,
        target: String,
    },
    EnemySpawn {
        id: String,
        position: Vec3,
        enemy_type: String,
        spawn_group: String,
    },
    Pickup {
        id: String,
        position: Vec3,
        pickup_type: String,
        amount: i32,
    },
    InteractableDoor {
        id: String,
        position: Vec3,
        door_type: String,
        closed_bounds: VoxelBounds,
        open_cost: i32,
    },
    WallWeapon {
        id: String,
        position: Vec3,
        weapon_type: String,
        cost: i32,
        yaw_degrees: u16,
    },
    LiminalObjective {
        id: String,
        position: Vec3,
        objective_type: String,
        room_id: String,
    },
    EchoReceiver {
        id: String,
        position: Vec3,
        output_seconds: f32,
        puzzle_id: String,
    },
    EchoPipe {
        id: String,
        position: Vec3,
        puzzle_id: String,
        sequence: i32,
    },
    EchoDoor {
        id: String,
        position: Vec3,
        puzzle_id: String,
        closed_bounds: VoxelBounds,
        normal: VoxelCoord,
        starting_side_anchor: Vec3,
        far_side_anchor: Vec3,
    },
    LiminalRoom {
        id: String,
        position: Vec3,
        room_id: String,
        bounds: VoxelBounds,
        room_type: String,
        sign: String,
    },
    LiminalConnection {
        id: String,
        position: Vec3,
        from_room: String,
        to_room: String,
    },
}
impl MapMarker {
    pub fn id(&self) -> &str {
        match self {
            Self::PlayerSpawn { id, .. }
            | Self::Exit { id, .. }
            | Self::EnemySpawn { id, .. }
            | Self::Pickup { id, .. }
            | Self::InteractableDoor { id, .. }
            | Self::WallWeapon { id, .. }
            | Self::LiminalObjective { id, .. }
            | Self::EchoReceiver { id, .. }
            | Self::EchoPipe { id, .. }
            | Self::EchoDoor { id, .. }
            | Self::LiminalRoom { id, .. }
            | Self::LiminalConnection { id, .. } => id,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CompiledMap {
    pub metadata: MapMetadata,
    pub world: VoxelWorld,
    pub player_start: MapMarker,
    pub markers: Vec<MapMarker>,
    pub operations: Vec<MapOperation>,
    pub placed_assets: Vec<PlacedAsset>,
    pub source: PathBuf,
}
#[derive(Clone, Debug)]
pub struct MapSession {
    pub world: VoxelWorld,
    pub player_start: PlayerStart,
    pub markers: Vec<MapMarker>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerStart {
    pub position: Vec3,
    pub yaw_degrees: u16,
}
impl CompiledMap {
    pub fn fresh_session(&self) -> MapSession {
        let MapMarker::PlayerSpawn {
            position,
            yaw_degrees,
            ..
        } = self.player_start
        else {
            unreachable!("compiled map player start")
        };
        MapSession {
            world: self.world.clone(),
            player_start: PlayerStart {
                position,
                yaw_degrees,
            },
            markers: self.markers.clone(),
        }
    }
    pub fn zombies(&self) -> Result<ZombiesMap, String> {
        ZombiesMap::from_markers(&self.metadata.mode, &self.markers)
    }
    pub fn liminal(&self) -> Result<LiminalMap, String> {
        LiminalMap::from_markers(&self.metadata.mode, &self.markers)
    }
    pub fn echolocation(&self) -> Result<EcholocationMap, String> {
        EcholocationMap::from_markers(&self.metadata.mode, &self.markers)
    }
}
#[derive(Clone, Debug)]
pub struct ZombiesMap {
    pub doors: Vec<MapMarker>,
    pub spawns: Vec<MapMarker>,
    pub wall_weapons: Vec<MapMarker>,
}
impl ZombiesMap {
    fn from_markers(mode: &str, markers: &[MapMarker]) -> Result<Self, String> {
        if mode != "zombies" {
            return Err("not a zombies map".into());
        };
        let out = Self {
            doors: markers
                .iter()
                .filter(|m| matches!(m, MapMarker::InteractableDoor { .. }))
                .cloned()
                .collect(),
            spawns: markers
                .iter()
                .filter(|m| matches!(m, MapMarker::EnemySpawn { .. }))
                .cloned()
                .collect(),
            wall_weapons: markers
                .iter()
                .filter(|m| matches!(m, MapMarker::WallWeapon { .. }))
                .cloned()
                .collect(),
        };
        if out.doors.is_empty() || out.spawns.is_empty() || out.wall_weapons.is_empty() {
            Err("zombies map requires doors, enemy spawns, and wall weapons".into())
        } else {
            Ok(out)
        }
    }
}
#[derive(Clone, Debug)]
pub struct LiminalMap {
    pub rooms: Vec<MapMarker>,
    pub connections: Vec<MapMarker>,
    pub objective: MapMarker,
}
impl LiminalMap {
    fn from_markers(mode: &str, markers: &[MapMarker]) -> Result<Self, String> {
        if mode != "liminal" {
            return Err("not a liminal map".into());
        };
        let rooms: Vec<_> = markers
            .iter()
            .filter(|m| matches!(m, MapMarker::LiminalRoom { .. }))
            .cloned()
            .collect();
        let connections: Vec<_> = markers
            .iter()
            .filter(|m| matches!(m, MapMarker::LiminalConnection { .. }))
            .cloned()
            .collect();
        let objective = markers
            .iter()
            .find(|m| matches!(m, MapMarker::LiminalObjective { .. }))
            .cloned()
            .ok_or("liminal map requires an objective")?;
        if rooms.is_empty() {
            Err("liminal map requires rooms".into())
        } else {
            Ok(Self {
                rooms,
                connections,
                objective,
            })
        }
    }
}
#[derive(Clone, Debug)]
pub struct EcholocationMap {
    pub receiver: MapMarker,
    pub pipes: Vec<MapMarker>,
    pub doors: Vec<MapMarker>,
}
impl EcholocationMap {
    fn from_markers(mode: &str, markers: &[MapMarker]) -> Result<Self, String> {
        if mode != "echolocation" {
            return Err("not an echolocation map".into());
        };
        let receiver = markers
            .iter()
            .find(|m| matches!(m, MapMarker::EchoReceiver { .. }))
            .cloned()
            .ok_or("echolocation map requires a receiver")?;
        let mut pipes: Vec<_> = markers
            .iter()
            .filter(|m| matches!(m, MapMarker::EchoPipe { .. }))
            .cloned()
            .collect();
        pipes.sort_by_key(|m| {
            if let MapMarker::EchoPipe { sequence, .. } = m {
                *sequence
            } else {
                0
            }
        });
        let doors: Vec<_> = markers
            .iter()
            .filter(|m| matches!(m, MapMarker::EchoDoor { .. }))
            .cloned()
            .collect();
        if pipes.is_empty() || doors.is_empty() {
            Err("echolocation map requires pipes and doors".into())
        } else {
            Ok(Self {
                receiver,
                pipes,
                doors,
            })
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct MapCatalog {
    pub maps: Vec<CompiledMap>,
    pub errors: Vec<MapLoadError>,
}
impl MapCatalog {
    pub fn discover(directory: impl AsRef<Path>, assets: &AssetCatalog) -> Self {
        let directory = directory.as_ref();
        let mut out = Self::default();
        let Ok(read) = fs::read_dir(directory) else {
            return out;
        };
        let mut paths: Vec<_> = read
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|x| x.to_str())
                    .is_some_and(|n| n.ends_with(".hbmap.json"))
            })
            .collect();
        paths.sort();
        let mut ids = HashSet::new();
        for path in paths {
            match compile_file(&path, assets) {
                Ok(map) => {
                    if !ids.insert(map.metadata.id.clone()) {
                        out.errors.push(err(&path, "duplicate map id"))
                    } else {
                        out.maps.push(map)
                    }
                }
                Err(message) => out.errors.push(err(&path, message)),
            }
        }
        out
    }
    pub fn get(&self, id: &str) -> Option<&CompiledMap> {
        self.maps.iter().find(|m| m.metadata.id == id)
    }
}
fn err(path: &Path, message: impl Into<String>) -> MapLoadError {
    MapLoadError {
        path: path.to_owned(),
        message: message.into(),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMap {
    format_version: u32,
    id: String,
    name: String,
    mode: String,
    category: String,
    bounds: RawBounds,
    player_start: RawMarker,
    operations: Vec<RawOperation>,
    markers: Vec<RawMarker>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBounds {
    min: [i32; 3],
    max: [i32; 3],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CityGeneratorParameters {
    road_width: i32,
    block_size: i32,
    max_height: i32,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RawOperation {
    FillBox {
        min: [i32; 3],
        max: [i32; 3],
        material: String,
    },
    ClearBox {
        min: [i32; 3],
        max: [i32; 3],
    },
    PlaceAsset {
        id: String,
        asset_id: String,
        position: [i32; 3],
        yaw_degrees: u16,
    },
    GeneratorRegion {
        id: String,
        generator: String,
        bounds: RawBounds,
        seed: u64,
        parameters: serde_json::Value,
    },
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RawMarker {
    PlayerSpawn {
        id: String,
        position: [f32; 3],
        yaw_degrees: u16,
    },
    Exit {
        id: String,
        position: [f32; 3],
        target: String,
    },
    EnemySpawn {
        id: String,
        position: [f32; 3],
        enemy_type: String,
        spawn_group: String,
    },
    Pickup {
        id: String,
        position: [f32; 3],
        pickup_type: String,
        amount: i32,
    },
    InteractableDoor {
        id: String,
        position: [f32; 3],
        door_type: String,
        closed_bounds: RawBounds,
        open_cost: i32,
    },
    WallWeapon {
        id: String,
        position: [f32; 3],
        weapon_type: String,
        cost: i32,
        yaw_degrees: u16,
    },
    LiminalObjective {
        id: String,
        position: [f32; 3],
        objective_type: String,
        room_id: String,
    },
    EchoReceiver {
        id: String,
        position: [f32; 3],
        output_seconds: f32,
        puzzle_id: String,
    },
    EchoPipe {
        id: String,
        position: [f32; 3],
        puzzle_id: String,
        sequence: i32,
    },
    EchoDoor {
        id: String,
        position: [f32; 3],
        puzzle_id: String,
        closed_bounds: RawBounds,
        normal: [i32; 3],
        starting_side_anchor: [f32; 3],
        far_side_anchor: [f32; 3],
    },
    LiminalRoom {
        id: String,
        position: [f32; 3],
        room_id: String,
        bounds: RawBounds,
        room_type: String,
        sign: String,
    },
    LiminalConnection {
        id: String,
        position: [f32; 3],
        from_room: String,
        to_room: String,
    },
}
fn coord(v: [i32; 3]) -> VoxelCoord {
    VoxelCoord::new(v[0], v[1], v[2])
}
fn bounds(b: RawBounds) -> Result<VoxelBounds, String> {
    let b = VoxelBounds {
        min: coord(b.min),
        max: coord(b.max),
    };
    if b.min.x > b.max.x || b.min.y > b.max.y || b.min.z > b.max.z {
        Err("invalid bounds".into())
    } else {
        Ok(b)
    }
}
fn pos(v: [f32; 3]) -> Result<Vec3, String> {
    if v.iter().all(|x| x.is_finite()) {
        Ok(Vec3::new(v[0], v[1], v[2]))
    } else {
        Err("position must be finite".into())
    }
}
fn yaw(v: u16) -> Result<u16, String> {
    if [0, 90, 180, 270].contains(&v) {
        Ok(v)
    } else {
        Err("yaw_degrees must be a quarter turn".into())
    }
}
fn material(s: &str) -> Result<VoxelMaterial, String> {
    use VoxelMaterial::*;
    Ok(match s {
        "Regolith" => Regolith,
        "Basalt" => Basalt,
        "Ocean" => Ocean,
        "Ice" => Ice,
        "Grass" => Grass,
        "Dirt" => Dirt,
        "Stone" => Stone,
        "Sand" => Sand,
        "Wood" => Wood,
        "Leaves" => Leaves,
        "Zombie" => Zombie,
        "CornStalk" => CornStalk,
        "CarbonLife" => CarbonLife,
        "SiliconLife" => SiliconLife,
        "Habitat" => Habitat,
        "ShipHull" => ShipHull,
        "Glass" => Glass,
        "Beacon" => Beacon,
        "Gate" => Gate,
        "Receiver" => Receiver,
        "SignalPipe" => SignalPipe,
        "PuzzleDoor" => PuzzleDoor,
        "PressurePlate" => PressurePlate,
        _ => return Err(format!("unknown material '{s}'")),
    })
}
fn inside(b: VoxelBounds, c: VoxelCoord) -> bool {
    c.x >= b.min.x
        && c.x <= b.max.x
        && c.y >= b.min.y
        && c.y <= b.max.y
        && c.z >= b.min.z
        && c.z <= b.max.z
}
fn check_box(b: VoxelBounds, x: VoxelBounds) -> Result<(), String> {
    if !inside(b, x.min) || !inside(b, x.max) {
        return Err("geometry lies outside map bounds".into());
    };
    let n = (x.max.x as i64 - x.min.x as i64 + 1)
        * (x.max.y as i64 - x.min.y as i64 + 1)
        * (x.max.z as i64 - x.min.z as i64 + 1);
    if n > MAX_BOX {
        Err("box exceeds voxel limit".into())
    } else {
        Ok(())
    }
}
fn compile_file(path: &Path, assets: &AssetCatalog) -> Result<CompiledMap, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_FILE {
        return Err("file exceeds 8 MiB limit".into());
    };
    let raw: RawMap = serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if raw.format_version != 1 || raw.id.is_empty() || raw.name.is_empty() {
        return Err("format_version must be 1 and id/name must be non-empty".into());
    };
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".hbmap.json"))
        .unwrap_or("");
    if raw.id != stem {
        return Err("id must match filename".into());
    };
    if !matches!(
        raw.mode.as_str(),
        "city"
            | "doom"
            | "corn_maze"
            | "bar"
            | "sandbox"
            | "zombies"
            | "liminal"
            | "drone_gate"
            | "echolocation"
    ) || !matches!(raw.category.as_str(), "authored" | "procedural" | "hybrid")
    {
        return Err("unknown mode or category".into());
    };
    let map_bounds = bounds(raw.bounds)?;
    if raw.operations.len() > 10_000 {
        return Err("too many operations".into());
    };
    let start = marker(raw.player_start)?;
    if !matches!(start, MapMarker::PlayerSpawn { .. }) {
        return Err("player_start must be player_spawn".into());
    };
    let mut ids = HashSet::from([start.id().to_owned()]);
    let mut markers = Vec::new();
    for m in raw.markers {
        let m = marker(m)?;
        if matches!(m, MapMarker::PlayerSpawn { .. }) || !ids.insert(m.id().to_owned()) {
            return Err("duplicate marker id or player_spawn marker".into());
        };
        if !marker_allowed(&raw.mode, &m) {
            return Err("marker is not compatible with map mode".into());
        };
        validate_marker_bounds(map_bounds, &m)?;
        let p = marker_position(&m);
        if !p.x.is_finite()
            || !p.y.is_finite()
            || !p.z.is_finite()
            || p.x < map_bounds.min.x as f32
            || p.x > map_bounds.max.x as f32 + 1.0
            || p.y < map_bounds.min.y as f32
            || p.y > map_bounds.max.y as f32 + 1.0
            || p.z < map_bounds.min.z as f32
            || p.z > map_bounds.max.z as f32 + 1.0
        {
            return Err("marker position outside map bounds".into());
        };
        markers.push(m)
    }
    validate_required_markers(&raw.mode, &markers)?;
    let mut world = VoxelWorld::new();
    let mut operations = Vec::new();
    let mut placed_assets = Vec::new();
    let mut op_ids = HashSet::new();
    for op in raw.operations {
        match op {
            RawOperation::FillBox {
                min,
                max,
                material: m,
            } => {
                let x = VoxelBounds {
                    min: coord(min),
                    max: coord(max),
                };
                check_box(map_bounds, x)?;
                let m = material(&m)?;
                for z in x.min.z..=x.max.z {
                    for y in x.min.y..=x.max.y {
                        for x in x.min.x..=x.max.x {
                            world.set(VoxelCoord::new(x, y, z), VoxelCell::new(m));
                        }
                    }
                }
                operations.push(MapOperation::FillBox {
                    min: x.min,
                    max: x.max,
                    material: m,
                })
            }
            RawOperation::ClearBox { min, max } => {
                let x = VoxelBounds {
                    min: coord(min),
                    max: coord(max),
                };
                check_box(map_bounds, x)?;
                for z in x.min.z..=x.max.z {
                    for y in x.min.y..=x.max.y {
                        for x in x.min.x..=x.max.x {
                            world.clear(VoxelCoord::new(x, y, z));
                        }
                    }
                }
                operations.push(MapOperation::ClearBox {
                    min: x.min,
                    max: x.max,
                })
            }
            RawOperation::PlaceAsset {
                id,
                asset_id,
                position,
                yaw_degrees,
            } => {
                if !op_ids.insert(id.clone()) {
                    return Err("duplicate operation id".into());
                };
                let a = assets
                    .assets
                    .get(&asset_id)
                    .ok_or_else(|| format!("unknown asset '{asset_id}'"))?;
                let p = coord(position);
                if !inside(map_bounds, p) {
                    return Err("asset position outside map bounds".into());
                };
                let placed = PlacedAsset {
                    id,
                    asset_id,
                    position: p,
                    yaw_degrees: yaw(yaw_degrees)?,
                    voxel_size: a.voxel_size,
                };
                if a.voxel_size == 1.0 {
                    for (v, m) in &a.voxels {
                        let q = rotate(*v, placed.yaw_degrees);
                        let q = VoxelCoord::new(p.x + q.x, p.y + q.y, p.z + q.z);
                        if !inside(map_bounds, q) {
                            return Err("asset extent outside map bounds".into());
                        };
                        world.set(q, VoxelCell::new(*m));
                    }
                }
                operations.push(MapOperation::PlaceAsset(placed.clone()));
                placed_assets.push(placed)
            }
            RawOperation::GeneratorRegion {
                id,
                generator,
                bounds: region,
                seed,
                parameters,
            } => {
                if !op_ids.insert(id.clone()) {
                    return Err("duplicate operation id".into());
                };
                let region = bounds(region)?;
                check_box(map_bounds, region)?;
                match generator.as_str() {
                    "city" => {
                        let parameters: CityGeneratorParameters =
                            serde_json::from_value(parameters).map_err(|error| {
                                format!("invalid city generator parameters: {error}")
                            })?;
                        let half_extent = region.max.x;
                        if region.min.x != -half_extent
                            || region.min.z != -half_extent
                            || region.max.z != half_extent
                            || region.min.y != 0
                            || half_extent < 8
                        {
                            return Err(
                                "city generator region must be a square from [-extent, 0, -extent] to [extent, y, extent]"
                                    .into(),
                            );
                        }
                        let generated = CityGenerator::new(CityConfig {
                            seed,
                            half_extent,
                            block_size: parameters.block_size,
                            road_width: parameters.road_width,
                            max_height: parameters.max_height,
                        })
                        .generate();
                        let generated_bounds = generated
                            .bounds()
                            .ok_or("city generator produced an empty world")?;
                        if !inside(region, generated_bounds.min)
                            || !inside(region, generated_bounds.max)
                        {
                            return Err("city generator output lies outside its region".into());
                        }
                        for z in generated_bounds.min.z..=generated_bounds.max.z {
                            for y in generated_bounds.min.y..=generated_bounds.max.y {
                                for x in generated_bounds.min.x..=generated_bounds.max.x {
                                    let voxel = VoxelCoord::new(x, y, z);
                                    if let Some(cell) = generated.get(voxel) {
                                        world.set(voxel, cell);
                                    }
                                }
                            }
                        }
                    }
                    _ => return Err(format!("generator '{generator}' not registered")),
                }
                operations.push(MapOperation::GeneratorRegion { id, generator });
            }
        }
    }
    if world.voxel_count() > MAX_OCCUPIED {
        return Err("world exceeds occupied voxel limit".into());
    };
    Ok(CompiledMap {
        metadata: MapMetadata {
            id: raw.id,
            name: raw.name,
            mode: raw.mode,
            category: raw.category,
            bounds: map_bounds,
        },
        world,
        player_start: start,
        markers,
        operations,
        placed_assets,
        source: path.to_owned(),
    })
}
fn marker_allowed(mode: &str, m: &MapMarker) -> bool {
    match m {
        MapMarker::Exit { .. } => true,
        MapMarker::EnemySpawn { .. } | MapMarker::Pickup { .. } => {
            matches!(mode, "doom" | "zombies")
        }
        MapMarker::InteractableDoor { .. } | MapMarker::WallWeapon { .. } => mode == "zombies",
        MapMarker::LiminalObjective { .. }
        | MapMarker::LiminalRoom { .. }
        | MapMarker::LiminalConnection { .. } => mode == "liminal",
        MapMarker::EchoReceiver { .. }
        | MapMarker::EchoPipe { .. }
        | MapMarker::EchoDoor { .. } => mode == "echolocation",
        MapMarker::PlayerSpawn { .. } => true,
    }
}
fn marker_bounds(m: &MapMarker) -> Option<VoxelBounds> {
    match m {
        MapMarker::InteractableDoor { closed_bounds, .. }
        | MapMarker::EchoDoor { closed_bounds, .. }
        | MapMarker::LiminalRoom {
            bounds: closed_bounds,
            ..
        } => Some(*closed_bounds),
        _ => None,
    }
}
fn validate_marker_bounds(map: VoxelBounds, m: &MapMarker) -> Result<(), String> {
    if let Some(b) = marker_bounds(m) {
        check_box(map, b)?
    };
    match m {
        MapMarker::EchoDoor {
            normal,
            starting_side_anchor,
            far_side_anchor,
            ..
        } => {
            if normal.x.abs() + normal.y.abs() + normal.z.abs() != 1 {
                return Err("echo door normal must be axis aligned".into());
            };
            if !starting_side_anchor.x.is_finite()
                || !starting_side_anchor.y.is_finite()
                || !starting_side_anchor.z.is_finite()
                || !far_side_anchor.x.is_finite()
                || !far_side_anchor.y.is_finite()
                || !far_side_anchor.z.is_finite()
            {
                return Err("echo door anchors must be finite".into());
            }
        }
        MapMarker::EchoReceiver { output_seconds, .. } if *output_seconds <= 0.0 => {
            return Err("receiver output_seconds must be positive".into())
        }
        MapMarker::EchoPipe { sequence, .. } if *sequence < 0 => {
            return Err("pipe sequence must be non-negative".into())
        }
        _ => {}
    }
    Ok(())
}
fn validate_required_markers(mode: &str, markers: &[MapMarker]) -> Result<(), String> {
    let has = |f: fn(&MapMarker) -> bool| markers.iter().any(f);
    match mode {
        "zombies"
            if !(has(|m| matches!(m, MapMarker::InteractableDoor { .. }))
                && has(|m| matches!(m, MapMarker::EnemySpawn { .. }))
                && has(|m| matches!(m, MapMarker::WallWeapon { .. }))) =>
        {
            Err("zombies map requires door, spawn, and wall weapon markers".into())
        }
        "liminal"
            if !(has(|m| matches!(m, MapMarker::LiminalRoom { .. }))
                && has(|m| matches!(m, MapMarker::LiminalObjective { .. }))) =>
        {
            Err("liminal map requires room and objective markers".into())
        }
        "echolocation"
            if !(has(|m| matches!(m, MapMarker::EchoReceiver { .. }))
                && has(|m| matches!(m, MapMarker::EchoPipe { .. }))
                && has(|m| matches!(m, MapMarker::EchoDoor { .. }))) =>
        {
            Err("echolocation map requires receiver, pipe, and door markers".into())
        }
        _ => Ok(()),
    }
}
fn marker_position(m: &MapMarker) -> Vec3 {
    match m {
        MapMarker::PlayerSpawn { position, .. }
        | MapMarker::Exit { position, .. }
        | MapMarker::EnemySpawn { position, .. }
        | MapMarker::Pickup { position, .. }
        | MapMarker::InteractableDoor { position, .. }
        | MapMarker::WallWeapon { position, .. }
        | MapMarker::LiminalObjective { position, .. }
        | MapMarker::EchoReceiver { position, .. }
        | MapMarker::EchoPipe { position, .. }
        | MapMarker::EchoDoor { position, .. }
        | MapMarker::LiminalRoom { position, .. }
        | MapMarker::LiminalConnection { position, .. } => *position,
    }
}
fn rotate(v: VoxelCoord, yaw: u16) -> VoxelCoord {
    match yaw {
        0 => v,
        90 => VoxelCoord::new(v.z, v.y, -v.x),
        180 => VoxelCoord::new(-v.x, v.y, -v.z),
        270 => VoxelCoord::new(-v.z, v.y, v.x),
        _ => unreachable!(),
    }
}
fn marker(m: RawMarker) -> Result<MapMarker, String> {
    match m {
        RawMarker::PlayerSpawn {
            id,
            position,
            yaw_degrees,
        } => Ok(MapMarker::PlayerSpawn {
            id,
            position: pos(position)?,
            yaw_degrees: yaw(yaw_degrees)?,
        }),
        RawMarker::Exit {
            id,
            position,
            target,
        } => Ok(MapMarker::Exit {
            id,
            position: pos(position)?,
            target,
        }),
        RawMarker::EnemySpawn {
            id,
            position,
            enemy_type,
            spawn_group,
        } => Ok(MapMarker::EnemySpawn {
            id,
            position: pos(position)?,
            enemy_type,
            spawn_group,
        }),
        RawMarker::Pickup {
            id,
            position,
            pickup_type,
            amount,
        } => Ok(MapMarker::Pickup {
            id,
            position: pos(position)?,
            pickup_type,
            amount,
        }),
        RawMarker::InteractableDoor {
            id,
            position,
            door_type,
            closed_bounds,
            open_cost,
        } => Ok(MapMarker::InteractableDoor {
            id,
            position: pos(position)?,
            door_type,
            closed_bounds: bounds(closed_bounds)?,
            open_cost,
        }),
        RawMarker::WallWeapon {
            id,
            position,
            weapon_type,
            cost,
            yaw_degrees,
        } => Ok(MapMarker::WallWeapon {
            id,
            position: pos(position)?,
            weapon_type,
            cost,
            yaw_degrees: yaw(yaw_degrees)?,
        }),
        RawMarker::LiminalObjective {
            id,
            position,
            objective_type,
            room_id,
        } => Ok(MapMarker::LiminalObjective {
            id,
            position: pos(position)?,
            objective_type,
            room_id,
        }),
        RawMarker::EchoReceiver {
            id,
            position,
            output_seconds,
            puzzle_id,
        } => Ok(MapMarker::EchoReceiver {
            id,
            position: pos(position)?,
            output_seconds,
            puzzle_id,
        }),
        RawMarker::EchoPipe {
            id,
            position,
            puzzle_id,
            sequence,
        } => Ok(MapMarker::EchoPipe {
            id,
            position: pos(position)?,
            puzzle_id,
            sequence,
        }),
        RawMarker::EchoDoor {
            id,
            position,
            puzzle_id,
            closed_bounds,
            normal,
            starting_side_anchor,
            far_side_anchor,
        } => Ok(MapMarker::EchoDoor {
            id,
            position: pos(position)?,
            puzzle_id,
            closed_bounds: bounds(closed_bounds)?,
            normal: coord(normal),
            starting_side_anchor: pos(starting_side_anchor)?,
            far_side_anchor: pos(far_side_anchor)?,
        }),
        RawMarker::LiminalRoom {
            id,
            position,
            room_id,
            bounds: b,
            room_type,
            sign,
        } => Ok(MapMarker::LiminalRoom {
            id,
            position: pos(position)?,
            room_id,
            bounds: bounds(b)?,
            room_type,
            sign,
        }),
        RawMarker::LiminalConnection {
            id,
            position,
            from_room,
            to_room,
        } => Ok(MapMarker::LiminalConnection {
            id,
            position: pos(position)?,
            from_room,
            to_room,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hb-map-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn ordered_fill_and_clear_compiles() {
        let d = dir();
        fs::write(d.join("test.hbmap.json"),r#"{"format_version":1,"id":"test","name":"T","mode":"bar","category":"authored","bounds":{"min":[0,0,0],"max":[2,2,2]},"player_start":{"id":"start","kind":"player_spawn","position":[0.5,1.0,0.5],"yaw_degrees":0},"operations":[{"kind":"fill_box","min":[0,0,0],"max":[1,1,1],"material":"Stone"},{"kind":"clear_box","min":[1,1,1],"max":[1,1,1]}],"markers":[]}"#).unwrap();
        let c = MapCatalog::discover(&d, &AssetCatalog::default());
        assert_eq!(c.maps.len(), 1);
        assert_eq!(c.maps[0].world.voxel_count(), 7);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn city_generator_region_compiles_deterministically() {
        let d = dir();
        fs::write(d.join("city.hbmap.json"), r#"{"format_version":1,"id":"city","name":"City","mode":"city","category":"procedural","bounds":{"min":[-8,0,-8],"max":[8,20,8]},"player_start":{"id":"start","kind":"player_spawn","position":[0.5,1.7,-5.5],"yaw_degrees":0},"operations":[{"kind":"generator_region","id":"city-world","generator":"city","bounds":{"min":[-8,0,-8],"max":[8,20,8]},"seed":12,"parameters":{"road_width":3,"block_size":8,"max_height":12}}],"markers":[]}"#).unwrap();
        let a = MapCatalog::discover(&d, &AssetCatalog::default());
        let b = MapCatalog::discover(&d, &AssetCatalog::default());
        assert!(a.errors.is_empty(), "{:?}", a.errors);
        assert_eq!(a.maps[0].world.voxel_count(), b.maps[0].world.voxel_count());
        assert_eq!(a.maps[0].world.bounds(), b.maps[0].world.bounds());
        assert!(matches!(
            a.maps[0].operations[0],
            MapOperation::GeneratorRegion { .. }
        ));
        let _ = fs::remove_dir_all(d);
    }
}
