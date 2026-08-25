use std::cmp::Ordering;

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
    pub overlays: Vec<Overlay>,
}

impl Scene {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            layers: Vec::new(),
            overlays: Vec::new(),
        }
    }

    pub fn sort_layers(&mut self) {
        self.layers.sort_by(|a, b| a.z.cmp(&b.z));
        self.overlays.sort_by(|a, b| match a.z.cmp(&b.z) {
            Ordering::Equal => a.y.cmp(&b.y).then(a.x.cmp(&b.x)),
            other => other,
        });
    }
}
