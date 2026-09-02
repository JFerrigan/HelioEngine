use heliobound_core::{Vec3, VoxelCoord, VoxelMaterial};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextStyle {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug)]
pub struct SceneCell {
    pub x: i32,
    pub y: i32,
    pub glyph: char,
    pub style: TextStyle,
}

#[derive(Clone, Debug)]
pub struct Overlay {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub text: String,
    pub style: TextStyle,
}

/// A small indexed pixel-art sprite rendered in physical framebuffer pixels.
///
/// Unlike terminal overlays, a sprite is not scaled by the ASCII cell size.
/// This keeps compact editor affordances crisp while the rest of the scene
/// remains character-based.
#[derive(Clone, Debug)]
pub struct PixelSprite {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// One bit per pixel, most-significant bit first, for each of 16 rows.
    pub rows: [u16; 16],
    /// Physical framebuffer pixels represented by one sprite pixel.
    pub scale: u8,
    pub fg: String,
    pub bg: Option<String>,
}

/// One placed imported asset. The authored sparse grid remains local; rays
/// transform into that space and traverse only the cells they cross.
#[derive(Clone, Debug)]
pub struct RenderAsset {
    pub min: Vec3,
    pub max: Vec3,
    pub voxels: HashMap<VoxelCoord, VoxelMaterial>,
    pub dimensions: [i32; 3],
    pub voxel_size: f32,
    pub pivot: [f32; 3],
    pub anchor: Vec3,
    pub yaw_degrees: u16,
    /// Placement previews are deliberately dimmed while retaining palette.
    pub ghost: bool,
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub z: i32,
    pub cells: Vec<SceneCell>,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub viewport: Viewport,
    pub layers: Vec<Layer>,
    pub pixel_sprites: Vec<PixelSprite>,
    pub overlays: Vec<Overlay>,
}

impl Scene {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            layers: Vec::new(),
            pixel_sprites: Vec::new(),
            overlays: Vec::new(),
        }
    }

    pub fn sort_layers(&mut self) {
        self.layers.sort_by(|a, b| a.z.cmp(&b.z));
        self.pixel_sprites.sort_by(|a, b| a.z.cmp(&b.z));
        self.overlays.sort_by(|a, b| match a.z.cmp(&b.z) {
            Ordering::Equal => a.y.cmp(&b.y).then(a.x.cmp(&b.x)),
            other => other,
        });
    }
}
