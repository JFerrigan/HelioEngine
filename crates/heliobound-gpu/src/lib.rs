//! GPU renderer building blocks. Window and event-loop ownership remains in
//! `heliobound-cli`; this crate never makes CPU world state non-authoritative.

use font8x8::{UnicodeFonts, BASIC_FONTS};
use heliobound_core::{Camera, ChunkCoord, ChunkSnapshot, VoxelBounds, VoxelMaterial};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use winit::{dpi::PhysicalSize, window::Window};

pub const LOGICAL_WIDTH: u32 = 160;
pub const LOGICAL_HEIGHT: u32 = 90;
pub const EMPTY_VOXEL: u32 = 0;
pub const CHUNK_EDGE: u32 = 16;
pub const CHUNK_VOXELS: u32 = CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE;
pub const GLYPH_ATLAS_COLUMNS: u32 = 16;
pub const GLYPH_ATLAS_ROWS: u32 = 16;
pub const GLYPH_WIDTH: u32 = 8;
pub const GLYPH_HEIGHT: u32 = 8;
const CUSTOM_TAG: u32 = 0x80_00_00_00;

/// Stable renderer IDs for built-in materials. The value zero remains empty;
/// imported palette colors use `CUSTOM_TAG | rgb` instead.
pub fn encode_voxel(material: VoxelMaterial) -> u32 {
    match material {
        VoxelMaterial::Regolith => 1,
        VoxelMaterial::Basalt => 2,
        VoxelMaterial::Ocean => 3,
        VoxelMaterial::Ice => 4,
        VoxelMaterial::Grass => 5,
        VoxelMaterial::Dirt => 6,
        VoxelMaterial::Stone => 7,
        VoxelMaterial::Sand => 8,
        VoxelMaterial::Wood => 9,
        VoxelMaterial::Leaves => 10,
        VoxelMaterial::Zombie => 11,
        VoxelMaterial::CornStalk => 12,
        VoxelMaterial::CarbonLife => 13,
        VoxelMaterial::SiliconLife => 14,
        VoxelMaterial::Habitat => 15,
        VoxelMaterial::ShipHull => 16,
        VoxelMaterial::Glass => 17,
        VoxelMaterial::Beacon => 18,
        VoxelMaterial::Gate => 19,
        VoxelMaterial::Receiver => 20,
        VoxelMaterial::SignalPipe => 21,
        VoxelMaterial::PuzzleDoor => 22,
        VoxelMaterial::PressurePlate => 23,
        VoxelMaterial::Custom([r, g, b]) => {
            CUSTOM_TAG | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
        }
    }
}

/// GPU-ready upload payload. A dense slot is always exactly 16³ u32 values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkUpload {
    pub coord: ChunkCoord,
    pub revision: u64,
    pub voxels: Vec<u32>,
}

/// Layout shared by the camera uniform and `dda.wgsl`. It intentionally uses
/// only 16-byte fields, so its Rust byte representation has the same offsets
/// as WGSL's uniform-address-space layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub position_and_max_distance: [f32; 4],
    pub forward_and_aspect: [f32; 4],
    pub right_and_tan_half_fov: [f32; 4],
    pub up_and_padding: [f32; 4],
    pub bounds_min_and_padding: [i32; 4],
    /// xyz are inclusive voxel bounds; w is the fixed DDA traversal cap.
    pub bounds_max_and_max_steps: [i32; 4],
    pub table_origin_and_padding: [i32; 4],
    /// xyz are bounded lookup-table dimensions; w is unused.
    pub table_dimensions_and_padding: [u32; 4],
}

impl CameraUniform {
    pub const BYTE_SIZE: u64 = std::mem::size_of::<Self>() as u64;

    pub fn from_camera(
        camera: Camera,
        bounds: VoxelBounds,
        table: ChunkTableLayout,
        max_steps: u32,
    ) -> Self {
        let position = camera.position;
        let forward = camera.forward();
        let right = camera.right();
        let up = camera.up();
        Self {
            position_and_max_distance: [position.x, position.y, position.z, camera.max_distance],
            forward_and_aspect: [
                forward.x,
                forward.y,
                forward.z,
                LOGICAL_WIDTH as f32 / LOGICAL_HEIGHT as f32,
            ],
            right_and_tan_half_fov: [
                right.x,
                right.y,
                right.z,
                (camera.fov_y_radians * 0.5).tan(),
            ],
            up_and_padding: [up.x, up.y, up.z, 0.0],
            bounds_min_and_padding: [bounds.min.x, bounds.min.y, bounds.min.z, 0],
            bounds_max_and_max_steps: [
                bounds.max.x,
                bounds.max.y,
                bounds.max.z,
                max_steps.min(i32::MAX as u32) as i32,
            ],
            table_origin_and_padding: [table.origin.x, table.origin.y, table.origin.z, 0],
            table_dimensions_and_padding: [
                table.dimensions[0],
                table.dimensions[1],
                table.dimensions[2],
                0,
            ],
        }
    }
}

/// A bounded, camera-relative chunk-coordinate lookup table. A table entry is
/// `slot + 1`; zero is empty. This avoids a GPU hash table while preserving
/// sparse-world uploads and negative chunk coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkTableLayout {
    pub origin: ChunkCoord,
    pub dimensions: [u32; 3],
}

impl ChunkTableLayout {
    pub fn new(origin: ChunkCoord, dimensions: [u32; 3]) -> Self {
        assert!(
            dimensions.iter().all(|&axis| axis > 0),
            "chunk table axes must be non-zero"
        );
        Self { origin, dimensions }
    }

    pub fn len(self) -> usize {
        self.dimensions.iter().product::<u32>() as usize
    }

    pub fn contains(self, coord: ChunkCoord) -> bool {
        let local = [
            coord.x as i64 - self.origin.x as i64,
            coord.y as i64 - self.origin.y as i64,
            coord.z as i64 - self.origin.z as i64,
        ];
        local
            .iter()
            .zip(self.dimensions)
            .all(|(&value, dimension)| value >= 0 && value < dimension as i64)
    }

    pub fn index(self, coord: ChunkCoord) -> Option<usize> {
        if !self.contains(coord) {
            return None;
        }
        let x = (coord.x - self.origin.x) as usize;
        let y = (coord.y - self.origin.y) as usize;
        let z = (coord.z - self.origin.z) as usize;
        Some(
            x + y * self.dimensions[0] as usize
                + z * self.dimensions[0] as usize * self.dimensions[1] as usize,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkSlotUpload {
    pub slot: u32,
    pub upload: ChunkUpload,
}

/// Pure residency policy for the GPU's fixed-capacity dense chunk-slot buffer.
/// `sync_visible_snapshots` must receive every occupied chunk in `layout`;
/// it then evicts chunks that disappeared or left the bounded table.
#[derive(Debug)]
pub struct ResidentChunkTable {
    layout: ChunkTableLayout,
    slots: BTreeMap<ChunkCoord, u32>,
    revisions: BTreeMap<ChunkCoord, u64>,
    free_slots: BTreeSet<u32>,
    next_slot: u32,
}

impl ResidentChunkTable {
    pub fn new(layout: ChunkTableLayout) -> Self {
        Self {
            layout,
            slots: BTreeMap::new(),
            revisions: BTreeMap::new(),
            free_slots: BTreeSet::new(),
            next_slot: 0,
        }
    }

    pub fn layout(&self) -> ChunkTableLayout {
        self.layout
    }
    pub fn resident_count(&self) -> usize {
        self.slots.len()
    }
    pub fn slot_count(&self) -> u32 {
        self.next_slot
    }

    pub fn recenter(&mut self, layout: ChunkTableLayout) {
        self.layout = layout;
        let evicted = self
            .slots
            .keys()
            .copied()
            .filter(|coord| !layout.contains(*coord))
            .collect::<Vec<_>>();
        for coord in evicted {
            self.free_slots
                .insert(self.slots.remove(&coord).expect("resident slot exists"));
            self.revisions.remove(&coord);
        }
    }

    pub fn sync_visible_snapshots<'a>(
        &mut self,
        snapshots: impl IntoIterator<Item = &'a ChunkSnapshot>,
    ) -> Vec<ChunkSlotUpload> {
        let snapshots = snapshots
            .into_iter()
            .filter(|snapshot| self.layout.contains(snapshot.coord))
            .collect::<Vec<_>>();
        let seen = snapshots
            .iter()
            .map(|snapshot| snapshot.coord)
            .collect::<BTreeSet<_>>();
        let gone = self
            .slots
            .keys()
            .copied()
            .filter(|coord| self.layout.contains(*coord) && !seen.contains(coord))
            .collect::<Vec<_>>();
        for coord in gone {
            self.free_slots
                .insert(self.slots.remove(&coord).expect("resident slot exists"));
            self.revisions.remove(&coord);
        }

        let mut updates = Vec::new();
        for snapshot in snapshots {
            let slot = *self.slots.entry(snapshot.coord).or_insert_with(|| {
                if let Some(slot) = self.free_slots.iter().next().copied() {
                    self.free_slots.remove(&slot);
                    slot
                } else {
                    let slot = self.next_slot;
                    self.next_slot += 1;
                    slot
                }
            });
            if self.revisions.get(&snapshot.coord) != Some(&snapshot.revision) {
                self.revisions.insert(snapshot.coord, snapshot.revision);
                updates.push(ChunkSlotUpload {
                    slot,
                    upload: snapshot.into(),
                });
            }
        }
        updates
    }

    pub fn lookup_entries(&self) -> Vec<u32> {
        let mut entries = vec![0; self.layout.len()];
        for (&coord, &slot) in &self.slots {
            entries[self
                .layout
                .index(coord)
                .expect("resident coordinate in table")] = slot + 1;
        }
        entries
    }
}

impl From<&ChunkSnapshot> for ChunkUpload {
    fn from(snapshot: &ChunkSnapshot) -> Self {
        Self {
            coord: snapshot.coord,
            revision: snapshot.revision,
            voxels: snapshot
                .cells
                .iter()
                .map(|cell| {
                    cell.map(|c| encode_voxel(c.material))
                        .unwrap_or(EMPTY_VOXEL)
                })
                .collect(),
        }
    }
}

/// Tracks resident chunks by coordinate/revision, independent of world object
/// identity. The renderer owns actual wgpu buffers; this pure type makes its
/// upload policy deterministic and testable without an adapter.
#[derive(Default, Debug)]
pub struct ResidentChunks {
    revisions: std::collections::BTreeMap<ChunkCoord, u64>,
}

impl ResidentChunks {
    pub fn updates<'a>(
        &mut self,
        snapshots: impl IntoIterator<Item = &'a ChunkSnapshot>,
    ) -> Vec<ChunkUpload> {
        let mut seen = std::collections::BTreeSet::new();
        let mut uploads = Vec::new();
        for snapshot in snapshots {
            seen.insert(snapshot.coord);
            if self.revisions.get(&snapshot.coord) != Some(&snapshot.revision) {
                self.revisions.insert(snapshot.coord, snapshot.revision);
                uploads.push(snapshot.into());
            }
        }
        self.revisions.retain(|coord, _| seen.contains(coord));
        uploads
    }

    pub fn resident_count(&self) -> usize {
        self.revisions.len()
    }
}

pub const DDA_SHADER: &str = include_str!("dda.wgsl");
pub const GLYPH_SHADER: &str = include_str!("glyph.wgsl");

/// A compact logical-cell overlay for the GPU presentation path. Colours are
/// packed as `0xRRGGBBAA`; set `flags & 1` to draw an opaque background.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiCell {
    pub x: i32,
    pub y: i32,
    pub glyph: u32,
    pub flags: u32,
    pub foreground_rgba: u32,
    pub background_rgba: u32,
}

