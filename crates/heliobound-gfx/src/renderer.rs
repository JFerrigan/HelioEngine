use crate::scene::Scene;

pub trait Renderer {
    fn render(&self, scene: &Scene) -> String;
}

#[derive(Clone, Debug, Default)]
pub struct AsciiRenderer;

impl Renderer for AsciiRenderer {
    fn render(&self, scene: &Scene) -> String {
        let mut scene = scene.clone();
        scene.sort_layers();

        let mut canvas = Canvas::new(scene.viewport.width, scene.viewport.height);

        for layer in &scene.layers {
            for cell in &layer.cells {
                canvas.set(cell.x, cell.y, cell.glyph);
            }
        }

        for overlay in &scene.overlays {
            canvas.write_text(overlay.x, overlay.y, &overlay.text);
        }

        canvas.finish()
    }
}

struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<char>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![' '; width * height],
        }
    }

    fn set(&mut self, x: i32, y: i32, glyph: char) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return;
        }
        self.cells[y * self.width + x] = glyph;
    }

    fn write_text(&mut self, x: i32, y: i32, text: &str) {
        for (offset, glyph) in text.chars().enumerate() {
            self.set(x + offset as i32, y, glyph);
        }
    }

    fn finish(&self) -> String {
        let mut out = String::with_capacity(self.width * self.height + self.height);
        for y in 0..self.height {
            let start = y * self.width;
            let end = start + self.width;
            let line: String = self.cells[start..end].iter().collect();
            out.push_str(&line);
            if y + 1 < self.height {
                out.push('\n');
            }
        }
        out
    }
}
