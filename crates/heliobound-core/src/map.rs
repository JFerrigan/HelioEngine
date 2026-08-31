//! Strict, startup-loadable v1 map blueprints.  The catalog deliberately owns
//! no mutable game state: callers clone `CompiledMap::world` for a session.
use crate::{
    CityConfig, CityGenerator, Vec3, VoxelBounds, VoxelCell, VoxelCoord, VoxelMaterial, VoxelWorld,
};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Debug, PartialEq)]
pub struct MapMetadata {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub category: String,
    pub bounds: VoxelBounds,
}
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
/// A mutable, editor-facing map working copy.
///
/// Its voxel world is the authoritative final static geometry. Exporting it
/// intentionally does not retain historical fill/clear/generator operations:
/// it emits a deterministic canonical snapshot instead. Asset instances stay
/// structured entities; painting through an asset is therefore not supported
/// until that instance is removed or moved by the editor.
#[derive(Clone, Debug)]
pub struct EditableMap {
    pub metadata: MapMetadata,
    pub world: VoxelWorld,
    pub player_start: MapMarker,
    pub markers: Vec<MapMarker>,
    pub placed_assets: Vec<PlacedAsset>,
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

    /// Clones the immutable blueprint into the representation a future editor
    /// may freely mutate before exporting or saving-as.
    pub fn editable(&self) -> EditableMap {
        EditableMap {
            metadata: self.metadata.clone(),
            world: self.world.clone(),
            player_start: self.player_start.clone(),
            markers: self.markers.clone(),
            placed_assets: self.placed_assets.clone(),
        }
    }
}

/// Serializes a working map into the canonical v1 JSON form.
///
/// Geometry is greedily coalesced into same-material axis-aligned boxes in a
/// stable order. The result is an authored final-world snapshot, not an edit
/// history; procedural regions are deliberately materialized on export.
pub fn export_map(map: &EditableMap, assets: &AssetCatalog) -> Result<String, String> {
    if !matches!(map.player_start, MapMarker::PlayerSpawn { .. }) {
        return Err("player_start must be player_spawn".into());
    }
    if map.world.voxel_count() == 0 {
        return Err("cannot export an empty world".into());
    }

    // Unit-scale assets already appear in the compiled collision world. Omit
    // their matching cells from direct geometry so a custom asset palette does
    // not leak into the direct-material namespace. Asset entities are emitted
    // after geometry and remain the final overlay, matching compiler order.
    let mut asset_cells = HashMap::new();
    for placed in &map.placed_assets {
        let asset = assets
            .assets
            .get(&placed.asset_id)
            .ok_or_else(|| format!("unknown asset '{}'", placed.asset_id))?;
        if asset.voxel_size != placed.voxel_size {
            return Err(format!("asset '{}' changed voxel size", placed.asset_id));
        }
        if asset.voxel_size == 1.0 {
            for (voxel, material) in &asset.voxels {
                let rotated = rotate(*voxel, placed.yaw_degrees);
                let coord = VoxelCoord::new(
                    placed.position.x + rotated.x,
                    placed.position.y + rotated.y,
                    placed.position.z + rotated.z,
                );
                if !inside(map.metadata.bounds, coord) {
                    return Err("asset extent lies outside map bounds".into());
                }
                asset_cells.insert(coord, *material);
            }
        }
    }

    let mut cells = BTreeMap::new();
    for (coord, cell) in map.world.voxels() {
        if asset_cells.get(&coord) == Some(&cell.material) {
            continue;
        }
        if !inside(map.metadata.bounds, coord) {
            return Err("world geometry lies outside map bounds".into());
        }
        material_name(cell.material)?;
        cells.insert(coord, cell.material);
    }
    let mut operations = Vec::new();
    while let Some((&start, &material)) = cells.iter().next() {
        let mut max_x = start.x;
        while max_x < i32::MAX
            && cells.get(&VoxelCoord::new(max_x + 1, start.y, start.z)) == Some(&material)
        {
            max_x += 1;
        }
        let mut max_z = start.z;
        'z_expand: loop {
            let Some(next_z) = max_z.checked_add(1) else {
                break;
            };
            for x in start.x..=max_x {
                if cells.get(&VoxelCoord::new(x, start.y, next_z)) != Some(&material) {
                    break 'z_expand;
                }
            }
            max_z = next_z;
        }
        let mut max_y = start.y;
        'y_expand: loop {
            let Some(next_y) = max_y.checked_add(1) else {
                break;
            };
            for z in start.z..=max_z {
                for x in start.x..=max_x {
                    if cells.get(&VoxelCoord::new(x, next_y, z)) != Some(&material) {
                        break 'y_expand;
                    }
                }
            }
            max_y = next_y;
        }
        for z in start.z..=max_z {
            for y in start.y..=max_y {
                for x in start.x..=max_x {
                    cells.remove(&VoxelCoord::new(x, y, z));
                }
            }
        }
        operations.push(serde_json::json!({
            "kind": "fill_box",
            "min": [start.x, start.y, start.z],
            "max": [max_x, max_y, max_z],
            "material": material_name(material)?,
        }));
    }
    if operations.len() + map.placed_assets.len() > 10_000 {
        return Err("export exceeds operation limit".into());
    }
    for asset in &map.placed_assets {
        operations.push(serde_json::json!({
            "kind": "place_asset",
            "id": asset.id,
            "asset_id": asset.asset_id,
            "position": [asset.position.x, asset.position.y, asset.position.z],
            "yaw_degrees": asset.yaw_degrees,
        }));
    }
    let document = ExportedMap {
        format_version: 1,
        id: &map.metadata.id,
        name: &map.metadata.name,
        mode: &map.metadata.mode,
        category: "authored",
        bounds: ExportedBounds::from(map.metadata.bounds),
        player_start: marker_json(&map.player_start),
        operations,
        markers: map.markers.iter().map(marker_json).collect(),
    };
    serde_json::to_string_pretty(&document).map_err(|error| error.to_string())
}