impl UiCell {
    pub const OPAQUE_BACKGROUND: u32 = 1;

    pub const fn new(x: i32, y: i32, glyph: char, foreground_rgba: u32) -> Self {
        Self {
            x,
            y,
            glyph: glyph as u32,
            flags: 0,
            foreground_rgba,
            background_rgba: 0,
        }
    }

    pub const fn with_background(mut self, background_rgba: u32) -> Self {
        self.flags |= Self::OPAQUE_BACKGROUND;
        self.background_rgba = background_rgba;
        self
    }
}

/// One CPU-composed sky cell, passed to the GPU terrain pass in row-major
/// logical-cell order. Terrain hits replace this value; misses retain it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BackgroundCell {
    pub glyph: u32,
    pub foreground_rgba: u32,
}

/// A 16 by 16 indexed pixel sprite in the CPU framebuffer's 1280 by 720
/// coordinate space. It is scaled with the logical presentation grid, rather
/// than treated as a terminal cell overlay.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PixelSprite {
    pub x: i32,
    pub y: i32,
    pub scale: u32,
    pub flags: u32,
    pub foreground_rgba: u32,
    pub background_rgba: u32,
    pub rows: [u32; 16],
}

impl PixelSprite {
    pub const OPAQUE_BACKGROUND: u32 = 1;
}

/// A direct `wgpu` presentation surface owned by the future CLI GPU backend.
///
/// This deliberately owns no window or world state.  The caller keeps the
/// `Window` alive for at least as long as this value, handles its event loop,
/// and calls [`Self::resize`] on size changes.  The current diagnostic pass is
/// a fullscreen triangle; later renderer passes reuse this surface, device,
/// queue and surface configuration rather than creating a second presentation
/// path.
pub struct SurfaceRenderer<'window> {
    instance: wgpu::Instance,
    window: &'window Window,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    diagnostic_pipeline: wgpu::RenderPipeline,
    terrain_pipeline: wgpu::RenderPipeline,
    terrain_bind_group_layout: wgpu::BindGroupLayout,
    terrain: TerrainGpuBuffers,
    logical_targets: LogicalTargets,
    glyph_pipeline: wgpu::RenderPipeline,
    glyph_bind_group: wgpu::BindGroup,
    ui_pipeline: wgpu::RenderPipeline,
    ui_bind_group_layout: wgpu::BindGroupLayout,
    ui: UiGpuBuffer,
    ui_count: u32,
    overlay_ui: UiGpuBuffer,
    overlay_ui_count: u32,
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_bind_group_layout: wgpu::BindGroupLayout,
    sprites: SpriteGpuBuffer,
    sprite_count: u32,
    glyph_atlas: wgpu::TextureView,
    presentation: wgpu::Buffer,
}

struct TerrainGpuBuffers {
    camera: wgpu::Buffer,
    lookup: wgpu::Buffer,
    background: wgpu::Buffer,
    voxels: wgpu::Buffer,
    dynamic_voxels: wgpu::Buffer,
    assets: wgpu::Buffer,
    asset_voxels: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    lookup_capacity: u64,
    voxel_capacity: u64,
    dynamic_capacity: u64,
    asset_capacity: u64,
    asset_voxel_capacity: u64,
}

struct LogicalTargets {
    #[cfg_attr(not(test), allow(dead_code))]
    glyph_texture: wgpu::Texture,
    glyphs: wgpu::TextureView,
    #[cfg_attr(not(test), allow(dead_code))]
    colour_texture: wgpu::Texture,
    colours: wgpu::TextureView,
}

struct UiGpuBuffer {
    cells: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    capacity: u64,
}

struct SpriteGpuBuffer {
    sprites: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    capacity: u64,
}

/// Shared by the compositor shaders. `scale_and_origin` defines an integer
/// letterboxed cell grid so glyphs are never filtered or stretched.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PresentationUniform {
    physical_size: [f32; 2],
    logical_size: [f32; 2],
    scale_and_origin: [f32; 4],
    /// One when the final render target performs linear-to-sRGB encoding.
    /// The compositor encodes explicitly for the rare non-sRGB surface format.
    surface_is_srgb: [u32; 4],
}

/// Per-frame terrain-cache synchronization input. `lookup` is the complete
/// bounded chunk-coordinate table; `updates` contains only newly resident or
/// revision-changed slots. `slot_count` is the high-water slot count from
/// [`ResidentChunkTable::slot_count`], not the resident chunk count.
pub struct TerrainFrame<'a> {
    pub camera: CameraUniform,
    pub lookup: &'a [u32],
    pub background: &'a [BackgroundCell],
    pub slot_count: u32,
    pub updates: &'a [ChunkSlotUpload],
    /// Frame-local solid geometry. It never enters the revisioned static
    /// cache, so moving actors and doors cannot invalidate map residency.
    pub dynamic_voxels: &'a [DynamicVoxel],
    /// Ordered mixed-resolution presentation assets. They remain separate
    /// from static chunk residency and dynamic unit voxels.
    pub assets: &'a [RenderAsset],
    pub asset_voxels: &'a [AssetVoxel],
}

/// A single authoritative, frame-local solid voxel. Request order is stable
/// and later equal-coordinate entries intentionally win, matching ordered
/// simulation stamping.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DynamicVoxel {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub material: u32,
}

/// Sparse local cell in a mixed-resolution render asset.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AssetVoxel {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub material: u32,
}

/// GPU presentation record for one transformed asset. `voxel_offset` and
/// `dimensions[3]` address the ordered `asset_voxels` snapshot.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderAsset {
    pub min: [f32; 4],
    pub max: [f32; 4],
    pub anchor: [f32; 4],
    pub voxel_size: f32,
    pub yaw_degrees: f32,
    pub ghost: u32,
    pub voxel_offset: u32,
    pub dimensions: [u32; 4],
    pub pivot: [f32; 4],
}

/// The terrain source for one complete GPU frame.  A direct DDA request keeps
/// voxel terrain on the GPU; `Empty` is useful for a complete logical scene
/// supplied by the caller (menus and the CPU reference fallback bridge).
/// Both variants retain a logical background so the terrain pass always
/// initializes every cell deterministically.
pub enum TerrainSource<'a> {
    Dda(TerrainFrame<'a>),
    Empty { background: &'a [BackgroundCell] },
}

/// One post-simulation presentation request.  This is the only public frame
/// submission boundary: terrain/background, ordered logical cells, physical
/// pixel sprites, and final text overlays are supplied together.  It keeps
/// GPU cache state strictly presentation-only while preventing a caller from
/// accidentally reusing UI data from a previous frame.
pub struct RenderRequest<'a> {
    pub terrain: TerrainSource<'a>,
    pub scene_cells: &'a [UiCell],
    pub pixel_sprites: &'a [PixelSprite],
    pub overlay_cells: &'a [UiCell],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainUploadStats {
    pub dirty_chunks: u32,
    pub bytes_uploaded: u64,
    pub voxel_capacity_bytes: u64,
    pub dynamic_voxels: u32,
    pub dynamic_capacity_bytes: u64,
    pub assets: u32,
    pub asset_capacity_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainDataError {
    EmptyLookup,
    InvalidBackgroundLength { actual: usize },
    InvalidChunkLength { slot: u32, actual: usize },
    SlotOutsideCapacity { slot: u32, slot_count: u32 },
}

impl fmt::Display for TerrainDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLookup => write!(f, "terrain chunk lookup must contain at least one entry"),
            Self::InvalidBackgroundLength { actual } => write!(
                f,
                "terrain background contains {actual} cells; expected {}",
                LOGICAL_WIDTH * LOGICAL_HEIGHT
            ),
            Self::InvalidChunkLength { slot, actual } => write!(
                f,
                "terrain chunk slot {slot} contains {actual} voxels; expected {CHUNK_VOXELS}"
            ),
            Self::SlotOutsideCapacity { slot, slot_count } => write!(
                f,
                "terrain chunk slot {slot} is outside slot count {slot_count}"
            ),
        }
    }
}

impl std::error::Error for TerrainDataError {}

#[derive(Debug)]
pub enum RenderRequestError {
    Terrain(TerrainDataError),
    Surface(SurfaceRendererError),
}

impl fmt::Display for RenderRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terrain(error) => error.fmt(f),
            Self::Surface(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for RenderRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Terrain(error) => Some(error),
            Self::Surface(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum SurfaceRendererError {
    CreateSurface(wgpu::CreateSurfaceError),
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
    NoSurfaceFormat,
    SurfaceValidation,
}

impl fmt::Display for SurfaceRendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSurface(error) => write!(f, "could not create GPU surface: {error}"),
            Self::NoAdapter => write!(f, "no compatible GPU adapter was available"),
            Self::RequestDevice(error) => write!(f, "could not request GPU device: {error}"),
            Self::NoSurfaceFormat => write!(f, "GPU surface reported no usable texture format"),
            Self::SurfaceValidation => {
                write!(f, "GPU surface acquisition reported a validation error")
            }
        }
    }
}

impl std::error::Error for SurfaceRendererError {}

impl<'window> SurfaceRenderer<'window> {
    /// Synchronously create a presentation surface for a native `winit`
    /// window. GPU initialization happens once during backend selection, not
    /// in the frame loop.
    pub fn new(window: &'window Window) -> Result<Self, SurfaceRendererError> {
        pollster::block_on(Self::new_async(window))
    }

