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
    glyph_atlas: wgpu::TextureView,
    presentation: wgpu::Buffer,
}

struct TerrainGpuBuffers {
    camera: wgpu::Buffer,
    lookup: wgpu::Buffer,
    voxels: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    lookup_capacity: u64,
    voxel_capacity: u64,
}

struct LogicalTargets {
    glyphs: wgpu::TextureView,
    colours: wgpu::TextureView,
}

struct UiGpuBuffer {
    cells: wgpu::Buffer,
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
}

/// Per-frame terrain-cache synchronization input. `lookup` is the complete
/// bounded chunk-coordinate table; `updates` contains only newly resident or
/// revision-changed slots. `slot_count` is the high-water slot count from
/// [`ResidentChunkTable::slot_count`], not the resident chunk count.
pub struct TerrainFrame<'a> {
    pub camera: CameraUniform,
    pub lookup: &'a [u32],
    pub slot_count: u32,
    pub updates: &'a [ChunkSlotUpload],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainUploadStats {
    pub dirty_chunks: u32,
    pub bytes_uploaded: u64,
    pub voxel_capacity_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainDataError {
    EmptyLookup,
    InvalidChunkLength { slot: u32, actual: usize },
    SlotOutsideCapacity { slot: u32, slot_count: u32 },
}

impl fmt::Display for TerrainDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLookup => write!(f, "terrain chunk lookup must contain at least one entry"),
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
        let ui_pipeline = create_ui_pipeline(&device, format, &ui_bind_group_layout);

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
        );
        self.queue
            .write_buffer(&self.terrain.camera, 0, bytemuck::bytes_of(&frame.camera));
        self.queue
            .write_buffer(&self.terrain.lookup, 0, bytemuck::cast_slice(frame.lookup));
        let mut bytes_uploaded =
            CameraUniform::BYTE_SIZE + (frame.lookup.len() * std::mem::size_of::<u32>()) as u64;
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
        Ok(TerrainUploadStats {
            dirty_chunks: frame.updates.len() as u32,
            bytes_uploaded,
            voxel_capacity_bytes: self.terrain.voxel_capacity,
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
            bytemuck::bytes_of(&presentation_uniform(self.size)),
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
        let voxels = create_storage_buffer(
            device,
            "heliobound terrain chunk slots",
            CHUNK_VOXELS as u64,
        );
        let bind_group = create_terrain_bind_group(device, layout, &camera, &lookup, &voxels);
        Self {
            camera,
            lookup,
            voxels,
            bind_group,
            lookup_capacity: 1,
            voxel_capacity: CHUNK_VOXELS as u64 * 4,
        }
    }

    fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        lookup_values: u64,
        voxel_values: u64,
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
        if changed {
            self.bind_group =
                create_terrain_bind_group(device, layout, &self.camera, &self.lookup, &self.voxels);
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
                    min_binding_size: wgpu::BufferSize::new(4),
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
    voxels: &wgpu::Buffer,
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
                resource: voxels.as_entire_binding(),
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
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
            device
                .create_texture(&wgpu::TextureDescriptor {
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
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        Self {
            glyphs: create("heliobound logical glyph IDs", wgpu::TextureFormat::R32Uint),
            colours: create(
                "heliobound logical glyph colours",
                wgpu::TextureFormat::Rgba8Unorm,
            ),
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

fn presentation_uniform(size: PhysicalSize<u32>) -> PresentationUniform {
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
    use heliobound_core::{Vec3, VoxelCell, VoxelCoord};

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
}