#[derive(Serialize)]
struct ExportedMap<'a> {
    format_version: u32,
    id: &'a str,
    name: &'a str,
    mode: &'a str,
    category: &'a str,
    bounds: ExportedBounds,
    player_start: serde_json::Value,
    operations: Vec<serde_json::Value>,
    markers: Vec<serde_json::Value>,
}
#[derive(Serialize)]
struct ExportedBounds {
    min: [i32; 3],
    max: [i32; 3],
}
impl From<VoxelBounds> for ExportedBounds {
    fn from(bounds: VoxelBounds) -> Self {
        Self {
            min: [bounds.min.x, bounds.min.y, bounds.min.z],
            max: [bounds.max.x, bounds.max.y, bounds.max.z],
        }
    }
}
fn material_name(material: VoxelMaterial) -> Result<&'static str, String> {
    use VoxelMaterial::*;
    match material {
        Regolith => Ok("Regolith"),
        Basalt => Ok("Basalt"),
        Ocean => Ok("Ocean"),
        Ice => Ok("Ice"),
        Grass => Ok("Grass"),
        Dirt => Ok("Dirt"),
        Stone => Ok("Stone"),
        Sand => Ok("Sand"),
        Wood => Ok("Wood"),
        Leaves => Ok("Leaves"),
        Zombie => Ok("Zombie"),
        CornStalk => Ok("CornStalk"),
        CarbonLife => Ok("CarbonLife"),
        SiliconLife => Ok("SiliconLife"),
        Habitat => Ok("Habitat"),
        ShipHull => Ok("ShipHull"),
        Glass => Ok("Glass"),
        Beacon => Ok("Beacon"),
        Gate => Ok("Gate"),
        Receiver => Ok("Receiver"),
        SignalPipe => Ok("SignalPipe"),
        PuzzleDoor => Ok("PuzzleDoor"),
        PressurePlate => Ok("PressurePlate"),
        Custom(_) => Err("custom-material voxels must belong to a placed asset".into()),
    }
}
fn marker_json(marker: &MapMarker) -> serde_json::Value {
    let position = |p: Vec3| serde_json::json!([p.x, p.y, p.z]);
    let bounds = |b: VoxelBounds| serde_json::json!({"min": [b.min.x, b.min.y, b.min.z], "max": [b.max.x, b.max.y, b.max.z]});
    match marker {
        MapMarker::PlayerSpawn {
            id,
            position: p,
            yaw_degrees,
        } => {
            serde_json::json!({"id": id, "kind": "player_spawn", "position": position(*p), "yaw_degrees": yaw_degrees})
        }
        MapMarker::Exit {
            id,
            position: p,
            target,
        } => {
            serde_json::json!({"id": id, "kind": "exit", "position": position(*p), "target": target})
        }
        MapMarker::EnemySpawn {
            id,
            position: p,
            enemy_type,
            spawn_group,
        } => {
            serde_json::json!({"id": id, "kind": "enemy_spawn", "position": position(*p), "enemy_type": enemy_type, "spawn_group": spawn_group})
        }
        MapMarker::Pickup {
            id,
            position: p,
            pickup_type,
            amount,
        } => {
            serde_json::json!({"id": id, "kind": "pickup", "position": position(*p), "pickup_type": pickup_type, "amount": amount})
        }
        MapMarker::InteractableDoor {
            id,
            position: p,
            door_type,
            closed_bounds,
            open_cost,
        } => {
            serde_json::json!({"id": id, "kind": "interactable_door", "position": position(*p), "door_type": door_type, "closed_bounds": bounds(*closed_bounds), "open_cost": open_cost})
        }
        MapMarker::WallWeapon {
            id,
            position: p,
            weapon_type,
            cost,
            yaw_degrees,
        } => {
            serde_json::json!({"id": id, "kind": "wall_weapon", "position": position(*p), "weapon_type": weapon_type, "cost": cost, "yaw_degrees": yaw_degrees})
        }
        MapMarker::LiminalObjective {
            id,
            position: p,
            objective_type,
            room_id,
        } => {
            serde_json::json!({"id": id, "kind": "liminal_objective", "position": position(*p), "objective_type": objective_type, "room_id": room_id})
        }
        MapMarker::EchoReceiver {
            id,
            position: p,
            output_seconds,
            puzzle_id,
        } => {
            serde_json::json!({"id": id, "kind": "echo_receiver", "position": position(*p), "output_seconds": output_seconds, "puzzle_id": puzzle_id})
        }
        MapMarker::EchoPipe {
            id,
            position: p,
            puzzle_id,
            sequence,
        } => {
            serde_json::json!({"id": id, "kind": "echo_pipe", "position": position(*p), "puzzle_id": puzzle_id, "sequence": sequence})
        }
        MapMarker::EchoDoor {
            id,
            position: p,
            puzzle_id,
            closed_bounds,
            normal,
            starting_side_anchor,
            far_side_anchor,
        } => {
            serde_json::json!({"id": id, "kind": "echo_door", "position": position(*p), "puzzle_id": puzzle_id, "closed_bounds": bounds(*closed_bounds), "normal": [normal.x, normal.y, normal.z], "starting_side_anchor": position(*starting_side_anchor), "far_side_anchor": position(*far_side_anchor)})
        }
        MapMarker::LiminalRoom {
            id,
            position: p,
            room_id,
            bounds: room_bounds,
            room_type,
            sign,
        } => {
            serde_json::json!({"id": id, "kind": "liminal_room", "position": position(*p), "room_id": room_id, "bounds": bounds(*room_bounds), "room_type": room_type, "sign": sign})
        }
        MapMarker::LiminalConnection {
            id,
            position: p,
            from_room,
            to_room,
        } => {
            serde_json::json!({"id": id, "kind": "liminal_connection", "position": position(*p), "from_room": from_room, "to_room": to_room})
        }
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

    #[test]
    fn exported_working_map_round_trips_geometry_markers_and_assets() {
        let d = dir();
        let mut assets = AssetCatalog::default();
        assets.assets.insert(
            "unit-asset".into(),
            AssetDefinition {
                id: "unit-asset".into(),
                name: "Unit asset".into(),
                voxel_size: 1.0,
                pivot: [0.0, 0.0, 0.0],
                voxels: vec![(VoxelCoord::new(0, 0, 0), VoxelMaterial::Glass)],
            },
        );
        fs::write(d.join("roundtrip.hbmap.json"), r#"{"format_version":1,"id":"roundtrip","name":"Round trip","mode":"bar","category":"authored","bounds":{"min":[0,0,0],"max":[5,3,3]},"player_start":{"id":"start","kind":"player_spawn","position":[0.5,1.0,0.5],"yaw_degrees":0},"operations":[{"kind":"fill_box","min":[0,0,0],"max":[2,1,1],"material":"Stone"},{"kind":"clear_box","min":[1,1,1],"max":[1,1,1]},{"kind":"place_asset","id":"window","asset_id":"unit-asset","position":[4,1,1],"yaw_degrees":0}],"markers":[{"id":"exit","kind":"exit","position":[2.5,1.0,2.5],"target":"menu"}]}"#).unwrap();
        let original = MapCatalog::discover(&d, &assets).maps.remove(0);
        let editable = original.editable();
        let exported = export_map(&editable, &assets).unwrap();
        assert_eq!(exported, export_map(&editable, &assets).unwrap());
        fs::write(d.join("roundtrip.hbmap.json"), exported).unwrap();

        let catalog = MapCatalog::discover(&d, &assets);
        assert!(catalog.errors.is_empty(), "{:?}", catalog.errors);
        let reloaded = &catalog.maps[0];
        assert_eq!(reloaded.metadata, original.metadata);
        assert_eq!(reloaded.player_start, original.player_start);
        assert_eq!(reloaded.markers, original.markers);
        assert_eq!(reloaded.placed_assets, original.placed_assets);
        assert_eq!(reloaded.world.voxels(), original.world.voxels());
        assert!(reloaded.operations.len() < original.world.voxel_count());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn exporter_rejects_unowned_custom_voxels() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 0, 0),
            VoxelCell::new(VoxelMaterial::Custom([1, 2, 3])),
        );
        let map = EditableMap {
            metadata: MapMetadata {
                id: "custom".into(),
                name: "Custom".into(),
                mode: "bar".into(),
                category: "authored".into(),
                bounds: VoxelBounds {
                    min: VoxelCoord::new(0, 0, 0),
                    max: VoxelCoord::new(1, 1, 1),
                },
            },
            world,
            player_start: MapMarker::PlayerSpawn {
                id: "start".into(),
                position: Vec3::new(0.5, 1.0, 0.5),
                yaw_degrees: 0,
            },
            markers: vec![],
            placed_assets: vec![],
        };
        assert!(export_map(&map, &AssetCatalog::default())
            .unwrap_err()
            .contains("custom-material"));
    }
}