    /// Async form for applications that already own an async startup path.
    pub async fn new_async(window: &'window Window) -> Result<Self, SurfaceRendererError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
        let surface = instance
            .create_surface(window)
            .map_err(SurfaceRendererError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| SurfaceRendererError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("heliobound GPU renderer device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(SurfaceRendererError::RequestDevice)?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(SurfaceRendererError::NoSurfaceFormat)?;
        let present_mode = if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            wgpu::PresentMode::Fifo
        } else {
            capabilities.present_modes[0]
        };
        let alpha_mode = capabilities.alpha_modes[0];
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let diagnostic_pipeline = create_diagnostic_pipeline(&device, format);
        let terrain_bind_group_layout = create_terrain_bind_group_layout(&device);
        let terrain_pipeline = create_terrain_pipeline(&device, &terrain_bind_group_layout);
        let terrain = TerrainGpuBuffers::new(&device, &terrain_bind_group_layout);
        let logical_targets = LogicalTargets::new(&device);
        let glyph_atlas = create_glyph_atlas(&device, &queue);
        let presentation = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heliobound glyph presentation uniform"),
            size: std::mem::size_of::<PresentationUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glyph_layout = create_glyph_bind_group_layout(&device);
        let glyph_bind_group = create_glyph_bind_group(
            &device,
            &glyph_layout,
            &logical_targets,
            &glyph_atlas,
            &presentation,
        );
        let glyph_pipeline = create_glyph_pipeline(&device, format, &glyph_layout);
        let ui_bind_group_layout = create_ui_bind_group_layout(&device);
        let ui = UiGpuBuffer::new(&device, &ui_bind_group_layout, &glyph_atlas, &presentation);
        let overlay_ui =
            UiGpuBuffer::new(&device, &ui_bind_group_layout, &glyph_atlas, &presentation);
        let ui_pipeline = create_ui_pipeline(&device, format, &ui_bind_group_layout);
        let sprite_bind_group_layout = create_sprite_bind_group_layout(&device);
        let sprites = SpriteGpuBuffer::new(&device, &sprite_bind_group_layout, &presentation);
        let sprite_pipeline = create_sprite_pipeline(&device, format, &sprite_bind_group_layout);

        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            config,
            size,
            diagnostic_pipeline,
            terrain_pipeline,
            terrain_bind_group_layout,
            terrain,
            logical_targets,
            glyph_pipeline,
            glyph_bind_group,
            ui_pipeline,
            ui_bind_group_layout,
            ui,
            ui_count: 0,
            overlay_ui,
            overlay_ui_count: 0,
            sprite_pipeline,
            sprite_bind_group_layout,
            sprites,
            sprite_count: 0,
            glyph_atlas,
            presentation,
        })
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Replace the logical HUD/overlay cells for the next GPU frame. This is
    /// independent of terrain uploads and supports menus, reticles, markers,
    /// and diagnostic text without a CPU framebuffer.
    pub fn set_ui_cells(&mut self, cells: &[UiCell]) {
        self.ui.ensure_capacity(
            &self.device,
            &self.ui_bind_group_layout,
            &self.glyph_atlas,
            &self.presentation,
            cells.len() as u64,
        );
        if !cells.is_empty() {
            self.queue
                .write_buffer(&self.ui.cells, 0, bytemuck::cast_slice(cells));
        }
        self.ui_count = cells.len() as u32;
    }

    /// Replace text overlay cells. These deliberately render after pixel
    /// sprites, matching the CPU painter's final overlay pass.
    pub fn set_overlay_cells(&mut self, cells: &[UiCell]) {
        self.overlay_ui.ensure_capacity(
            &self.device,
            &self.ui_bind_group_layout,
            &self.glyph_atlas,
            &self.presentation,
            cells.len() as u64,
        );
        if !cells.is_empty() {
            self.queue
                .write_buffer(&self.overlay_ui.cells, 0, bytemuck::cast_slice(cells));
        }
        self.overlay_ui_count = cells.len() as u32;
    }

    /// Replace static pixel sprites for the next frame. The caller preserves
    /// painter order; this pass is rendered after logical scene cells and
    /// before text overlays.
    pub fn set_pixel_sprites(&mut self, sprites: &[PixelSprite]) {
        self.sprites.ensure_capacity(
            &self.device,
            &self.sprite_bind_group_layout,
            &self.presentation,
            sprites.len() as u64,
        );
        if !sprites.is_empty() {
            self.queue
                .write_buffer(&self.sprites.sprites, 0, bytemuck::cast_slice(sprites));
        }
        self.sprite_count = sprites.len() as u32;
    }

    /// Submit a complete presentation request.  The individual upload/setter
    /// methods remain available for focused renderer tests, but application
    /// code should use this method so every submitted frame replaces all
    /// transient composition buffers.
    pub fn render(
        &mut self,
        request: RenderRequest<'_>,
    ) -> Result<TerrainUploadStats, RenderRequestError> {
        let upload = match request.terrain {
            TerrainSource::Dda(frame) => self.update_terrain(frame),
            TerrainSource::Empty { background } => self.update_terrain(TerrainFrame {
                camera: CameraUniform::from_camera(
                    Camera::new(heliobound_core::Vec3::ZERO),
                    VoxelBounds::new(heliobound_core::VoxelCoord::new(0, 0, 0)),
                    ChunkTableLayout::new(ChunkCoord::new(0, 0, 0), [1; 3]),
                    1,
                ),
                lookup: &[0],
                background,
                slot_count: 0,
                updates: &[],
                dynamic_voxels: &[],
                assets: &[],
                asset_voxels: &[],
            }),
        }
        .map_err(RenderRequestError::Terrain)?;
        self.set_ui_cells(request.scene_cells);
        self.set_pixel_sprites(request.pixel_sprites);
        self.set_overlay_cells(request.overlay_cells);
        self.render_terrain().map_err(RenderRequestError::Surface)?;
        Ok(upload)
    }

    /// Synchronize the direct-GPU terrain cache. The CPU remains authoritative:
    /// this method only writes the current camera/table and the caller-selected
    /// dirty chunk slots. It performs no GPU-to-CPU readback.
    pub fn update_terrain(
        &mut self,
        frame: TerrainFrame<'_>,
    ) -> Result<TerrainUploadStats, TerrainDataError> {
        if frame.lookup.is_empty() {
            return Err(TerrainDataError::EmptyLookup);
        }
        if frame.background.len() != (LOGICAL_WIDTH * LOGICAL_HEIGHT) as usize {
            return Err(TerrainDataError::InvalidBackgroundLength {
                actual: frame.background.len(),
            });
        }
        for update in frame.updates {
            if update.upload.voxels.len() != CHUNK_VOXELS as usize {
                return Err(TerrainDataError::InvalidChunkLength {
                    slot: update.slot,
                    actual: update.upload.voxels.len(),
                });
            }
            if update.slot >= frame.slot_count {
                return Err(TerrainDataError::SlotOutsideCapacity {
                    slot: update.slot,
                    slot_count: frame.slot_count,
                });
            }
        }
        self.terrain.ensure_capacity(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            frame.lookup.len() as u64,
            frame.slot_count.max(1) as u64 * CHUNK_VOXELS as u64,
            frame.dynamic_voxels.len() as u64,
            frame.assets.len() as u64,
            frame.asset_voxels.len() as u64,
        );
        let mut camera = frame.camera;
        // The otherwise padding `w` is an explicit frame-local count; the
        // static lookup dimensions remain xyz.
        camera.table_dimensions_and_padding[3] = frame.dynamic_voxels.len() as u32;
        camera.up_and_padding[3] = frame.assets.len() as f32;
        self.queue
            .write_buffer(&self.terrain.camera, 0, bytemuck::bytes_of(&camera));
        self.queue
            .write_buffer(&self.terrain.lookup, 0, bytemuck::cast_slice(frame.lookup));
        self.queue.write_buffer(
            &self.terrain.background,
            0,
            bytemuck::cast_slice(frame.background),
        );
        let mut bytes_uploaded = CameraUniform::BYTE_SIZE
            + (frame.lookup.len() * std::mem::size_of::<u32>()) as u64
            + std::mem::size_of_val(frame.background) as u64;
        for update in frame.updates {
            let offset =
                update.slot as u64 * CHUNK_VOXELS as u64 * std::mem::size_of::<u32>() as u64;
            self.queue.write_buffer(
                &self.terrain.voxels,
                offset,
                bytemuck::cast_slice(&update.upload.voxels),
            );
            bytes_uploaded += (CHUNK_VOXELS as usize * std::mem::size_of::<u32>()) as u64;
        }
        if !frame.dynamic_voxels.is_empty() {
            self.queue.write_buffer(
                &self.terrain.dynamic_voxels,
                0,
                bytemuck::cast_slice(frame.dynamic_voxels),
            );
            bytes_uploaded += std::mem::size_of_val(frame.dynamic_voxels) as u64;
        }
        if !frame.assets.is_empty() {
            self.queue
                .write_buffer(&self.terrain.assets, 0, bytemuck::cast_slice(frame.assets));
            bytes_uploaded += std::mem::size_of_val(frame.assets) as u64;
        }
        if !frame.asset_voxels.is_empty() {
            self.queue.write_buffer(
                &self.terrain.asset_voxels,
                0,
                bytemuck::cast_slice(frame.asset_voxels),
            );
            bytes_uploaded += std::mem::size_of_val(frame.asset_voxels) as u64;
        }
        Ok(TerrainUploadStats {
            dirty_chunks: frame.updates.len() as u32,
            bytes_uploaded,
            voxel_capacity_bytes: self.terrain.voxel_capacity,
            dynamic_voxels: frame.dynamic_voxels.len() as u32,
            dynamic_capacity_bytes: self.terrain.dynamic_capacity,
            assets: frame.assets.len() as u32,
            asset_capacity_bytes: self.terrain.asset_capacity + self.terrain.asset_voxel_capacity,
        })
    }

    /// Reconfigure after a window resize. A zero-sized minimized surface is
    /// retained but not submitted until it receives a non-zero size again.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Draw the diagnostic fullscreen triangle. Surface loss, outdating, and
    /// suboptimal acquisition are recovered internally. A device-lost callback
    /// still belongs to the CLI backend selector, which can fall back to CPU.
    pub fn render_diagnostic(&mut self) -> Result<(), SurfaceRendererError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }
        let mut reconfigure_after_present = false;
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                reconfigure_after_present = true;
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(SurfaceRendererError::SurfaceValidation)
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("heliobound diagnostic surface encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heliobound diagnostic fullscreen pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.diagnostic_pipeline);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        if reconfigure_after_present {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }

    /// Render one DDA invocation per logical ASCII cell, then compose its
    /// glyphs (and any UI cells) directly to the presentation surface. The
    /// interactive path performs no terrain or framebuffer readback.
    pub fn render_terrain(&mut self) -> Result<(), SurfaceRendererError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }
        let mut reconfigure_after_present = false;
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                reconfigure_after_present = true;
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(SurfaceRendererError::SurfaceValidation)
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("heliobound terrain surface encoder"),
            });
        // The terrain target is deliberately fixed at 160 by 90: physical
        // window size changes only alter the crisp glyph compositor.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heliobound logical terrain DDA pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.logical_targets.glyphs,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.logical_targets.colours,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.terrain_pipeline);
            pass.set_bind_group(0, &self.terrain.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.write_buffer(
            &self.presentation,
            0,
            bytemuck::bytes_of(&presentation_uniform(
                self.size,
                self.config.format.is_srgb(),
            )),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heliobound glyph upscale pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.glyph_pipeline);
            pass.set_bind_group(0, &self.glyph_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if self.ui_count != 0 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heliobound logical UI glyph pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_bind_group(0, &self.ui.bind_group, &[]);
            pass.draw(0..6, 0..self.ui_count);
        }
        if self.sprite_count != 0 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heliobound pixel sprite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, &self.sprites.bind_group, &[]);
            pass.draw(0..6, 0..self.sprite_count);
        }
        if self.overlay_ui_count != 0 {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("heliobound text overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_bind_group(0, &self.overlay_ui.bind_group, &[]);
            pass.draw(0..6, 0..self.overlay_ui_count);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        if reconfigure_after_present {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }

    fn recreate_surface(&mut self) -> Result<(), SurfaceRendererError> {
        self.surface = self
            .instance
            .create_surface(self.window)
            .map_err(SurfaceRendererError::CreateSurface)?;
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }
}

impl TerrainGpuBuffers {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heliobound terrain camera uniform"),
            size: CameraUniform::BYTE_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lookup = create_storage_buffer(device, "heliobound terrain chunk lookup", 1);
        let background = create_storage_buffer(
            device,
            "heliobound logical sky cells",
            (LOGICAL_WIDTH * LOGICAL_HEIGHT * 2) as u64,
        );
        let voxels = create_storage_buffer(
            device,
            "heliobound terrain chunk slots",
            CHUNK_VOXELS as u64,
        );
        let dynamic_voxels = create_storage_buffer(device, "heliobound dynamic voxels", 4);
        let assets = create_storage_buffer(device, "heliobound render assets", 24);
        let asset_voxels = create_storage_buffer(device, "heliobound render asset voxels", 4);
        let bind_group = create_terrain_bind_group(
            device,
            layout,
            &camera,
            &lookup,
            &background,
            &voxels,
            &dynamic_voxels,
            &assets,
            &asset_voxels,
        );
        Self {
            camera,
            lookup,
            background,
            voxels,
            dynamic_voxels,
            assets,
            asset_voxels,
            bind_group,
            lookup_capacity: 1,
            voxel_capacity: CHUNK_VOXELS as u64 * 4,
            dynamic_capacity: 16,
            asset_capacity: 96,
            asset_voxel_capacity: 16,
        }
    }

    fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        lookup_values: u64,
        voxel_values: u64,
        dynamic_values: u64,
        asset_values: u64,
        asset_voxel_values: u64,
    ) {
        let required_lookup = lookup_values.max(1);
        let required_voxels = voxel_values.max(1) * 4;
        let mut changed = false;
        if required_lookup > self.lookup_capacity {
            self.lookup_capacity = required_lookup.next_power_of_two();
            self.lookup = create_storage_buffer(
                device,
                "heliobound terrain chunk lookup",
                self.lookup_capacity,
            );
            changed = true;
        }
        if required_voxels > self.voxel_capacity {
            // Preserve existing slots during geometric buffer growth. New slots
            // are subsequently initialized exclusively by dirty uploads.
            let new_capacity = required_voxels.next_power_of_two();
            let new_voxels =
                create_storage_buffer(device, "heliobound terrain chunk slots", new_capacity / 4);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("heliobound terrain slot grow copy"),
            });
            encoder.copy_buffer_to_buffer(&self.voxels, 0, &new_voxels, 0, self.voxel_capacity);
            queue.submit([encoder.finish()]);
            self.voxels = new_voxels;
            self.voxel_capacity = new_capacity;
            changed = true;
        }
        let required_dynamic =
            (dynamic_values.max(1) * std::mem::size_of::<DynamicVoxel>() as u64).max(16);
        if required_dynamic > self.dynamic_capacity {
            self.dynamic_capacity = required_dynamic.next_power_of_two();
            self.dynamic_voxels = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("heliobound dynamic voxels"),
                size: self.dynamic_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            changed = true;
        }
        let required_assets =
            (asset_values.max(1) * std::mem::size_of::<RenderAsset>() as u64).max(96);
        if required_assets > self.asset_capacity {
            self.asset_capacity = required_assets.next_power_of_two();
            self.assets = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("heliobound render assets"),
                size: self.asset_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            changed = true;
        }
        let required_asset_voxels =
            (asset_voxel_values.max(1) * std::mem::size_of::<AssetVoxel>() as u64).max(16);
        if required_asset_voxels > self.asset_voxel_capacity {
            self.asset_voxel_capacity = required_asset_voxels.next_power_of_two();
            self.asset_voxels = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("heliobound render asset voxels"),
                size: self.asset_voxel_capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            changed = true;
        }
        if changed {
            self.bind_group = create_terrain_bind_group(
                device,
                layout,
                &self.camera,
                &self.lookup,
                &self.background,
                &self.voxels,
                &self.dynamic_voxels,
                &self.assets,
                &self.asset_voxels,
            );
        }
    }
}

fn create_storage_buffer(device: &wgpu::Device, label: &'static str, values: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (values.max(1) * std::mem::size_of::<u32>() as u64).max(4),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

fn create_terrain_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("heliobound terrain bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(CameraUniform::BYTE_SIZE),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    // `BackgroundCell` contains a glyph and a packed RGBA
                    // foreground, so each storage-array element occupies two
                    // u32 values in WGSL. Keeping this layout contract exact
                    // matters on backends that validate the shader-required
                    // binding size while the terrain pipeline is created.
                    min_binding_size: wgpu::BufferSize::new(8),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(96),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            },
        ],
    })
}

fn create_terrain_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    lookup: &wgpu::Buffer,
    background: &wgpu::Buffer,
    voxels: &wgpu::Buffer,
    dynamic_voxels: &wgpu::Buffer,
    assets: &wgpu::Buffer,
    asset_voxels: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("heliobound terrain bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: lookup.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: background.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: voxels.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: dynamic_voxels.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: assets.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: asset_voxels.as_entire_binding(),
            },
        ],
    })
}

fn create_diagnostic_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("heliobound diagnostic fullscreen shader"),
        source: wgpu::ShaderSource::Wgsl(DDA_SHADER.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("heliobound diagnostic fullscreen pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_diagnostic"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_terrain_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("heliobound terrain DDA shader"),
        source: wgpu::ShaderSource::Wgsl(DDA_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("heliobound terrain pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("heliobound terrain DDA pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_terrain"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    // Authored terrain colours are sRGB bytes. The terrain
                    // shader writes their linear equivalents, and the sRGB
                    // target preserves the authored bytes for the glyph pass.
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl LogicalTargets {
    fn new(device: &wgpu::Device) -> Self {
        let create = |label, format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: LOGICAL_WIDTH,
                    height: LOGICAL_HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let glyph_texture = create("heliobound logical glyph IDs", wgpu::TextureFormat::R32Uint);
        let colour_texture = create(
            "heliobound logical glyph colours",
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        Self {
            glyphs: glyph_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            glyph_texture,
            colours: colour_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            colour_texture,
        }
    }
}

fn create_glyph_atlas(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    let width = GLYPH_ATLAS_COLUMNS * GLYPH_WIDTH;
    let height = GLYPH_ATLAS_ROWS * GLYPH_HEIGHT;
    let mut pixels = vec![0u8; (width * height) as usize];
    for code in 0u32..256 {
        let glyph = char::from_u32(code)
            .and_then(|ch| BASIC_FONTS.get(ch))
            .or_else(|| BASIC_FONTS.get('?'));
        let Some(rows) = glyph else { continue };
        let ox = (code % GLYPH_ATLAS_COLUMNS) * GLYPH_WIDTH;
        let oy = (code / GLYPH_ATLAS_COLUMNS) * GLYPH_HEIGHT;
        for (y, bits) in rows.iter().enumerate() {
            for x in 0..8u32 {
                if bits & (1 << x) != 0 {
                    pixels[((oy + y as u32) * width + ox + x) as usize] = 255;
                }
            }
        }
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("heliobound immutable font8x8 glyph atlas"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn presentation_uniform(size: PhysicalSize<u32>, surface_is_srgb: bool) -> PresentationUniform {
    let scale = ((size.width / LOGICAL_WIDTH).min(size.height / LOGICAL_HEIGHT)).max(1) as f32;
    let used_width = LOGICAL_WIDTH as f32 * scale;
    let used_height = LOGICAL_HEIGHT as f32 * scale;
    PresentationUniform {
        physical_size: [size.width as f32, size.height as f32],
        logical_size: [LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32],
        scale_and_origin: [
            scale,
            (size.width as f32 - used_width) * 0.5,
            (size.height as f32 - used_height) * 0.5,
            0.0,
        ],
        surface_is_srgb: [surface_is_srgb as u32, 0, 0, 0],
    }
}

fn create_glyph_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("heliobound glyph compose bind layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}
fn create_glyph_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    targets: &LogicalTargets,
    atlas: &wgpu::TextureView,
    presentation: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("heliobound glyph compose bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&targets.glyphs),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&targets.colours),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(atlas),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: presentation.as_entire_binding(),
            },
        ],
    })
}
fn create_glyph_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_graphics_pipeline(
        device,
        format,
        layout,
        "fs_glyph",
        "heliobound glyph compositor",
        None,
    )
}

fn create_ui_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("heliobound UI cells bind layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}
impl UiGpuBuffer {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        atlas: &wgpu::TextureView,
        presentation: &wgpu::Buffer,
    ) -> Self {
        let cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heliobound UI cells"),
            size: std::mem::size_of::<UiCell>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = create_ui_bind_group(device, layout, &cells, atlas, presentation);
        Self {
            cells,
            bind_group,
            capacity: 1,
        }
    }
    fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        atlas: &wgpu::TextureView,
        presentation: &wgpu::Buffer,
        count: u64,
    ) {
        if count <= self.capacity {
            return;
        }
        self.capacity = count.next_power_of_two();
        self.cells = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heliobound UI cells"),
            size: self.capacity * std::mem::size_of::<UiCell>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.bind_group = create_ui_bind_group(device, layout, &self.cells, atlas, presentation);
    }
}
fn create_ui_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    cells: &wgpu::Buffer,
    atlas: &wgpu::TextureView,
    presentation: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("heliobound UI cells bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: cells.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(atlas),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: presentation.as_entire_binding(),
            },
        ],
    })
}
fn create_ui_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_graphics_pipeline(
        device,
        format,
        layout,
        "fs_ui",
        "heliobound UI glyph pipeline",
        Some("vs_ui"),
    )
}
fn create_sprite_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("heliobound pixel sprites bind layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn create_sprite_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sprites: &wgpu::Buffer,
    presentation: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("heliobound pixel sprites bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: sprites.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: presentation.as_entire_binding(),
            },
        ],
    })
}

impl SpriteGpuBuffer {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        presentation: &wgpu::Buffer,
    ) -> Self {
        let sprites = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heliobound pixel sprites"),
            size: std::mem::size_of::<PixelSprite>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = create_sprite_bind_group(device, layout, &sprites, presentation);
        Self {
            sprites,
            bind_group,
            capacity: 1,
        }
    }

    fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        presentation: &wgpu::Buffer,
        count: u64,
    ) {
        if count <= self.capacity {
            return;
        }
        self.capacity = count.next_power_of_two();
        self.sprites = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heliobound pixel sprites"),
            size: self.capacity * std::mem::size_of::<PixelSprite>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.bind_group = create_sprite_bind_group(device, layout, &self.sprites, presentation);
    }
}

fn create_sprite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_graphics_pipeline(
        device,
        format,
        layout,
        "fs_sprite",
        "heliobound pixel sprite pipeline",
        Some("vs_sprite"),
    )
}
fn create_graphics_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
    fragment: &str,
    label: &'static str,
    vertex_entry: Option<&str>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(GLYPH_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: vertex_entry.or(Some("vs_fullscreen")),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(fragment),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use heliobound_core::{Vec3, VoxelCell, VoxelCoord, VoxelWorld};
    use heliobound_gfx::{
        background_glyph_for_direction, raycast, GraphicsConfig, MaterialGlyphMap,
        RenderAsset as CpuRenderAsset, SceneBuilder, Viewport,
    };
    use std::sync::mpsc;

    /// Adapter-backed test helper. It renders the same logical 160 by 90
    /// terrain targets as the interactive path, then reads those test-only
    /// targets back as glyph IDs and RGBA cells.
    struct OffscreenTerrainReadback {
        device: wgpu::Device,
        queue: wgpu::Queue,
        terrain_layout: wgpu::BindGroupLayout,
        terrain_pipeline: wgpu::RenderPipeline,
    }

    impl OffscreenTerrainReadback {
        fn new() -> Option<Self> {
            pollster::block_on(async {
                let instance = wgpu::Instance::new(
                    wgpu::InstanceDescriptor::new_without_display_handle().with_env(),
                );
                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        compatible_surface: None,
                        force_fallback_adapter: false,
                    })
                    .await
                    .ok()?;
                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        label: Some("heliobound offscreen terrain parity device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        experimental_features: wgpu::ExperimentalFeatures::disabled(),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                        trace: wgpu::Trace::Off,
                    })
                    .await
                    .ok()?;
                let terrain_layout = create_terrain_bind_group_layout(&device);
                let terrain_pipeline = create_terrain_pipeline(&device, &terrain_layout);
                Some(Self {
                    device,
                    queue,
                    terrain_layout,
                    terrain_pipeline,
                })
            })
        }

        fn render(
            &self,
            world: &VoxelWorld,
            camera: Camera,
            dynamic_voxels: &[DynamicVoxel],
            assets: &[RenderAsset],
            asset_voxels: &[AssetVoxel],
        ) -> Vec<(u32, [u8; 4])> {
            // Dynamic cells and asset broad-phase bounds must participate in
            // the slab just as they do in the interactive request path. This
            // intentionally permits an otherwise empty static world.
            let mut bounds = world.bounds();
            for voxel in dynamic_voxels {
                let coord = VoxelCoord::new(voxel.x, voxel.y, voxel.z);
                match &mut bounds {
                    Some(bounds) => bounds.include(coord),
                    None => bounds = Some(VoxelBounds::new(coord)),
                }
            }
            for asset in assets {
                let min = VoxelCoord::new(
                    asset.min[0].floor() as i32,
                    asset.min[1].floor() as i32,
                    asset.min[2].floor() as i32,
                );
                let max = VoxelCoord::new(
                    (asset.max[0].ceil() as i32).saturating_sub(1),
                    (asset.max[1].ceil() as i32).saturating_sub(1),
                    (asset.max[2].ceil() as i32).saturating_sub(1),
                );
                match &mut bounds {
                    Some(bounds) => {
                        bounds.include(min);
                        bounds.include(max);
                    }
                    None => {
                        let mut asset_bounds = VoxelBounds::new(min);
                        asset_bounds.include(max);
                        bounds = Some(asset_bounds);
                    }
                }
            }
            let bounds = bounds.expect("parity fixture must contain geometry");
            let min = ChunkCoord::new(
                bounds.min.x.div_euclid(CHUNK_EDGE as i32),
                bounds.min.y.div_euclid(CHUNK_EDGE as i32),
                bounds.min.z.div_euclid(CHUNK_EDGE as i32),
            );
            let max = ChunkCoord::new(
                bounds.max.x.div_euclid(CHUNK_EDGE as i32),
                bounds.max.y.div_euclid(CHUNK_EDGE as i32),
                bounds.max.z.div_euclid(CHUNK_EDGE as i32),
            );
            let layout = ChunkTableLayout::new(
                min,
                [
                    (max.x - min.x + 1) as u32,
                    (max.y - min.y + 1) as u32,
                    (max.z - min.z + 1) as u32,
                ],
            );
            let snapshots = world.chunk_snapshots_in(min, max);
            let mut resident = ResidentChunkTable::new(layout);
            let updates = resident.sync_visible_snapshots(&snapshots);
            let lookup = resident.lookup_entries();
            let camera_uniform = CameraUniform::from_camera(camera, bounds, layout, 16_384);
            let camera_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("heliobound offscreen terrain camera"),
                size: CameraUniform::BYTE_SIZE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let lookup_buffer = create_storage_buffer(
                &self.device,
                "heliobound offscreen terrain lookup",
                lookup.len() as u64,
            );
            let voxel_buffer = create_storage_buffer(
                &self.device,
                "heliobound offscreen terrain voxels",
                resident.slot_count().max(1) as u64 * CHUNK_VOXELS as u64,
            );
            self.queue
                .write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
            self.queue
                .write_buffer(&lookup_buffer, 0, bytemuck::cast_slice(&lookup));
            let background = (0..LOGICAL_HEIGHT as usize)
                .flat_map(|y| {
                    (0..LOGICAL_WIDTH as usize).map(move |x| BackgroundCell {
                        glyph: background_glyph_for_direction(
                            camera
                                .ray_for_cell(x, y, LOGICAL_WIDTH as usize, LOGICAL_HEIGHT as usize)
                                .direction,
                        ) as u32,
                        foreground_rgba: 0x505866ff,
                    })
                })
                .collect::<Vec<_>>();
            let background_buffer = create_storage_buffer(
                &self.device,
                "heliobound offscreen terrain sky",
                (LOGICAL_WIDTH * LOGICAL_HEIGHT * 2) as u64,
            );
            self.queue
                .write_buffer(&background_buffer, 0, bytemuck::cast_slice(&background));
            let dynamic_buffer = create_storage_buffer(
                &self.device,
                "heliobound offscreen dynamic voxels",
                (dynamic_voxels.len().max(1) * std::mem::size_of::<DynamicVoxel>()) as u64,
            );
            let asset_buffer = create_storage_buffer(
                &self.device,
                "heliobound offscreen render assets",
                (assets.len().max(1) * std::mem::size_of::<RenderAsset>()) as u64,
            );
            let asset_voxel_buffer = create_storage_buffer(
                &self.device,
                "heliobound offscreen render asset voxels",
                (asset_voxels.len().max(1) * std::mem::size_of::<AssetVoxel>()) as u64,
            );
            for update in &updates {
                self.queue.write_buffer(
                    &voxel_buffer,
                    update.slot as u64 * CHUNK_VOXELS as u64 * 4,
                    bytemuck::cast_slice(&update.upload.voxels),
                );
            }
            if !dynamic_voxels.is_empty() {
                self.queue
                    .write_buffer(&dynamic_buffer, 0, bytemuck::cast_slice(dynamic_voxels));
            }
            if !assets.is_empty() {
                self.queue
                    .write_buffer(&asset_buffer, 0, bytemuck::cast_slice(assets));
            }
            if !asset_voxels.is_empty() {
                self.queue
                    .write_buffer(&asset_voxel_buffer, 0, bytemuck::cast_slice(asset_voxels));
            }
            let mut camera_uniform = camera_uniform;
            camera_uniform.table_dimensions_and_padding[3] = dynamic_voxels.len() as u32;
            camera_uniform.up_and_padding[3] = assets.len() as f32;
            self.queue
                .write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
            let bind_group = create_terrain_bind_group(
                &self.device,
                &self.terrain_layout,
                &camera_buffer,
                &lookup_buffer,
                &background_buffer,
                &voxel_buffer,
                &dynamic_buffer,
                &asset_buffer,
                &asset_voxel_buffer,
            );
            let targets = LogicalTargets::new(&self.device);
            let byte_len = (LOGICAL_WIDTH * LOGICAL_HEIGHT * 4) as u64;
            let glyph_readback = readback_buffer(
                &self.device,
                "heliobound offscreen glyph readback",
                byte_len,
            );
            let colour_readback = readback_buffer(
                &self.device,
                "heliobound offscreen colour readback",
                byte_len,
            );
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("heliobound offscreen terrain parity encoder"),
                });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("heliobound offscreen terrain parity pass"),
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &targets.glyphs,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                        Some(wgpu::RenderPassColorAttachment {
                            view: &targets.colours,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.terrain_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            for (texture, buffer) in [
                (&targets.glyph_texture, &glyph_readback),
                (&targets.colour_texture, &colour_readback),
            ] {
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(LOGICAL_WIDTH * 4),
                            rows_per_image: Some(LOGICAL_HEIGHT),
                        },
                    },
                    wgpu::Extent3d {
                        width: LOGICAL_WIDTH,
                        height: LOGICAL_HEIGHT,
                        depth_or_array_layers: 1,
                    },
                );
            }
            self.queue.submit([encoder.finish()]);
            let glyph_bytes = map_readback(&self.device, &glyph_readback);
            let colour_bytes = map_readback(&self.device, &colour_readback);
            glyph_bytes
                .chunks_exact(4)
                .zip(colour_bytes.chunks_exact(4))
                .map(|(glyph, colour)| {
                    (
                        u32::from_le_bytes(glyph.try_into().expect("u32 glyph")),
                        colour.try_into().expect("RGBA colour"),
                    )
                })
                .collect()
        }

        /// Exercise the final glyph, UI, sprite, and overlay compositor against
        /// a readback target. This is deliberately separate from logical DDA
        /// parity: a correct terrain target can still present wrong bytes.
        fn render_presentation(&self, format: wgpu::TextureFormat) -> Vec<u8> {
            let size = PhysicalSize::new(LOGICAL_WIDTH * 8, LOGICAL_HEIGHT * 8);
            let targets = LogicalTargets::new(&self.device);
            let atlas = create_glyph_atlas(&self.device, &self.queue);
            let presentation = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("heliobound offscreen presentation uniform"),
                size: std::mem::size_of::<PresentationUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(
                &presentation,
                0,
                bytemuck::bytes_of(&presentation_uniform(size, format.is_srgb())),
            );
            let glyph_layout = create_glyph_bind_group_layout(&self.device);
            let glyph_bind_group = create_glyph_bind_group(
                &self.device,
                &glyph_layout,
                &targets,
                &atlas,
                &presentation,
            );
            let glyph_pipeline = create_glyph_pipeline(&self.device, format, &glyph_layout);
            let ui_layout = create_ui_bind_group_layout(&self.device);
            let scene_cells =
                [UiCell::new(11, 10, 'A', 0xf0_c6_5b_ff).with_background(0x0c_22_38_ff)];
            let overlay_cells =
                [UiCell::new(10, 10, 'A', 0xc8_dc_f0_ff).with_background(0x1a_24_36_ff)];
            let scene_ui = UiGpuBuffer::new(&self.device, &ui_layout, &atlas, &presentation);
            let overlay_ui = UiGpuBuffer::new(&self.device, &ui_layout, &atlas, &presentation);
            self.queue
                .write_buffer(&scene_ui.cells, 0, bytemuck::cast_slice(&scene_cells));
            self.queue
                .write_buffer(&overlay_ui.cells, 0, bytemuck::cast_slice(&overlay_cells));
            let ui_pipeline = create_ui_pipeline(&self.device, format, &ui_layout);
            let sprite_layout = create_sprite_bind_group_layout(&self.device);
            let sprites = SpriteGpuBuffer::new(&self.device, &sprite_layout, &presentation);
            let sprite = PixelSprite {
                x: 200,
                y: 200,
                scale: 1,
                flags: PixelSprite::OPAQUE_BACKGROUND,
                foreground_rgba: 0xff_50_78_ff,
                background_rgba: 0x0a_14_1e_ff,
                rows: [0x8000; 16],
            };
            self.queue
                .write_buffer(&sprites.sprites, 0, bytemuck::bytes_of(&sprite));
            let sprite_pipeline = create_sprite_pipeline(&self.device, format, &sprite_layout);

            let mut glyphs = vec![0u32; (LOGICAL_WIDTH * LOGICAL_HEIGHT) as usize];
            let mut colours = vec![[0u8; 4]; (LOGICAL_WIDTH * LOGICAL_HEIGHT) as usize];
            let terrain = 3 + 3 * LOGICAL_WIDTH as usize;
            let sky = 5 + 3 * LOGICAL_WIDTH as usize;
            glyphs[terrain] = '#' as u32;
            colours[terrain] = [0xa8, 0x86, 0x62, 255];
            glyphs[sky] = '.' as u32;
            colours[sky] = [0x50, 0x58, 0x66, 255];
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &targets.glyph_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&glyphs),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(LOGICAL_WIDTH * 4),
                    rows_per_image: Some(LOGICAL_HEIGHT),
                },
                wgpu::Extent3d {
                    width: LOGICAL_WIDTH,
                    height: LOGICAL_HEIGHT,
                    depth_or_array_layers: 1,
                },
            );
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &targets.colour_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&colours),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(LOGICAL_WIDTH * 4),
                    rows_per_image: Some(LOGICAL_HEIGHT),
                },
                wgpu::Extent3d {
                    width: LOGICAL_WIDTH,
                    height: LOGICAL_HEIGHT,
                    depth_or_array_layers: 1,
                },
            );
            let output = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("heliobound offscreen final presentation"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = output.create_view(&wgpu::TextureViewDescriptor::default());
            let readback = readback_buffer(
                &self.device,
                "heliobound offscreen final presentation readback",
                (size.width * size.height * 4) as u64,
            );
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("heliobound offscreen final compositor encoder"),
                });
            let mut draw = |pipeline: &wgpu::RenderPipeline,
                            bind_group: &wgpu::BindGroup,
                            vertices,
                            instances,
                            load| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("heliobound offscreen compositor pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..vertices, 0..instances);
            };
            draw(
                &glyph_pipeline,
                &glyph_bind_group,
                3,
                1,
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            );
            draw(&ui_pipeline, &scene_ui.bind_group, 6, 1, wgpu::LoadOp::Load);
            draw(
                &sprite_pipeline,
                &sprites.bind_group,
                6,
                1,
                wgpu::LoadOp::Load,
            );
            draw(
                &ui_pipeline,
                &overlay_ui.bind_group,
                6,
                1,
                wgpu::LoadOp::Load,
            );
            drop(draw);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &output,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size.width * 4),
                        rows_per_image: Some(size.height),
                    },
                },
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );
            self.queue.submit([encoder.finish()]);
            map_readback(&self.device, &readback)
        }
    }

    fn readback_buffer(device: &wgpu::Device, label: &'static str, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }

    fn map_readback(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
        let slice = buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).expect("map receiver alive")
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("GPU device poll");
        receiver
            .recv()
            .expect("GPU map callback")
            .expect("GPU readback mapping");
        let bytes = slice.get_mapped_range().to_vec();
        buffer.unmap();
        bytes
    }

    #[test]
    fn custom_colours_are_tagged_and_chunk_uploads_are_revision_driven() {
        assert_eq!(encode_voxel(VoxelMaterial::Custom([1, 2, 3])), 0x80010203);
        let mut cells = vec![None; ChunkSnapshot::VOLUME];
        cells[0] = Some(VoxelCell::new(VoxelMaterial::Basalt));
        let snapshot = ChunkSnapshot {
            coord: ChunkCoord::new(0, 0, 0),
            revision: 1,
            cells,
        };
        let mut resident = ResidentChunks::default();
        assert_eq!(resident.updates([&snapshot]).len(), 1);
        assert!(resident.updates([&snapshot]).is_empty());
        assert_eq!(
            ChunkUpload::from(&snapshot).voxels[VoxelCoord::new(0, 0, 0).x as usize],
            2
        );
    }

    #[test]
    fn chunk_table_handles_negative_coordinates_and_reuses_only_evicted_slots() {
        let layout = ChunkTableLayout::new(ChunkCoord::new(-2, -1, -2), [4, 2, 4]);
        assert_eq!(layout.index(ChunkCoord::new(-2, -1, -2)), Some(0));
        assert_eq!(layout.index(ChunkCoord::new(1, 0, 1)), Some(31));
        assert_eq!(layout.index(ChunkCoord::new(-3, -1, -2)), None);

        let snapshot = |coord, revision| ChunkSnapshot {
            coord,
            revision,
            cells: vec![None; ChunkSnapshot::VOLUME],
        };
        let left = snapshot(ChunkCoord::new(-1, 0, -1), 1);
        let right = snapshot(ChunkCoord::new(0, 0, 0), 1);
        let mut table = ResidentChunkTable::new(layout);
        let initial = table.sync_visible_snapshots([&left, &right]);
        assert_eq!(
            initial.iter().map(|update| update.slot).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(table.sync_visible_snapshots([&left, &right]).is_empty());
        assert_eq!(table.lookup_entries()[layout.index(left.coord).unwrap()], 1);

        table.recenter(ChunkTableLayout::new(ChunkCoord::new(0, -1, -2), [2, 2, 4]));
        let replacement = snapshot(ChunkCoord::new(1, 0, 0), 1);
        let update = table.sync_visible_snapshots([&right, &replacement]);
        assert_eq!(
            update.iter().map(|item| item.slot).collect::<Vec<_>>(),
            vec![0]
        );
    }

    #[test]
    fn camera_uniform_has_wgsl_uniform_alignment_and_cpu_camera_basis() {
        assert_eq!(std::mem::size_of::<CameraUniform>(), 128);
        assert_eq!(std::mem::align_of::<CameraUniform>(), 4);
        let camera = Camera::new(Vec3::new(3.0, 4.0, 5.0))
            .looking_at(0.4, -0.2)
            .with_max_distance(80.0);
        let uniform = CameraUniform::from_camera(
            camera,
            VoxelBounds::new(VoxelCoord::new(-4, -2, 8)),
            ChunkTableLayout::new(ChunkCoord::new(-1, -1, -1), [3, 3, 3]),
            4096,
        );
        assert_eq!(uniform.position_and_max_distance, [3.0, 4.0, 5.0, 80.0]);
        assert_eq!(
            &uniform.forward_and_aspect[..3],
            &[camera.forward().x, camera.forward().y, camera.forward().z]
        );
        assert_eq!(uniform.table_dimensions_and_padding[..3], [3, 3, 3]);
        assert_eq!(CameraUniform::BYTE_SIZE, 128);
    }

    #[test]
    fn dda_wgsl_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(DDA_SHADER).expect("DDA WGSL must parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("DDA WGSL must validate");
    }

    #[test]
    fn glyph_wgsl_parses_and_validates() {
        let module =
            naga::front::wgsl::parse_str(GLYPH_SHADER).expect("glyph compositor WGSL must parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("glyph compositor WGSL must validate");
    }

    #[test]
    fn adapter_backed_final_compositor_preserves_authored_srgb_bytes() {
        let Some(gpu) = OffscreenTerrainReadback::new() else {
            eprintln!("skipping GPU presentation parity fixture: no adapter is available");
            return;
        };
        let srgb = gpu.render_presentation(wgpu::TextureFormat::Rgba8UnormSrgb);
        let fallback = gpu.render_presentation(wgpu::TextureFormat::Rgba8Unorm);
        let ink = glyph_ink_pixel('A');
        let blank = glyph_blank_pixel('A');
        let checks = [
            // Terrain material colour and deterministic sky colour pass through
            // the sRGB logical target and final glyph compositor.
            (
                3 * 8 + glyph_ink_pixel('#').0,
                3 * 8 + glyph_ink_pixel('#').1,
                [0xa8, 0x86, 0x62, 255],
                "terrain material",
            ),
            (
                5 * 8 + glyph_ink_pixel('.').0,
                3 * 8 + glyph_ink_pixel('.').1,
                [0x50, 0x58, 0x66, 255],
                "sky",
            ),
            // Styled scene UI preserves foreground and opaque background.
            (
                11 * 8 + ink.0,
                10 * 8 + ink.1,
                [0xf0, 0xc6, 0x5b, 255],
                "scene UI foreground",
            ),
            (
                11 * 8 + blank.0,
                10 * 8 + blank.1,
                [0x0c, 0x22, 0x38, 255],
                "scene UI background",
            ),
            // Final overlay is painter-ordered after UI and sprites.
            (
                10 * 8 + ink.0,
                10 * 8 + ink.1,
                [0xc8, 0xdc, 0xf0, 255],
                "overlay text foreground",
            ),
            (
                10 * 8 + blank.0,
                10 * 8 + blank.1,
                [0x1a, 0x24, 0x36, 255],
                "overlay text background",
            ),
            (200, 200, [0xff, 0x50, 0x78, 255], "pixel sprite foreground"),
            (201, 200, [0x0a, 0x14, 0x1e, 255], "pixel sprite background"),
        ];
        for (x, y, expected, label) in checks {
            let presented = presentation_pixel(&srgb, x, y);
            let fallback_presented = presentation_pixel(&fallback, x, y);
            assert_rgba_near(presented, expected, label);
            assert_rgba_near(fallback_presented, expected, label);
            assert_rgba_near(
                fallback_presented,
                presented,
                "sRGB and non-sRGB presentation equivalence",
            );
        }
    }

    fn glyph_ink_pixel(glyph: char) -> (u32, u32) {
        let rows = BASIC_FONTS
            .get(glyph)
            .expect("test glyph is present in font atlas");
        for (y, row) in rows.iter().enumerate() {
            for x in 0..8 {
                if row & (1 << x) != 0 {
                    return (x, y as u32);
                }
            }
        }
        panic!("test glyph must contain ink");
    }

    fn glyph_blank_pixel(glyph: char) -> (u32, u32) {
        let rows = BASIC_FONTS
            .get(glyph)
            .expect("test glyph is present in font atlas");
        for (y, row) in rows.iter().enumerate() {
            for x in 0..8 {
                if row & (1 << x) == 0 {
                    return (x, y as u32);
                }
            }
        }
        panic!("test glyph must contain a blank pixel");
    }

    fn presentation_pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
        let index = ((y * LOGICAL_WIDTH * 8 + x) * 4) as usize;
        bytes[index..index + 4].try_into().expect("RGBA pixel")
    }

    fn assert_rgba_near(actual: [u8; 4], expected: [u8; 4], label: &str) {
        for channel in 0..4 {
            assert!(
                (actual[channel] as i16 - expected[channel] as i16).abs() <= 1,
                "{label}: expected {expected:?}, received {actual:?}"
            );
        }
    }

    #[test]
    fn adapter_backed_logical_terrain_matches_cpu_reference_fixtures() {
        let Some(gpu) = OffscreenTerrainReadback::new() else {
            eprintln!("skipping GPU parity fixtures: no adapter is available");
            return;
        };
        for (name, world, camera) in parity_fixtures() {
            let actual = gpu.render(&world, camera, &[], &[], &[]);
            let materials = MaterialGlyphMap;
            for y in 0..LOGICAL_HEIGHT as usize {
                for x in 0..LOGICAL_WIDTH as usize {
                    let index = y * LOGICAL_WIDTH as usize + x;
                    let expected = raycast(
                        &world,
                        camera.ray_for_cell(x, y, LOGICAL_WIDTH as usize, LOGICAL_HEIGHT as usize),
                        camera.max_distance,
                    );
                    let (expected_glyph, expected_colour) = if let Some(hit) = expected {
                        (
                            materials.glyph_for(hit) as u32,
                            hex_colour(
                                materials
                                    .style_for(hit)
                                    .fg
                                    .as_deref()
                                    .expect("voxel colour"),
                            ),
                        )
                    } else {
                        (
                            background_glyph_for_direction(
                                camera
                                    .ray_for_cell(
                                        x,
                                        y,
                                        LOGICAL_WIDTH as usize,
                                        LOGICAL_HEIGHT as usize,
                                    )
                                    .direction,
                            ) as u32,
                            [0x50, 0x58, 0x66, 255],
                        )
                    };
                    assert_eq!(
                        actual[index].0, expected_glyph,
                        "{name}: glyph mismatch at logical cell {x},{y}; CPU hit: {expected:?}"
                    );
                    for channel in 0..4 {
                        assert!(
                            (actual[index].1[channel] as i16 - expected_colour[channel] as i16).abs() <= 1,
                            "{name}: colour mismatch at logical cell {x},{y}, channel {channel}: GPU {:?}, CPU {:?}",
                            actual[index].1,
                            expected_colour,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn adapter_backed_dynamic_and_asset_terrain_matches_cpu_reference() {
        let Some(gpu) = OffscreenTerrainReadback::new() else {
            eprintln!("skipping GPU dynamic/asset parity fixtures: no adapter is available");
            return;
        };

        let mut static_world = VoxelWorld::new();
        fill(
            &mut static_world,
            VoxelCoord::new(-6, 0, 8),
            VoxelCoord::new(6, 0, 18),
            VoxelMaterial::Stone,
        );
        static_world.set(
            VoxelCoord::new(0, 2, 11),
            VoxelCell::new(VoxelMaterial::Gate),
        );
        static_world.set(
            VoxelCoord::new(2, 2, 11),
            VoxelCell::new(VoxelMaterial::PuzzleDoor),
        );
        static_world.set(
            VoxelCoord::new(-2, 2, 11),
            VoxelCell::new(VoxelMaterial::Gate),
        );

        // This is the same authoritative post-simulation world that the CPU
        // fallback renders: a replacement, an opened door, and geometry past
        // the original static edge.
        let dynamic = [
            DynamicVoxel {
                x: 0,
                y: 2,
                z: 11,
                material: encode_voxel(VoxelMaterial::Receiver),
            },
            DynamicVoxel {
                x: 2,
                y: 2,
                z: 11,
                material: 0,
            },
            DynamicVoxel {
                x: 0,
                y: 2,
                z: 11,
                material: encode_voxel(VoxelMaterial::Beacon),
            },
            DynamicVoxel {
                x: 0,
                y: 3,
                z: 24,
                material: encode_voxel(VoxelMaterial::CarbonLife),
            },
        ];
        let mut assembled = static_world.clone();
        assembled.set(
            VoxelCoord::new(0, 2, 11),
            VoxelCell::new(VoxelMaterial::Beacon),
        );
        assembled.clear(VoxelCoord::new(2, 2, 11));
        assembled.set(
            VoxelCoord::new(0, 3, 24),
            VoxelCell::new(VoxelMaterial::CarbonLife),
        );

        let mut cpu_assets = Vec::new();
        let mut gpu_assets = Vec::new();
        let mut gpu_asset_voxels = Vec::new();
        // Exact ties are intentional: static terrain wins at -2, while the
        // later dynamic entry wins the same static coordinate at 0.
        for x in [-2.0, 0.0] {
            let anchor = Vec3::new(x, 2.0, 11.0);
            let mut voxels = std::collections::HashMap::new();
            voxels.insert(
                VoxelCoord::new(0, 0, 0),
                VoxelMaterial::Custom([240, 60, 80]),
            );
            cpu_assets.push(CpuRenderAsset {
                min: anchor,
                max: anchor + Vec3::new(1.0, 1.0, 1.0),
                voxels,
                dimensions: [1, 1, 1],
                voxel_size: 1.0,
                pivot: [0.0; 3],
                anchor,
                yaw_degrees: 0,
                ghost: false,
            });
            let offset = gpu_asset_voxels.len() as u32;
            gpu_asset_voxels.push(AssetVoxel {
                x: 0,
                y: 0,
                z: 0,
                material: encode_voxel(VoxelMaterial::Custom([240, 60, 80])),
            });
            gpu_assets.push(RenderAsset {
                min: [anchor.x, anchor.y, anchor.z, 0.0],
                max: [anchor.x + 1.0, anchor.y + 1.0, anchor.z + 1.0, 0.0],
                anchor: [anchor.x, anchor.y, anchor.z, 0.0],
                voxel_size: 1.0,
                yaw_degrees: 0.0,
                ghost: 0,
                voxel_offset: offset,
                dimensions: [1, 1, 1, 1],
                pivot: [0.0; 4],
            });
        }
        for (turn, yaw) in [0_u16, 90, 180, 270].into_iter().enumerate() {
            let anchor = Vec3::new(-3.0 + turn as f32 * 2.0, 1.0, 14.0 + turn as f32);
            let mut voxels = std::collections::HashMap::new();
            voxels.insert(
                VoxelCoord::new(0, 0, 0),
                VoxelMaterial::Custom([20, 180, 240]),
            );
            voxels.insert(VoxelCoord::new(1, 1, 1), VoxelMaterial::Beacon);
            cpu_assets.push(CpuRenderAsset {
                min: anchor - Vec3::new(0.5, 0.0, 0.5),
                max: anchor + Vec3::new(0.5, 1.0, 0.5),
                voxels,
                dimensions: [2, 2, 2],
                voxel_size: 0.5,
                pivot: [1.0, 0.0, 1.0],
                anchor,
                yaw_degrees: yaw,
                ghost: turn % 2 == 1,
            });
            let offset = gpu_asset_voxels.len() as u32;
            gpu_asset_voxels.extend([
                AssetVoxel {
                    x: 0,
                    y: 0,
                    z: 0,
                    material: encode_voxel(VoxelMaterial::Custom([20, 180, 240])),
                },
                AssetVoxel {
                    x: 1,
                    y: 1,
                    z: 1,
                    material: encode_voxel(VoxelMaterial::Beacon),
                },
            ]);
            gpu_assets.push(RenderAsset {
                min: [anchor.x - 0.5, anchor.y, anchor.z - 0.5, 0.0],
                max: [anchor.x + 0.5, anchor.y + 1.0, anchor.z + 0.5, 0.0],
                anchor: [anchor.x, anchor.y, anchor.z, 0.0],
                voxel_size: 0.5,
                yaw_degrees: yaw as f32,
                ghost: (turn % 2 == 1) as u32,
                voxel_offset: offset,
                dimensions: [2, 2, 2, 2],
                pivot: [1.0, 0.0, 1.0, 0.0],
            });
        }
        assert_gpu_frame_matches_cpu(
            &gpu,
            "dynamic replacement/removal and transformed assets",
            &static_world,
            &assembled,
            &cpu_assets,
            parity_camera(Vec3::new(0.0, 2.5, -4.0)),
            &dynamic,
            &gpu_assets,
            &gpu_asset_voxels,
        );
    }

    fn assert_gpu_frame_matches_cpu(
        gpu: &OffscreenTerrainReadback,
        name: &str,
        static_world: &VoxelWorld,
        cpu_world: &VoxelWorld,
        cpu_assets: &[CpuRenderAsset],
        camera: Camera,
        dynamic_voxels: &[DynamicVoxel],
        assets: &[RenderAsset],
        asset_voxels: &[AssetVoxel],
    ) {
        let actual = gpu.render(static_world, camera, dynamic_voxels, assets, asset_voxels);
        let builder = SceneBuilder::new(
            GraphicsConfig {
                viewport: Viewport {
                    width: LOGICAL_WIDTH as usize,
                    height: LOGICAL_HEIGHT as usize,
                },
                max_distance: camera.max_distance,
            },
            MaterialGlyphMap,
        );
        let scene = builder.build_with_render_assets(cpu_world, cpu_assets, &camera, 0);
        let mut expected = (0..LOGICAL_HEIGHT as usize)
            .flat_map(|y| {
                (0..LOGICAL_WIDTH as usize).map(move |x| {
                    (
                        background_glyph_for_direction(
                            camera
                                .ray_for_cell(x, y, LOGICAL_WIDTH as usize, LOGICAL_HEIGHT as usize)
                                .direction,
                        ) as u32,
                        [0x50, 0x58, 0x66, 255],
                    )
                })
            })
            .collect::<Vec<_>>();
        let terrain = scene
            .layers
            .iter()
            .find(|layer| layer.name == "voxels")
            .expect("CPU reference terrain layer");
        for cell in &terrain.cells {
            let index = cell.y as usize * LOGICAL_WIDTH as usize + cell.x as usize;
            expected[index] = (
                cell.glyph as u32,
                hex_colour(cell.style.fg.as_deref().expect("voxel colour")),
            );
        }
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.0, expected.0,
                "{name}: glyph mismatch at cell {index}"
            );
            for channel in 0..4 {
                assert!(
                    (actual.1[channel] as i16 - expected.1[channel] as i16).abs() <= 1,
                    "{name}: colour mismatch at cell {index}, channel {channel}: GPU {:?}, CPU {:?}",
                    actual.1,
                    expected.1,
                );
            }
        }
    }

    fn hex_colour(value: &str) -> [u8; 4] {
        let channel =
            |offset| u8::from_str_radix(&value[offset..offset + 2], 16).expect("hex style");
        [channel(1), channel(3), channel(5), 255]
    }

    fn parity_camera(position: Vec3) -> Camera {
        Camera::new(position).with_max_distance(128.0)
    }

    fn fill(world: &mut VoxelWorld, min: VoxelCoord, max: VoxelCoord, material: VoxelMaterial) {
        for z in min.z..=max.z {
            for y in min.y..=max.y {
                for x in min.x..=max.x {
                    world.set(VoxelCoord::new(x, y, z), VoxelCell::new(material));
                }
            }
        }
    }

    fn parity_fixtures() -> Vec<(&'static str, VoxelWorld, Camera)> {
        let mut enclosed_room = VoxelWorld::new();
        fill(
            &mut enclosed_room,
            VoxelCoord::new(-8, 0, 0),
            VoxelCoord::new(8, 0, 16),
            VoxelMaterial::Stone,
        );
        fill(
            &mut enclosed_room,
            VoxelCoord::new(-8, 1, 16),
            VoxelCoord::new(8, 6, 16),
            VoxelMaterial::Habitat,
        );
        fill(
            &mut enclosed_room,
            VoxelCoord::new(-8, 1, 0),
            VoxelCoord::new(-8, 6, 16),
            VoxelMaterial::Habitat,
        );
        fill(
            &mut enclosed_room,
            VoxelCoord::new(8, 1, 0),
            VoxelCoord::new(8, 6, 16),
            VoxelMaterial::Habitat,
        );

        let mut pillars_and_corners = VoxelWorld::new();
        fill(
            &mut pillars_and_corners,
            VoxelCoord::new(-12, 0, 0),
            VoxelCoord::new(12, 0, 24),
            VoxelMaterial::Dirt,
        );
        for &(x, z, material) in &[
            (-5, 6, VoxelMaterial::Wood),
            (4, 9, VoxelMaterial::Stone),
            (7, 16, VoxelMaterial::Basalt),
        ] {
            fill(
                &mut pillars_and_corners,
                VoxelCoord::new(x, 1, z),
                VoxelCoord::new(x + 1, 7, z + 1),
                material,
            );
        }

        let mut stairs = VoxelWorld::new();
        fill(
            &mut stairs,
            VoxelCoord::new(-10, 0, 0),
            VoxelCoord::new(10, 0, 24),
            VoxelMaterial::Grass,
        );
        for step in 0..8 {
            fill(
                &mut stairs,
                VoxelCoord::new(-3, 1, 5 + step),
                VoxelCoord::new(3, 1 + step, 5 + step),
                VoxelMaterial::ShipHull,
            );
        }

        let mut corridor = VoxelWorld::new();
        fill(
            &mut corridor,
            VoxelCoord::new(-3, 0, 0),
            VoxelCoord::new(3, 0, 40),
            VoxelMaterial::Stone,
        );
        fill(
            &mut corridor,
            VoxelCoord::new(-3, 1, 0),
            VoxelCoord::new(-3, 5, 40),
            VoxelMaterial::Habitat,
        );
        fill(
            &mut corridor,
            VoxelCoord::new(3, 1, 0),
            VoxelCoord::new(3, 5, 40),
            VoxelMaterial::Habitat,
        );
        fill(
            &mut corridor,
            VoxelCoord::new(-3, 1, 40),
            VoxelCoord::new(3, 5, 40),
            VoxelMaterial::PuzzleDoor,
        );

        let mut open_view = VoxelWorld::new();
        fill(
            &mut open_view,
            VoxelCoord::new(-40, -1, -8),
            VoxelCoord::new(40, -1, 48),
            VoxelMaterial::Sand,
        );
        fill(
            &mut open_view,
            VoxelCoord::new(-2, 0, 22),
            VoxelCoord::new(2, 4, 24),
            VoxelMaterial::Beacon,
        );

        let mut chunk_boundaries = VoxelWorld::new();
        fill(
            &mut chunk_boundaries,
            VoxelCoord::new(14, 0, 14),
            VoxelCoord::new(18, 5, 18),
            VoxelMaterial::Glass,
        );
        fill(
            &mut chunk_boundaries,
            VoxelCoord::new(-18, 0, 22),
            VoxelCoord::new(-14, 5, 26),
            VoxelMaterial::Gate,
        );

        let mut negative_coordinates = VoxelWorld::new();
        fill(
            &mut negative_coordinates,
            VoxelCoord::new(-40, 0, -12),
            VoxelCoord::new(-1, 0, 24),
            VoxelMaterial::Basalt,
        );
        fill(
            &mut negative_coordinates,
            VoxelCoord::new(-8, 1, 8),
            VoxelCoord::new(-4, 6, 12),
            VoxelMaterial::CarbonLife,
        );

        let mut dense_world = VoxelWorld::new();
        fill(
            &mut dense_world,
            VoxelCoord::new(-16, 0, 0),
            VoxelCoord::new(16, 12, 32),
            VoxelMaterial::Stone,
        );
        fill(
            &mut dense_world,
            VoxelCoord::new(-6, 1, 1),
            VoxelCoord::new(6, 10, 20),
            VoxelMaterial::Glass,
        );
        fill(
            &mut dense_world,
            VoxelCoord::new(-5, 2, 2),
            VoxelCoord::new(5, 9, 19),
            VoxelMaterial::Ocean,
        );

        let mut edited_world = VoxelWorld::new();
        fill(
            &mut edited_world,
            VoxelCoord::new(-10, 0, 0),
            VoxelCoord::new(10, 0, 28),
            VoxelMaterial::Stone,
        );
        fill(
            &mut edited_world,
            VoxelCoord::new(-4, 1, 10),
            VoxelCoord::new(4, 6, 10),
            VoxelMaterial::Habitat,
        );
        for y in 2..=4 {
            edited_world.clear(VoxelCoord::new(0, y, 10));
        }
        edited_world.set(
            VoxelCoord::new(0, 3, 10),
            VoxelCell::new(VoxelMaterial::Receiver),
        );
        edited_world.set(
            VoxelCoord::new(1, 3, 10),
            VoxelCell::new(VoxelMaterial::SignalPipe),
        );

        vec![
            (
                "enclosed room",
                enclosed_room,
                parity_camera(Vec3::new(0.5, 2.5, -4.0)),
            ),
            (
                "pillars and corners",
                pillars_and_corners,
                parity_camera(Vec3::new(0.5, 2.5, -5.0)),
            ),
            ("stairs", stairs, parity_camera(Vec3::new(0.5, 2.5, -5.0))),
            (
                "corridor",
                corridor,
                parity_camera(Vec3::new(0.0, 2.5, -4.0)),
            ),
            (
                "open view",
                open_view,
                parity_camera(Vec3::new(0.0, 2.0, -4.0)),
            ),
            (
                "chunk boundaries",
                chunk_boundaries,
                parity_camera(Vec3::new(0.0, 2.5, -5.0)),
            ),
            (
                "negative coordinates",
                negative_coordinates,
                parity_camera(Vec3::new(-12.0, 2.5, -5.0)),
            ),
            (
                "dense world",
                dense_world,
                parity_camera(Vec3::new(0.0, 6.0, -5.0)),
            ),
            (
                "edited world",
                edited_world,
                parity_camera(Vec3::new(0.0, 2.5, -5.0)),
            ),
        ]
    }
}
